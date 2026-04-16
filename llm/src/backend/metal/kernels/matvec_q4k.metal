// Q4_K matvec (decode) — llama.cpp compatible format
// Q4_K: 256-element super-blocks with 6-bit sub-block scales.
//
// struct block_q4_K {
//     half d;            // super-block scale (2 bytes)
//     half dmin;         // super-block minimum (2 bytes)
//     uint8_t scales[12]; // 8 sub-block scales + 8 mins, 6-bit packed (12 bytes)
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

// get_scale_min_k4: unpack 6-bit scale and min for sub-block j (0..7)
// j < 4:  sc = scales[j] & 63,                     m = scales[j+4] & 63
// j >= 4: sc = (scales[j+4] & 0xF) | ((scales[j-4] >> 6) << 4)
//         m  = (scales[j+4] >> 4)  | ((scales[j]   >> 6) << 4)
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

                acc1  += x0 * float(qb & 0xFu);
                xsum1 += x0;
                acc2  += x1 * float(qb >> 4u);
                xsum2 += x1;
            }

            sum += d1 * acc1 - m1v * xsum1 + d2 * acc2 - m2v * xsum2;
        }
    }
    Y[col] = half(sum);
}
