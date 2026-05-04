//! Metal Rotary Position Embedding (NeoX-style pairing).
//!
//! Input shape: x is f32 [n_rows, head_dim] flattened. pos is f32 length 1.
//! Each row gets independently rotated. NeoX pairs (j, j+head_dim/2).
//! Identical math to backend/cpu/rope.rs.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

// MSL template — caller substitutes `__HEAD_DIM__` and `__HALF__` with literal
// constants at pipeline-build time. Apple Metal scheduler regresses when more
// than one non-constant integer is used as an address-offset; baking these in
// keeps the kernel single-non-constant.
const MSL_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint HEAD_DIM = __HEAD_DIM__u;
constant constexpr uint HALF     = __HALF__u;

struct Dims { uint n_rows; uint rope_half; uint pad0; uint pad1; };

kernel void kmain(
    device const float  *x      [[buffer(0)]],
    device const float  *cos_t  [[buffer(1)]],
    device const float  *sin_t  [[buffer(2)]],
    device       float  *y      [[buffer(3)]],
    constant     Dims   &dims   [[buffer(4)]],
    uint2                gid    [[thread_position_in_grid]]
) {
    uint row = gid.y;
    uint j   = gid.x;
    if (row >= dims.n_rows || j >= HALF) return;
    uint base = row * HEAD_DIM;

    float x1 = x[base + j];
    float x2 = x[base + j + HALF];

    if (j < dims.rope_half) {
        float c = cos_t[j];
        float s = sin_t[j];
        y[base + j]         = x1 * c - x2 * s;
        y[base + j + HALF]  = x1 * s + x2 * c;
    } else {
        y[base + j]         = x1;
        y[base + j + HALF]  = x2;
    }
}
"#;

pub fn msl_for(head_dim: usize) -> String {
    MSL_TEMPLATE
        .replace("__HEAD_DIM__", &head_dim.to_string())
        .replace("__HALF__", &(head_dim / 2).to_string())
}

// Default MSL for the qwen3-0.6b head_dim=128 path. The runtime can rebuild
// pipelines for other models via `msl_for(head_dim)`.
pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint HEAD_DIM = 128u;
constant constexpr uint HALF     = 64u;

struct Dims { uint n_rows; uint rope_half; uint pad0; uint pad1; };

kernel void kmain(
    device const float  *x      [[buffer(0)]],
    device const float  *cos_t  [[buffer(1)]],
    device const float  *sin_t  [[buffer(2)]],
    device       float  *y      [[buffer(3)]],
    constant     Dims   &dims   [[buffer(4)]],
    uint2                gid    [[thread_position_in_grid]]
) {
    uint row = gid.y;
    uint j   = gid.x;
    if (row >= dims.n_rows || j >= HALF) return;
    uint base = row * HEAD_DIM;

    float x1 = x[base + j];
    float x2 = x[base + j + HALF];

    if (j < dims.rope_half) {
        float c = cos_t[j];
        float s = sin_t[j];
        y[base + j]         = x1 * c - x2 * s;
        y[base + j + HALF]  = x1 * s + x2 * c;
    } else {
        y[base + j]         = x1;
        y[base + j + HALF]  = x2;
    }
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
    struct Dims { n_rows: u32, rope_half: u32, pad0: u32, pad1: u32 }
    let dims = Dims { n_rows, rope_half: rope_dim / 2, pad0: 0, pad1: 0 };
    let half = (head_dim / 2) as usize;

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|enc| {
                enc.bind(pipeline);
                enc.bind_buffer(x, 0, 0);
                enc.bind_buffer(cos_t, 0, 1);
                enc.bind_buffer(sin_t, 0, 2);
                enc.bind_buffer(&out, 0, 3);
                let bytes = std::slice::from_raw_parts(
                    &dims as *const Dims as *const u8,
                    std::mem::size_of::<Dims>(),
                );
                enc.push(bytes, 4);
                // 2D dispatch: (half threads, n_rows groups)
                enc.launch_groups((1, n_rows as usize, 1), (half, 1, 1));
            });
        });
    }
    Ok(out)
}
