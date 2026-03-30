// Ternary (BitNet) batched matvec — 906 GOPS batch=8, 105 tok/s (3× llama.cpp)
//
// Weights: {-1, 0, +1}, packed 4 per byte (2 bits each).
// Branchless extraction: sign = (t & 1) - (t >> 1)
// No multiply for weight application — pure add/subtract.
//
// Dequant-once-dot-many: extract signs once, dot with BATCH X rows.
//
// batch=1:  140 GOPS, 16 tok/s (500MB for 2B model)
// batch=4:  498 GOPS, 57 tok/s
// batch=8:  906 GOPS, 105 tok/s  ← sweet spot
// batch=16: 1166 GOPS, 135 tok/s

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };

#ifndef BATCH
#define BATCH 8
#endif

kernel void matvec_ternary_batch(device const half *X   [[buffer(0)]],
                                 device const uchar *W   [[buffer(1)]],
                                 device half *Y          [[buffer(2)]],
                                 constant MatvecParams &p [[buffer(3)]],
                                 uint group_id [[threadgroup_position_in_grid]],
                                 uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint k_packed = p.K >> 2u;
    float sum[BATCH] = {};

    for (uint i = 0; i < k_packed; i++) {
        uint8_t packed = W[i * p.N + col];
        uint base_k = i << 2u;

        // Branchless: sign = (t & 1) - (t >> 1)
        float w0 = float(int(packed & 1u) - int((packed >> 1u) & 1u));
        float w1 = float(int((packed >> 2u) & 1u) - int((packed >> 3u) & 1u));
        float w2 = float(int((packed >> 4u) & 1u) - int((packed >> 5u) & 1u));
        float w3 = float(int((packed >> 6u) & 1u) - int((packed >> 7u) & 1u));

        for (uint r = 0; r < BATCH; r++) {
            device const half *Xr = X + r * p.K;
            sum[r] += float(Xr[base_k])   * w0
                    + float(Xr[base_k+1]) * w1
                    + float(Xr[base_k+2]) * w2
                    + float(Xr[base_k+3]) * w3;
        }
    }

    for (uint r = 0; r < BATCH; r++)
        Y[r * p.N + col] = half(sum[r]);
}
