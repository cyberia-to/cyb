//! Metal scaled dot-product attention for decode mode (single query).
//!
//! Inputs:
//!   q       — [num_heads, head_dim] f32
//!   k_cache — [kv_heads, max_seq, head_dim] f32 (only first total_seq rows valid)
//!   v_cache — same shape
//!
//! Output:
//!   out     — [num_heads, head_dim] f32
//!
//! Per head:
//!   scores[s] = scale * dot(q[h], k_cache[h/repeat, s])  for s in 0..total_seq
//!   softmax(scores)
//!   out[h, d] = sum_s scores[s] * v_cache[h/repeat, s, d]
//!
//! Sliding window (optional): mask out positions s < total_seq - window.
//!
//! Parallelism: one threadgroup per head, 32 threads (1 SIMD group).
//! Each thread strides over score positions; simdgroup reductions handle
//! softmax. Then each thread strides over output dims.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

pub const MAX_SEQ_SHARED: u32 = 2048;

pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Params {
    uint  num_heads;
    uint  kv_heads;
    uint  head_dim;
    uint  total_seq;     // valid sequence length in cache
    uint  max_seq;       // cache stride
    uint  window;        // 0 = no sliding window
    float scale;
    uint  pad;
};

constant constexpr uint MAX_SEQ_SHARED_C = 2048u;
constant constexpr uint LANES = 32u;

kernel void kmain(
    device const float  *q       [[buffer(0)]],
    device const float  *k_cache [[buffer(1)]],
    device const float  *v_cache [[buffer(2)]],
    device       float  *out     [[buffer(3)]],
    constant     Params &p       [[buffer(4)]],
    uint                 h       [[threadgroup_position_in_grid]],
    uint                 lane    [[thread_index_in_simdgroup]]
) {
    if (h >= p.num_heads) return;
    uint repeat = p.num_heads / p.kv_heads;
    uint kv_h = h / repeat;
    uint q_off = h * p.head_dim;
    uint kv_base = kv_h * p.max_seq * p.head_dim;

    threadgroup float scores[MAX_SEQ_SHARED_C];

    uint window_start = (p.window > 0u && p.total_seq > p.window) ? (p.total_seq - p.window) : 0u;

    // 1) Compute scaled dot-product scores Q@K^T
    for (uint s = lane; s < p.total_seq; s += LANES) {
        if (s < window_start) {
            scores[s] = -1.0e4f;
            continue;
        }
        float acc = 0.0f;
        uint k_off = kv_base + s * p.head_dim;
        for (uint d = 0; d < p.head_dim; d++) {
            acc = fma(q[q_off + d], k_cache[k_off + d], acc);
        }
        scores[s] = acc * p.scale;
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    // 2) Softmax: find max, exp, sum, normalize.
    float local_max = -INFINITY;
    for (uint s = lane; s < p.total_seq; s += LANES) {
        local_max = max(local_max, scores[s]);
    }
    float global_max = simd_max(local_max);

    float local_sum = 0.0f;
    for (uint s = lane; s < p.total_seq; s += LANES) {
        float v = exp(scores[s] - global_max);
        scores[s] = v;
        local_sum += v;
    }
    float global_sum = simd_sum(local_sum);
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float inv_sum = 1.0f / global_sum;
    for (uint s = lane; s < p.total_seq; s += LANES) {
        scores[s] *= inv_sum;
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    // 3) Output: out[h, d] = sum_s scores[s] * V[kv_h, s, d]
    uint out_off = h * p.head_dim;
    for (uint d = lane; d < p.head_dim; d += LANES) {
        float acc = 0.0f;
        for (uint s = 0; s < p.total_seq; s++) {
            acc = fma(scores[s], v_cache[kv_base + s * p.head_dim + d], acc);
        }
        out[out_off + d] = acc;
    }
}
"#;

pub fn dispatch(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    q: &aruminium::Buffer,
    k_cache: &aruminium::Buffer,
    v_cache: &aruminium::Buffer,
    num_heads: u32,
    kv_heads: u32,
    head_dim: u32,
    total_seq: u32,
    max_seq: u32,
    window: u32,
    scale: f32,
) -> Result<aruminium::Buffer, BackendError> {
    let out = dev.alloc((num_heads as usize * head_dim as usize * 4).max(4))?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Params {
        num_heads: u32,
        kv_heads: u32,
        head_dim: u32,
        total_seq: u32,
        max_seq: u32,
        window: u32,
        scale: f32,
        pad: u32,
    }
    let params = Params {
        num_heads, kv_heads, head_dim, total_seq, max_seq, window, scale, pad: 0,
    };

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|enc| {
                enc.bind(pipeline);
                enc.bind_buffer(q, 0, 0);
                enc.bind_buffer(k_cache, 0, 1);
                enc.bind_buffer(v_cache, 0, 2);
                enc.bind_buffer(&out, 0, 3);
                let bytes = std::slice::from_raw_parts(
                    &params as *const Params as *const u8,
                    std::mem::size_of::<Params>(),
                );
                enc.push(bytes, 4);
                enc.launch_groups((num_heads as usize, 1, 1), (32, 1, 1));
            });
        });
    }
    Ok(out)
}
