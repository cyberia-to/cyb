# Scope

cyb-llm runtime executes a **graph** of tensor operations on any
hardware. Models are graphs. Scope is defined by the set of primitive
operations the runtime implements — not by which models it "supports".

## Why this framing

Traditional ML runtimes define scope by model family: llama.cpp
runs Llama and friends. ONNX Runtime runs ONNX graphs. Our runtime
sits closer to ONNX: an op set with conforming backends, but smaller,
content-addressed, and verifiable.

Consequences:
- Adding a new model family is **zero runtime work** if its graph
  uses existing primitives. Just add a graph template in import.
- New primitives are added only when no composition of existing ones
  suffices. The primitive set grows deliberately.
- "Does it support X?" = "is X expressible in our op set?" —
  answerable by checking the graph, not by reading release notes.

## Modality coverage

The primitive set must be complete enough to express the
computational graphs behind every form of intelligence we care
about. Not "chat", "embedding", "image gen" as product categories —
primitives don't care about product categories.

| Modality | What graph looks like | Primitives required |
|---|---|---|
| Text (LLM) | embed → N × (RMSNorm + GQA-SDPA + SwiGLU) → norm → logits | matmul, RMSNorm, RoPE, SDPA, SwiGLU, softmax |
| Text encoder | embed + pos → N × (LayerNorm + full-SDPA + GeLU-MLP) → pool | matmul, LayerNorm, PosEmbed, SDPA(non-causal), GELU |
| Vision (ViT) | patch → N × (LayerNorm + SDPA + GELU-MLP) → pool | Conv2d (patch embed), LayerNorm, SDPA, GELU |
| Vision (CNN) | Conv2d + Pool + LayerNorm + MLP | Conv2d, BatchNorm, ReLU, Pool |
| Audio (Whisper) | Conv1d-stem → N × encoder → N × decoder-cross-attn | Conv1d, LayerNorm, SDPA, SDPA-cross, softmax |
| TTS (VITS/XTTS) | text embed → transformer + flow + HiFi-GAN | Conv1d, FlowStep, ConvTranspose, GELU, attention |
| Diffusion (UNet) | timestep + latent → ResBlocks + cross-attn → denoised | Conv2d, GroupNorm, SinusoidalEmbed, SDPA-cross, SiLU |
| Diffusion (DiT/video) | patchify → N × (AdaLN + SDPA + MLP) → unpatchify | PatchEmbed, AdaLN, SDPA, GELU, PixelShuffle, Conv3d |
| SSM / Mamba | embed → N × (RMSNorm + SSM-scan + Gate) → norm | Scan (NEW), gate, matmul, RMSNorm |
| Multimodal (VL) | vision encoder + projection + LLM | all of the above, glued |

All rows except "SSM / Mamba" are expressible in the current IR.
SSM requires one new primitive (`Scan`). MoE requires one (`RoutedMatmul`).

## Primitive set (v1)

This is the **closed** set we commit to implementing correctly on
every backend. Versioned — additions require spec update.

### Linear algebra (core)
Matmul, Add, Mul, Sub, Div, Transpose, Reshape, Permute,
Concat, Split, Chunk, Clamp, NanToNum, Argmax

### Attention
Sdpa (causal/non-causal, GQA), SdpaCross, SdpaWindow,
FlashAttention (as composite), KvCache, KvCompress

### Position encoding
Rope (1D for LLMs, multi-axis for DiT/video), SinusoidalEmbed,
RelativePosEmbedding, PosEmbed (learned), TokenEmbed

### Normalization
RmsNorm, LayerNorm, BatchNorm, GroupNorm, InstanceNorm, AdaLN

### Activation
Silu, Gelu (standard, tanh-approx), Relu, LeakyRelu, PRelu,
Sigmoid, Tanh, Softmax, SwiGlu, GeGlu, Glu

### Convolution
Conv1d, Conv2d, Conv3d, ConvTranspose2d, CausalConv1d,
DepthwiseConv, Pool (max/avg)

### Spatial
Interpolate, PixelShuffle, PixelUnshuffle, PatchEmbed, Unpatchify

### Quantization (orthogonal to ops — any matmul accepts any dtype)
F32, F16, BF16, Q8_0, Q4_0, Q4_K, Q5_K, Q6_K, Q3_K, Q2_K, Ternary,
Quantize, Dequantize

### Diffusion / flow / sampling
NoiseSchedule, FlowStep, Sample (top-p, top-k, temperature, grammar)

### Adapters (runtime composition)
LoraApply (low-rank addition), Kron, MatrixInverse

### Fused (optimization — semantics = composition)
FusedNormMatmul, FusedSkipNorm, FusedSwiGlu — performance only,
always equivalent to unfused graph.

## Primitives explicitly NOT yet in the set

These are planned but need a spec addition before we claim them.

- **Scan** — sequential state propagation for SSM/Mamba/RWKV/RNN
- **RoutedMatmul** — sparse-expert dispatch for MoE
- **ContinuousBatching** — not a primitive; scheduler concern
- **Backward / Autograd** — training is a separate system

## Hardware backends

Each backend must implement every primitive correctly (test.md).

| Backend | Target | Tier |
|---|---|---|
| `cpu` | Reference (slow, always correct, golden values) | 1 |
| `wgpu` | Portable GPU (Windows, Linux, Android, Web) | 1 |
| `metal` | Apple GPU with aruminium zero-copy | 1 |
| `ane` | Apple Neural Engine (MIL graph compile) | 2 |
| `cuda` | NVIDIA (future) | 2 |

A model runs on a backend if the backend implements every primitive
the model's graph uses. Missing primitive = clear error ("backend
`ane` does not support Op::Scan"), not silent corruption.

## Acceptance criteria for "universal portable runtime"

v1 of the spec is complete when:

1. **Coverage**: every primitive in the set has a math definition
   (ops.md), a reference CPU implementation, and at least one GPU backend.

2. **Correctness**: every primitive passes its golden test (input X
   → output Y, diff < 1e-4 vs F32 reference) on every implementing
   backend.

3. **Composition**: any graph composed of primitives produces the
   same output (diff < 1e-3) regardless of which backend runs it.

4. **Import**: any GGUF, safetensors, MLX, or ONNX model expressible
   in the primitive set imports to a graph without manual work.
   Unsupported primitive in source = actionable error, not silent drop.

5. **Speed**: for decoder-only LLMs on GPU, within 2× of llama.cpp;
   for vision models on GPU, within 3× of HF transformers.

6. **Modalities demonstrated**: at least one model from each row in
   the modality table above runs end-to-end correctly.

## Versioning

The primitive set is versioned. Adding a primitive increments the
minor version. Changing semantics of an existing primitive is a
breaking change (major version). Models declare the primitive-set
version they require.

## Growth philosophy

A new primitive is added when:
- An existing composition would need O(N) fused kernels per model,
  not O(1) (e.g. flash attention)
- It represents a fundamentally new dataflow pattern (scan for SSM)
- It enables a modality that composition can't reach

A new primitive is NOT added when:
- Existing primitives compose to the same result
- It's only useful for a single paper's variant
- It's a fused optimization (those are optimization-layer concerns,
  not primitives)

The test: can we write 10 models using the primitive alone, across
different modalities? If not, it's too specific — fuse at the
model level, don't add a primitive.

## Decision log

- 2026-04-17: Scope defined by primitives, not models. Growth of
  model support comes from graph templates, not runtime changes.
- 2026-04-17: Scan and RoutedMatmul identified as required primitives
  for SSM and MoE; add when first consumer lands.
- 2026-04-17: Training out of scope for cyb-llm runtime — different
  system (gradient + optimizer + data pipeline).
- 2026-04-17: Backends are conforming implementations — a backend
  that doesn't implement a primitive produces a clear error, never
  silent wrong output.
