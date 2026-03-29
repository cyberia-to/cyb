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

strategy: native backend per platform, wgpu as universal fallback.

| platform | primary backend | why | fallback |
|----------|----------------|-----|----------|
| macOS/iOS | Metal (objc2-metal) | simdgroup matrix, residency sets, zero translation overhead | wgpu |
| macOS/iOS | ANE (rustane) | matmul+conv offload to neural engine, 3-5W power | Metal |
| NVIDIA | CUDA (cudarc) | tensor cores, cuBLAS | wgpu (Vulkan) |
| AMD | ROCm (cubecl-hip-sys) | WMMA, native HIP | wgpu (Vulkan) |
| Linux/Windows (any GPU) | wgpu (Vulkan/DX12) | universal, no vendor SDK needed | CPU |
| Android (Qualcomm) | QNN via ort | Hexagon NPU, 100x vs CPU | wgpu (Vulkan) |
| Android (other) | NNAPI via ort | vendor NPU abstraction | wgpu (Vulkan) |
| browser | wgpu (WebGPU) | only option, 25-40 tok/s for 1B | WASM CPU |
| CPU everywhere | SIMD (NEON/AVX2/AVX512) | always available, no GPU needed | — |

### Metal vs wgpu — why both

wgpu on Metal translates WGSL→MSL via Naga. this loses:
- `simdgroup_matrix_multiply_accumulate` (Apple tensor core)
- fine-grained threadgroup memory barriers
- residency sets (macOS 15+, keeps GPU memory wired)
- compile-time specialization constants

for matmul-heavy workloads (LLM inference), native Metal is 2-5x faster. for light workloads (softmax, normalization), wgpu is fine.

approach: write Metal shaders (.metal) for the hot path (~5 ops: matmul, attention, rope, conv2d, quantized variants). use wgpu for everything else. runtime detects platform and dispatches.

### ANE — the free accelerator

Apple Neural Engine runs at 3-5W, leaving GPU free for other work. `rustane` crate provides direct access via private APIs. constraints: dimensions must be divisible by 128, efficiency cliff at dim=5120.

### why not MLX?

MLX is Apple's Python/C++ ML framework. it uses Metal under the hood. our runtime targets Metal directly — this IS the Rust equivalent of MLX, with more control. MLX abstracts away Metal details. we want those details (simdgroup ops, residency sets, buffer management). soma-runtime replaces MLX, not wraps it.

models distributed as "MLX format" (.safetensors + config.json) load identically to any safetensors model — the format is the same, only the Python runtime differs.

## Rust feasibility — can we actually build all backends?

| backend | Rust crate | maturity | from Rust? |
|---------|-----------|----------|------------|
| Metal | `objc2-metal` | production (wgpu uses it) | yes — native Rust, full API access |
| ANE | `rustane` | experimental (30B params validated) | yes — private API via objc2, fragile but working |
| CUDA | `cudarc` | production (3.1M downloads) | yes — safe bindings to CUDA toolkit |
| ROCm | `cubecl-hip-sys` | early (Burn team) | yes — sys bindings to HIP runtime |
| wgpu | `wgpu` | production (18.7M downloads) | yes — native Rust, the gold standard |
| WebGPU | `wgpu` → WASM | production | yes — same crate, compile to wasm32 target |
| CPU SIMD | `std::arch` | stable Rust | yes — NEON/AVX2/AVX512 intrinsics in std |
| Android NNAPI | FFI to `libneuralnetworks.so` | needs unsafe FFI | yes — C API, ~200 lines of bindings |
| Qualcomm QNN | FFI to QNN SDK | needs unsafe FFI | yes — C API, NDK build. or use `ort` crate with QNN EP as bridge |

the hard parts:

1. Android NPU — no Rust crate exists. two paths: (a) write thin FFI bindings to NNAPI C API (~200 lines unsafe), or (b) use `ort` crate which wraps ONNX Runtime's NNAPI/QNN execution providers. path (b) adds a C++ dependency but covers all Android NPUs immediately.

2. ANE — `rustane` uses private Apple APIs that can break between macOS versions. production path: compile subgraphs to Core ML format (.mlmodelc) and dispatch via Core ML framework (stable public API) while keeping Metal as primary.

3. ROCm — `cubecl-hip-sys` is alpha. AMD GPU support is lowest priority — wgpu Vulkan fallback covers AMD adequately for now.

everything else is production-ready from Rust today. the runtime is feasible.

use ANE for: always-on tier 0 models (classifiers, embeddings) where latency matters less than power efficiency. use Metal GPU for: generative models where throughput matters.

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
| 10 | ANE offload | power-efficient always-on inference | rustane integration |
| 11 | NNAPI/QNN FFI | Android NPU inference | ~200 lines unsafe FFI or ort bridge |

after phase 6: one binary runs 90% of [[soma]] models.
after phase 8: full media stack.
after phase 10: optimal power management on Apple hardware.

after phase 11: Android NPU via NNAPI FFI.

see [[soma]] for the model architecture this runtime serves.
