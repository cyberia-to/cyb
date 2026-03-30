// Fused KV cache append — write both K and V in one dispatch

#include <metal_stdlib>
using namespace metal;

struct DualAppendParams {
    uint head_dim;
    uint num_kv_heads;
    uint write_pos;
    uint max_seq;
};

kernel void fused_kv_append(device const half *new_k [[buffer(0)]],
                            device const half *new_v [[buffer(1)]],
                            device half *cache_k [[buffer(2)]],
                            device half *cache_v [[buffer(3)]],
                            constant DualAppendParams &p [[buffer(4)]],
                            uint gid [[thread_position_in_grid]]) {
    uint total = p.num_kv_heads * p.head_dim;
    if (gid >= total) return;

    uint head = gid / p.head_dim;
    uint d = gid % p.head_dim;
    uint idx = head * p.max_seq * p.head_dim + p.write_pos * p.head_dim + d;

    cache_k[idx] = new_k[gid];
    cache_v[idx] = new_v[gid];
}
