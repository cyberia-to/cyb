// Q4_0 matvec (decode) — 242 GFLOPS single, 714 GFLOPS batch=8
// Row-major B layout [K/32][N] for coalesced reads.
// 256 threads, each handles 1 output column.

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };
struct block_q4_0 { half scale; uint8_t qs[16]; };

// Single decode: 1 × K × K × N
kernel void matvec_q4(device const half *X        [[buffer(0)]],
                      device const block_q4_0 *B   [[buffer(1)]],
                      device half *Y               [[buffer(2)]],
                      constant MatvecParams &p      [[buffer(3)]],
                      uint group_id [[threadgroup_position_in_grid]],
                      uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint bpc = p.K >> 5u;
    float sum = 0.0f;

    for (uint bk = 0; bk < bpc; bk++) {
        device const block_q4_0 &blk = B[bk * p.N + col];
        float scale = float(blk.scale);
        uint base_k = bk << 5u;
        device const half4 *X4 = (device const half4 *)(X + base_k);

        float acc = 0.0f;
        for (uint j = 0; j < 4u; j++) {
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
    Y[col] = half(sum);
}
