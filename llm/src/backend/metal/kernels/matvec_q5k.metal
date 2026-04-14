// Q5_K matvec (decode) — llama.cpp compatible format
// Q5_K: 256-element super-blocks with 5-bit quants + 6-bit sub-block scales.
//
// struct block_q5_K {
//     half    d;           // super-block scale (2 bytes)
//     half    dmin;        // super-block minimum (2 bytes)
//     uint8_t scales[12];  // 8 sub-block scales + 8 mins, 6-bit packed (12 bytes)
//     uint8_t qh[32];     // 5th bit for each of 256 values (32 bytes)
//     uint8_t qs[128];    // low 4 bits of each value (128 bytes)
// } // = 176 bytes per 256 weights = 5.5 bits/weight
//
// Dequant: val = d * sc * (nibble | (qh_bit << 4)) - dmin * m

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };

struct block_q5_K {
    half    d;
    half    dmin;
    uint8_t scales[12];
    uint8_t qh[32];
    uint8_t qs[128];
};

// get_scale_min_k4: same 6-bit packing as Q4_K
static inline void get_scale_min_k4(uint j, device const uint8_t *scales,
                                     thread float &sc, thread float &m) {
    if (j < 4u) {
        sc = float(scales[j] & 63u);
        m  = float(scales[j + 4u] & 63u);
    } else {
        sc = float((scales[j + 4u] & 0xFu) | ((scales[j - 4u] >> 6u) << 4u));
        m  = float((scales[j + 4u] >> 4u)  | ((scales[j]      >> 6u) << 4u));
    }
}

kernel void matvec_q5k(device const half *X            [[buffer(0)]],
                       device const block_q5_K *B       [[buffer(1)]],
                       device half *Y                    [[buffer(2)]],
                       constant MatvecParams &p          [[buffer(3)]],
                       uint group_id [[threadgroup_position_in_grid]],
                       uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint bpc = p.K >> 8u;  // blocks per col = K/256
    float sum = 0.0f;

    for (uint bk = 0; bk < bpc; bk++) {
        device const block_q5_K &blk = B[bk * p.N + col];
        float d    = float(blk.d);
        float dmin = float(blk.dmin);
        uint base_k = bk << 8u;

        // 4 groups of 64 values, each group = 2 sub-blocks of 32
        for (uint grp = 0; grp < 4u; grp++) {
            float sc1, m1, sc2, m2;
            get_scale_min_k4(grp * 2u,     blk.scales, sc1, m1);
            get_scale_min_k4(grp * 2u + 1u, blk.scales, sc2, m2);

            float d1   = d * sc1;
            float m1v  = dmin * m1;
            float d2   = d * sc2;
            float m2v  = dmin * m2;

            uint qs_off = grp * 32u;
            uint k_base = base_k + grp * 64u;

            float acc1 = 0.0f;
            float acc2 = 0.0f;
            float xsum1 = 0.0f;
            float xsum2 = 0.0f;

            for (uint l = 0; l < 32u; l++) {
                uint8_t qb = blk.qs[qs_off + l];
                float x0 = float(X[k_base + l]);
                float x1 = float(X[k_base + 32u + l]);

                // 5th bit from qh
                uint lo_idx = grp * 64u + l;
                uint hi_idx = grp * 64u + 32u + l;
                uint lo_qh = (uint(blk.qh[lo_idx / 8u]) >> (lo_idx % 8u)) & 1u;
                uint hi_qh = (uint(blk.qh[hi_idx / 8u]) >> (hi_idx % 8u)) & 1u;

                uint lo_q = uint(qb & 0xFu) | (lo_qh << 4u);
                uint hi_q = uint(qb >> 4u)  | (hi_qh << 4u);

                acc1  += x0 * float(lo_q);
                xsum1 += x0;
                acc2  += x1 * float(hi_q);
                xsum2 += x1;
            }

            sum += d1 * acc1 - m1v * xsum1 + d2 * acc2 - m2v * xsum2;
        }
    }
    Y[col] = half(sum);
}
