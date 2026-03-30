// Q4_0 batched matvec (decode) — 714 GFLOPS batch=8, 83 tok/s (2.4× llama.cpp)
//
// Key insight: dequant-once-dot-many.
// Each thread dequantizes one Q4 block (32 weights) into registers,
// then dots with BATCH X rows. Dequant cost amortized by batch factor.
//
// batch=1: 242 GFLOPS, 29 tok/s
// batch=2: 451 GFLOPS, 52 tok/s
// batch=4: 575 GFLOPS, 66 tok/s
// batch=8: 714 GFLOPS, 83 tok/s  ← sweet spot (2.4× llama.cpp)
// batch=16: 814 GFLOPS, 94 tok/s (register pressure starts)
//
// BATCH is a compile-time constant (template parameter via format!).
// Row-major B layout [K/32][N] for coalesced reads.

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };
struct block_q4_0 { half scale; uint8_t qs[16]; };

// BATCH must be defined at compile time: -DBATCH=8
#ifndef BATCH
#define BATCH 8
#endif

kernel void matvec_q4_batch(device const half *X        [[buffer(0)]],
                            device const block_q4_0 *W   [[buffer(1)]],
                            device half *Y               [[buffer(2)]],
                            constant MatvecParams &p      [[buffer(3)]],
                            uint group_id [[threadgroup_position_in_grid]],
                            uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint bpc = p.K >> 5u;
    float sum[BATCH] = {};

    for (uint bk = 0; bk < bpc; bk++) {
        device const block_q4_0 &blk = W[bk * p.N + col];
        float scale = float(blk.scale);
        uint base_k = bk << 5u;

        // Dequant once → 32 weights in registers
        float w[32];
        for (uint j = 0; j < 16u; j++) {
            uint8_t qb = blk.qs[j];
            w[j*2]   = float(int(qb & 0xFu) - 8);
            w[j*2+1] = float(int(qb >> 4u) - 8);
        }

        // Dot with each batch row
        for (uint r = 0; r < BATCH; r++) {
            device const half *Xr = X + r * p.K + base_k;
            float acc = 0.0f;
            for (uint j = 0; j < 8u; j++) {
                float4 xv = float4(*(device const half4 *)(Xr + j * 4u));
                acc += xv.x * w[j*4] + xv.y * w[j*4+1]
                     + xv.z * w[j*4+2] + xv.w * w[j*4+3];
            }
            sum[r] += scale * acc;
        }
    }

    for (uint r = 0; r < BATCH; r++)
        Y[r * p.N + col] = half(sum[r]);
}
