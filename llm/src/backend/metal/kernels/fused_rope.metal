// Fused RoPE for Q and K in one dispatch — saves 1 barrier per layer
// Q: [num_q_heads * head_dim], K: [num_kv_heads * head_dim]
// Both get rotated with same cos/sin position cache

#include <metal_stdlib>
using namespace metal;

struct FusedRopeParams {
    uint half_dim;     // head_dim / 2
    uint head_dim;
    uint num_q_heads;
    uint num_kv_heads;
};

kernel void fused_rope_qk(device half *q [[buffer(0)]],
                          device half *k [[buffer(1)]],
                          device const half *cos_cache [[buffer(2)]],
                          device const half *sin_cache [[buffer(3)]],
                          constant FusedRopeParams &p [[buffer(4)]],
                          uint gid [[thread_position_in_grid]]) {
    uint total_q = p.num_q_heads * p.half_dim;
    uint total_k = p.num_kv_heads * p.half_dim;
    uint total = total_q + total_k;
    if (gid >= total) return;

    // Determine if this thread handles Q or K
    device half *buf;
    uint local_gid;
    uint num_heads;
    if (gid < total_q) {
        buf = q;
        local_gid = gid;
        num_heads = p.num_q_heads;
    } else {
        buf = k;
        local_gid = gid - total_q;
        num_heads = p.num_kv_heads;
    }

    uint head = local_gid / p.half_dim;
    uint d = local_gid % p.half_dim;
    uint base = head * p.head_dim;

    float c = float(cos_cache[d]);
    float s = float(sin_cache[d]);
    float x0 = float(buf[base + d]);
    float x1 = float(buf[base + d + p.half_dim]);

    buf[base + d] = half(x0 * c - x1 * s);
    buf[base + d + p.half_dim] = half(x1 * c + x0 * s);
}
