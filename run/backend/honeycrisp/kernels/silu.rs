//! Metal Silu.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Params { uint n; uint pad0; uint pad1; uint pad2; };

kernel void kmain(
    device const float *x [[buffer(0)]],
    device float *y [[buffer(1)]],
    constant Params &params [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= params.n) return;
    float v = x[gid];
    y[gid] = v / (1.0f + exp(-v));
}
"#;

pub fn dispatch(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    x: &aruminium::Buffer,
    n: u32,
) -> Result<aruminium::Buffer, BackendError> {
    let out = dev.alloc((n * 4) as usize)?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Params {
        n: u32,
        pad0: u32,
        pad1: u32,
        pad2: u32,
    }
    let params = Params {
        n,
        pad0: 0,
        pad1: 0,
        pad2: 0,
    };

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|b_enc| {
                b_enc.bind(pipeline);
                b_enc.bind_buffer(x, 0, 0);
                b_enc.bind_buffer(&out, 0, 1);
                let bytes = std::slice::from_raw_parts(
                    &params as *const Params as *const u8,
                    std::mem::size_of::<Params>(),
                );
                b_enc.push(bytes, 2);
                b_enc.launch_groups((((n as usize) + 255) / 256, 1, 1), (256, 1, 1));
            });
        });
    }
    Ok(out)
}
