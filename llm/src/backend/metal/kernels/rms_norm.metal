// RMS Norm — one workgroup per row, simd_sum reduction
// Accumulates sum-of-squares in f32 for precision, output in fp16
// Supports multi-row dispatch (num_rows workgroups) for QK norm

#include <metal_stdlib>
using namespace metal;

struct NormParams { uint hidden; float eps; };

kernel void rms_norm_f16(device const half *input [[buffer(0)]],
                         device const half *weight [[buffer(1)]],
                         device half *output [[buffer(2)]],
                         constant NormParams &p [[buffer(3)]],
                         uint group_id [[threadgroup_position_in_grid]],
                         uint lid [[thread_index_in_threadgroup]],
                         uint simd_lane [[thread_index_in_simdgroup]],
                         uint simd_group [[simdgroup_index_in_threadgroup]]) {

    threadgroup float partial[8]; // max 8 simdgroups

    uint row = group_id;
    uint base = row * p.hidden;

    // Phase 1: sum of squares (f32 accumulation)
    float sum_sq = 0.0f;
    for (uint i = lid; i < p.hidden; i += 256u) {
        float v = float(input[base + i]);
        sum_sq += v * v;
    }
    sum_sq = simd_sum(sum_sq);

    if (simd_lane == 0u) { partial[simd_group] = sum_sq; }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (simd_group == 0u && simd_lane < 8u) {
        sum_sq = (simd_lane < (256u / 32u)) ? partial[simd_lane] : 0.0f;
        sum_sq = simd_sum(sum_sq);
    }

    if (lid == 0u) { partial[0] = sum_sq; }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    float rms = rsqrt(partial[0] / float(p.hidden) + p.eps);

    // Phase 2: normalize and scale
    for (uint i = lid; i < p.hidden; i += 256u) {
        float v = float(input[base + i]);
        output[base + i] = half(v * rms * float(weight[i]));
    }
}
