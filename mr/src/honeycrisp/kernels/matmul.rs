//! Metal matmul: y = x @ W^T

use crate::backend::BackendError;
use crate::honeycrisp::device::HoneycrispDevice;

const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Params {
    uint batch;
    uint n;
    uint k;
    uint pad;
};

kernel void kmain(
    device const float *x [[buffer(0)]],
    device const float *w [[buffer(1)]],
    device float *y [[buffer(2)]],
    constant Params &params [[buffer(3)]],
    uint gid [[thread_position_in_grid]]
) {
    uint total = params.batch * params.n;
    if (gid >= total) return;
    uint b = gid / params.n;
    uint i = gid % params.n;
    float acc = 0.0f;
    for (uint j = 0; j < params.k; j++) {
        acc += x[b * params.k + j] * w[i * params.k + j];
    }
    y[gid] = acc;
}
"#;

pub fn dispatch(
    dev: &HoneycrispDevice,
    x: &aruminium::Buffer,
    w: &aruminium::Buffer,
    batch: u32,
    n: u32,
    k: u32,
) -> Result<aruminium::Buffer, BackendError> {
    let pipeline = dev.pipeline(MSL)?;
    let out_size = (batch * n * 4) as usize;
    let out = dev.alloc(out_size)?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Params {
        batch: u32,
        n: u32,
        k: u32,
        pad: u32,
    }
    let params = Params {
        batch,
        n,
        k,
        pad: 0,
    };

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|batch_enc| {
                batch_enc.bind(&pipeline);
                batch_enc.bind_buffer(x, 0, 0);
                batch_enc.bind_buffer(w, 0, 1);
                batch_enc.bind_buffer(&out, 0, 2);
                let bytes = std::slice::from_raw_parts(
                    &params as *const Params as *const u8,
                    std::mem::size_of::<Params>(),
                );
                batch_enc.push(bytes, 3);
                let total = (batch * n) as usize;
                batch_enc.launch_groups(((total + 63) / 64, 1, 1), (64, 1, 1));
            });
        });
    }
    Ok(out)
}
