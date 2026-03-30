// Rotary Position Embedding — per-head rotation
// Input: [num_heads * head_dim] fp16 (Q or K vector)
// cos/sin: [half_dim] fp16 (single position's cache)
// Output: [num_heads * head_dim] fp16
//
// Each thread handles one (head, dim_pair) — rotates x[i] and x[i + half_dim]
// within the same head.

#include <metal_stdlib>
using namespace metal;

struct RopeParams {
    uint half_dim;   // head_dim / 2
    uint head_dim;   // full head dimension
    uint num_heads;  // number of heads to process
};

kernel void rope_f16(device const half *input [[buffer(0)]],
                     device const half *cos_cache [[buffer(1)]],
                     device const half *sin_cache [[buffer(2)]],
                     device half *output [[buffer(3)]],
                     constant RopeParams &p [[buffer(4)]],
                     uint gid [[thread_position_in_grid]]) {
    uint total = p.num_heads * p.half_dim;
    if (gid >= total) return;

    uint head = gid / p.half_dim;
    uint d = gid % p.half_dim;

    uint base = head * p.head_dim;
    float c = float(cos_cache[d]);
    float s = float(sin_cache[d]);
    float x0 = float(input[base + d]);
    float x1 = float(input[base + d + p.half_dim]);

    output[base + d] = half(x0 * c - x1 * s);
    output[base + d + p.half_dim] = half(x1 * c + x0 * s);
}
