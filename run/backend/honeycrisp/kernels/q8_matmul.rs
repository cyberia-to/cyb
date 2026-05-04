//! Metal canonical Q8 fused dequant+matmul.
//!
//! Block layout (34 bytes, 32 values):
//!   i16 scale (8.8 fixed-point)   — bytes 0..2
//!   i8  qs[32]                    — bytes 2..34
//! Dequant: x[i] = qs[i] * (scale_i16 / 256) / 127
//!
//! Output: y[b, row] = sum_k x[b, k] * dequant(W[row, k])
//!
//! Parallelism: one SIMD group (32 threads) computes one (batch, row).
//! Each thread inside the group walks a stride of 32 over the K blocks,
//! then a simd_sum reduces across the group. Threadgroup size: SIMDS_PER_GROUP
//! SIMD groups × 32 threads = SIMDS_PER_GROUP rows per threadgroup launch.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

pub const BLOCK_SIZE: usize = 32;
pub const BLOCK_BYTES: usize = 34;
pub const SIMDS_PER_GROUP: u32 = 8;

pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 8;
constant constexpr uint LANES = 32;
// Max K = SIMDS_PER_GROUP rows × LANES lanes worth — but actually we need
// MAX_BLOCKS for x cache. With K up to 4096 (4096/32 = 128 blocks) we could
// hold all in shared memory. Cap at 128 blocks → 4096 floats × 4B = 16 KB.
constant constexpr uint MAX_BLOCKS = 128;

// SIMDS_PER_GROUP SIMD groups per threadgroup, each computes one row.
// All SIMD groups share a threadgroup-local copy of the x activation, loaded
// cooperatively at the start. Per row each SIMD's 32 threads stride over
// blocks and reduce via simd_sum.
kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const uchar   *w      [[buffer(1)]],
    device       float   *y      [[buffer(2)]],
    constant     Dims    &dims   [[buffer(3)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];
    constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    // Cooperatively load x_b into threadgroup memory.
    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * 32u + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * dims.n_blocks * 32u;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *w_row = w + row * dims.n_blocks * 34u;

    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += 32u) {
        device const uchar *block = w_row + blk * 34u;
        // i16 scale: 2-byte aligned read.
        device const short *scale_p = (device const short *)(block);
        int scale_i = int(*scale_p);
        float scale = float(scale_i) * (1.0f / 256.0f / 127.0f);

        threadgroup const float *xb = &x_shared[blk * 32u];
        // qs starts at byte offset 2 within block — 2-byte aligned.
        // Read as char2 (16 reads of 2 bytes each = 32 i8 values).
        device const char2 *qs2 = (device const char2 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        // 16 char2 → 8 char4-equivalent → process 4 at a time as float4 fmas.
        for (uint c = 0; c < 8u; c++) {
            char2 a = qs2[c * 2u];
            char2 b2 = qs2[c * 2u + 1u];
            float4 q = float4(float(a.x), float(a.y), float(b2.x), float(b2.y));
            float4 xv = float4(xb[c*4u], xb[c*4u + 1u], xb[c*4u + 2u], xb[c*4u + 3u]);
            acc4 = fma(q, xv, acc4);
        }
        float acc = acc4.x + acc4.y + acc4.z + acc4.w;
        my_sum = fma(scale, acc, my_sum);
    }
    float reduced = simd_sum(my_sum);
    if (lane == 0) {
        y[b * dims.n_rows + row] = reduced;
    }
}
"#;

pub fn dispatch(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    x: &aruminium::Buffer,
    w: &aruminium::Buffer,
    batch: u32,
    n_rows: u32,
    n_blocks: u32,
) -> Result<aruminium::Buffer, BackendError> {
    let total_rows = batch * n_rows;
    let out = dev.alloc((total_rows * 4) as usize)?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
    let dims = Dims { batch, n_rows, n_blocks, pad: 0 };

    // Launch: total_rows SIMD groups packed SIMDS_PER_GROUP per threadgroup.
    // Threads per threadgroup: SIMDS_PER_GROUP * 32.
    let groups_x = (total_rows + SIMDS_PER_GROUP - 1) / SIMDS_PER_GROUP;
    let threads_per_group = SIMDS_PER_GROUP * 32;

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|enc| {
                enc.bind(pipeline);
                enc.bind_buffer(x, 0, 0);
                enc.bind_buffer(w, 0, 1);
                enc.bind_buffer(&out, 0, 2);
                let bytes = std::slice::from_raw_parts(
                    &dims as *const Dims as *const u8,
                    std::mem::size_of::<Dims>(),
                );
                enc.push(bytes, 3);
                enc.launch_groups(
                    (groups_x as usize, 1, 1),
                    (threads_per_group as usize, 1, 1),
                );
            });
        });
    }
    Ok(out)
}
