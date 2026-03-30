// Fused gate+up projection — 2 matvec_q4 in 1 dispatch
// Both read same input (post-normed hidden), write to separate outputs.

#include <metal_stdlib>
using namespace metal;

struct GateUpParams {
    uint N;     // intermediate_size (same for both)
    uint K;     // hidden_size
    uint wg_gate; // workgroups for gate = ceil(N / 8)
};
struct block_q4_0 { half scale; uint8_t qs[16]; };

constant uint NR = 8u;

kernel void fused_gate_up_q4(
    device const half *X            [[buffer(0)]],
    device const block_q4_0 *W_gate [[buffer(1)]],
    device const block_q4_0 *W_up   [[buffer(2)]],
    device half *gate_out           [[buffer(3)]],
    device half *up_out             [[buffer(4)]],
    constant GateUpParams &p        [[buffer(5)]],
    uint wg_id [[threadgroup_position_in_grid]],
    uint lid [[thread_index_in_threadgroup]],
    uint simd_lane [[thread_index_in_simdgroup]],
    uint simd_group [[simdgroup_index_in_threadgroup]])
{
    device const block_q4_0 *W;
    device half *Y;
    uint local_wg;

    if (wg_id < p.wg_gate) {
        W = W_gate; Y = gate_out; local_wg = wg_id;
    } else {
        W = W_up; Y = up_out; local_wg = wg_id - p.wg_gate;
    }

    uint row = local_wg * NR + simd_group;
    if (row >= p.N) return;

    uint bpc = p.K >> 5u;
    float sum = 0.0f;

    for (uint bk = simd_lane; bk < bpc; bk += 32u) {
        device const block_q4_0 &blk = W[bk * p.N + row];
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

    sum = simd_sum(sum);
    if (simd_lane == 0u) {
        Y[row] = half(sum);
    }
}
