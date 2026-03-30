// Embedding lookup — token ID → fp16 hidden vector
// Grid: ceil(hidden / 256), one workgroup per hidden chunk

#include <metal_stdlib>
using namespace metal;

struct EmbedParams { uint hidden; };

kernel void embed_f16(device const half *table [[buffer(0)]],
                      device const uint *token_id [[buffer(1)]],
                      device half *out [[buffer(2)]],
                      constant EmbedParams &p [[buffer(3)]],
                      uint gid [[thread_position_in_grid]]) {
    if (gid >= p.hidden) return;
    uint tid = token_id[0];
    out[gid] = table[tid * p.hidden + gid];
}
