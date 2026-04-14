# Format Support Plan

Status: approved
Created: 2026-04-14

Every GGUF model must work out of the box. No silent degradation.

## Canonical formats

The runtime supports 8 weight formats. Import normalizes variations into these:

| Format | Bits | Superblock | Use |
|--------|------|-----------|-----|
| Q2_K   | 2    | 256 vals / 84B  | extreme compression |
| Q3_K   | 3    | 256 vals / 110B | small models |
| Q4_K   | 4    | 256 vals / 144B | standard (most common) |
| Q5_K   | 5    | 256 vals / 176B | quality models |
| Q6_K   | 6    | 256 vals / 210B | high precision layers |
| Q8     | 8    | 32 vals / 34B   | near-lossless |
| F16    | 16   | 1 val / 2B      | half precision |
| F32    | 32   | 1 val / 4B      | norms, biases |

## Import normalization

Old/variant formats are converted to canonical K-quant at import:

| Input | Output | Reason |
|-------|--------|--------|
| Q4_0  | Q4_K   | Q4_K is strictly better (sub-block scales) |
| Q4_1  | Q4_K   | Q4_K is strictly better |
| BF16  | F16    | Same precision, standard encoding |

Everything else passes through as-is. No cross-bit-width conversion.

## Implementation

### Phase 1: full K-quant chain (priority)

Each format needs 3 things:
1. **Dequant** in safetensors_to_f32 (for CPU fallback / format conversion)
2. **WGSL shader** (WGPU compute kernel)
3. **Metal kernel** (MSL compute kernel)

Current state:
- Q4_K: dequant ✓, WGSL ✓, Metal ✓
- Q6_K: dequant ✓, WGSL ✗, Metal ✗ (kernel exists but not wired for .model)
- Q2_K: dequant ✗, WGSL ✗, Metal ✗
- Q3_K: dequant ✗, WGSL ✗, Metal ✗
- Q5_K: dequant ✗, WGSL ✗, Metal ✗

All K-quant shaders share the same pattern:
- Read superblock header (d, dmin as f16)
- Unpack 6-bit sub-block scales via get_scale_min_k4
- Unpack N-bit quantized values
- Dequant: val = d * sc * q - dmin * m

The difference is only in how the quantized values are packed.

Build order (by frequency in real models):
1. Q6_K — present in EVERY Q*_K_M model
2. Q5_K — Q5_K_M is popular
3. Q3_K — Q3_K_M for small models
4. Q2_K — Q2_K for extreme compression

### Phase 2: import normalization
- [ ] Q4_0 → Q4_K conversion at import (dequant → requant)
- [ ] Q4_1 → Q4_K conversion at import
- [ ] BF16 → F16 conversion at import
- [ ] Validate: after import, .model only contains canonical formats
- [ ] Reject unknown formats with clear error

### Phase 3: runtime safety
- [ ] LoadedModel: validate all encodings on load
- [ ] WGPU: panic with format name if no shader (not silent zeros)
- [ ] Metal: same
- [ ] Status table: show encoding distribution per model

## Validation

A model is "supported" when:
1. Import succeeds without conversion warnings
2. Load succeeds on both backends
3. SANE=✓ on status bench
4. tok/s > 0 on both backends
