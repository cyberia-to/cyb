// Q2_K matvec (decode) — llama.cpp compatible format
// Q2_K: 256-element super-blocks with 2-bit quants.
//
// struct block_q2_K {
//     uint8_t scales[16]; // 4-bit packed scale + min per sub-block (16 bytes)
//     uint8_t qs[64];     // 2 bits per value (64 bytes)
//     half    d;          // super-block scale (2 bytes)
//     half    dmin;       // super-block minimum (2 bytes)
// } // = 84 bytes per 256 weights = 2.625 bits/weight
//
// Scale unpacking: scales[j] & 0xF = sub-block scale, scales[j] >> 4 = sub-block min
//
// Dequant: val = d * sc * q2_value - dmin * m
//   where q2_value = (qs_byte >> (2*position)) & 3

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };

struct block_q2_K {
    uint8_t scales[16];
    uint8_t qs[64];
    half    d;
    half    dmin;
};

kernel void matvec_q2k(device const half *X            [[buffer(0)]],
                       device const block_q2_K *B       [[buffer(1)]],
                       device half *Y                    [[buffer(2)]],
                       constant MatvecParams &p          [[buffer(3)]],
                       uint group_id [[threadgroup_position_in_grid]],
                       uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint bpc = p.K >> 8u;
    float sum = 0.0f;

    for (uint bk = 0; bk < bpc; bk++) {
        device const block_q2_K &blk = B[bk * p.N + col];
        float d    = float(blk.d);
        float dmin = float(blk.dmin);
        uint base_k = bk << 8u;

        // 16 sub-blocks of 16 values
        for (uint sb = 0; sb < 16u; sb++) {
            uint sc_byte = uint(blk.scales[sb]);
            float sc = float(sc_byte & 0xFu);
            float m  = float(sc_byte >> 4u);
            float ds = d * sc;
            float dm = dmin * m;

            for (uint l = 0; l < 16u; l++) {
                uint j = sb * 16u + l;
                // 4 values per byte, 2 bits each
                uint qs_byte = uint(blk.qs[j / 4u]);
                uint q2 = (qs_byte >> ((j % 4u) * 2u)) & 3u;

                sum += (ds * float(q2) - dm) * float(X[base_k + j]);
            }
        }
    }
    Y[col] = half(sum);
}
