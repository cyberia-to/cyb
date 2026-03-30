// Q4_0 matvec — cooperative simd reduction, 8 rows per workgroup
// Each simdgroup (32 threads) handles one output row cooperatively.
// 8 simdgroups = 8 rows = 256 threads per workgroup.
//
// For K=896 (28 Q4 blocks): each thread handles ~1 block. 87.5% util.
// For K=4864 (152 blocks): each thread handles ~5 blocks. 100% util.

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };
struct block_q4_0 { half scale; uint8_t qs[16]; };

constant uint NR = 8u;  // rows per workgroup (= simdgroups per workgroup)

kernel void matvec_q4_fast(device const half *X        [[buffer(0)]],
                           device const block_q4_0 *B   [[buffer(1)]],
                           device half *Y               [[buffer(2)]],
                           constant MatvecParams &p      [[buffer(3)]],
                           uint wg_id [[threadgroup_position_in_grid]],
                           uint lid [[thread_index_in_threadgroup]],
                           uint simd_lane [[thread_index_in_simdgroup]],
                           uint simd_group [[simdgroup_index_in_threadgroup]]) {

    uint row = wg_id * NR + simd_group;
    if (row >= p.N) return;

    uint bpc = p.K >> 5u;  // blocks per column = K / 32
    float sum = 0.0f;

    // Each of 32 threads in the simdgroup handles blocks [simd_lane, simd_lane+32, ...]
    for (uint bk = simd_lane; bk < bpc; bk += 32u) {
        device const block_q4_0 &blk = B[bk * p.N + row];
        float scale = float(blk.scale);
        uint base_k = bk << 5u;
        device const half4 *X4 = (device const half4 *)(X + base_k);

        float acc = 0.0f;
        for (uint j = 0u; j < 4u; j++) {
            float4 x0 = float4(X4[j * 2u]);
            float4 x1 = float4(X4[j * 2u + 1u]);
            uint8_t b0 = blk.qs[j * 4u];
            uint8_t b1 = blk.qs[j * 4u + 1u];
            uint8_t b2 = blk.qs[j * 4u + 2u];
            uint8_t b3 = blk.qs[j * 4u + 3u];
            float4 w0 = float4(float(int(b0 & 0xFu) - 8), float(int(b0 >> 4u) - 8),
                               float(int(b1 & 0xFu) - 8), float(int(b1 >> 4u) - 8));
            float4 w1 = float4(float(int(b2 & 0xFu) - 8), float(int(b2 >> 4u) - 8),
                               float(int(b3 & 0xFu) - 8), float(int(b3 >> 4u) - 8));
            acc += dot(x0, w0) + dot(x1, w1);
        }
        sum += scale * acc;
    }

    // simd reduction — combine 32 partial sums into one
    sum = simd_sum(sum);

    // First thread in simdgroup writes the result
    if (simd_lane == 0u) {
        Y[row] = half(sum);
    }
}
