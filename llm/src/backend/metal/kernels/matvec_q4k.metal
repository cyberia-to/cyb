// Q4_K matvec (decode) — llama.cpp compatible format
// 112 GFLOPS v1 (needs optimization: sub-block processing)
//
// Q4_K: 256-element super-blocks with 6-bit sub-block scales.
// struct block_q4_K {
//     half d;            // super-block scale (2 bytes)
//     half dmin;         // super-block minimum (2 bytes)
//     uint8_t scales[12]; // 16 sub-block scales, 6-bit packed (12 bytes)
//     uint8_t qs[128];   // 256 × 4-bit nibbles (128 bytes)
// } // = 144 bytes per 256 weights = 4.5 bits/weight

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };

struct block_q4_K {
    half d;
    half dmin;
    uint8_t scales[12];
    uint8_t qs[128];
};

kernel void matvec_q4k(device const half *X            [[buffer(0)]],
                       device const block_q4_K *B       [[buffer(1)]],
                       device half *Y                    [[buffer(2)]],
                       constant MatvecParams &p          [[buffer(3)]],
                       uint group_id [[threadgroup_position_in_grid]],
                       uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint bpc = p.K >> 8u;  // blocks per col = K/256
    float sum = 0.0f;

    for (uint bk = 0; bk < bpc; bk++) {
        device const block_q4_K &blk = B[bk * p.N + col];
        float d = float(blk.d);
        uint base_k = bk << 8u;

        float acc = 0.0f;
        // 16 sub-blocks of 16 weights each
        for (uint sb = 0; sb < 16u; sb++) {
            uint8_t sc_byte = blk.scales[sb < 8u ? sb : sb - 8u + 4u];
            float sc = float(sc_byte & 0x3Fu);

            float sub_acc = 0.0f;
            uint qs_off = sb * 8u;

            for (uint j = 0; j < 8u; j++) {
                uint8_t qb = blk.qs[qs_off + j];
                uint k_idx = base_k + sb * 16u + j * 2u;

                float x0 = float(X[k_idx]);
                float x1 = float(X[k_idx + 1u]);
                float w0 = float(int(qb & 0xFu));
                float w1 = float(int(qb >> 4u));

                sub_acc += x0 * w0 + x1 * w1;
            }
            acc += d * sc * sub_acc;
        }
        sum += acc;
    }
    Y[col] = half(sum);
}
