// Q4_0 batched matvec — simd cooperative + dequant-once-dot-many
// Combines matvec_q4_fast (simd reduction per row) with batch (multiple X rows).
// 8 simdgroups per workgroup, each handles 1 output row for ALL batch items.
//
// BATCH defined at compile time.

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };
struct block_q4_0 { half scale; uint8_t qs[16]; };

#ifndef BATCH
#define BATCH 4
#endif

constant uint NR = 8u;

kernel void matvec_q4_fast_batch(
    device const half *X        [[buffer(0)]],  // [BATCH, K]
    device const block_q4_0 *W  [[buffer(1)]],  // [K/32, N]
    device half *Y              [[buffer(2)]],  // [BATCH, N]
    constant MatvecParams &p    [[buffer(3)]],
    uint wg_id [[threadgroup_position_in_grid]],
    uint lid [[thread_index_in_threadgroup]],
    uint simd_lane [[thread_index_in_simdgroup]],
    uint simd_group [[simdgroup_index_in_threadgroup]])
{
    uint row = wg_id * NR + simd_group;
    if (row >= p.N) return;

    uint bpc = p.K >> 5u;
    float sums[BATCH] = {};

    for (uint bk = simd_lane; bk < bpc; bk += 32u) {
        device const block_q4_0 &blk = W[bk * p.N + row];
        float scale = float(blk.scale);
        uint base_k = bk << 5u;

        // Dequant once
        float w[32];
        for (uint j = 0u; j < 16u; j++) {
            uint8_t qb = blk.qs[j];
            w[j*2]   = float(int(qb & 0xFu) - 8);
            w[j*2+1] = float(int(qb >> 4u) - 8);
        }

        // Dot with each batch row
        for (uint r = 0u; r < BATCH; r++) {
            device const half *Xr = X + r * p.K + base_k;
            float acc = 0.0f;
            for (uint j = 0u; j < 8u; j++) {
                float4 xv = float4(*(device const half4 *)(Xr + j * 4u));
                acc += xv.x * w[j*4] + xv.y * w[j*4+1]
                     + xv.z * w[j*4+2] + xv.w * w[j*4+3];
            }
            sums[r] += scale * acc;
        }
    }

    // simd reduction per batch item
    for (uint r = 0u; r < BATCH; r++) {
        sums[r] = simd_sum(sums[r]);
    }

    if (simd_lane == 0u) {
        for (uint r = 0u; r < BATCH; r++) {
            Y[r * p.N + row] = half(sums[r]);
        }
    }
}
