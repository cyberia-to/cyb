// fp16 matvec — Y[N] = X[K] @ W[N,K]^T (all fp16)
// For LM head projection (embed_table × hidden → vocab logits)
// 256 threads, one threadgroup per 4 output rows (NR=4)

#include <metal_stdlib>
using namespace metal;

struct MatvecParams { uint N; uint K; };

kernel void f16_matvec(device const half *X [[buffer(0)]],
                       device const half *W [[buffer(1)]],
                       device half *Y [[buffer(2)]],
                       constant MatvecParams &p [[buffer(3)]],
                       uint group_id [[threadgroup_position_in_grid]],
                       uint lid [[thread_index_in_threadgroup]],
                       uint simd_lane [[thread_index_in_simdgroup]],
                       uint simd_group [[simdgroup_index_in_threadgroup]]) {

    threadgroup float partial[32]; // 8 simdgroups × 4 rows
    uint base_row = group_id * 4u;
    uint num_sgs = 256u / 32u;

    float sums[4] = {0.0f, 0.0f, 0.0f, 0.0f};

    for (uint i = lid; i < p.K; i += 256u) {
        float x = float(X[i]);
        for (uint r = 0u; r < 4u; r++) {
            uint row = base_row + r;
            if (row < p.N) {
                sums[r] += x * float(W[row * p.K + i]);
            }
        }
    }

    for (uint r = 0u; r < 4u; r++) {
        sums[r] = simd_sum(sums[r]);
    }

    if (simd_lane == 0u) {
        for (uint r = 0u; r < 4u; r++) {
            partial[simd_group * 4u + r] = sums[r];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (simd_group == 0u && simd_lane < num_sgs) {
        for (uint r = 0u; r < 4u; r++) {
            sums[r] = partial[simd_lane * 4u + r];
        }
        for (uint r = 0u; r < 4u; r++) {
            sums[r] = simd_sum(sums[r]);
        }
        if (simd_lane == 0u) {
            for (uint r = 0u; r < 4u; r++) {
                uint row = base_row + r;
                if (row < p.N) {
                    Y[row] = half(sums[r]);
                }
            }
        }
    }
}
