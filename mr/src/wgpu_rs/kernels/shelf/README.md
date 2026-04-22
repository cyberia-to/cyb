# Shader shelf

WGSL kernels ported verbatim from the old `llm/src/backend/wgpu/shaders/`.
None of them are wired into `mr`'s op dispatch yet — they sit here as
ready-to-use source so the implementation knowledge ships forward.

## Wire-up prerequisites

Every kernel in this directory expects:

1. `FrameAllocator` for buffer lifetime (present: `mr/src/wgpu_rs/alloc.rs`).
2. `dispatch_2d(n)` for workgroup grid sizing when `N > 65535`
   (present: `mr/src/wgpu_rs/alloc.rs`).
3. A graph executor that dispatches one op per compute pass
   (**absent**: `mr/src/ir/executor.rs` is a stub).

Once the executor lands, each shader moves one directory up (out of `shelf/`)
and gets a Rust `kernels/<name>.rs` binding.

## Inventory

### Matmul kernels

- **`q4_matmul.wgsl`** — Q4_0 (32-value block, 18 bytes/block) matmul.
  Baseline. Sub-group reduce inside workgroup.
- **`q4k_matmul.wgsl`** — Q4_K (256-value superblock, 144 bytes/block) matmul.
  Reads raw superblocks directly on GPU, no CPU-side conversion. Inline
  decodes the packed 12-byte scale region using llama.cpp's
  `get_scale_min_k4` bit extraction. Known needs-opt: superblock decode
  inside workgroup loop loses parallelism (audit: 13 tok/s vs Q4_0's 29).
- **`q6k_matmul.wgsl`** — Q6_K (256-value superblock, 210 bytes/block) matmul.
  Used for lm\_head in most qwen models.
- **`ternary_matmul.wgsl`** — 2-bit packed BitNet weights ({-1, 0, +1}).
  Multiply-free inner loop: branches on encoding to fadd / fsub / skip.
  8× less bandwidth than Q4 → memory-bound decode wins proportionally.
  Delivered 102 tok/s end-to-end on BitNet-2B in the old runtime.

### Attention kernel

- **`attention.wgsl`** — single-pass fused SDPA decode. One workgroup per
  head, 256 threads. Online softmax via `subgroupMax` → 8-slot shared
  reduction → `subgroupAdd` for exp sum. All in registers; no second pass
  over the sequence. Requires `enable subgroups;` (WGSL spec-optional).

### Fused ops

- **`fused_norm_q4.wgsl`** — RMSNorm inlined into Q4 matmul. Normalizes
  into shared memory, Q4 dequant reads from shared instead of DRAM. Saves
  1 dispatch + 1 global write + 1 global read per linear layer. Limited
  to `hidden_size ≤ 4096` (hardcoded shared-memory cap).
- **`fused_skip_norm.wgsl`** — Add + RMSNorm in one workgroup. Produces
  two outputs (normalized and raw residual) from one pass.

## What did not make the shelf

From the old shaders/ dir, intentionally omitted:
- `turbo_quant.wgsl` — non-atomic `f32` add masquerading as `atomicAdd`;
  correct only because each token gets its own workgroup. Needs a proper
  atomic path before shipping.
- `q4k_embed.wgsl`, `q2k`/`q3k`/`q5k` matmul — less commonly used quants;
  copy when a model needs them.
- `activations.wgsl`, `elementwise.wgsl`, `argmax.wgsl`, `rms_norm.wgsl`,
  `rope.wgsl`, `layernorm.wgsl`, `norm.wgsl`, `conv.wgsl`, `spatial.wgsl`,
  `special.wgsl`, `adapter.wgsl`, `kv_cache.wgsl`, `f16_matmul.wgsl`,
  `f32_matmul.wgsl` — straightforward ops whose mr/ native kernels already
  exist or are trivial to write fresh when needed.
