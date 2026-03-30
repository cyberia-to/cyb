// Attention decode — one workgroup per query head, online softmax
// Supports GQA: multiple query heads share same KV head
// Q: [num_heads * head_dim] fp16
// K, V: [kv_num_heads * total_seq * head_dim] fp16 (from KV cache)
// Output: [num_heads * head_dim] fp16

#include <metal_stdlib>
using namespace metal;

struct AttnParams {
    uint head_dim;
    uint total_seq;   // actual sequence length (past + 1)
    uint num_heads;
    float scale;
    uint kv_num_heads; // for GQA
    uint max_seq;      // KV cache stride (pre-allocated size)
};

kernel void attention_decode_f16(
    device const half *q [[buffer(0)]],
    device const half *k [[buffer(1)]],
    device const half *v [[buffer(2)]],
    device half *output [[buffer(3)]],
    constant AttnParams &p [[buffer(4)]],
    uint group_id [[threadgroup_position_in_grid]],
    uint lid [[thread_index_in_threadgroup]],
    uint simd_lane [[thread_index_in_simdgroup]],
    uint simd_group [[simdgroup_index_in_threadgroup]])
{
    threadgroup float scores[2048];
    threadgroup float partial[8];

    uint head = group_id;
    if (head >= p.num_heads) return;

    // GQA: map query head → kv head
    uint gqa_ratio = p.num_heads / p.kv_num_heads;
    uint kv_head = head / gqa_ratio;

    uint q_base = head * p.head_dim;
    uint kv_base = kv_head * p.max_seq * p.head_dim;
    uint num_sgs = 256u / 32u;

    // Step 1: Q·K^T scores
    float local_max = -1e20f;
    for (uint t = lid; t < p.total_seq; t += 256u) {
        float dot = 0.0f;
        for (uint d = 0u; d < p.head_dim; d++) {
            dot += float(q[q_base + d]) * float(k[kv_base + t * p.head_dim + d]);
        }
        dot *= p.scale;
        scores[t] = dot;
        local_max = max(local_max, dot);
    }

    // Max reduction
    local_max = simd_max(local_max);
    if (simd_lane == 0u) partial[simd_group] = local_max;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    if (simd_group == 0u && simd_lane < num_sgs) {
        local_max = partial[simd_lane];
        local_max = simd_max(local_max);
    }
    if (lid == 0u) partial[0] = local_max;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float global_max = partial[0];

    // Step 2: exp + sum
    float local_sum = 0.0f;
    for (uint t = lid; t < p.total_seq; t += 256u) {
        float e = exp(scores[t] - global_max);
        scores[t] = e;
        local_sum += e;
    }

    local_sum = simd_sum(local_sum);
    if (simd_lane == 0u) partial[simd_group] = local_sum;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    if (simd_group == 0u && simd_lane < num_sgs) {
        local_sum = partial[simd_lane];
        local_sum = simd_sum(local_sum);
    }
    if (lid == 0u) partial[0] = local_sum;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float inv_sum = 1.0f / partial[0];

    // Normalize
    for (uint t = lid; t < p.total_seq; t += 256u) {
        scores[t] *= inv_sum;
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    // Step 3: weighted sum of V
    for (uint d = lid; d < p.head_dim; d += 256u) {
        float sum = 0.0f;
        for (uint t = 0u; t < p.total_seq; t++) {
            sum += scores[t] * float(v[kv_base + t * p.head_dim + d]);
        }
        output[q_base + d] = half(sum);
    }
}
