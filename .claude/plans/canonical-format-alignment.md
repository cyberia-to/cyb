# Canonical .model format alignment

## Goal

Bring the live `import/` writer + `run/` reader into alignment with
the canonical `.model` spec at
[`cyb/cyb-model`](https://cyber.page/cyb/cyb-model)
(source: `cyb/root/cyb-model.md`).

## Background

The canonical spec has been the authoritative `.model` definition
since before the current code. The implementation took the path of
least resistance — kept GGUF K-quants as-is, used floats in weights
and config — and accumulated divergences. The MVP plan calls for
"verified-correct + fast" manifest models, but verification against
a non-canonical format would lock in the divergence.

## What canonical demands

| Dimension | Canonical |
|---|---|
| Encodings | five fixed: `u32`, `u16`, `q8`, `q4`, `ternary` |
| Floats | banned in weight storage and config |
| u32 layout | 16.16 fixed-point: `u32 = round(f32 * 65536)` |
| u16 layout | 8.8 fixed-point: `u16 = round(f * 256)` |
| q8 layout | 32-value blocks, `u16` scale + 32×i8, dequant `i8 * scale / 127` |
| q4 layout | 32-value blocks, `u16` scale + 32×4-bit, dequant `(nibble - 8) * scale / 8` |
| ternary layout | 2 bits per value: `00=0`, `01=+1`, `10=-1` |
| Sections | seven required: `card`, `config`, `program`, `tensors`, `vocab`, `eval`, `weights` |
| Eps storage | inverted integer: `rms_norm_eps = 1000000` ⇒ ε = 1e-6 |
| Sampling storage | per-mille integers: `temperature = 700` ⇒ 0.7 |
| Eval scores | per-mille: `score = 991` ⇒ 99.1% |
| Program | `.rs` (Rust → native binary). `.tri` is **out of MVP scope** |

## Where today diverges

| Dimension | Today (`import/cyb_format.rs` + `run/format.rs`) |
|---|---|
| Encodings | open: F16, Q4_K, Q5_K, Q6_K, Q8_0, IQ2/3/4 (GGUF set) |
| Floats | F16 is canonical for non-quantized weights; configs hold floats |
| u32 / u16 / q4 / q8 / ternary canonical | **not implemented** as encoders |
| Sections | seven + optional `~~~graph` (extension over canonical) |
| Eps storage | direct float `1e-6` |
| Sampling storage | floats |
| Eval scores | floats |

## Phasing

### Phase 0 — canonical encoders + decoders, round-trip proven (1 session)

- [ ] `import/quant.rs`: implement encoders for the canonical 5
      (`u32`, `u16`, `q8`, `q4`, `ternary`)
- [ ] `run/backend/cpu/quant.rs`: implement decoders for the same 5
- [ ] cross-test: f32 tensor → encoder → bytes → decoder → f32 ⇒
      tolerances per spec (q4: 1.5e-2/weight, q8: 4e-3/weight,
      u16: 4e-3/weight, u32: 1.5e-5/weight, ternary: as defined
      by the values themselves)
- [ ] property tests for boundary cases (zeros, max magnitude,
      NaN handling — input NaN aborts, never silently encodes)

Deliverable: a passing test suite proving the canonical encoders
and decoders agree, before any model touches them.

### Phase 1 — migrate qwen3-0.6b-abl end-to-end (2 sessions)

Smallest manifest model; smallest blast radius.

- [ ] `import` reads its current GGUF source
- [ ] All weights dequant to f32, then re-encode to canonical:
      - matmul weights → `q4`
      - layer norms / biases → `u32`
      - other f16 → `u16`
- [ ] `import` writes config in integer form
      (`rms_norm_eps = 1000000`, sampling per-mille)
- [ ] `import` strips the `~~~graph` section path (out of canonical;
      reintroduce as a real spec extension later if useful)
- [ ] `run/format.rs` reader updated to expect canonical config
      (parse integer eps, sampling)
- [ ] `run/backend/cpu` matmul accepts `q4` canonical layout
- [ ] forward pass produces tokens; compared bit-for-bit against
      the pre-alignment .model output on the same prompt — accept
      delta only if the delta is from the encoding precision
      change (numeric, not structural)

Deliverable: `qwen3-0.6b-abl.model` is a canonical .model file
that produces correct tokens through `run` on the cpu backend.

### Phase 2 — kernels across backends (2 sessions)

- [ ] `run/backend/wgpu`: WGSL kernels for `q4` / `q8` / `u16` matmul
      and dequant
- [ ] `run/backend/honeycrisp`: Metal kernels for the same
- [ ] cross-backend correctness check on qwen3-0.6b-abl

Deliverable: qwen3-0.6b-abl runs correctly on all three backends
against canonical weights.

### Phase 3 — remaining manifest models (2 sessions)

- [ ] `qwen2.5-coder-1.5b-abl` — proves the LlamaStyle attn_bias
      variant under canonical encoding
- [ ] `qwen2.5-coder-14b-abl` — proves the large-model load path
      with canonical `q4` (currently blocked on disk space; see
      operational note below)
- [ ] `gemma-4-31b` — also requires the LlamaStyle+ extras
      (softcapping, sliding window, K=V), independent of this
      alignment but unblocked by it

Deliverable: all four manifest models live as canonical `.model`
files, verified-correct on at least the cpu backend.

### Phase 4 — drop divergent code paths (1 session)

- [ ] `import/types.rs::DType` enum trimmed to canonical 5 +
      whatever source dtypes we still read from GGUF
- [ ] `run/backend/cpu/quant.rs` GGUF K-quant decoders removed
- [ ] `run/backend/wgpu` GGUF kernels removed
- [ ] `run/backend/honeycrisp` GGUF kernels removed
- [ ] specs swept: `run/specs/quant.md` and `run/specs/format.md`
      align with canonical (or are reduced to pointers into
      `cyb/cyb-model`)

Deliverable: no GGUF K-quant decoder lives in the runtime; the
runtime decodes only the canonical 5.

## Risks and open questions

- **u16 fixed-point range**. `u16 = round(f * 256)` saturates at
  ±128. Empirically check that no manifest-model weight exceeds
  this. If any does, the canonical spec needs an addendum (per-
  tensor scale, similar to GGUF blocks). Treat first overflow as
  a spec ambiguity to resolve, not a code workaround.
- **u32 fixed-point range**. `u32 = round(f * 65536)` saturates at
  ±32767.99. Norms and biases are well within this; no expected
  failure.
- **ternary scope**. None of the four manifest models is a BitNet
  variant, so `ternary` decoders are deferrable until Phase 4.
- **Round-trip identity**. The canonical encodings are lossy
  relative to F16/F32 source; we don't claim bit-identity, only
  correctness within tolerance. Verification must use a tolerance
  budget consistent with `run/specs/test.md` Tier 0.
- **Optional graph section**. The `~~~graph` extension layered on
  today's writer is not part of canonical. Either: (a) drop it
  during Phase 1 alignment and reintroduce later as a formal
  extension, (b) propose an addendum to `cyb/cyb-model` adding
  the optional `~~~graph` section. Defer the call to Phase 1 work.
- **`run/specs/format.md` scope**. After alignment, this spec
  largely duplicates `cyb/cyb-model`. Phase 4 either reduces it
  to runtime-only concerns (mmap layout, scan procedure) or
  redirects to the graph page.

## Estimate

~8 sessions (24 focused hours). The verification work in Phases
1–3 dominates; the encoder/decoder code is small.

## Out of scope

- `.tri` program emission. Canonical spec allows `.rs`; we ship
  with `.rs` and revisit `.tri` after MVP.
- Atomic .model write (temp file + rename). Orthogonal hardening.
- Weights blob checksum. Orthogonal hardening.

## How this fits the cyb-mvp plan

This work *precedes* the MVP plan's Phase 0 verification step.
Verifying the four manifest models against a non-canonical format
would lock in the divergence; aligning first and then verifying
makes the verification meaningful for the canonical pipeline.
