// Fused residual add + RMS norm — 2 ops in 1 dispatch
// out_residual = a + b  (write back to residual buffer)
// out_normed = rms_norm(out_residual, weight)

#include <metal_stdlib>
using namespace metal;

struct AddNormParams { uint hidden; float eps; };

kernel void fused_add_norm_f16(
    device const half *a [[buffer(0)]],           // residual
    device const half *b [[buffer(1)]],           // delta to add
    device half *out_residual [[buffer(2)]],      // a + b (can alias a)
    device const half *norm_weight [[buffer(3)]], // norm scale
    device half *out_normed [[buffer(4)]],        // normalized output
    constant AddNormParams &p [[buffer(5)]],
    uint lid [[thread_index_in_threadgroup]],
    uint simd_lane [[thread_index_in_simdgroup]],
    uint simd_group [[simdgroup_index_in_threadgroup]])
{
    threadgroup float partial[8];

    // Phase 1: add + compute sum of squares
    float sum_sq = 0.0f;
    for (uint i = lid; i < p.hidden; i += 256u) {
        float val = float(a[i]) + float(b[i]);
        out_residual[i] = half(val);
        sum_sq += val * val;
    }

    // Reduction
    sum_sq = simd_sum(sum_sq);
    if (simd_lane == 0u) partial[simd_group] = sum_sq;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    if (simd_group == 0u && simd_lane < 8u) {
        sum_sq = (simd_lane < (256u / 32u)) ? partial[simd_lane] : 0.0f;
        sum_sq = simd_sum(sum_sq);
    }
    if (lid == 0u) partial[0] = sum_sq;
    threadgroup_barrier(mem_flags::mem_threadgroup);

    float rms = rsqrt(partial[0] / float(p.hidden) + p.eps);

    // Phase 2: normalize
    for (uint i = lid; i < p.hidden; i += 256u) {
        float val = float(out_residual[i]);
        out_normed[i] = half(val * rms * float(norm_weight[i]));
    }
}
