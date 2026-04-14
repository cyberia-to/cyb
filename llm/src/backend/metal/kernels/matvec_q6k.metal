// Q6_K matvec (decode) — llama.cpp compatible format
// Q6_K: 256-element super-blocks with int8 sub-block scales.
//
// struct block_q6_K {
//     uint8_t ql[128];    // lower 4 bits of quants (128 bytes)
//     uint8_t qh[64];     // upper 2 bits of quants (64 bytes)
//     int8_t  scales[16]; // sub-block scales, int8 (16 bytes)
//     half    d;          // super-block scale (2 bytes)
// } // = 210 bytes per 256 weights = 6.5625 bits/weight
//
// Dequant for value j (0..255):
//   ql_byte = ql[j/2], ql_nib = low if j%2==0, high if j%2==1
//   qh_bit  = (qh[j/4] >> ((j%4)*2)) & 3
//   q = (qh_bit << 4) | ql_nib
//   scale = int8(scales[j/16])
//   value = d * scale * (q - 32)

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };

struct block_q6_K {
    uint8_t ql[128];
    uint8_t qh[64];
    int8_t  scales[16];
    half    d;
};

kernel void matvec_q6k(device const half *X            [[buffer(0)]],
                       device const block_q6_K *B       [[buffer(1)]],
                       device half *Y                    [[buffer(2)]],
                       constant MatvecParams &p          [[buffer(3)]],
                       uint group_id [[threadgroup_position_in_grid]],
                       uint lid [[thread_index_in_threadgroup]]) {

    uint col = group_id * 256u + lid;
    if (col >= p.N) return;

    uint bpc = p.K >> 8u;  // blocks per col = K/256
    float sum = 0.0f;

    for (uint bk = 0; bk < bpc; bk++) {
        device const block_q6_K &blk = B[bk * p.N + col];
        float d = float(blk.d);
        uint base_k = bk << 8u;

        // Linear j formula matching CPU dequant
        for (uint j = 0; j < 256u; j++) {
            uint8_t ql_byte = blk.ql[j / 2u];
            uint ql_nib = (j % 2u == 0u) ? uint(ql_byte & 0xFu) : uint(ql_byte >> 4u);
            uint qh_bit = (uint(blk.qh[j / 4u]) >> ((j % 4u) * 2u)) & 3u;
            uint q = (qh_bit << 4u) | ql_nib;
            float sc = float(blk.scales[j / 16u]);
            sum += d * sc * (float(q) - 32.0f) * float(X[base_k + j]);
        }
    }
    Y[col] = half(sum);
}
