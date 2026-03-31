// Argmax — find index of max value in fp16 array
// Two-pass: parallel scan across workgroups, then final reduction
// Each workgroup scans a chunk and writes its local max to partial buffer.
// Last workgroup (detected via atomic counter) reduces the partials.

#include <metal_stdlib>
using namespace metal;

struct ArgmaxParams { uint n; uint num_groups; };

kernel void argmax_f16(device const half *input [[buffer(0)]],
                       device uint *result [[buffer(1)]],
                       constant ArgmaxParams &p [[buffer(2)]],
                       device float *partial_vals [[buffer(3)]],
                       device uint *partial_idxs [[buffer(4)]],
                       device atomic_uint *counter [[buffer(5)]],
                       uint lid [[thread_index_in_threadgroup]],
                       uint group_id [[threadgroup_position_in_grid]],
                       uint simd_lane [[thread_index_in_simdgroup]],
                       uint simd_group [[simdgroup_index_in_threadgroup]]) {

    threadgroup float max_vals[8];
    threadgroup uint max_idxs[8];

    // Each workgroup scans elements [group_id * chunk .. (group_id+1) * chunk)
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

    // Last workgroup to finish does the final reduction
    threadgroup_barrier(mem_flags::mem_device_and_threadgroup);
    if (lid == 0u) {
        uint finished = atomic_fetch_add_explicit(counter, 1u, memory_order_relaxed) + 1u;
        if (finished == p.num_groups) {
            // We are the last workgroup — reduce partials
            float best = partial_vals[0];
            uint best_idx = partial_idxs[0];
            for (uint g = 1u; g < p.num_groups; g++) {
                if (partial_vals[g] > best) {
                    best = partial_vals[g];
                    best_idx = partial_idxs[g];
                }
            }
            result[0] = best_idx;
            // Reset counter for next call
            atomic_store_explicit(counter, 0u, memory_order_relaxed);
        }
    }
}
