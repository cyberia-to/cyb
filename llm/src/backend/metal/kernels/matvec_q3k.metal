// Q3_K matvec (decode) — llama.cpp compatible format
// Q3_K: 256-element super-blocks with 3-bit quants.
//
// struct block_q3_K {
//     uint8_t hmask[32];  // high bit for each of 256 values (32 bytes)
//     uint8_t qs[64];     // low 2 bits per value (64 bytes)
//     uint8_t scales[12]; // 6-bit packed scales (12 bytes)
//     half    d;          // super-block scale (2 bytes)
// } // = 110 bytes per 256 weights = 3.4375 bits/weight
//
// Scale unpacking (DIFFERENT from Q4_K):
//   16 sub-blocks of 16 values, each with a 6-bit signed scale (subtract 32).
//
// Dequant: val = d * sc * (q3_value - 4)
//   where q3_value = (ql_2bits | (hm_1bit << 2))

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };

struct block_q3_K {
    uint8_t hmask[32];
    uint8_t qs[64];
    uint8_t scales[12];
    half    d;
};

// Unpack Q3_K scale for sub-block j (0..15)
// Returns signed scale (after subtracting 32)
static inline float get_q3k_scale(uint j, device const uint8_t *scales) {
    uint us;
    if (j < 4u) {
        us = (scales[j] & 0xFu) | (((scales[8u + (j >> 1u)] >> (4u * (j & 1u))) & 3u) << 4u);
    } else if (j < 8u) {
        uint jj = j - 4u;
        us = (scales[4u + jj] & 0xFu) | (((scales[10u + (jj >> 1u)] >> (4u * (jj & 1u))) & 3u) << 4u);
    } else if (j < 12u) {
        uint jj = j - 8u;
        us = (scales[jj] >> 4u) | (((scales[8u + (jj >> 1u)] >> (4u * (jj & 1u) + 2u)) & 3u) << 4u);
    } else {
        uint jj = j - 12u;
        us = (scales[4u + jj] >> 4u) | (((scales[10u + (jj >> 1u)] >> (4u * (jj & 1u) + 2u)) & 3u) << 4u);
    }
    return float(int(us) - 32);
}

kernel void matvec_q3k(device const half *X            [[buffer(0)]],
                       device const block_q3_K *B       [[buffer(1)]],
                       device half *Y                    [[buffer(2)]],
                       constant MatvecParams &p          [[buffer(3)]],
                       uint group_id [[threadgroup_position_in_grid]],
                       uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint bpc = p.K >> 8u;
    float sum = 0.0f;

    for (uint bk = 0; bk < bpc; bk++) {
        device const block_q3_K &blk = B[bk * p.N + col];
        float d = float(blk.d);
        uint base_k = bk << 8u;

        // 16 sub-blocks of 16 values
        for (uint sb = 0; sb < 16u; sb++) {
            float sc = get_q3k_scale(sb, blk.scales);
            float ds = d * sc;

            for (uint l = 0; l < 16u; l++) {
                uint j = sb * 16u + l;

                // low 2 bits: 4 values per byte
                uint qs_byte = uint(blk.qs[j / 4u]);
                uint ql = (qs_byte >> ((j % 4u) * 2u)) & 3u;

                // high bit from hmask
                uint hm = (uint(blk.hmask[j / 8u]) >> (j % 8u)) & 1u;

                int q3 = int(ql | (hm << 2u)) - 4;
                sum += ds * float(q3) * float(X[base_k + j]);
            }
        }
    }
    Y[col] = half(sum);
}
