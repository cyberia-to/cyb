//! Metal Q4_K fused dequant+matmul.
//!
//! Q4_K block layout (144 bytes):
//!   f16 d, f16 dmin, scales[12] (6-bit packed), qs[128] (4-bit nibbles)
//! Dequant: x = d * s[j] * nibble - dmin * m[j]
//! Output: y[b, row] = sum_k x[b, k] * W[row, k]

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

kernel void kmain(
    device const float   *x    [[buffer(0)]],
    device const uint8_t *w    [[buffer(1)]],
    device       float   *y    [[buffer(2)]],
    constant     Dims    &dims [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= dims.batch * dims.n_rows) return;
    uint b   = gid / dims.n_rows;
    uint row = gid % dims.n_rows;

    device const uint8_t *w_row = w + row * dims.n_blocks * 144u;
    device const float   *x_b   = x + b * dims.n_blocks * 256u;

    float sum = 0.0f;
    for (uint blk = 0; blk < dims.n_blocks; blk++) {
        device const uint8_t *block = w_row + blk * 144u;
        float d    = float(as_type<half>((ushort)(uint(block[0]) | (uint(block[1]) << 8))));
        float dmin = float(as_type<half>((ushort)(uint(block[2]) | (uint(block[3]) << 8))));
        device const uint8_t *sc = block + 4u;
        device const uint8_t *qs = block + 16u;
        device const float   *xb = x_b + blk * 256u;

        for (uint j = 0; j < 8u; j++) {
            uint s, m;
            if (j < 4u) {
                s = uint(sc[j])     & 0x3Fu;
                m = uint(sc[j + 4]) & 0x3Fu;
            } else {
                s = (uint(sc[j + 4]) & 0x0Fu) | ((uint(sc[j - 4]) >> 6u) << 4u);
                m = (uint(sc[j + 4]) >> 4u)   | ((uint(sc[j])     >> 6u) << 4u);
            }
            float ds = d * float(s);
            float dm = dmin * float(m);

            uint qs_off = (j / 2u) * 32u;
            uint shift  = (j & 1u) * 4u;
            device const float *xj = xb + j * 32u;

            float nibble_dot = 0.0f, x_sum = 0.0f;
            for (uint l = 0; l < 32u; l++) {
                float nib = float((uint(qs[qs_off + l]) >> shift) & 0x0Fu);
                nibble_dot += nib * xj[l];
                x_sum      += xj[l];
            }
            sum += ds * nibble_dot - dm * x_sum;
        }
    }
    y[gid] = sum;
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
    let total = batch * n_rows;
    let out = dev.alloc((total * 4) as usize)?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
    let dims = Dims { batch, n_rows, n_blocks, pad: 0 };

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
                let n = total as usize;
                enc.launch_groups(((n + 63) / 64, 1, 1), (64, 1, 1));
            });
        });
    }
    Ok(out)
}
