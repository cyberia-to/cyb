//! Metal element-wise ops: Add, Mul, Sub, Div, Silu, SiluMul.
//!
//! All take f32 inputs and produce f32 outputs. Broadcasting handled by
//! the caller (forward.rs computes shapes; kernel uses linear indexing).

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

pub const ADD_MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Params { uint n; uint a_len; uint b_len; uint pad; };

// y[i] = a[i % a_len] + b[i % b_len]. With a_len == b_len == n, this is plain
// element-wise. With b_len == 1 (or smaller), broadcasts.
kernel void kmain(
    device const float *a [[buffer(0)]],
    device const float *b [[buffer(1)]],
    device       float *y [[buffer(2)]],
    constant   Params &p   [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= p.n) return;
    float av = a[gid % p.a_len];
    float bv = b[gid % p.b_len];
    y[gid] = av + bv;
}
"#;

pub const SILU_MUL_MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Params { uint n; uint pad0; uint pad1; uint pad2; };

// y[i] = silu(gate[i]) * up[i],  silu(x) = x / (1 + exp(-x)).
kernel void kmain(
    device const float *gate [[buffer(0)]],
    device const float *up   [[buffer(1)]],
    device       float *y    [[buffer(2)]],
    constant   Params &p     [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= p.n) return;
    float g = gate[gid];
    float u = up[gid];
    float s = g / (1.0f + exp(-g));
    y[gid] = s * u;
}
"#;

pub fn dispatch_add(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    a: &aruminium::Buffer,
    b: &aruminium::Buffer,
    n: u32,
    a_len: u32,
    b_len: u32,
) -> Result<aruminium::Buffer, BackendError> {
    let out = dev.alloc((n * 4) as usize)?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Params { n: u32, a_len: u32, b_len: u32, pad: u32 }
    let params = Params { n, a_len, b_len, pad: 0 };

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|enc| {
                enc.bind(pipeline);
                enc.bind_buffer(a, 0, 0);
                enc.bind_buffer(b, 0, 1);
                enc.bind_buffer(&out, 0, 2);
                let bytes = std::slice::from_raw_parts(
                    &params as *const Params as *const u8,
                    std::mem::size_of::<Params>(),
                );
                enc.push(bytes, 3);
                let total = n as usize;
                enc.launch_groups(((total + 63) / 64, 1, 1), (64, 1, 1));
            });
        });
    }
    Ok(out)
}

pub fn dispatch_silu_mul(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    gate: &aruminium::Buffer,
    up: &aruminium::Buffer,
    n: u32,
) -> Result<aruminium::Buffer, BackendError> {
    let out = dev.alloc((n * 4) as usize)?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Params { n: u32, pad0: u32, pad1: u32, pad2: u32 }
    let params = Params { n, pad0: 0, pad1: 0, pad2: 0 };

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|enc| {
                enc.bind(pipeline);
                enc.bind_buffer(gate, 0, 0);
                enc.bind_buffer(up, 0, 1);
                enc.bind_buffer(&out, 0, 2);
                let bytes = std::slice::from_raw_parts(
                    &params as *const Params as *const u8,
                    std::mem::size_of::<Params>(),
                );
                enc.push(bytes, 3);
                let total = n as usize;
                enc.launch_groups(((total + 63) / 64, 1, 1), (64, 1, 1));
            });
        });
    }
    Ok(out)
}
