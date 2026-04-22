# llm/experimental — parked techniques

These are non-trivial techniques from the retired `llm/` runtime that are
**not** currently wired into anything shipping (the live runtime is `mr/`),
but represent real hard-won knowledge we don't want to re-derive.

Everything here is preserved verbatim. Import paths and external deps may
be broken; the code is archival, not buildable as-is.

## Contents

### `polar_quant.rs` (881 LOC)
Two related KV-cache compression schemes:

- **PolarQuant** — applies a random orthogonal rotation (Gram-Schmidt QR)
  to each KV vector, then recursively decomposes via polar coordinates.
  Level-1 angles get 4 bits, levels 2-4 get 2 bits each, radius in f16.
  Target: 3.875 bits/value (~4.13× vs f16) with near-zero perplexity loss.

  **Known bug** at `compress()` (line ~226): averages per-block radii
  into one total, then `decompress()` rescales by that single value —
  incorrect for multi-block vectors. Must be fixed before shipping.

- **QJL** (Quantized Johnson-Lindenstrauss) — 1-bit keys for attention.
  Random projection down to m bits, store only sign; score via
  `sqrt(π/2) · ‖k‖ · ⟨Sq, sign(Sk)⟩ / m`, provably unbiased. Keys at
  ~1 bit/dim for head_dim=128. Combined with PolarQuant as `TurboQuantState`.

Test coverage exists (`test_roundtrip_identity`, `test_qjl_unbiased`),
but neither technique is wired into the `mr/` forward path.

### `kv_compress.rs` (210 LOC)
Orchestration layer on top of PolarQuant/QJL — `KvCompressor` that
decides per-layer whether to compress and stores sidecar metadata.

### `apple_iosurface.rs` (176 LOC)
Design sketch for a zero-copy Apple Silicon backend. Pins weights,
scratch, and KV history into one `IOSurface`-backed region so CPU
(AMX/NEON), GPU (Metal), and ANE address the same DRAM without copies.

`AppleEngine` has `layout`, `weight_map`, and a weight-loader path;
`forward()` is a stub. 64-byte alignment is enforced for AMX access.

### `ane_mil/` (652 LOC)
Apple Neural Engine MIL (Model Intermediate Language) codegen:

- `mil/sdpa.rs` — emits MIL programs for forward and backward SDPA,
  including QKV projection, RoPE, GQA head-tiling (via `concat`
  repeated gqa times), causal masking, and softmax.
- `mil/ffn.rs`, `mil/projection.rs`, `mil/rane_mil.rs` — other
  building blocks.
- `surface.rs` — IOSurface wrapper (with NEON inline-asm bulk
  f16↔f32 at 8 values/iter via `fcvtl`/`fcvtn`).

Generators are complete. `AneModel::compile → load → run` plumbing
exists in `rane_api.rs` (kept in llm/src/backend/ane/ for now since
it's still referenced by the old runtime).

### `METAL_TUNING_RECORD.md`
25-experiment optimization log from the retired Metal backend. Concrete
numbers for each knob:
  - no bounds checking: **+30%**  (biggest single win)
  - `simdgroup_multiply_accumulate` vs naive: **+35%**
  - `half4` vectorized loads: **+23%**
  - Double buffering, 128×128 tiles, stream-K, LUT dequant,
    transposed B: all *slower* on Apple Silicon (counterintuitive).
  - 2.6× gap from theoretical memory bandwidth ceiling.
  - 0.85 ms irreducible dispatch overhead in 3.99 ms total.

When `mr/` grows a Metal backend (honeycrisp is still WIP), this is
the starting point, not a blank page.

## What's **not** parked here

Everything in `llm/src/backend/{wgpu,metal,ane}/` that's *not* in the
audit's "high-value" list stays in the live old-runtime tree until
Phase F deletes it. That includes:
- `backend/wgpu/` minus the already-copied shader shelf
  (`mr/src/wgpu_rs/kernels/shelf/`)
- `backend/metal/` full tree (deleted in Phase F)
- `backend/ane/` minus the MIL generators already parked here
- `backend/cpu/` (obsoleted by `mr/src/cpu/`)

Reviving any of these means reading them in-situ before Phase F lands
or checking out the commit before deletion.
