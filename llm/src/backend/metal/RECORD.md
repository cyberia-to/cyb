# Metal backend performance record

M1 Pro 16-core GPU, 1.296 GHz, 200 GB/s, 24MB SLC.
measured with GPU timestamps (MTLCommandBuffer.GPUStartTime/EndTime).

## peak numbers

| kernel | GFLOPS | tok/s | vs llama.cpp |
|--------|--------|-------|-------------|
| matmul_f16 (sustained) | 3,708 | — | best known on Metal |
| matmul_f16 (cold peak) | 3,800 | — | 90.1% of MMA ceiling |
| matmul_q4 (prefill) | 3,204 | — | +60% |
| matvec_q4 batch=1 | 242 | 29 | -17% |
| matvec_q4 batch=8 | 714 | 83 | +137% |
| matvec_ternary batch=1 | 140 | 16 | N/A |
| matvec_ternary batch=8 | 906 | 105 | 3× equiv |
| matvec_q4k v1 | 112 | 13 | needs opt |

## best kernel config (matmul_f16)

- BM=64, BN=64, BK=32
- tA[64][33] pad+1, tB[32][66] pad+2 = 8448 bytes TG
- 16 simdgroups, 512 threads
- simdgroup_half8x8 acc[2][2] per sg
- hand-unrolled 4× MMA per K-step
- half4 vectorized cooperative loads, bitshift addressing
- no bounds checking for aligned dimensions

## 25 tested optimizations

### what works
- simdgroup_multiply_accumulate (+35%)
- half4 vectorized loads (+23%)
- no bounds checking (+30% — biggest single win)
- half accumulation (+6%)
- hand-unrolled MMA (+4%)
- bitshift addressing (+4%)
- pad+1/+2 TG memory (+1.3%)
- batched decode dequant-once-dot-many (+137%)
- row-major Q4 layout (+10% matvec)
- GPU timestamps revealed +5% hidden perf

### what doesn't work on Apple Silicon
- half8 (doesn't exist in Metal)
- double buffering (2× TG memory → occupancy loss)
- 128×128 tiles (register pressure)
- persistent kernel (Apple scheduler better)
- direct device load (no TG reuse)
- stream-K (atomic contention)
- extract_bits (not faster than mask+shift)
- SIMD shuffle dequant (no sharing in batch=1)
- LUT dequant (bank conflicts)
- transposed B layout (TG write scatter > device read benefit)
- BK=64 (occupancy loss)
- pad+0/+0 4 TG/core (bank conflicts > occupancy gain)

## hardware facts
- fp16 = fp32 throughput on M1+ (Apple doubled fp32 pipelines)
- simdgroup_matrix = 79.5% of ALU peak (hardware limit)
- max 3 TG/core at 8448 bytes (32KB / 8448 = 3)
- command buffer overhead = 0.2ms fixed
- SLC prefetcher handles strided device access well
- Apple MPP (tensor_ops) = M5+ only, simdgroup optimal for M1-M4
