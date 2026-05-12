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
constant constexpr uint ITERS = HEAD_DIM / LANES;  // d-chunks per thread (4 for head_dim=128)

struct Params {
    uint  total_seq;
    uint  window;
    float scale;
    uint  pad;
};

// Flash decode attention: single pass over s, reads K and V together.
// Online softmax (no TGS scores buffer, no barriers needed).
// All 32 lanes parallelise over HEAD_DIM for coalesced reads.
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
    uint kv_h = h / REPEAT;
    uint q_off = h * HEAD_DIM;
    uint kv_base = kv_h * MAX_SEQ * HEAD_DIM;

    uint window_start = (p.window > 0u && p.total_seq > p.window)
        ? (p.total_seq - p.window) : 0u;

    // Q cached in registers — read once, reused across all s positions.
    float q_reg[ITERS];
    for (uint i = 0; i < ITERS; i++) {
        q_reg[i] = q[q_off + lane + i * LANES];
    }

    // Flash attention: single sequential pass over s.
    // Online softmax tracks running max and sum; no TGS memory needed.
    float max_so_far = -INFINITY;
    float sum_so_far = 0.0f;
    float acc_out[ITERS];
    for (uint i = 0; i < ITERS; i++) acc_out[i] = 0.0f;

    for (uint s = 0; s < p.total_seq; s++) {
        uint kv_off = kv_base + s * HEAD_DIM;

        // Dot product Q·K[s]: all lanes read coalesced k_cache[s, lane+i*32].
        float dot = 0.0f;
        for (uint i = 0; i < ITERS; i++) {
            dot = fma(q_reg[i], k_cache[kv_off + lane + i * LANES], dot);
        }
        float score = simd_sum(dot) * p.scale;
        if (s < window_start) score = -1.0e4f;

        // Online softmax update.
        float new_max   = max(max_so_far, score);
        float correction = exp(max_so_far - new_max);  // 0 when max_so_far=-inf
        float exp_score  = exp(score - new_max);

        sum_so_far = fma(sum_so_far, correction, exp_score);

        // Accumulate V weighted by exp_score, correcting old accumulator.
        // All lanes read v_cache[s, lane+i*32] coalesced.
        for (uint i = 0; i < ITERS; i++) {
            float v = v_cache[kv_off + lane + i * LANES];
            acc_out[i] = fma(exp_score, v, acc_out[i] * correction);
        }
        max_so_far = new_max;
    }

    // Normalize and write output.
    float inv_sum = 1.0f / sum_so_far;
    uint out_off = h * HEAD_DIM;
    for (uint i = 0; i < ITERS; i++) {
        out[out_off + lane + i * LANES] = acc_out[i] * inv_sum;
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

/// Fused RoPE + KV-append + Flash-decode SDPA in one dispatch.
///
/// Eliminates the kv_append dispatch and the barrier between kv_append and SDPA.
/// Grid: (NUM_HEADS, 1, 1) × (32, 1, 1)  — same parallelism as the standalone SDPA dispatch.
///
/// Each TG handles one Q head q_h:
///   1. Compute RoPE'd K and write to k_cache[kv_h, position] (idempotent for GQA: multiple
///      Q-heads sharing the same kv_h all write the same value to the same location).
///   2. Copy v_raw[kv_h] to v_cache[kv_h, position] (same idempotency argument).
///   3. Compute RoPE'd Q in registers (no intermediate buffer).
///   4. Flash-decode SDPA using the updated k_cache and v_cache.
///
/// Steps 1-2 and 4 use the SAME lane index pattern for stores and loads at position p.position,
/// so within-thread ordering guarantees that each thread's store is visible to its own load —
/// no explicit inter-thread barrier is required.
///
/// HEAD_DIM=128, HALF=64 hardcoded. Templates: NUM_HEADS, KV_HEADS, MAX_SEQ.
/// Buffers: 0=q_raw, 1=k_raw, 2=v_raw, 3=cos, 4=sin, 5=k_cache, 6=v_cache, 7=out, 8=Params.
const FUSED_ROPE_KV_ATTN_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint NUM_HEADS = __NUM_HEADS__u;
constant constexpr uint KV_HEADS  = __KV_HEADS__u;
constant constexpr uint HEAD_DIM  = 128u;
constant constexpr uint HALF      = 64u;
constant constexpr uint MAX_SEQ   = __MAX_SEQ__u;
constant constexpr uint REPEAT    = NUM_HEADS / KV_HEADS;
constant constexpr uint ITERS     = HEAD_DIM / 32u;  // 4

struct Params { uint total_seq; uint position; uint window; float scale; };

kernel void kmain(
    device const float *q_raw   [[buffer(0)]],
    device const float *k_raw   [[buffer(1)]],
    device const float *v_raw   [[buffer(2)]],
    device const float *cos_t   [[buffer(3)]],
    device const float *sin_t   [[buffer(4)]],
    device       float *k_cache [[buffer(5)]],
    device       float *v_cache [[buffer(6)]],
    device       float *out     [[buffer(7)]],
    constant     Params &p      [[buffer(8)]],
    uint q_h  [[threadgroup_position_in_grid]],
    uint lane [[thread_index_in_simdgroup]]
) {
    if (q_h >= NUM_HEADS) return;
    uint kv_h    = q_h / REPEAT;
    uint kv_src  = kv_h * HEAD_DIM;
    uint kv_dst  = kv_h * MAX_SEQ * HEAD_DIM + p.position * HEAD_DIM;
    uint kv_base = kv_h * MAX_SEQ * HEAD_DIM;

    // Step 1-2: RoPE-encode k_raw[kv_h] → k_cache[kv_h, position]
    //           Copy v_raw[kv_h]         → v_cache[kv_h, position]
    // Multiple Q-heads sharing kv_h write the same values (idempotent — OK).
    // Within-thread ordering ensures our stores are visible to our subsequent loads.
    {
        float x0 = k_raw[kv_src + lane],      x2 = k_raw[kv_src + lane + HALF];
        float c0 = cos_t[lane],               s0 = sin_t[lane];
        k_cache[kv_dst + lane]        = x0*c0 - x2*s0;
        k_cache[kv_dst + lane + HALF] = x0*s0 + x2*c0;
    }
    {
        float x1 = k_raw[kv_src + lane + 32], x3 = k_raw[kv_src + lane + 96];
        float c1 = cos_t[lane + 32],           s1 = sin_t[lane + 32];
        k_cache[kv_dst + lane + 32]   = x1*c1 - x3*s1;
        k_cache[kv_dst + lane + 96]   = x1*s1 + x3*c1;
    }
    v_cache[kv_dst + lane]        = v_raw[kv_src + lane];
    v_cache[kv_dst + lane + 32]   = v_raw[kv_src + lane + 32];
    v_cache[kv_dst + lane + HALF] = v_raw[kv_src + lane + HALF];
    v_cache[kv_dst + lane + 96]   = v_raw[kv_src + lane + 96];

    // Step 3: RoPE q_raw[q_h] into registers.
    float qr[ITERS];
    {
        uint q_base = q_h * HEAD_DIM;
        float x0 = q_raw[q_base + lane],      x2 = q_raw[q_base + lane + HALF];
        float c0 = cos_t[lane],               s0 = sin_t[lane];
        qr[0] = x0*c0 - x2*s0;
        qr[2] = x0*s0 + x2*c0;

        float x1 = q_raw[q_base + lane + 32], x3 = q_raw[q_base + lane + 96];
        float c1 = cos_t[lane + 32],           s1 = sin_t[lane + 32];
        qr[1] = x1*c1 - x3*s1;
        qr[3] = x1*s1 + x3*c1;
    }

    // Step 4: Flash-decode SDPA (online softmax, sequential pass over KV cache).
    uint window_start = (p.window > 0u && p.total_seq > p.window)
        ? (p.total_seq - p.window) : 0u;

    float max_sf = -INFINITY, sum_sf = 0.0f;
    float acc[ITERS] = {0.0f, 0.0f, 0.0f, 0.0f};

    for (uint s = 0; s < p.total_seq; s++) {
        uint kv_off = kv_base + s * HEAD_DIM;
        float dot = 0.0f;
        for (uint i = 0; i < ITERS; i++) {
            dot = fma(qr[i], k_cache[kv_off + lane + i * 32u], dot);
        }
        float score = simd_sum(dot) * p.scale;
        if (s < window_start) score = -1.0e4f;

        float new_max = max(max_sf, score);
        float corr    = exp(max_sf - new_max);
        float es      = exp(score  - new_max);
        sum_sf = fma(sum_sf, corr, es);
        for (uint i = 0; i < ITERS; i++) {
            acc[i] = fma(es, v_cache[kv_off + lane + i * 32u], acc[i] * corr);
        }
        max_sf = new_max;
    }

    float inv_sum = 1.0f / sum_sf;
    uint out_off = q_h * HEAD_DIM;
    for (uint i = 0; i < ITERS; i++) {
        out[out_off + lane + i * 32u] = acc[i] * inv_sum;
    }
}
"#;

pub fn fused_rope_kv_attn_msl_for(num_heads: usize, kv_heads: usize, max_seq: usize) -> String {
    FUSED_ROPE_KV_ATTN_TEMPLATE
        .replace("__NUM_HEADS__", &num_heads.to_string())
        .replace("__KV_HEADS__", &kv_heads.to_string())
        .replace("__MAX_SEQ__", &max_seq.to_string())
}

#[allow(dead_code)]
pub const MSL: &str = ""; // built dynamically via msl_for
