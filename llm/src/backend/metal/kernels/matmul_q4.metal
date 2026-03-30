// Q4_0 matmul (prefill) — 3204 GFLOPS on M1 Pro
// Dequantize in cooperative load phase, simdgroup MMA compute.
// BM=64 BN=64 BK=32 (one Q4 block = 32 elements)

#include <metal_stdlib>
#include <metal_simdgroup_matrix>
using namespace metal;

struct MatmulParams { uint M; uint N; uint K; };
struct block_q4_0 { half scale; uint8_t qs[16]; };

kernel void matmul_q4(device const half *A          [[buffer(0)]],
                      device const block_q4_0 *B     [[buffer(1)]],
                      device half *C                  [[buffer(2)]],
                      constant MatmulParams &p        [[buffer(3)]],
                      uint2 group_id [[threadgroup_position_in_grid]],
                      uint sgid [[simdgroup_index_in_threadgroup]],
                      uint lid [[thread_index_in_threadgroup]]) {

    threadgroup half tA[64][33];
    threadgroup half tB[32][66];

    uint grow = group_id.y << 6u;
    uint gcol = group_id.x << 6u;
    uint sg_row = (sgid >> 2u) << 4u;
    uint sg_col = (sgid & 3u) << 4u;

    simdgroup_half8x8 acc[2][2];
    for (uint i = 0; i < 2u; i++)
        for (uint j = 0; j < 2u; j++)
            acc[i][j] = make_filled_simdgroup_matrix<half, 8>(half(0));

    device const half4 *A4 = (device const half4 *)A;
    uint a4s = p.K >> 2u;

    for (uint t = 0; t < p.K; t += 32u) {
        uint block_k = t >> 5u;

        // A: half4 vectorized
        {
            uint r = lid >> 3u;
            uint c4 = lid & 7u;
            half4 v = A4[(grow + r) * a4s + (t >> 2u) + c4];
            uint bc = c4 << 2u;
            tA[r][bc] = v.x; tA[r][bc+1] = v.y;
            tA[r][bc+2] = v.z; tA[r][bc+3] = v.w;
        }
        // B: dequant Q4 blocks, row-major B[block_k * N + col]
        {
            uint bn_idx = lid >> 3u;
            uint sub = lid & 7u;
            uint col = gcol + bn_idx;
            device const block_q4_0 &blk = B[block_k * p.N + col];
            half scale = blk.scale;
            uint bo = sub << 1u;
            uint ko = sub << 2u;
            uint8_t b0 = blk.qs[bo];
            uint8_t b1 = blk.qs[bo + 1u];
            tB[ko    ][bn_idx] = scale * half(int(b0 & 0xFu) - 8);
            tB[ko + 1][bn_idx] = scale * half(int(b0 >> 4u) - 8);
            tB[ko + 2][bn_idx] = scale * half(int(b1 & 0xFu) - 8);
            tB[ko + 3][bn_idx] = scale * half(int(b1 >> 4u) - 8);
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);

        simdgroup_half8x8 at0, at1, bt0, bt1;
        #define MMA_STEP(KK) \
        simdgroup_load(at0, &tA[sg_row][KK], 33); \
        simdgroup_load(at1, &tA[sg_row+8u][KK], 33); \
        simdgroup_load(bt0, &tB[KK][sg_col], 66); \
        simdgroup_load(bt1, &tB[KK][sg_col+8u], 66); \
        simdgroup_multiply_accumulate(acc[0][0], at0, bt0, acc[0][0]); \
        simdgroup_multiply_accumulate(acc[0][1], at0, bt1, acc[0][1]); \
        simdgroup_multiply_accumulate(acc[1][0], at1, bt0, acc[1][0]); \
        simdgroup_multiply_accumulate(acc[1][1], at1, bt1, acc[1][1]);
        MMA_STEP(0); MMA_STEP(8); MMA_STEP(16); MMA_STEP(24);
        #undef MMA_STEP

        threadgroup_barrier(mem_flags::mem_threadgroup);
    }

    for (uint i = 0; i < 2u; i++)
        for (uint j = 0; j < 2u; j++)
            simdgroup_store(acc[i][j],
                C + (grow + sg_row + i*8u) * p.N + (gcol + sg_col + j*8u), p.N);
}
