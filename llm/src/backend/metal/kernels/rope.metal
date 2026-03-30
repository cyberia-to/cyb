// Rotary Position Embedding — apply cos/sin rotation to Q/K
// Input/output: fp16, cos/sin cache: fp16

#include <metal_stdlib>
using namespace metal;

struct RopeParams { uint half_dim; uint total; };

kernel void rope_f16(device const half *input [[buffer(0)]],
                     device const half *cos_cache [[buffer(1)]],
                     device const half *sin_cache [[buffer(2)]],
                     device half *output [[buffer(3)]],
                     constant RopeParams &p [[buffer(4)]],
                     uint gid [[thread_position_in_grid]]) {
    if (gid >= p.half_dim) return;

    float c = float(cos_cache[gid]);
    float s = float(sin_cache[gid]);
    float x0 = float(input[gid]);
    float x1 = float(input[gid + p.half_dim]);

    output[gid] = half(x0 * c - x1 * s);
    output[gid + p.half_dim] = half(x1 * c + x0 * s);
}
