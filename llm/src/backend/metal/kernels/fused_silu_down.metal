// Fused SiLU×gate + down projection — 2 ops in 1 dispatch
// Computes: Y = down_proj(silu(gate) * up)
// Saves 1 dispatch + eliminates intermediate buffer write/read.
// SiLU activation computed inline in registers per block.

#include <metal_stdlib>
using namespace metal;

struct SiluDownParams { uint N; uint K; };
struct block_q4_0 { half scale; uint8_t qs[16]; };

constant uint NR = 8u;

kernel void fused_silu_down_q4(
    device const half *gate       [[buffer(0)]],  // [K] gate projection output
    device const half *up         [[buffer(1)]],  // [K] up projection output
    device const block_q4_0 *W    [[buffer(2)]],  // [K/32, N] down_proj weights
    device half *Y                [[buffer(3)]],  // [N] output
    constant SiluDownParams &p    [[buffer(4)]],
    uint wg_id [[threadgroup_position_in_grid]],
    uint lid [[thread_index_in_threadgroup]],
    uint simd_lane [[thread_index_in_simdgroup]],
    uint simd_group [[simdgroup_index_in_threadgroup]])
{
    uint row = wg_id * NR + simd_group;
    if (row >= p.N) return;

    uint bpc = p.K >> 5u;
    float sum = 0.0f;

    for (uint bk = simd_lane; bk < bpc; bk += 32u) {
        device const block_q4_0 &blk = W[bk * p.N + row];
        float scale = float(blk.scale);
        uint base_k = bk << 5u;

        // Read gate + up, compute SiLU activation inline, dot with dequantized weight
        float acc = 0.0f;
        for (uint j = 0u; j < 16u; j++) {
            uint8_t qb = blk.qs[j];
            float w0 = float(int(qb & 0xFu) - 8);
            float w1 = float(int(qb >> 4u) - 8);

            uint idx0 = base_k + j * 2u;
            uint idx1 = idx0 + 1u;

            float g0 = float(gate[idx0]);
            float g1 = float(gate[idx1]);
            float u0 = float(up[idx0]);
            float u1 = float(up[idx1]);

            // SiLU(g) * u = g * sigmoid(g) * u
            float s0 = g0 / (1.0f + exp(-g0)) * u0;
            float s1 = g1 / (1.0f + exp(-g1)) * u1;

            acc += s0 * w0 + s1 * w1;
        }
        sum += scale * acc;
    }

    sum = simd_sum(sum);
    if (simd_lane == 0u) {
        Y[row] = half(sum);
    }
}
