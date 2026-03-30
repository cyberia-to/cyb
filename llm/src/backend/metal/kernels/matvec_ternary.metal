// Ternary (BitNet) matvec — 906 GOPS batch=8, 105 tok/s
// Weights: {-1, 0, +1}, packed 4 per byte (2 bits each).
// Branchless: sign = (t & 1) - (t >> 1)

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };

kernel void matvec_ternary(device const half *X   [[buffer(0)]],
                           device const uchar *W   [[buffer(1)]],
                           device half *Y          [[buffer(2)]],
                           constant MatvecParams &p [[buffer(3)]],
                           uint group_id [[threadgroup_position_in_grid]],
                           uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint k_packed = p.K >> 2u;
    float sum = 0.0f;

    for (uint i = 0; i < k_packed; i += 4u) {
        uint base_k = i << 2u;
        device const half4 *X4 = (device const half4 *)(X + base_k);

        for (uint j = 0; j < 4u; j++) {
            uint8_t packed = W[(i + j) * p.N + col];

            // Branchless: sign = (t & 1) - (t >> 1)
            float s0 = float(int(packed & 1u) - int((packed >> 1u) & 1u));
            float s1 = float(int((packed >> 2u) & 1u) - int((packed >> 3u) & 1u));
            float s2 = float(int((packed >> 4u) & 1u) - int((packed >> 5u) & 1u));
            float s3 = float(int((packed >> 6u) & 1u) - int((packed >> 7u) & 1u));

            float4 xv = float4(X4[j]);
            sum += xv.x * s0 + xv.y * s1 + xv.z * s2 + xv.w * s3;
        }
    }
    Y[col] = half(sum);
}
