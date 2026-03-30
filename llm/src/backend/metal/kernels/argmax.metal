// Argmax — find index of max value in fp16 array
// Single workgroup, 256 threads, simd_max reduction

#include <metal_stdlib>
using namespace metal;

struct ArgmaxParams { uint n; };

kernel void argmax_f16(device const half *input [[buffer(0)]],
                       device uint *result [[buffer(1)]],
                       constant ArgmaxParams &p [[buffer(2)]],
                       uint lid [[thread_index_in_threadgroup]],
                       uint simd_lane [[thread_index_in_simdgroup]],
                       uint simd_group [[simdgroup_index_in_threadgroup]]) {

    threadgroup float max_vals[8];
    threadgroup uint max_idxs[8];

    float local_max = -1e20f;
    uint local_idx = 0u;

    for (uint i = lid; i < p.n; i += 256u) {
        float v = float(input[i]);
        if (v > local_max) {
            local_max = v;
            local_idx = i;
        }
    }

    // Simdgroup reduction (max)
    for (uint offset = 16u; offset > 0u; offset >>= 1u) {
        float other_val = simd_shuffle_xor(local_max, offset);
        uint other_idx = simd_shuffle_xor(local_idx, offset);
        if (other_val > local_max) {
            local_max = other_val;
            local_idx = other_idx;
        }
    }

    if (simd_lane == 0u) {
        max_vals[simd_group] = local_max;
        max_idxs[simd_group] = local_idx;
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    // Final reduction across simdgroups
    if (lid == 0u) {
        uint num_sgs = 256u / 32u;
        float best = max_vals[0];
        uint best_idx = max_idxs[0];
        for (uint s = 1u; s < num_sgs; s++) {
            if (max_vals[s] > best) {
                best = max_vals[s];
                best_idx = max_idxs[s];
            }
        }
        result[0] = best_idx;
    }
}
