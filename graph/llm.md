---
tags: cyb, core, soma, architecture
crystal-type: spec
crystal-domain: cyber
alias: llm runtime, inference runtime, tensor runtime
---

# llm — universal inference runtime

a single Rust binary that loads any model format and runs inference on any hardware. replaces the zoo: llama.cpp, whisper.cpp, bitnet.cpp, ONNX Runtime, CoreML, mflux, PyTorch.

## why this doesn't exist yet

five reasons no one has shipped a universal Rust inference runtime:

1. vendor lock-in economics — NVIDIA captured 86% of AI datacenter revenue in 2025. CUDA dominance means no incentive to standardize
2. hardware feature divergence — Apple simdgroup_matrix, NVIDIA Tensor Cores, AMD WMMA, Qualcomm Hexagon HVX are architecturally different. lowest-common-denominator abstraction loses 5-10x performance
3. translation overhead — wgpu translates WGSL→MSL/SPIR-V/HLSL, losing access to vendor-specific intrinsics. native Metal shaders are 2-5x faster than wgpu for matmul on Apple Silicon
4. ecosystem immaturity — Rust GPU tooling (codegen, memory profiling, debugging) is years behind CUDA
5. nobody needs ALL backends — cloud targets CUDA. Apple targets CoreML. mobile targets NNAPI. each ecosystem solves its own slice

soma needs all of them because [[neuron]] runs on any hardware — phone, laptop, server, browser.

## architecture

```
┌──────────────────────────────────────────┐
│              model loader                 │
│  .onnx  .safetensors  .gguf  .bin        │
│  format = storage, not runtime            │
└─────────────────┬────────────────────────┘
                  │ parse weights → tensors
                  ▼
┌──────────────────────────────────────────┐
│              graph IR                     │
│  nodes: [(op, inputs, outputs, attrs)]   │
│  weights: typed tensor store             │
│  quantization: f16/q8/q4/ternary per     │
│  tensor, not per model                   │
└─────────────────┬────────────────────────┘
                  │ schedule
                  ▼
┌──────────────────────────────────────────┐
│              op registry (~30 ops)        │
│  each op: trait with backend impls       │
│  dispatch: op × dtype × backend → kernel │
└─────────────────┬────────────────────────┘
                  │ execute
                  ▼
┌──────────────────────────────────────────┐
│              backend layer                │
│                                          │
│  ┌─────────┐ ┌──────┐ ┌──────┐ ┌─────┐  │
│  │  Metal  │ │ wgpu │ │ CUDA │ │ CPU │  │
│  │ (Apple) │ │(cross)│ │(NV)  │ │(SIMD)│ │
│  └────┬────┘ └──┬───┘ └──┬───┘ └──┬──┘  │
│       │         │        │        │      │
│  ┌────▼────┐                             │
│  │   ANE   │  (Apple Neural Engine,      │
│  │         │   subgraph offload)         │
│  └─────────┘                             │
└──────────────────────────────────────────┘
```

## the 30 ops

every neural network — transformer, CNN, diffusion, TTS, BitNet — reduces to these operations:

### core linear algebra
- `matmul` — 60% of all compute. variants: f16, q8, q4, ternary (BitNet)
- `add`, `mul`, `sub` — elementwise
- `transpose`, `reshape`, `permute`, `concat`

### attention
- `sdpa` — scaled dot-product attention + flash attention path
- `sdpa_cross` — cross-attention (whisper decoder, diffusion)
- `kv_cache` — append/lookup, memory lifecycle
- `rope` — rotary position embedding

### normalization
- `rmsnorm` — llama, qwen, mistral
- `layernorm` — BERT, DeBERTa, whisper
- `batchnorm` — YOLO
- `groupnorm` — diffusion UNet/DiT

### activation
- `silu` — llama, qwen
- `gelu` — BERT, GPT
- `relu` — YOLO, classic CNNs
- `sigmoid`, `tanh`, `softmax`

### convolution
- `conv2d` — YOLO, VAE (diffusion), VITS (TTS)
- `conv1d` — TTS, audio models
- `depthwise_conv` — efficient mobile CNNs
- `pooling` — max, avg

### embedding
- `token_embed` — lookup table
- `pos_embed` — learned or sinusoidal

### special
- `noise_schedule` — diffusion timestep
- `flow_step` — normalizing flow (VITS/TTS)
- `quantize` / `dequantize` — runtime Q4/Q8 conversion
- `sample` — top-p, top-k, temperature

## backend strategy

the key insight: wgpu is too slow for peak performance on Apple Silicon. Metal native is 2-5x faster for matmul because of `simdgroup_matrix_multiply_accumulate` — Apple's tensor core instruction that wgpu cannot access.

strategy: native backend per platform, wgpu as universal fallback. zero C++ anywhere — pure Rust + thin FFI to system C APIs.

| platform | backend | Rust crate | maturity | why |
|----------|---------|-----------|----------|-----|
| macOS/iOS | Metal | `objc2-metal` | production | simdgroup matrix, residency sets, zero translation. 2-5x faster than wgpu for matmul |
| macOS/iOS | ANE | custom (pure Rust, no objc) | in-house | direct ANE access bypassing obj-c/CoreML. 3-5W power. dims must be ×128 |
| NVIDIA | CUDA | `cudarc` | production (3.1M downloads) | tensor cores, cuBLAS |
| AMD | ROCm | `cubecl-hip-sys` | early (Burn team) | WMMA, native HIP. low priority — wgpu Vulkan covers AMD |
| any GPU | wgpu | `wgpu` | production (18.7M downloads) | Vulkan/DX12/Metal/WebGPU. universal fallback |
| browser | WebGPU | `wgpu` → WASM | production | 25-40 tok/s for 1B models |
| Android (Qualcomm) | QNN | FFI to `libQnnHtp.so` | needs unsafe FFI | Hexagon NPU, 100x vs CPU. dlopen + C API |
| Android (other) | NNAPI | FFI to `libneuralnetworks.so` | needs unsafe FFI | vendor NPU abstraction. dlopen + ~30 C functions |
| CPU everywhere | SIMD | `std::arch` | stable Rust | NEON/AVX2/AVX512. always available |

### Metal vs wgpu — why both

wgpu on Metal translates WGSL→MSL via Naga. this loses `simdgroup_matrix_multiply_accumulate` (Apple tensor core), fine-grained threadgroup barriers, residency sets (macOS 15+), and compile-time specialization constants.

approach: Metal shaders (.metal) for the hot path (~5 ops: matmul, attention, rope, conv2d, quantized variants). wgpu for everything else. runtime detects platform and dispatches.

### ANE — the free accelerator

Apple Neural Engine at 3-5W, leaving GPU free. custom pure Rust implementation — direct ANE access without obj-c bridge or CoreML dependency. no `rustane`, no `objc2`, no fragile private API wrappers. use ANE for always-on tier 0 models (low power), Metal GPU for generative (throughput).

### why not MLX?

MLX is Apple's Python/C++ framework over Metal. we target Metal directly — more control (simdgroup ops, residency sets, buffer management). MLX format (.safetensors + config.json) loads identically to any safetensors — the format is the same, only the Python runtime differs. soma-runtime replaces MLX.

## model format loading

format is just storage — parse once, run on any backend.

| format | what stores it | loader |
|--------|---------------|--------|
| .safetensors | HuggingFace models | parse header → mmap tensors |
| .gguf | llama.cpp quantized | parse metadata → extract Q4/Q8 tensors |
| .onnx | ONNX exported models | parse protobuf → build graph IR |
| .bin | fasttext, custom | format-specific parser |
| .mlx | Apple MLX format | numpy-compatible loader |

key: all formats converge to the same in-memory representation — graph IR + typed tensor store. the runtime doesn't care where the weights came from.

## quantization as a first-class concept

quantization is per-tensor, not per-model. a single model can have:
- attention weights in Q4
- embedding table in f16
- output projection in Q8
- BitNet layers in ternary

the op registry dispatches to the right kernel based on input tensor dtype:
```
matmul(a: f16, b: q4)    → kernel_matmul_f16_q4
matmul(a: f16, b: ternary) → kernel_matmul_ternary  // add/subtract only
matmul(a: f16, b: f16)   → kernel_matmul_f16
```

## memory management

three tiers mirroring [[soma]] cognitive architecture:
- resident (tier 0): always in GPU memory, never evicted
- cached (tier 1): loaded on first use, LRU eviction
- streamed (tier 2): loaded per-inference, freed after

KV cache lifecycle:
- allocate on first token
- grow with each generated token
- free when generation completes
- shared pool across concurrent inferences

## provability

every op execution produces a trace entry:
```
(op_id, input_hashes, output_hash, timing_ns)
```

the trace is a STARK-compatible execution record. given the same weights and input, any verifier can replay the trace and confirm the output. this is what makes [[soma]] inference provable — the model cannot lie because every matrix multiply is auditable.

## multi-model orchestration

soma runs 8+ models in parallel (tier 0) + loads/unloads tier 1-2 on demand. the runtime is not a "run one model" tool — it is a model scheduler.

### concurrent execution
- multiple models share GPU memory simultaneously
- priority queue: tier 0 models preempt tier 1-2
- memory pressure → shed lowest-priority model, not crash

### hot-swap
- load new model weights while old model still serves
- atomic switch: old → new without dropping requests
- use case: model update without downtime

### model composition
- chain models in a pipeline: whisper → LLM → TTS
- output tensor of model A feeds directly as input to model B
- zero-copy between models on same device (shared GPU buffers)

## inference optimizations

### speculative decoding
use small model (tier 1) to draft N tokens, large model (tier 2) to verify in one forward pass. 2-3x speedup for autoregressive generation. the runtime manages draft/verify loop automatically when both models are loaded.

### multi-token prediction (MTP)
MiMo and Step 3.5 Flash generate 2-3 tokens per forward pass. the runtime supports variable output length per step — not hardcoded to 1 token.

### prefill vs decode
two distinct phases with different optimization strategies:
- prefill (prompt processing): batch all tokens, maximize GPU utilization, parallelize
- decode (token generation): sequential, optimize for latency, use KV cache

### continuous batching
handle multiple inference requests concurrently. new requests join mid-batch without waiting. vLLM-style iteration-level scheduling.

### graph fusion
fuse sequential ops into single kernels at graph IR level:
- matmul + bias + activation → single kernel
- attention (Q×K, scale, mask, softmax, ×V) → flash attention kernel
- rmsnorm + matmul → single dispatch

### KV cache optimization
- paged attention: allocate KV cache in fixed-size blocks, not contiguous. eliminates fragmentation
- KV cache quantization: compress cached keys/values to Q8 or Q4 during long generations
- prefix caching: reuse KV cache for common prompt prefixes across requests

### MoE routing
Mixture-of-Experts models (Wan2.2, MiMo-V2-Flash) select top-K experts per token. the runtime handles:
- expert weight loading (only active experts in GPU memory)
- token-to-expert dispatch
- load balancing across experts

## adaptive resource management

### graceful degradation
when memory pressure hits, the runtime does not crash — it adapts:
```
OOM detected
  → drop KV cache precision (f16 → q8)
  → if still OOM → shed lowest-priority model
  → if still OOM → offload layers to CPU
  → if still OOM → reduce context window
  → never crash
```

### adaptive precision
dynamically switch quantization based on available memory:
- plenty of RAM → f16 weights, f16 KV cache
- moderate pressure → q8 weights, f16 KV cache
- heavy pressure → q4 weights, q8 KV cache
- extreme → q4 weights, q4 KV cache, reduced context

### device splitting
split a single model across multiple backends:
- GPU + CPU: bottom layers on GPU, top layers on CPU (layer offloading)
- GPU + ANE: attention on GPU, matmul on ANE (op-level split)
- multi-GPU: tensor parallel across devices

## observability

every inference produces metrics:
```
{
  model: "qwen3.5-9b",
  tokens_generated: 142,
  prefill_ms: 340,
  decode_ms: 4200,
  tok_per_sec: 33.8,
  peak_memory_mb: 5840,
  ops: [
    { name: "matmul_q4", calls: 2840, total_ms: 3100 },
    { name: "attention",  calls: 284,  total_ms: 890 },
    ...
  ]
}
```

hot path identification: top-5 ops by time are the optimization targets. the runtime surfaces this automatically.

## security

- weight integrity: sha256 hash verification on load. tampered weights → refuse to run
- input bounds: tensor shape/dtype validation before every op. malformed input → error, not UB
- memory isolation: each model's buffers are separate. one model cannot read another's weights
- no network: the runtime never phones home. fully offline. weights are local files

## what this enables

one `cargo build` produces a binary that:
- loads whisper.gguf and transcribes speech
- loads qwen3.5-9b.safetensors and reasons
- loads flux-schnell.safetensors and generates images
- loads yolov11.onnx and detects objects from cameras
- loads bitnet-2b.bin and runs ternary inference
- runs on MacBook (Metal+ANE), Linux server (CUDA), Android phone (Vulkan+NPU), or browser (WebGPU)

no Python. no pip. no conda. no Docker. one binary.

## existing prior art

| project | what it does | what it lacks |
|---------|-------------|---------------|
| llama.cpp | fast LLM inference, Metal/CUDA | only LLMs, C, no graph IR |
| whisper.cpp | fast ASR, Metal | only whisper, C |
| ONNX Runtime | 15+ backends via C++ | bloated, C++, not Rust, weak for autoregressive |
| candle | Rust ML, Metal/CUDA | no wgpu, no Vulkan, no mobile NPU |
| burn/CubeCL | Rust, wgpu+CUDA+ROCm | alpha quality, heavy abstractions, no ANE |
| mflux | Apple Silicon diffusion | only diffusion, only Apple |
| bitnet.cpp | ternary inference | only BitNet, C |

none of them solve the full problem. this runtime does.

## implementation order

| phase | ops | unlocks | effort |
|-------|-----|---------|--------|
| 0 (done) | matmul_f16, attention, rope, rmsnorm, silu | transformer decoder — all LLMs | done |
| 1 | matmul_q4, matmul_q8 | quantized LLMs at production quality | 1 shader each |
| 2 | matmul_ternary | BitNet models — <1GB for 2B quality | 1 shader |
| 3 | Metal native matmul | 2-5x speedup on Apple Silicon | port from llama.cpp Metal |
| 4 | layernorm, encoder path | BERT/DeBERTa classifiers, embeddings | partial |
| 5 | cross-attention | whisper (ASR) | ~50 lines |
| 6 | conv2d, batchnorm, pooling | YOLO (cameras) | ~200 lines |
| 7 | groupnorm, noise_schedule | diffusion (image gen) | medium |
| 8 | conv1d, flow layers | TTS (voice output) | medium |
| 9 | CUDA backend | NVIDIA server deployment | cudarc integration |
| 10 | ANE offload | power-efficient always-on inference | custom pure Rust ANE driver |
| 11 | NNAPI/QNN FFI | Android NPU inference | dlopen + ~30 extern "C" functions, zero C++ |

after phase 6: one binary runs 90% of [[soma]] models.
after phase 8: full media stack.
after phase 10: optimal power management on Apple hardware.

after phase 11: Android NPU via NNAPI FFI.

see [[soma]] for the model architecture this runtime serves.
