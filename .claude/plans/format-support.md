# Format Support Plan

Status: proposed
Created: 2026-04-14

Every GGUF model must work out of the box. No format should silently degrade or crash.

## Current state

| Format | GGUF | Import | WGPU | Metal | Real models |
|--------|------|--------|------|-------|-------------|
| F32    | ✓    | ✓      | ✓    | ✓     | all (norms) |
| F16    | ✓    | ✓      | ✓    | ✓     | some        |
| Q4_0   | ✓    | ✓      | ✓    | ✓     | qwen3-0.6b  |
| Q8_0   | ✓    | ✓      | ✓    | ✓     | some        |
| Q4_K   | ✓    | ✓      | ✓    | ✓     | qwen2.5-14b |
| Q6_K   | ✓    | →Q4_K  | ✗    | ✗     | mixed in Q4_K_M |
| Q4_1   | ✓    | →Q4    | ~    | ~     | rare        |
| Q2_K   | ✓    | stored | CRASH| CRASH | Q2_K GGUFs  |
| Q3_K   | ✓    | stored | CRASH| CRASH | Q3_K_M GGUFs|
| Q5_K   | ✓    | stored | CRASH| CRASH | Q5_K_M GGUFs|

## Root cause

The runtime was built for Q4_0 models. K-quant support was added piecemeal for Q4_K only. Other K-quant formats crash at runtime because:
1. No dequant code in safetensors_to_f32 (Q2_K, Q3_K, Q5_K)
2. No GPU compute shaders (only Q4_K has a WGSL/Metal kernel)
3. Import stores raw bytes but runtime can't process them

## The fix: normalize at import time

Instead of adding 5 more GPU shaders, normalize ALL weights to two formats at import:
- **Q4_K**: for all quantized 2D weight tensors (projections)
- **F32**: for all 1D tensors (norms, biases)

This means:
- Q2_K, Q3_K, Q5_K, Q6_K → dequant to f32 → requant to Q4_K
- Q4_1 → dequant to f32 → requant to Q4_K  
- Q4_0 → keep as-is (already supported natively)
- Q4_K → keep as-is (already supported natively)
- Q8_0 → dequant to f32 → requant to Q4_K (or keep as Q8 if Q8 shader exists)
- F16, BF16 → dequant to f32 → requant to Q4_K (or keep as F16)
- F32 → keep as-is

After import, the .model file contains ONLY: q4k, q4, u32, q8, u16, ternary.
Runtime needs ONLY: Q4_K shader, Q4_0 shader, Q8 shader, F32 shader, F16 shader.

## Implementation steps

### Phase 1: import normalization (1 session)
- [ ] Add dequant for Q2_K, Q3_K, Q5_K to safetensors_to_f32
- [ ] Import: convert ALL K-quant 2D tensors to Q4_K (not just Q6_K)
- [ ] Import: convert Q4_1 to Q4_0 or Q4_K
- [ ] Import: reject unsupported formats with clear error instead of silent fallback
- [ ] Test: import Q4_K_M GGUF → all tensors are q4k or u32
- [ ] Test: import Q3_K_M GGUF → same result

### Phase 2: runtime safety (1 session)
- [ ] LoadedModel: validate all tensor encodings before returning
- [ ] WGPU: explicit error if unknown QuantFormat in dispatch (not silent zero)
- [ ] Metal: same
- [ ] Test: load .model with invalid encoding → clear error message

### Phase 3: quality (1 session)
- [ ] Native Q6_K WGSL shader (avoids Q6_K→Q4_K quality loss)
- [ ] Native Q6_K Metal kernel (already exists: matvec_q4k.metal pattern)
- [ ] Import: keep Q6_K as q6k when native support exists
- [ ] Benchmark: Q4_K_M with native Q6_K vs all-Q4_K

## Validation checklist for "works out of the box"

Before any model is declared supported:
1. Import produces valid .model (all tensors readable)
2. LoadedModel.load() succeeds
3. Tokenizer builds (special tokens registered)
4. WGPU forward produces non-zero logits
5. Metal forward produces non-zero logits
6. Output is coherent text (SANE=✓ on status bench)
7. No silent format conversions that lose > 5% quality

## GGUF quant types in the wild

| GGUF variant | Contains | Popularity |
|-------------|----------|------------|
| Q4_0        | Q4_0 only | Common (old) |
| Q4_K_M      | Q4_K + Q6_K mixed | Most common |
| Q3_K_M      | Q3_K + Q4_K + Q6_K mixed | Medium |
| Q5_K_M      | Q5_K + Q6_K mixed | Medium |
| Q6_K        | Q6_K only | Less common |
| Q8_0        | Q8_0 only | Quality-focused |
| Q2_K        | Q2_K + Q4_K + Q6_K mixed | Smallest |

Every _M (mixed) variant contains Q6_K for sensitive layers. This is why native Q6_K support is the highest-value addition.
