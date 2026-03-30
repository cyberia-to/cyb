// fp16 matmul — best configuration from rmetal benchmarks
// 3708 GFLOPS GPU sustained on M1 Pro 16-core (87.9% of MMA ceiling)
//
// BM=64 BN=64 BK=32, pad+1/+2 (8448B TG memory)
// 16 simdgroups, 512 threads
// simdgroup_half8x8 acc[2][2] per sg
// hand-unrolled 4× MMA per K-step
// half4 vectorized cooperative loads, bitshift addressing

#include <metal_stdlib>
#include <metal_simdgroup_matrix>
using namespace metal;

struct MatmulParams { uint M; uint N; uint K; };

kernel void matmul_f16(device const half *A [[buffer(0)]],
                       device const half *B [[buffer(1)]],
                       device half *C       [[buffer(2)]],
                       constant MatmulParams &p [[buffer(3)]],
                       uint2 group_id [[threadgroup_position_in_grid]],
                       uint sgid [[simdgroup_index_in_threadgroup]],
                       uint lid [[thread_index_in_threadgroup]]) {

    threadgroup half tA[64][33];  // BK + pad(1)
    threadgroup half tB[32][66];  // BN + pad(2)
    uint grow = group_id.y << 6u;
    uint gcol = group_id.x << 6u;
    uint sg_row = (sgid >> 2u) << 4u;
    uint sg_col = (sgid & 3u) << 4u;

    simdgroup_half8x8 acc[2][2];
    for (uint i = 0; i < 2u; i++)
        for (uint j = 0; j < 2u; j++)
            acc[i][j] = make_filled_simdgroup_matrix<half, 8>(half(0));

    device const half4 *A4 = (device const half4 *)A;
    device const half4 *B4 = (device const half4 *)B;
    uint a4s = p.K >> 2u;
    uint b4s = p.N >> 2u;

    for (uint t = 0; t < p.K; t += 32u) {
        // A load: half4 vectorized
        {
            uint r = lid >> 3u;
            uint c4 = lid & 7u;
            half4 v = A4[(grow + r) * a4s + (t >> 2u) + c4];
            uint bc = c4 << 2u;
            tA[r][bc] = v.x; tA[r][bc+1] = v.y;
            tA[r][bc+2] = v.z; tA[r][bc+3] = v.w;
        }
        // B load: half4 vectorized
        {
            uint r = lid >> 4u;
            uint c4 = lid & 15u;
            half4 v = B4[(t + r) * b4s + (gcol >> 2u) + c4];
            uint bc = c4 << 2u;
            tB[r][bc] = v.x; tB[r][bc+1] = v.y;
            tB[r][bc+2] = v.z; tB[r][bc+3] = v.w;
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);

        // Hand-unrolled 4× MMA
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

        MMA_STEP(0);
        MMA_STEP(8);
        MMA_STEP(16);
        MMA_STEP(24);
        #undef MMA_STEP

        threadgroup_barrier(mem_flags::mem_threadgroup);
    }

    for (uint i = 0; i < 2u; i++)
        for (uint j = 0; j < 2u; j++)
            simdgroup_store(acc[i][j],
                C + (grow + sg_row + i*8u) * p.N + (gcol + sg_col + j*8u), p.N);
}
