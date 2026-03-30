// Fused Q+K+V projection — 3 matvec_q4 in 1 dispatch
// All three read same input (normed hidden), write to separate outputs.
// Saves 2 dispatch barriers per layer.
//
// Uses simd cooperative reduction (same as matvec_q4_fast).
// Workgroup routing: first N_q/8 workgroups → Q, next N_k/8 → K, rest → V.

#include <metal_stdlib>
using namespace metal;

struct QKVParams {
    uint N_q;       // q_dim (num_heads * head_dim)
    uint N_k;       // kv_dim (kv_num_heads * head_dim)
    uint N_v;       // kv_dim (same as N_k usually)
    uint K;         // hidden_size (input dim)
    uint wg_q;      // workgroups for Q = ceil(N_q / 8)
    uint wg_k;      // workgroups for K = ceil(N_k / 8)
};
struct block_q4_0 { half scale; uint8_t qs[16]; };

constant uint NR = 8u;

kernel void fused_qkv_q4(
    device const half *X            [[buffer(0)]],  // normed hidden [K]
    device const block_q4_0 *W_q    [[buffer(1)]],  // [K/32, N_q]
    device const block_q4_0 *W_k    [[buffer(2)]],  // [K/32, N_k]
    device const block_q4_0 *W_v    [[buffer(3)]],  // [K/32, N_v]
    device half *Q                  [[buffer(4)]],  // [N_q]
    device half *K_out              [[buffer(5)]],  // [N_k]
    device half *V                  [[buffer(6)]],  // [N_v]
    constant QKVParams &p           [[buffer(7)]],
    uint wg_id [[threadgroup_position_in_grid]],
    uint lid [[thread_index_in_threadgroup]],
    uint simd_lane [[thread_index_in_simdgroup]],
    uint simd_group [[simdgroup_index_in_threadgroup]])
{
    // Route workgroup to Q, K, or V
    device const block_q4_0 *W;
    device half *Y;
    uint N;
    uint local_wg;

    if (wg_id < p.wg_q) {
        W = W_q; Y = Q; N = p.N_q; local_wg = wg_id;
    } else if (wg_id < p.wg_q + p.wg_k) {
        W = W_k; Y = K_out; N = p.N_k; local_wg = wg_id - p.wg_q;
    } else {
        W = W_v; Y = V; N = p.N_v; local_wg = wg_id - p.wg_q - p.wg_k;
    }

    uint row = local_wg * NR + simd_group;
    if (row >= N) return;

    uint bpc = p.K >> 5u;
    float sum = 0.0f;

    for (uint bk = simd_lane; bk < bpc; bk += 32u) {
        device const block_q4_0 &blk = W[bk * N + row];
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
