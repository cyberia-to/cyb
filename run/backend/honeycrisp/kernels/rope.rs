//! Metal Rotary Position Embedding (NeoX-style pairing).
//!
//! Input shape: x is f32 [n_rows, head_dim] flattened. pos is f32 length 1.
//! Each row gets independently rotated. NeoX pairs (j, j+head_dim/2).
//! Identical math to backend/cpu/rope.rs.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

// Bisect: full trig with 2D position + dynamic index from struct.
pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Params {
    uint  n_rows;
    uint  head_dim;
    uint  rope_dim;
    float base;
};

kernel void kmain(
    device const float  *x      [[buffer(0)]],
    device const float  *pos    [[buffer(1)]],
    device       float  *y      [[buffer(2)]],
    constant     Params &p      [[buffer(3)]],
    uint                 row    [[threadgroup_position_in_grid]],
    uint                 j      [[thread_position_in_threadgroup]]
) {
    uint half = p.head_dim / 2u;
    if (row >= p.n_rows || j >= half) return;
    float pv = pos[0];
    uint base_off = row * p.head_dim;
    float x1 = x[base_off + j];
    float x2 = x[base_off + j + half];
    float exponent = 2.0f * float(j) / float(p.head_dim);
    float theta = pv / pow(p.base, exponent);
    float s = sin(theta);
    float c = cos(theta);
    y[base_off + j]        = x1 * c - x2 * s;
    y[base_off + j + half] = x1 * s + x2 * c;
}
"#;

pub fn dispatch(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    x: &aruminium::Buffer,
    cos_t: &aruminium::Buffer,
    sin_t: &aruminium::Buffer,
    n_rows: u32,
    head_dim: u32,
    rope_dim: u32,
) -> Result<aruminium::Buffer, BackendError> {
    let out = dev.alloc((n_rows as usize * head_dim as usize * 4).max(4))?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Params {
        n_rows: u32,
        head_dim: u32,
        rope_dim: u32,
        pad: u32,
    }
    let params = Params { n_rows, head_dim, rope_dim, pad: 0 };

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|enc| {
                enc.bind(pipeline);
                enc.bind_buffer(x, 0, 0);
                enc.bind_buffer(cos_t, 0, 1);
                enc.bind_buffer(sin_t, 0, 2);
                enc.bind_buffer(&out, 0, 3);
                let bytes = std::slice::from_raw_parts(
                    &params as *const Params as *const u8,
                    std::mem::size_of::<Params>(),
                );
                enc.push(bytes, 4);
                let half = (head_dim / 2) as usize;
                enc.launch_groups((n_rows as usize, 1, 1), (half, 1, 1));
            });
        });
    }
    Ok(out)
}
