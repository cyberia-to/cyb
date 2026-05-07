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

const MSL_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint NUM_HEADS = __NUM_HEADS__u;
constant constexpr uint KV_HEADS  = __KV_HEADS__u;
constant constexpr uint HEAD_DIM  = __HEAD_DIM__u;
constant constexpr uint MAX_SEQ   = __MAX_SEQ__u;
constant constexpr uint REPEAT    = NUM_HEADS / KV_HEADS;
constant constexpr uint MAX_SEQ_SHARED_C = 2048u;
constant constexpr uint LANES = 32u;

struct Params {
    uint  total_seq;
    uint  window;
    float scale;
    uint  pad;
};

kernel void kmain(
    device const float  *q       [[buffer(0)]],
    device const float  *k_cache [[buffer(1)]],
    device const float  *v_cache [[buffer(2)]],
    device       float  *out     [[buffer(3)]],
    constant     Params &p       [[buffer(4)]],
    uint                 h       [[threadgroup_position_in_grid]],
    uint                 lane    [[thread_index_in_simdgroup]]
) {
    if (h >= NUM_HEADS) return;
    // GQA: h maps to kv_h. NeoX-style assignment groups consecutive q-heads
    // to the same kv-head.
    uint kv_h = h / REPEAT;
    uint q_off = h * HEAD_DIM;
    uint kv_base = kv_h * MAX_SEQ * HEAD_DIM;

    threadgroup float scores[MAX_SEQ_SHARED_C];

    uint window_start = (p.window > 0u && p.total_seq > p.window)
        ? (p.total_seq - p.window) : 0u;

    // 1) Compute scaled dot-product scores Q@K^T
    for (uint s = lane; s < p.total_seq; s += LANES) {
        if (s < window_start) {
            scores[s] = -1.0e4f;
            continue;
        }
        float acc = 0.0f;
        uint k_off = kv_base + s * HEAD_DIM;
        for (uint d = 0; d < HEAD_DIM; d++) {
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
    // DIAGNOSTIC: also compare against bypass paths via env var.
    uint out_off = h * HEAD_DIM;
    for (uint d = lane; d < HEAD_DIM; d += LANES) {
        float acc = 0.0f;
        for (uint s = 0; s < p.total_seq; s++) {
            acc = fma(scores[s], v_cache[kv_base + s * HEAD_DIM + d], acc);
        }
        out[out_off + d] = acc;
    }
}

// Diagnostic: bypass attention, just write v to out (verifies plumbing).
kernel void kbypass(
    device const float  *q       [[buffer(0)]],
    device const float  *k_cache [[buffer(1)]],
    device const float  *v_cache [[buffer(2)]],
    device       float  *out     [[buffer(3)]],
    constant     Params &p       [[buffer(4)]],
    uint                 h       [[threadgroup_position_in_grid]],
    uint                 lane    [[thread_index_in_simdgroup]]
) {
    if (h >= NUM_HEADS) return;
    uint kv_h = h / REPEAT;
    uint q_off = h * HEAD_DIM;
    uint kv_base = kv_h * MAX_SEQ * HEAD_DIM;
    // Output = v_cache[kv_h, position=0, d] — corresponds to total_seq=1 attention.
    uint out_off = h * HEAD_DIM;
    for (uint d = lane; d < HEAD_DIM; d += LANES) {
        out[out_off + d] = v_cache[kv_base + d];
    }
}
"#;

// KV-append kernel — writes new k or v at given position into the cache.
// Constants baked: KV_HEADS, HEAD_DIM, MAX_SEQ.
const KV_APPEND_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint KV_HEADS  = __KV_HEADS__u;
constant constexpr uint HEAD_DIM  = __HEAD_DIM__u;
constant constexpr uint MAX_SEQ   = __MAX_SEQ__u;

struct Params { uint position; uint pad0; uint pad1; uint pad2; };

kernel void kmain(
    device const float  *src   [[buffer(0)]],   // [KV_HEADS, HEAD_DIM]
    device       float  *cache [[buffer(1)]],   // [KV_HEADS, MAX_SEQ, HEAD_DIM]
    constant     Params &p     [[buffer(2)]],
    uint2                gid   [[thread_position_in_grid]]
) {
    uint h = gid.y;
    uint d = gid.x;
    if (h >= KV_HEADS || d >= HEAD_DIM) return;
    uint src_off = h * HEAD_DIM + d;
    uint dst_off = h * MAX_SEQ * HEAD_DIM + p.position * HEAD_DIM + d;
    cache[dst_off] = src[src_off];
}
"#;

// KV-append-both: writes new K AND V in one dispatch, saving one kernel launch.
const KV_APPEND_BOTH_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint KV_HEADS  = __KV_HEADS__u;
constant constexpr uint HEAD_DIM  = __HEAD_DIM__u;
constant constexpr uint MAX_SEQ   = __MAX_SEQ__u;

struct Params { uint position; uint pad0; uint pad1; uint pad2; };

kernel void kmain(
    device const float  *k_src   [[buffer(0)]],
    device       float  *k_cache [[buffer(1)]],
    device const float  *v_src   [[buffer(2)]],
    device       float  *v_cache [[buffer(3)]],
    constant     Params &p       [[buffer(4)]],
    uint2                gid     [[thread_position_in_grid]]
) {
    uint h = gid.y;
    uint d = gid.x;
    if (h >= KV_HEADS || d >= HEAD_DIM) return;
    uint src_off = h * HEAD_DIM + d;
    uint dst_off = h * MAX_SEQ * HEAD_DIM + p.position * HEAD_DIM + d;
    k_cache[dst_off] = k_src[src_off];
    v_cache[dst_off] = v_src[src_off];
}
"#;

pub fn msl_for(num_heads: usize, kv_heads: usize, head_dim: usize, max_seq: usize) -> String {
    MSL_TEMPLATE
        .replace("__NUM_HEADS__", &num_heads.to_string())
        .replace("__KV_HEADS__", &kv_heads.to_string())
        .replace("__HEAD_DIM__", &head_dim.to_string())
        .replace("__MAX_SEQ__", &max_seq.to_string())
}

pub fn kv_append_msl_for(kv_heads: usize, head_dim: usize, max_seq: usize) -> String {
    KV_APPEND_TEMPLATE
        .replace("__KV_HEADS__", &kv_heads.to_string())
        .replace("__HEAD_DIM__", &head_dim.to_string())
        .replace("__MAX_SEQ__", &max_seq.to_string())
}

pub fn kv_append_both_msl_for(kv_heads: usize, head_dim: usize, max_seq: usize) -> String {
    KV_APPEND_BOTH_TEMPLATE
        .replace("__KV_HEADS__", &kv_heads.to_string())
        .replace("__HEAD_DIM__", &head_dim.to_string())
        .replace("__MAX_SEQ__", &max_seq.to_string())
}

#[allow(dead_code)]
pub const MSL: &str = ""; // built dynamically via msl_for
