//! Metal RmsNorm over last dim.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Params {
    uint batch;
    uint d;
    float eps;
    uint pad;
};

kernel void kmain(
    device const float *x [[buffer(0)]],
    device const float *g [[buffer(1)]],
    device float *y [[buffer(2)]],
    constant Params &params [[buffer(3)]],
    uint tid [[thread_position_in_threadgroup]],
    uint bid [[threadgroup_position_in_grid]]
) {
    threadgroup float shared_sum[256];
    uint b = bid;
    if (b >= params.batch) return;

    float acc = 0.0f;
    for (uint j = tid; j < params.d; j += 256) {
        float v = x[b * params.d + j];
        acc += v * v;
    }
    shared_sum[tid] = acc;
    threadgroup_barrier(mem_flags::mem_threadgroup);

    for (uint stride = 128; stride > 0; stride /= 2) {
        if (tid < stride) {
            shared_sum[tid] += shared_sum[tid + stride];
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);
    }
    float mean_sq = shared_sum[0] / (float)params.d;
    float inv_rms = 1.0f / sqrt(mean_sq + params.eps);

    for (uint j = tid; j < params.d; j += 256) {
        y[b * params.d + j] = x[b * params.d + j] * inv_rms * g[j];
    }
}
"#;

pub fn dispatch(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    x: &aruminium::Buffer,
    g: &aruminium::Buffer,
    batch: u32,
    d: u32,
    eps: f32,
) -> Result<aruminium::Buffer, BackendError> {
    let out = dev.alloc((batch * d * 4) as usize)?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Params {
        batch: u32,
        d: u32,
        eps: f32,
        pad: u32,
    }
    let params = Params {
        batch,
        d,
        eps,
        pad: 0,
    };

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|b_enc| {
                b_enc.bind(pipeline);
                b_enc.bind_buffer(x, 0, 0);
                b_enc.bind_buffer(g, 0, 1);
                b_enc.bind_buffer(&out, 0, 2);
                let bytes = std::slice::from_raw_parts(
                    &params as *const Params as *const u8,
                    std::mem::size_of::<Params>(),
                );
                b_enc.push(bytes, 3);
                b_enc.launch_groups((batch as usize, 1, 1), (256, 1, 1));
            });
        });
    }
    Ok(out)
}
