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

## graph IR

the IR is a directed acyclic graph (DAG) of typed tensor operations. not just a list of ops — it encodes how data flows between them.

### structure

```rust
struct Graph {
    nodes: Vec<Node>,
    tensors: TensorStore,       // weights + intermediate buffers
}

struct Node {
    op: Op,                     // matmul, attention, rmsnorm, ...
    inputs: Vec<TensorId>,      // edges in
    outputs: Vec<TensorId>,     // edges out
    attrs: Attrs,               // num_heads, eps, kernel_size, ...
    backend_hint: Option<Backend>, // prefer ANE, prefer Metal, ...
}

struct TensorMeta {
    shape: Shape,               // can be dynamic: [batch, seq_len, 2048]
    dtype: DType,               // f16, q4, q8, ternary, f32
    residency: Residency,       // resident | cached | streamed
}
```

each edge is a typed tensor with known shape and dtype. the scheduler uses this to allocate GPU buffers, plan memory reuse, and dispatch ops to backends.

### where does the graph come from?

weight files (.safetensors, .gguf) contain tensors without graph structure. the graph must be constructed separately.

| source | what it provides | how graph is built |
|--------|-----------------|-------------------|
| .onnx | explicit graph (protobuf) | parse directly into IR |
| .safetensors + config.json | named tensors + architecture params | architecture template instantiation |
| .gguf | named tensors + metadata | architecture template from metadata.architecture field |
| code | nothing on disk | programmatic graph construction |

### architecture templates

for safetensors/GGUF models, the runtime has built-in templates for common architectures:

```
transformer_decoder(config) → Graph
  for each layer in 0..config.num_layers:
    → rmsnorm(eps)
    → attention(num_heads, head_dim, rope)
    → rmsnorm(eps)
    → mlp(hidden_dim, intermediate_dim, silu)

transformer_encoder(config) → Graph
  for each layer:
    → layernorm(eps)
    → attention(num_heads, head_dim)  // no KV cache
    → layernorm(eps)
    → mlp(hidden_dim, intermediate_dim, gelu)

encoder_decoder(config) → Graph     // whisper
  encoder = transformer_encoder(enc_config)
  decoder = transformer_decoder(dec_config) + cross_attention

diffusion_dit(config) → Graph       // flux, wan2.2
  for each block:
    → layernorm → attention → layernorm → mlp
  noise_schedule + vae_decoder

cnn_detector(config) → Graph        // YOLO
  backbone: conv2d chains
  neck: feature pyramid
  head: detection + NMS

moe_decoder(config) → Graph         // MiMo-V2, Step 3.5
  for each layer:
    → rmsnorm → attention
    → rmsnorm → router(num_experts, top_k) → expert_mlps
```

config comes from config.json (HuggingFace) or GGUF metadata. template + config = concrete graph.

### dynamic shapes

batch size and sequence length change at runtime. the IR represents these as symbolic dimensions:
- shape `[B, S, 2048]` where B and S are resolved at inference time
- KV cache grows with S on each decode step
- the scheduler pre-allocates based on max expected S, resizes if exceeded

### graph optimizations

before execution, the IR is optimized:
- op fusion: matmul + bias + silu → single fused kernel
- dead node elimination: remove unused outputs
- constant folding: precompute anything that doesn't depend on input
- memory planning: assign buffer lifetimes, maximize reuse (op A's output buffer reused for op C if A is consumed before C starts)

### stateful ops

some ops carry state between inference calls:
- `kv_cache`: grows across tokens, persists across calls
- `moe_router`: load-balancing counters (optional)
- all other ops are pure functions (same input → same output)

stateful ops are explicitly marked in the IR. the STARK provability trace handles them by including state snapshots.

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

## tokenization

completely missing from the architecture — every LLM needs text → tokens → text. different models use different tokenizers.

| tokenizer type | models | Rust crate |
|---------------|--------|-----------|
| BPE (tiktoken) | GPT, Qwen | `tiktoken-rs` |
| SentencePiece | LLaMA, Mistral | `sentencepiece-rs` or custom |
| HuggingFace tokenizers | most HF models | `tokenizers` (HF Rust crate, production) |
| byte-level | RWKV, some custom | trivial |

the `tokenizers` crate by HuggingFace is native Rust, production (it is the same engine Python uses via bindings). handles BPE, WordPiece, Unigram, SentencePiece. loads tokenizer.json directly.

### chat templates

models expect different chat formats. wrong template → garbage output:
```
chatml:    <|im_start|>user\n{msg}<|im_end|>
llama:     [INST] {msg} [/INST]
qwen3:     <|user|>\n{msg}<|end|>
```
the runtime loads chat_template from tokenizer_config.json (Jinja2 format) and applies it. no hardcoded templates — parse from model config.

## sampling

`sample` op in the op list is a placeholder. real sampling is a subsystem:

- temperature scaling
- top-k filtering
- top-p (nucleus) filtering
- min-p filtering
- repetition penalty (frequency + presence)
- grammar-constrained decoding — force output to match a schema (JSON, regex). use finite state machine over token vocabulary
- beam search (rare but needed for whisper)
- speculative sampling (accept/reject draft tokens)

grammar-constrained decoding is critical for [[soma]] — the router (tier 0.1) must output valid JSON: `{"tier": 1, "slot": 3}`. unconstrained generation can produce malformed routing decisions.

## model registry

the gap between "load safetensors" and "run inference" is larger than the doc suggests. each model family has different tensor naming conventions:

```
qwen3:   model.layers.0.self_attn.q_proj.weight
llama:   model.layers.0.self_attn.q_proj.weight  (same)
mistral: model.layers.0.self_attn.q_proj.weight  (same)
bert:    encoder.layer.0.attention.self.query.weight  (different)
whisper: decoder.layers.0.self_attn.q_proj.weight  (different prefix)
yolo:    model.0.conv.weight  (completely different)
```

the registry maps: model_type (from config.json) → architecture template + tensor name mapping + tokenizer type + chat template.

```rust
struct ModelRegistry {
    // model_type → everything needed to instantiate
    entries: HashMap<String, ModelSpec>,
}

struct ModelSpec {
    template: ArchTemplate,          // transformer_decoder, cnn_detector, ...
    tensor_map: TensorNameMapping,   // weight name pattern → graph node
    tokenizer: TokenizerType,        // HF, sentencepiece, tiktoken
    chat_template: Option<String>,   // Jinja2 template string
    default_params: InferenceParams, // temperature, top_p, max_tokens
}
```

### supported model families (initial)

the registry must cover at minimum:
- qwen2, qwen2.5, qwen3, qwen3.5 (all soma tier 1-2 models)
- llama2, llama3, llama3.1, llama3.2
- mistral, mixtral
- deepseek, deepseek-r1
- mimo (Xiaomi)
- phi-3, phi-4
- bert, deberta (tier 0 classifiers)
- whisper (ASR)
- clip, siglip (vision encoders)
- stable-diffusion, flux, wan2.2 (diffusion)
- yolo (detector)
- vits, piper (TTS)

each family = one entry in registry. adding a new model family = adding one struct, not changing runtime code.

## API surface

the runtime is three things:

### library (embedded)
```rust
let rt = Runtime::new(Backend::auto())?;
let model = rt.load("qwen3.5-9b.safetensors", config)?;
let tokens = model.generate("hello", params)?;  // streaming iterator
```

### daemon (always-on)
```
soma-runtime serve --models tier0.toml
```
loads tier 0 models on startup, accepts requests via unix socket, manages model lifecycle. this is what [[soma]] main loop talks to.

### CLI (one-shot)
```
soma-runtime run --model qwen3.5-9b.gguf --prompt "hello"
soma-runtime bench --model qwen3.5-9b.gguf  // tok/s benchmark
```

## determinism and provability

for STARK traces, inference must be deterministic: same weights + same input → same output. this is harder than it sounds:

- floating point addition is not associative: `(a+b)+c ≠ a+(b+c)` at f16 precision
- GPU thread execution order varies between runs
- different backends (Metal vs CUDA) give different rounding

solution: fix reduction order in all kernels. use tree reduction with deterministic thread mapping. accept that Metal output ≠ CUDA output, but Metal output is always the same across Metal runs. provability is per-backend, not cross-backend.

## testing strategy

correctness = output matches reference implementation within tolerance.

| level | what | reference | tolerance |
|-------|------|-----------|-----------|
| op-level | each op in isolation | PyTorch reference output | max abs error < 1e-3 (f16) |
| model-level | full forward pass | llama.cpp output for same model | token-level agreement (same top-1 token) |
| end-to-end | generate N tokens | llama.cpp generates same sequence | exact match for greedy decoding (temp=0) |

test suite: save reference inputs/outputs as .npz files. CI runs every op against reference on every commit. regression = CI fails.

## context window management

what happens when input exceeds model's trained context?

- truncation: drop oldest tokens (simple, lossy)
- sliding window: process in chunks, carry KV cache forward (mistral-style)
- RoPE scaling: extend positional encoding beyond training length (YaRN, NTK-aware). requires config parameter `rope_scaling`
- the runtime reads `max_position_embeddings` and `rope_scaling` from config.json and applies automatically

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
