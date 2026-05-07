//! Metal canonical Q4 fused dequant+matmul.
//!
//! Block layout (18 bytes, 32 values):
//!   i16 scale (8.8 fixed-point)   — bytes 0..2
//!   16 packed nibbles             — bytes 2..18
//! Split nibble layout:
//!   byte i holds value[i] (lo nibble) and value[i+16] (hi nibble)
//! Dequant: x[i] = (nibble[i] - 8) * scale_f / 8
//!
//! One SIMD group (32 threads) computes one output row (batch, row).
//! Each thread walks stride-32 over K blocks, simd_sum reduces.
//! SIMDS_PER_GROUP=8 → 8 rows per threadgroup, 256 threads total.
//! Inner loop uses float4 vectorised FMA for higher ALU throughput.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

pub const BLOCK_SIZE: usize = 32;
pub const BLOCK_BYTES: usize = 18;
pub const SIMDS_PER_GROUP: u32 = 8;
pub const N_DST: u32 = 1;

pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 8;
constant constexpr uint LANES  = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 256
constant constexpr uint MAX_BLOCKS = 128;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const uchar   *w      [[buffer(1)]],
    device       float   *y      [[buffer(2)]],
    constant     Dims    &dims   [[buffer(3)]],
    uint                 tgpig_x [[threadgroup_position_in_grid]],
    uint                 lane    [[thread_index_in_simdgroup]],
    uint                 sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *w_row = w + row * dims.n_blocks * 18u;
    float my_sum = 0.0f;

    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 18u;
        device const short *sp = (device const short *)(block);
        float scale = float(int(*sp)) * (1.0f / 2048.0f);

        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);
        device const uchar4 *nb4 = (device const uchar4 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        for (uint s = 0; s < 4u; s++) {
            uchar4 b4 = nb4[s];
            float4 lo = float4(
                float(int(b4.x & 0x0Fu) - 8),
                float(int(b4.y & 0x0Fu) - 8),
                float(int(b4.z & 0x0Fu) - 8),
                float(int(b4.w & 0x0Fu) - 8)
            );
            float4 hi = float4(
                float(int((b4.x >> 4) & 0x0Fu) - 8),
                float(int((b4.y >> 4) & 0x0Fu) - 8),
                float(int((b4.z >> 4) & 0x0Fu) - 8),
                float(int((b4.w >> 4) & 0x0Fu) - 8)
            );
            acc4 += lo * xb4[s];
            acc4 += hi * xb4[s + 4u];
        }
        my_sum = fma(scale, acc4.x + acc4.y + acc4.z + acc4.w, my_sum);
    }

    float total = simd_sum(my_sum);
    if (lane == 0) {
        y[b * dims.n_rows + row] = total;
    }
}
"#;

/// Fused Q/K/V: single dispatch for all three projections.
/// Dims: { q_rows, kv_rows, n_blocks, pad }. Total = q_rows + 2*kv_rows rows.
/// Rows 0..q_rows → Q; q_rows..q_rows+kv_rows → K; rest → V.
/// Buffers: 0=x, 1=wq, 2=wk, 3=wv, 4=yq, 5=yk, 6=yv, 7=dims.
pub const MSL_QKV: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct DimsQKV { uint q_rows; uint kv_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 8;
constant constexpr uint LANES  = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;
constant constexpr uint MAX_BLOCKS = 128;

kernel void kmain(
    device const float   *x    [[buffer(0)]],
    device const uchar   *wq   [[buffer(1)]],
    device const uchar   *wk   [[buffer(2)]],
    device const uchar   *wv   [[buffer(3)]],
    device       float   *yq   [[buffer(4)]],
    device       float   *yk   [[buffer(5)]],
    device       float   *yv   [[buffer(6)]],
    constant     DimsQKV &dims [[buffer(7)]],
    uint                 tgpig_x [[threadgroup_position_in_grid]],
    uint                 lane    [[thread_index_in_simdgroup]],
    uint                 sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint total_rows  = dims.q_rows + 2u * dims.kv_rows;
    uint row_global  = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint row_local   = row_global % total_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    for (uint i = tid; i < k_floats; i += TGSIZE) {
        x_shared[i] = x[i];
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (row_global >= total_rows) return;

    // Route: Q, K, or V?
    device const uchar *w_row;
    device       float *y_out;
    uint out_row;
    if (row_local < dims.q_rows) {
        w_row   = wq + row_local * dims.n_blocks * 18u;
        y_out   = yq;
        out_row = row_local;
    } else if (row_local < dims.q_rows + dims.kv_rows) {
        uint kr = row_local - dims.q_rows;
        w_row   = wk + kr * dims.n_blocks * 18u;
        y_out   = yk;
        out_row = kr;
    } else {
        uint vr = row_local - dims.q_rows - dims.kv_rows;
        w_row   = wv + vr * dims.n_blocks * 18u;
        y_out   = yv;
        out_row = vr;
    }

    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 18u;
        device const short *sp = (device const short *)(block);
        float scale = float(int(*sp)) * (1.0f / 2048.0f);

        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);
        device const uchar4 *nb4 = (device const uchar4 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        for (uint s = 0; s < 4u; s++) {
            uchar4 b4 = nb4[s];
            float4 lo = float4(
                float(int(b4.x & 0x0Fu) - 8), float(int(b4.y & 0x0Fu) - 8),
                float(int(b4.z & 0x0Fu) - 8), float(int(b4.w & 0x0Fu) - 8));
            float4 hi = float4(
                float(int((b4.x >> 4) & 0x0Fu) - 8), float(int((b4.y >> 4) & 0x0Fu) - 8),
                float(int((b4.z >> 4) & 0x0Fu) - 8), float(int((b4.w >> 4) & 0x0Fu) - 8));
            acc4 += lo * xb4[s];
            acc4 += hi * xb4[s + 4u];
        }
        my_sum = fma(scale, acc4.x + acc4.y + acc4.z + acc4.w, my_sum);
    }

    float total = simd_sum(my_sum);
    if (lane == 0) {
        y_out[out_row] = total;
    }
}
"#;

/// Fused gate+up: single dispatch computes both gate and up projections.
/// Saves one dispatch vs calling kmain twice. Same x, two weight matrices,
/// two outputs. Per SIMD: reads 2×18 bytes/block, writes 2 output rows.
pub const MSL_DUAL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 8;
constant constexpr uint LANES  = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;
constant constexpr uint MAX_BLOCKS = 128;

kernel void kmain(
    device const float   *x   [[buffer(0)]],
    device const uchar   *wa  [[buffer(1)]],  // gate weights
    device const uchar   *wb  [[buffer(2)]],  // up weights
    device       float   *ya  [[buffer(3)]],  // gate output
    device       float   *yb  [[buffer(4)]],  // up output
    constant     Dims    &dims [[buffer(5)]],
    uint                 tgpig_x [[threadgroup_position_in_grid]],
    uint                 lane    [[thread_index_in_simdgroup]],
    uint                 sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *wa_row = wa + row * dims.n_blocks * 18u;
    device const uchar *wb_row = wb + row * dims.n_blocks * 18u;
    float sum_a = 0.0f, sum_b = 0.0f;

    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);

        device const uchar *ba = wa_row + blk * 18u;
        float scale_a = float(int(*((device const short *)ba))) * (1.0f / 2048.0f);
        device const uchar4 *na4 = (device const uchar4 *)(ba + 2u);
        float4 acc_a = float4(0.0f);
        for (uint s = 0; s < 4u; s++) {
            uchar4 n4 = na4[s];
            float4 lo = float4(float(int(n4.x & 0xFu) - 8), float(int(n4.y & 0xFu) - 8),
                               float(int(n4.z & 0xFu) - 8), float(int(n4.w & 0xFu) - 8));
            float4 hi = float4(float(int((n4.x >> 4) & 0xFu) - 8), float(int((n4.y >> 4) & 0xFu) - 8),
                               float(int((n4.z >> 4) & 0xFu) - 8), float(int((n4.w >> 4) & 0xFu) - 8));
            acc_a += lo * xb4[s];
            acc_a += hi * xb4[s + 4u];
        }
        sum_a = fma(scale_a, acc_a.x + acc_a.y + acc_a.z + acc_a.w, sum_a);

        device const uchar *bb = wb_row + blk * 18u;
        float scale_b = float(int(*((device const short *)bb))) * (1.0f / 2048.0f);
        device const uchar4 *nb4 = (device const uchar4 *)(bb + 2u);
        float4 acc_b = float4(0.0f);
        for (uint s = 0; s < 4u; s++) {
            uchar4 n4 = nb4[s];
            float4 lo = float4(float(int(n4.x & 0xFu) - 8), float(int(n4.y & 0xFu) - 8),
                               float(int(n4.z & 0xFu) - 8), float(int(n4.w & 0xFu) - 8));
            float4 hi = float4(float(int((n4.x >> 4) & 0xFu) - 8), float(int((n4.y >> 4) & 0xFu) - 8),
                               float(int((n4.z >> 4) & 0xFu) - 8), float(int((n4.w >> 4) & 0xFu) - 8));
            acc_b += lo * xb4[s];
            acc_b += hi * xb4[s + 4u];
        }
        sum_b = fma(scale_b, acc_b.x + acc_b.y + acc_b.z + acc_b.w, sum_b);
    }

    float ta = simd_sum(sum_a), tb = simd_sum(sum_b);
    if (lane == 0) {
        uint out_idx = b * dims.n_rows + row;
        ya[out_idx] = ta;
        yb[out_idx] = tb;
    }
}
"#;

/// gate+up+silu fused: computes silu(gate) * up → y in one dispatch.
/// Saves the separate silu_mul dispatch. Same grid as MSL_DUAL.
/// Buffers: 0=x, 1=wa(gate), 2=wb(up), 3=y(mid=silu(gate)*up), 4=dims.
pub const MSL_GATE_UP_SILU: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 8;
constant constexpr uint LANES  = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;
constant constexpr uint MAX_BLOCKS = 128;

kernel void kmain(
    device const float   *x   [[buffer(0)]],
    device const uchar   *wa  [[buffer(1)]],  // gate weights
    device const uchar   *wb  [[buffer(2)]],  // up weights
    device       float   *y   [[buffer(3)]],  // mid = silu(gate)*up
    constant     Dims    &dims [[buffer(4)]],
    uint                 tgpig_x [[threadgroup_position_in_grid]],
    uint                 lane    [[thread_index_in_simdgroup]],
    uint                 sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *wa_row = wa + row * dims.n_blocks * 18u;
    device const uchar *wb_row = wb + row * dims.n_blocks * 18u;
    float sum_a = 0.0f, sum_b = 0.0f;

    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);

        device const uchar *ba = wa_row + blk * 18u;
        float scale_a = float(int(*((device const short *)ba))) * (1.0f / 2048.0f);
        device const uchar4 *na4 = (device const uchar4 *)(ba + 2u);
        float4 acc_a = float4(0.0f);
        for (uint s = 0; s < 4u; s++) {
            uchar4 n4 = na4[s];
            float4 lo = float4(float(int(n4.x & 0xFu) - 8), float(int(n4.y & 0xFu) - 8),
                               float(int(n4.z & 0xFu) - 8), float(int(n4.w & 0xFu) - 8));
            float4 hi = float4(float(int((n4.x >> 4) & 0xFu) - 8), float(int((n4.y >> 4) & 0xFu) - 8),
                               float(int((n4.z >> 4) & 0xFu) - 8), float(int((n4.w >> 4) & 0xFu) - 8));
            acc_a += lo * xb4[s];
            acc_a += hi * xb4[s + 4u];
        }
        sum_a = fma(scale_a, acc_a.x + acc_a.y + acc_a.z + acc_a.w, sum_a);

        device const uchar *bb = wb_row + blk * 18u;
        float scale_b = float(int(*((device const short *)bb))) * (1.0f / 2048.0f);
        device const uchar4 *nb4 = (device const uchar4 *)(bb + 2u);
        float4 acc_b = float4(0.0f);
        for (uint s = 0; s < 4u; s++) {
            uchar4 n4 = nb4[s];
            float4 lo = float4(float(int(n4.x & 0xFu) - 8), float(int(n4.y & 0xFu) - 8),
                               float(int(n4.z & 0xFu) - 8), float(int(n4.w & 0xFu) - 8));
            float4 hi = float4(float(int((n4.x >> 4) & 0xFu) - 8), float(int((n4.y >> 4) & 0xFu) - 8),
                               float(int((n4.z >> 4) & 0xFu) - 8), float(int((n4.w >> 4) & 0xFu) - 8));
            acc_b += lo * xb4[s];
            acc_b += hi * xb4[s + 4u];
        }
        sum_b = fma(scale_b, acc_b.x + acc_b.y + acc_b.z + acc_b.w, sum_b);
    }

    float ta = simd_sum(sum_a), tb = simd_sum(sum_b);
    if (lane == 0) {
        float gate = ta;
        float silu = gate * (1.0f / (1.0f + exp(-gate)));
        y[b * dims.n_rows + row] = silu * tb;
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

    let rows_per_tg = N_DST * SIMDS_PER_GROUP;
    let groups_x = (total_rows + rows_per_tg - 1) / rows_per_tg;
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
