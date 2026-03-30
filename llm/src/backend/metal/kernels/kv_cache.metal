// KV cache ops — append new KV and GQA head expansion

#include <metal_stdlib>
using namespace metal;

struct AppendParams {
    uint head_dim;
    uint num_kv_heads;
    uint write_pos;  // past_seq_len — where to write new KV
    uint max_seq;
};

// Write new K/V at position write_pos in the pre-allocated cache
// cache: [num_kv_heads, max_seq, head_dim] fp16
// new_kv: [num_kv_heads * head_dim] fp16
kernel void kv_append_f16(device const half *new_kv [[buffer(0)]],
                          device half *cache [[buffer(1)]],
                          constant AppendParams &p [[buffer(2)]],
                          uint gid [[thread_position_in_grid]]) {
    uint total = p.num_kv_heads * p.head_dim;
    if (gid >= total) return;

    uint head = gid / p.head_dim;
    uint d = gid % p.head_dim;
    uint cache_idx = head * p.max_seq * p.head_dim + p.write_pos * p.head_dim + d;
    cache[cache_idx] = new_kv[gid];
}

struct ExpandParams {
    uint head_dim;
    uint num_heads;     // query heads
    uint num_kv_heads;  // kv heads (fewer)
    uint total_seq;
    uint max_seq;
};

// GQA expansion: repeat kv_heads to match num_heads
// src: [num_kv_heads, max_seq, head_dim] (cache, only total_seq valid)
// dst: [num_heads, total_seq, head_dim]
kernel void kv_expand_f16(device const half *src [[buffer(0)]],
                          device half *dst [[buffer(1)]],
                          constant ExpandParams &p [[buffer(2)]],
                          uint gid [[thread_position_in_grid]]) {
    uint total = p.num_heads * p.total_seq * p.head_dim;
    if (gid >= total) return;

    uint head = gid / (p.total_seq * p.head_dim);
    uint rem = gid % (p.total_seq * p.head_dim);
    uint t = rem / p.head_dim;
    uint d = rem % p.head_dim;

    uint gqa_ratio = p.num_heads / p.num_kv_heads;
    uint kv_head = head / gqa_ratio;

    dst[gid] = src[kv_head * p.max_seq * p.head_dim + t * p.head_dim + d];
}
