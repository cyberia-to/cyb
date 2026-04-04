// Argmax — find index of max value in fp16 array
// Two-dispatch approach (called from Rust):
//   1. argmax_partial: each workgroup finds local max, writes to partial buffers
//   2. argmax_reduce: single workgroup reduces partials to final result
// This avoids cross-workgroup atomic sync which is unreliable in Metal.

#include <metal_stdlib>
using namespace metal;

struct ArgmaxParams { uint n; uint num_groups; };

// Pass 1: each workgroup scans its chunk, writes partial max to device buffers
kernel void argmax_partial(device const half *input [[buffer(0)]],
                           constant ArgmaxParams &p [[buffer(1)]],
                           device float *partial_vals [[buffer(2)]],
                           device uint *partial_idxs [[buffer(3)]],
                           uint lid [[thread_index_in_threadgroup]],
                           uint group_id [[threadgroup_position_in_grid]],
                           uint simd_lane [[thread_index_in_simdgroup]],
                           uint simd_group [[simdgroup_index_in_threadgroup]]) {

    threadgroup float max_vals[8];
    threadgroup uint max_idxs[8];

    uint chunk = (p.n + p.num_groups - 1u) / p.num_groups;
    uint start = group_id * chunk;
    uint end = min(start + chunk, p.n);

    float local_max = -1e20f;
    uint local_idx = 0u;

    for (uint i = start + lid; i < end; i += 256u) {
        float v = float(input[i]);
        if (v > local_max) {
            local_max = v;
            local_idx = i;
        }
    }

    // Simdgroup reduction
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

    // Reduce across simdgroups within this workgroup
    if (lid == 0u) {
        float best = max_vals[0];
        uint best_idx = max_idxs[0];
        for (uint s = 1u; s < 8u; s++) {
            if (max_vals[s] > best) {
                best = max_vals[s];
                best_idx = max_idxs[s];
            }
        }
        partial_vals[group_id] = best;
        partial_idxs[group_id] = best_idx;
    }
}

// Pass 2: single workgroup reduces partial results to final answer
kernel void argmax_reduce(device float *partial_vals [[buffer(0)]],
                          device uint *partial_idxs [[buffer(1)]],
                          device uint *result [[buffer(2)]],
                          constant ArgmaxParams &p [[buffer(3)]],
                          uint lid [[thread_index_in_threadgroup]]) {
    // Only thread 0 does the work (num_groups is small, ~64)
    if (lid != 0u) return;

    float best = partial_vals[0];
    uint best_idx = partial_idxs[0];
    for (uint g = 1u; g < p.num_groups; g++) {
        if (partial_vals[g] > best) {
            best = partial_vals[g];
            best_idx = partial_idxs[g];
        }
    }
    result[0] = best_idx;
}
