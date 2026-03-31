// Ternary (BitNet) matvec — native 2-bit weights, safetensors layout
// Weights: {-1, 0, +1} packed 4 per byte along N dimension.
// Layout: [N/4, K] row-major — byte at W[n_packed * K + k] contains 4 outputs.
// Encoding: 0=-1, 1=0, 2=+1, extracted by (byte >> (sub*2)) & 3, then -1.
// NR=8 rows per workgroup (2 packed rows × 4 sub-neurons each).
// 4 simdgroups sharing same n_packed get L1 cache hits on same bytes.

#include <metal_stdlib>
using namespace metal;

struct TernaryParams { uint N; uint K; float scale; };

constant uint NR = 8u;

kernel void matvec_ternary(device const half *X   [[buffer(0)]],
                           device const uchar *W   [[buffer(1)]],
                           device half *Y          [[buffer(2)]],
                           constant TernaryParams &p [[buffer(3)]],
                           uint wg_id [[threadgroup_position_in_grid]],
                           uint lid [[thread_index_in_threadgroup]],
                           uint simd_lane [[thread_index_in_simdgroup]],
                           uint simd_group [[simdgroup_index_in_threadgroup]]) {

    uint out_row = wg_id * NR + simd_group;
    if (out_row >= p.N) return;

    uint n_packed = out_row >> 2u;
    uint sub = out_row & 3u;
    uint shift = sub << 1u;

    uint row_offset = n_packed * p.K;
    float sum = 0.0f;

    // Vectorized: read 4 weight bytes at once (= 4 weights for our sub-neuron)
    uint K4 = p.K >> 2u;
    for (uint k4 = simd_lane; k4 < K4; k4 += 32u) {
        uint base_k = k4 << 2u;

        // Read 4 bytes as one uint (single memory transaction)
        uint packed4 = *(device const uint *)(W + row_offset + base_k);
        int v0 = int((packed4 >> shift) & 3u) - 1;
        int v1 = int(((packed4 >> 8u) >> shift) & 3u) - 1;
        int v2 = int(((packed4 >> 16u) >> shift) & 3u) - 1;
        int v3 = int(((packed4 >> 24u) >> shift) & 3u) - 1;

        float4 xv = float4(*(device const half4 *)(X + base_k));
        sum += xv.x * float(v0) + xv.y * float(v1)
             + xv.z * float(v2) + xv.w * float(v3);
    }

    sum = simd_sum(sum);
    if (simd_lane == 0u) {
        Y[out_row] = half(sum * p.scale);
    }
}
