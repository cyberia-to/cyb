//! Metal Q6_K fused dequant+matmul.
//!
//! Q6_K block layout (210 bytes):
//!   ql[128]    low 4 bits per value
//!   qh[64]     high 2 bits per value
//!   scales[16] i8 per-sub-block (16 groups of 16)
//!   f16 d      superblock scale
//! Dequant: x = d * scale[j] * (q6 - 32)

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

    device const uint8_t *w_row = w + row * dims.n_blocks * 210u;
    device const float   *x_b   = x + b * dims.n_blocks * 256u;

    float sum = 0.0f;
    for (uint blk = 0; blk < dims.n_blocks; blk++) {
        device const uint8_t *block = w_row + blk * 210u;
        device const uint8_t *ql     = block;
        device const uint8_t *qh     = block + 128u;
        device const uint8_t *sc_raw = block + 192u;   // interpret as i8
        float d = float(as_type<half>((ushort)(uint(block[208]) | (uint(block[209]) << 8))));
        device const float *x_blk = x_b + blk * 256u;

        float block_sum = 0.0f;
        for (uint half_ = 0; half_ < 2u; half_++) {
            uint ql_off = half_ * 64u;
            uint qh_off = half_ * 32u;
            uint sc_off = half_ * 8u;
            device const float *xh = x_blk + half_ * 128u;

            for (uint l = 0; l < 32u; l++) {
                uint is = l / 16u;
                uint qh_byte = uint(qh[qh_off + l]);

                int q1 = int((uint(ql[ql_off + l])      & 0x0Fu) | ((qh_byte & 0x03u) << 4u)) - 32;
                int q2 = int((uint(ql[ql_off + l + 32]) & 0x0Fu) | ((qh_byte & 0x0Cu) << 2u)) - 32;
                int q3 = int((uint(ql[ql_off + l])      >> 4u)   | (qh_byte & 0x30u))          - 32;
                int q4 = int((uint(ql[ql_off + l + 32]) >> 4u)   | ((qh_byte & 0xC0u) >> 2u))  - 32;

                // Sign-extend u8 → i8 by checking high bit
                uint raw1 = uint(sc_raw[sc_off + is + 0u]);
                uint raw2 = uint(sc_raw[sc_off + is + 2u]);
                uint raw3 = uint(sc_raw[sc_off + is + 4u]);
                uint raw4 = uint(sc_raw[sc_off + is + 6u]);
                float s1 = float(raw1 > 127u ? int(raw1) - 256 : int(raw1));
                float s2 = float(raw2 > 127u ? int(raw2) - 256 : int(raw2));
                float s3 = float(raw3 > 127u ? int(raw3) - 256 : int(raw3));
                float s4 = float(raw4 > 127u ? int(raw4) - 256 : int(raw4));

                block_sum += s1 * float(q1) * xh[l];
                block_sum += s2 * float(q2) * xh[l + 32u];
                block_sum += s3 * float(q3) * xh[l + 64u];
                block_sum += s4 * float(q4) * xh[l + 96u];
            }
        }
        sum += d * block_sum;
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
