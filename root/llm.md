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
│  DAG of typed tensor operations          │
│  weights: per-tensor quantization        │
│  (f16/q8/q4/ternary)                    │
└─────────────────┬────────────────────────┘
                  │ decompose to atoms
                  ▼
┌──────────────────────────────────────────┐
│         8 atoms (reference interpreter)   │
│  mul add cmp exp read write reduce slide │
│  always correct, any backend, slow       │
└─────────────────┬────────────────────────┘
                  │ jet recognition (formula hash)
                  ▼
┌──────────────────────────────────────────┐
│         ~48 jets (fused GPU kernels)      │
│  matmul, attention, conv2d, adaln, ...   │
│  1000x faster, same result               │
└─────────────────┬────────────────────────┘
                  │ dispatch to backend
                  ▼
┌──────────────────────────────────────────┐
│              backend layer                │
│  ┌───────┐ ┌──────┐ ┌──────┐ ┌───────┐  │
│  │ Metal │ │ wgpu │ │ CUDA │ │  CPU  │  │
│  │+ ANE  │ │(cross)│ │(NV)  │ │(SIMD) │  │
│  └───────┘ └──────┘ └──────┘ └───────┘  │
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

## atom / jet architecture

the runtime follows the same pattern as [[Nox]]: a small set of atoms (interpretable, provable, slow) + a registry of jets (fused GPU kernels, fast). any composition of atoms can be recognized by formula hash and accelerated.

### 8 atoms

every neural network operation decomposes into these primitives:

| atom | what it does | type |
|------|-------------|------|
| `mul` | multiply two values | arithmetic |
| `add` | add two values | arithmetic |
| `cmp` | compare (max, min, less_than) | logic |
| `exp` | exponential function | transcendental |
| `read` | indexed memory lookup | memory |
| `write` | indexed memory store | memory |
| `reduce` | collapse dimension (sum, max) | aggregation |
| `slide` | windowed memory access pattern | pattern |

8 atoms are computationally complete for all tensor operations. a model expressed purely in atoms will run correctly on any backend — this is the reference interpreter.

### decomposition

every fused op is a composition of atoms:

```
matmul       = slide + mul + reduce(sum)
conv2d       = slide(2D) + mul + reduce(sum)
conv3d       = slide(3D) + mul + reduce(sum)
softmax      = exp + reduce(sum) + mul
sigmoid      = exp + add + mul
silu         = mul + sigmoid
relu         = cmp(max, 0)
rmsnorm      = mul + reduce(sum) + exp(rsqrt)
layernorm    = reduce(mean) + reduce(var) + add + mul
attention    = matmul + matmul + softmax + matmul
rope         = mul + add
embedding    = read(index)
kv_cache     = write + read
adaln        = mul + add
geglu        = split + gelu + mul
pixel_shuffle = reshape (zero compute, memory layout)
```

### jets — fused acceleration

a jet is a recognized composition of atoms replaced by a single GPU kernel dispatch. the runtime maintains a jet registry mapping formula hashes to fused implementations:

```
formula: {slide, mul, reduce(sum)} over [N,K] × [K,M]
  → jet hash: 0xa3f7...
  → dispatch: matmul_f16.metal (single GPU kernel, 1000x faster)

formula: {matmul, matmul, exp, reduce, mul, matmul} with KV cache
  → jet hash: 0xb2c1...
  → dispatch: flash_attention_f16.metal (single fused kernel)
```

### the three-level guarantee

```
level 0: atoms      — always correct, any backend, ~1000x slow
level 1: jets       — fused kernel, hardware-specific, ~1000x fast
level 2: STARK      — trace records (input, output, jet hash), verifiable

unknown model → runs through atoms (slow but works)
write jet     → 1000x speedup, same result
verifier      → replays atoms, confirms jet output matches
```

no jet for a new op? the model still runs. write the jet later. this is how the runtime stays universal without sacrificing performance: correctness is free, speed is incremental.

### mapping to [[Nox]]

| concept | Nox | llm runtime |
|---------|-----|-------------|
| primitives | 16 patterns (axis, quote, cons...) | 8 atoms (mul, add, cmp, exp, read, write, reduce, slide) |
| acceleration | jets recognized by formula hash | fused GPU kernels by op hash |
| provability | STARK trace over reductions | STARK trace over atom compositions |
| extensibility | new jet = new hash, same semantics | new shader = new hash, same atoms |

the llm runtime IS a [[Nox]] reduction engine specialized for tensor computation. the atoms are [[Ten]] (tensor language) primitives. the jets are [[Goldilocks field processor]] accelerations. the same architecture, different domain.

## the ~48 jets

the jet registry. each jet is a fused GPU kernel replacing a recognized composition of atoms. validated against ComfyUI source code — covers SD, SDXL, Flux, SD3, Wan2.2, ESRGAN, whisper, YOLO, TTS, BitNet.

### core linear algebra
- `matmul` — 60% of all compute. variants: f16, q8, q4, ternary (BitNet)
- `add`, `mul`, `sub` — elementwise (must support broadcasting for adaLN modulation)
- `transpose`, `reshape`, `permute`, `concat`, `split`, `chunk`
- `clamp`, `nan_to_num` — numerical stability for fp16/fp8

### attention
- `sdpa` — scaled dot-product attention + flash attention path
- `sdpa_cross` — cross-attention (whisper decoder, diffusion, IP-adapter)
- `sdpa_window` — shifted window attention (SwinIR): window partition, `roll`, masked attention
- `kv_cache` — append/lookup, memory lifecycle
- `rope` — rotary position embedding (1D for LLMs, multi-axis for DiT/video)
- `sinusoidal_embed` — timestep encoding for all diffusion models
- `relative_pos_bias` — learned position bias table (T5, SwinIR)

### normalization
- `rmsnorm` — llama, qwen, mistral, T5, Flux QKNorm
- `layernorm` — BERT, DeBERTa, whisper, CLIP, SwinIR
- `batchnorm` — YOLO
- `groupnorm` — diffusion UNet, VAE (groups=32)
- `instance_norm` — ACE Step audio
- `adaln` — adaptive layer norm: `shift + x * (1 + scale)` where shift/scale projected from conditioning. all DiT-family (Flux, SD3, Wan2.2, HunyuanDiT)

### activation
- `silu` — llama, qwen, UNet ResBlocks
- `gelu` — BERT, GPT, SwinIR. variants: standard, tanh-approximate, quick (`x * sigmoid(1.702x)`)
- `geglu` — gated GELU feedforward in UNet: `gelu(gate) * x`
- `swiglu` — gated SiLU feedforward in DiT/Flux: `w2(silu(w1(x)) * w3(x))`
- `glu` — gated linear unit with sigmoid (Stable Audio Conformer)
- `relu` — YOLO, classic CNNs
- `leaky_relu` — ESRGAN, upscalers (slope=0.2)
- `prelu` — parametric ReLU (some upscaler variants)
- `sigmoid`, `tanh`, `softmax`

### convolution
- `conv1d` — TTS, audio models
- `conv2d` — YOLO, UNet, VAE, ControlNet, upscalers
- `conv3d` — video models (SVD, Wan2.2, HunyuanVideo VAE)
- `conv_transpose2d` — learned upsampling in some decoder paths
- `causal_conv1d` — temporal causality with replicate padding (Wan video)
- `depthwise_conv` — groups=dim, audio Conformer (kernel=17), efficient mobile CNNs
- `pooling` — max, avg (2d and 3d)

### spatial ops
- `interpolate` — nearest/bilinear/area upsampling. used everywhere: UNet, VAE, resolution matching
- `pixel_shuffle` — sub-pixel convolution for upscaling: `(C*r², H, W) → (C, H*r, W*r)`. ESRGAN, SwinIR
- `pixel_unshuffle` — inverse: `(C, H, W) → (C*r², H/r, W/r)`. T2I-Adapter, WanCamAdapter
- `patch_embed` — Conv2d/Conv3d with kernel=patch_size, stride=patch_size. all DiT models
- `unpatchify` — reconstruct spatial tensor from patch sequence

### embedding
- `token_embed` — lookup table
- `pos_embed` — learned or sinusoidal

### special
- `noise_schedule` — diffusion timestep (cosine, linear, flow-matching sigma schedules)
- `flow_step` — normalizing flow (VITS/TTS)
- `quantize` / `dequantize` — runtime Q4/Q8 conversion
- `sample` — top-p, top-k, temperature, grammar-constrained

### adapter ops (LoRA runtime)
- `lora_apply` — `up @ down` with alpha/rank scaling, applied lazily during inference
- `kron` — Kronecker product for LoKr adapter
- `matrix_inverse` — Cayley transform for OFT adapter: `R = (I+Q)(I-Q)⁻¹`

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

MLX is Apple's Python/C++ framework over Metal. we target Metal directly — more control (simdgroup ops, residency sets, buffer management). MLX format (.safetensors + config.json) load identically to any safetensors — the format is the same, only the Python runtime differs. soma-runtime replaces MLX.

## scheduling — where to run each op

the scheduler decides which backend executes each op. four levels, each builds on the previous:

### level 0 — static rules (works immediately)

```
op is pure memory layout (reshape, permute, split)? → CPU (zero compute)
tensor < 4KB?  → CPU (GPU dispatch overhead > compute)
tensor ≥ 4KB?  → GPU
```

### level 1 — capability detection (at startup)

```rust
match detect_hardware() {
    AppleSilicon { ane }   → Metal + ANE + CPU
    NvidiaGpu              → CUDA + CPU
    AnyGpu                 → wgpu (Vulkan) + CPU
    Browser                → WebGPU + WASM
    Android { npu }        → NPU (QNN/NNAPI) + Vulkan + CPU
    NoGpu                  → CPU SIMD only
}
```

### level 2 — auto-tune (first run on new hardware)

benchmark each jet on each available backend. measure three dimensions:

| dimension | what it captures | why it matters |
|-----------|-----------------|----------------|
| compute | FLOPS per op | matmul is compute-bound on GPU |
| bandwidth | bytes/sec memory throughput | attention is bandwidth-bound (KV cache reads) |
| memory | peak allocation per op | large tensors may not fit on GPU, must split |

benchmark produces a dispatch table:

```
(jet, tensor_shape) → (backend, expected_ms, bytes_transferred)

matmul_f16(2048×2048): Metal=0.3ms/12MB  CPU=12ms/12MB  ANE=0.8ms/12MB → Metal
matmul_f16(64×64):     Metal=0.12ms/32KB CPU=0.05ms/32KB              → CPU
attention(2K ctx):     Metal=0.4ms/48MB  CPU=180ms/48MB               → Metal (bandwidth-bound)
attention(32K ctx):    Metal=2.1ms/780MB CPU=OOM                      → Metal (only option)
conv2d(3×3, 256ch):    Metal=0.2ms/4MB   ANE=0.15ms/4MB               → ANE
embedding(32K vocab):  CPU=0.02ms/128KB                               → CPU (tiny lookup)
rmsnorm(2048):         CPU=0.03ms/16KB                                → CPU
```

cached to disk. re-benchmark on hardware change or `soma-runtime autotune`.

the bandwidth dimension is critical: LLM decode is memory-bandwidth-bound, not compute-bound. a 7B model at Q4 reads ~3.5GB of weights per token. on M1 Pro (200 GB/s bandwidth), that is ~17ms per token — the theoretical floor. no compute optimization beats the memory wall. the scheduler must account for this: if two models compete for bandwidth, one must wait or move to CPU.

### level 3 — runtime adaptive (under load)

```
GPU busy (another model computing)?
  → route this op to CPU or ANE

bandwidth saturated?
  → queue op, don't block other models

ANE power budget exceeded?
  → fallback to Metal GPU

memory pressure?
  → offload large tensors to CPU RAM, compute there
  → or drop to lower quantization (f16 → q4)
```

the scheduler is a policy function over atoms — it decides WHERE, not WHAT. changing policy does not change computation results, only speed and power. the policy itself can be profiled and tuned per-device.

## soma model standard

one model = one directory. config.toml is the single entry point — the runtime reads it and knows everything: what the model is, what to load, how to run it. no JSON. no duplicates. no waste.

### canonical layout

```
model_name/
├── config.toml        # REQUIRED: model_type, architecture, params, components
├── tokenizer.json     # if model has tokenizer (kept for tokenizers crate)
├── vocab.toml         # tokenizer metadata (type, vocab_size)
├── chat.toml          # chat template + special tokens (generative models)
├── sampling.toml      # default inference params (generative models)
└── weights.*          # weights file(s) declared in config.toml
```

only config.toml and weights are required. everything else is optional based on model type.

### config.toml — the single entry point

every model has config.toml. it declares three things:
1. what the model is (`model_type`, `architecture`)
2. how big it is (dimensions, layers, vocab)
3. where the weights are (implicit `weights.*` or explicit `[components]`)

#### LLM decoder example (qwen, llama, deepseek)

```toml
model_type = "qwen3"
architecture = "Qwen3ForCausalLM"
hidden_size = 1024
num_attention_heads = 16
num_key_value_heads = 8
num_hidden_layers = 28
intermediate_size = 3072
vocab_size = 151936
max_position_embeddings = 40960
rope_theta = 1_000_000.0
rms_norm_eps = 1e-6
tie_word_embeddings = true
```

#### multi-component model (TTS, diffusion, VLM with vision projector)

```toml
model_type = "xtts"
architecture = "XttsModel"
gpt_layers = 30
gpt_n_model_channels = 1024

[components.gpt]
weights = "model.pth"
role = "autoregressive-decoder"

[components.dvae]
weights = "dvae.pth"
role = "discrete-vae"

[components.speakers]
weights = "speakers.pth"
role = "speaker-embeddings"
```

the `[components]` section replaces the single `weights.*` convention. runtime iterates components and loads each by role.

#### multi-voice TTS

```toml
model_type = "vits"
architecture = "VitsModel"
sample_rate = 22050

[[voices]]
name = "en_US-amy-medium"
language = "en"
weights = "en/en_US/amy/medium/en_US-amy-medium.onnx"

[[voices]]
name = "ru_RU-denis-medium"
language = "ru"
weights = "ru/ru_RU/denis/medium/ru_RU-denis-medium.onnx"
```

#### classifier/encoder (deberta, granite, jina)

```toml
model_type = "roberta"
architecture = "RobertaForSequenceClassification"
hidden_size = 768
num_attention_heads = 12
num_hidden_layers = 12
num_labels = 2
```

#### detector (YOLO)

```toml
model_type = "yolo"
architecture = "YOLOv11"
variant = "nano"
num_classes = 80
input_size = 640
```

#### language classifier (fasttext)

```toml
model_type = "fasttext"
num_languages = 2102
```

### model_type values

| model_type | runtime path | examples |
|-----------|-------------|----------|
| qwen2, qwen3 | transformer_decoder | qwen2.5-*, qwen3-* |
| llama | transformer_decoder | smollm2, mistral |
| phi3 | transformer_decoder | nuextract-1.5 |
| bitnet | transformer_decoder (ternary) | bitnet-2b |
| mimo | moe_decoder | mimo-7b-rl |
| roberta, deberta-v2 | transformer_encoder | granite-hap, deberta-zeroshot |
| modernbert, eurobert | transformer_encoder | modernbert, jina-v5 |
| moondream | transformer_decoder + vision | moondream2 |
| qwen2_vl, qwen3_5_vl | transformer_decoder + vision | qwen2.5-vl, qwen3.5-4b |
| whisper | encoder_decoder | whisper-small |
| yolo | cnn_detector | yolo11n |
| beats | transformer_encoder (audio) | beats |
| fasttext | fasttext | glotlid |
| vits | tts (VITS/piper) | piper-tts |
| xtts | tts (autoregressive) | xtts-v2 |
| wan | diffusion_dit | wan22-video |

### vocab.toml — tokenizer

```toml
type = "bpe"                          # bpe | unigram | wordpiece | byte
# merge rules and vocabulary embedded as TOML arrays
# converted from HF tokenizer.json at import time
```

### chat.toml — conversation format

```toml
template = """
{%- for message in messages %}
<|im_start|>{{ message.role }}
{{ message.content }}<|im_end|>
{%- endfor %}
"""
bos_token = "<|endoftext|>"
eos_token = "<|im_end|>"
pad_token = "<|endoftext|>"
```

### sampling.toml — inference defaults

```toml
temperature = 0.7
top_p = 0.9
top_k = 40
min_p = 0.05
repetition_penalty = 1.1
max_tokens = 2048
stop = ["<|im_end|>", "<|end|>"]
```

### .cyb — the model format

one file = complete model. CBOR header + [[nox]] program + vocabulary + BAO-chunked tensor data. content-addressable via CIDs.

see [[cyb-format]] for full specification.

```
cyb-llm import <source>   # download + convert → .cyb
cyb-llm run model.cyb     # inference
```

## quantization as a first-class concept

quantization is per-tensor, not per-model. a single model can have:
- attention weights in Q4
- embedding table in f16
- output projection in Q8
- BitNet layers in ternary

the jet registry dispatches to the right kernel based on input tensor dtype:
```
matmul(a: f16, b: q4)    → kernel_matmul_f16_q4
matmul(a: f16, b: ternary) → kernel_matmul_ternary  // add/subtract only
matmul(a: f16, b: f16)   → kernel_matmul_f16
```

## memory architecture — zero-copy physical

the single biggest performance unlock for inference on Apple Silicon. every existing framework (Ollama, llama.cpp, CoreML, MLX) leaks performance through copies:

```
current (everyone):
NVMe → kernel buf → mmap buf → malloc buf → Metal buf → GPU → result
                     copy 1      copy 2       copy 3

cyb-llm:
NVMe DMA → PhysPage → AMX / Metal / ANE → result
              ↑
     never moves, never copies
```

three copies of a 7B Q4 model (3.5GB) burn 10.5GB of bandwidth. on M1 Pro (200 GB/s), that is 5% of total bandwidth wasted before any computation. for tier 0 (8 models loaded simultaneously), the waste compounds.

Apple Silicon unified memory makes copies unnecessary. CPU, GPU, AMX, ANE, NVMe controller share the same physical DRAM. one buffer, visible to all hardware units, zero copies. [[unimem]] is the Rust crate that makes this accessible from userspace.

### the physical memory stack

```
┌─────────────────────────────────────────────────────┐
│ PhysPage — IOKit contiguous allocation               │
│   va: *mut u8 (CPU access)                          │
│   pa: u64     (hardware access)                     │
│   pinned: never swapped, never moved                │
├─────────────────────────────────────────────────────┤
│ HypRegion — Hypervisor stage-2 page tables           │
│   deterministic latency (~30ns)                     │
│   private TLB namespace (no shootdowns)             │
├─────────────────────────────────────────────────────┤
│ Arena — bump allocator over physical pages            │
│   alloc: ~4ns (single atomic fetch_add)             │
│   reset: ~0ns (move cursor to zero)                 │
│   pa_of(ptr): pure arithmetic, no syscall           │
├─────────────────────────────────────────────────────┤
│ Pool — fixed-size tensor slots                       │
│   acquire/release: ~10ns (lock-free queue)          │
│   pre-allocated at model load                       │
├─────────────────────────────────────────────────────┤
│ DmaTarget trait — one interface to all hardware      │
│   submit(pa, size, op) → DmaToken                   │
│   AMX, ANE, NVMe all implement the same trait       │
└─────────────────────────────────────────────────────┘
```

### how .cyb loads with zero copies

the .cyb file format is designed for this pipeline. tensor data section is 64-byte aligned — exactly what NVMe DMA and AMX require.

```
// current: 3 copies
let data = std::fs::read("model.cyb")?;           // NVMe → kernel → userspace (copy 1)
let buf = device.new_buffer_with_bytes(&data);      // userspace → GPU (copy 2)

// with unimem: 0 copies
let page = PhysPage::alloc(tensor_data_size)?;      // pinned physical memory
nvme.submit(page.pa(), tensor_data_size, Read);     // NVMe DMA → physical RAM directly
// page.pa() is the same DRAM that Metal/AMX/ANE see
// no copy — hardware reads from the same physical address
```

model load time goes from O(file_size / memcpy_bandwidth) to O(file_size / nvme_bandwidth). on M1 Pro: NVMe reads at ~5 GB/s. a 5GB model loads in 1 second with zero CPU involvement. the CPU is free to do other work while weights stream in.

### residency tiers on physical memory

three tiers mirroring [[soma]] cognitive architecture, now mapped to physical memory primitives:

| tier | soma role | memory primitive | lifecycle | budget |
|------|-----------|-----------------|-----------|--------|
| resident | tier 0 (8 always-on models) | PhysPage, pinned forever | alloc at boot, never freed | ~2GB |
| cached | tier 1-2 (on-demand models) | Arena region, LRU eviction | load on first use, evict under pressure | ~10GB |
| streamed | activations, KV cache | Pool slots, acquire/release | per-inference, O(1) recycle | ~2GB |

resident models never touch the allocator after boot. cached models load via NVMe DMA into Arena regions — eviction moves the arena cursor back, pages stay pinned. streamed allocations are pool slots recycled every forward pass — ~10ns per acquire/release, zero syscalls.

### KV cache on physical memory

KV cache is the single largest dynamic allocation during inference:

```
kv_size_per_token = num_layers × 2 × num_kv_heads × head_dim × sizeof(f16)
qwen3-0.6b: 28 × 2 × 8 × 128 × 2 = 114KB per token
qwen2.5-coder-14b: 48 × 2 × 8 × 128 × 2 = 196KB per token
at 4K context: 456MB — 786MB
```

with Pool: pre-allocate KV slots at model load. each decode step acquires the next slot (~10ns). no malloc, no page fault, no TLB miss. physical address known — AMX/ANE can read KV directly without any staging.

paged KV attention: allocate KV in fixed-size blocks (e.g. 256 tokens per block). blocks are Pool slots. eliminates fragmentation entirely.

### cross-unit pipeline — zero copies between stages

the full inference pipeline touches multiple hardware units. with physical memory, the handoff is instant:

```
PhysPage at pa=0x8_0000_0000:
  │
  ├─ NVMe DMA: load weights into pa       (NVMe controller → DRAM)
  │
  ├─ CPU NEON: dequantize Q4 → f16 at pa  (CPU reads/writes same DRAM)
  │
  ├─ AMX: matmul on pa                     (AMX reads same DRAM via fabric)
  │
  ├─ Metal GPU: attention kernel on pa     (GPU reads same DRAM via fabric)
  │
  ├─ ANE: tier 0 model on pa              (ANE reads same DRAM via fabric)
  │
  └─ CPU: read result from pa             (zero copy out)
```

every arrow is the same physical DRAM cells. no buffer copy, no staging, no synchronization barrier. the unified memory fabric handles coherency in hardware.

### the bandwidth wall — why this matters

LLM decode is memory-bandwidth-bound, not compute-bound. a 7B Q4 model reads ~3.5GB of weights per token. on M1 Pro (200 GB/s):

```
theoretical floor:  3.5GB / 200 GB/s = 17.5ms per token = 57 tok/s
with 3 copies:      3.5GB × 4 / 200 GB/s = 70ms per token = 14 tok/s
with zero-copy:     3.5GB / 200 GB/s = 17.5ms per token = 57 tok/s
```

zero-copy recovers 4x bandwidth. the theoretical limit becomes achievable. for smaller models (0.6B router, 0.5B intent): weights fit in L2/SLC cache after first pass — subsequent tokens are cache hits at ~400 GB/s, yielding 1000+ tok/s.

### what changes in the runtime

| component | current | with unimem |
|-----------|---------|-------------|
| model load | `std::fs::read()` + memcpy | `PhysPage::alloc()` + NVMe DMA |
| weight storage | `Vec<u8>` on heap | `PhysPage` pinned in DRAM |
| activation alloc | `device.new_buffer()` per layer | `Pool::acquire()` ~10ns |
| KV cache | `Vec::push()` growing | `Pool::acquire()` fixed blocks |
| Metal dispatch | copy to MTLBuffer | `pa_of(ptr)` → same DRAM, no copy |
| ANE dispatch | IOSurface copy | `pa` → ANE descriptor directly |
| cross-unit handoff | buffer copy + fence | same pa, hardware coherency |
| model eviction | `drop(Vec)` + dealloc | `arena.reset()` ~0ns, pages stay |

the API surface for unimem integration:

```rust
// in cyb-llm backend
trait MemoryBackend {
    fn alloc_weights(&self, size: usize) -> PhysRegion;
    fn alloc_activation(&self, size: usize) -> PoolSlot;
    fn pa_of(&self, ptr: *const u8) -> u64;
    fn load_from_cyb(&self, path: &Path, tensor_offset: u64, size: u64) -> PhysRegion;
}

// default: mmap backend (works everywhere, 3 copies)
// apple silicon: unimem backend (zero copies, 4x bandwidth)
```

runtime auto-detects Apple Silicon and uses unimem when available. mmap fallback for other platforms. same .cyb file, same inference code, different memory backend.

### write-new-only — the memory model

every write in the inference pipeline goes to a NEW physical address. never overwrite, never update in place. this is the same principle as [[cyberlinks]] (append-only), [[bbg]] (append-only blocks), and STARK traces (append-only log).

why:

| property | write-new-only | update-in-place |
|----------|---------------|-----------------|
| synchronization | none — no reader/writer conflict | fences, barriers, locks |
| hardware coherency | free — cache lines stay Shared | invalidation storms between units |
| provability | every intermediate hashable — full STARK trace | input destroyed on overwrite |
| pipeline parallelism | layer N+1 reads while N+2 loads — no stalls | must wait for write to complete |
| garbage collection | arena.reset() = O(1) | free each allocation individually |
| debugging | full history visible — time travel | state lost on overwrite |

the arena cursor IS the state of computation. at any point, everything before cursor = valid data, everything after = free space. rewinding the cursor = undoing computation. this is deterministic replay for free.

### orchestration — dataflow execution

with write-new-only, orchestration is pure dataflow. the Graph IR is already a DAG — each Node declares inputs and outputs. an op executes when all its inputs exist. "exist" means the producer wrote to that address and signaled completion.

```
scheduler loop:
    for each node in topological order:
        wait(all input DmaTokens)
        output_pa = arena.alloc(output_size)
        token = backend.submit(input_pas, output_pa, node.op)
        register(node.outputs, token)
```

no locks. no shared mutable state. no condition variables. the DmaToken IS the synchronization primitive — it represents "this physical address now contains valid data."

hardware units discover their turn through the dataflow graph:

```
NVMe: "weights at pa₀ ready" → token₀ complete
  │
  ▼
AMX/Metal: "matmul(pa₀, activation) → pa₁" → token₁ complete
  │                                              │
  ▼                                              ▼
Metal: "attention(pa₁, kv_cache) → pa₂"    ANE: "norm(pa₁) → pa₃"
  │                                              │
  ▼                                              ▼
        "add(pa₂, pa₃) → pa₄" → token₄ complete
```

multiple hardware units run concurrently on different physical addresses. NVMe loads layer N+1 weights while Metal computes layer N attention while ANE runs layer N normalization. all reading/writing different addresses — zero contention.

### arena lifecycle during inference

```
one forward pass (e.g. 28 layers of qwen3-0.6b):

cursor: 0                                              cursor: 47MB
  │                                                        │
  ▼                                                        ▼
  ┌────┬────┬────┬────┬─── ... ───┬────┬────┬────┬────┐
  │ Q₀ │ K₀ │ V₀ │att₀│          │Q₂₇│K₂₇│V₂₇│out │
  └────┴────┴────┴────┴─── ... ───┴────┴────┴────┴────┘

  all 28 layers computed, result at end of arena.

  next token: arena.reset() → cursor back to 0
  pages stay pinned. no deallocation. ~0ns.
  weights are in separate PhysPages — untouched.
```

KV cache lives in Pool slots (not Arena) because it persists across tokens:

```
Pool slots:          Arena (reset per token):
┌────┬────┬────┐    ┌────────────────────────┐
│KV₀ │KV₁ │KV₂ │    │ activations, temporary │
│perm│perm│perm│    │ reset after each token │
└────┴────┴────┘    └────────────────────────┘
  grows with context    fixed budget per pass
```

## integration with [[Nox]]

the llm runtime is not a standalone system — it is a [[Nox]] execution engine. every inference maps to cyber primitives:

### inference = [[Order]]

every inference request is a [[Nox]] [[Order]]:

```
Order {
    formula:  hash(model_weights + graph_ir),    // what computation
    input:    hash(prompt_tokens),                // input particle
    budget:   { compute: N, memory: M },          // resource limits
    sigma:    payment,                            // cost in Tok
}
```

the runtime executes the Order. if budget exceeded → Order fails, no sigma spent. if successful → result produced, sigma transferred.

### result = [[cyberlink]]

every inference output creates a [[cyberlink]] in the [[cybergraph]]:

```
cyberlink(input_particle, output_particle, weight: confidence)
```

- question → answer = cyberlink
- image → description = cyberlink (VLM)
- audio → transcript = cyberlink (whisper)
- prompt → generated_image = cyberlink (flux)

the cybergraph grows with every inference. knowledge accumulates. [[tri-kernel]] recomputes weights. high-quality answers gain gravity. the system learns which model produces the best links.

### trace = STARK proof

every op execution produces a trace entry:
```
(op_id, input_hashes, output_hash, timing_ns)
```

the trace is attached to the [[Order]] as a STARK-compatible execution record. given the same weights and input, any verifier can replay the trace and confirm the output. the model cannot lie because every matrix multiply is auditable.

### cost = [[Tok]] pricing

inference costs resources. the runtime meters:

```
Order cost = Σ(
    compute:   ops_executed × π_compute,
    memory:    peak_bytes × duration × π_memory,
    bandwidth: bytes_transferred × π_bandwidth
)
```

φ* prices derived from [[Tok]] conservation rules. the [[neuron]] pays for its own inference. profitable Orders earn more sigma than they cost. unprofitable Orders drain sigma. this is natural selection for useful computation.

### context = [[bbg]] state

the context window is not ephemeral RAM — significant context persists in [[bbg]]:

- system prompts = [[particle]]s in bbg (permanent, content-addressed)
- conversation history = chain of cyberlinks (append-only)
- tool results = cyberlinks with tool output as target particle
- model weights = nouns in bbg (content-addressed, shared across neurons)

ephemeral state (KV cache, intermediate tensors) lives in GPU memory only. everything else has a bbg address and can be proven.

## component boundaries

three layers. each layer has one job. violations of these boundaries create confusion.

### layer 1 — hardware drivers (know hardware, know nothing about models)

| crate | hardware | job | does NOT do |
|-------|----------|-----|-------------|
| [aluminum](https://github.com/cyberia-to/aluminum) | Metal GPU | device, buffer, pipeline, dispatch. one kernel fast | transformer forward pass, layer sequencing, weight management |
| [rane](https://github.com/cyberia-to/rane) | Apple Neural Engine | MIL compile, IOSurface, model load/run/unload | weight layout, KV cache policy, model config parsing |
| wgpu | Vulkan/DX12/WebGPU | cross-platform compute dispatch | anything hardware-specific |
| std::arch | CPU SIMD | NEON/AVX2/AVX512 intrinsics | anything above single op |

aluminum answers: "dispatch this MSL kernel on these buffers, return result, maximum GFLOPS."
rane answers: "compile this MIL program, load to ANE, run on this IOSurface, return result."
they do NOT answer: "run 28 transformer layers in sequence with KV cache."

### layer 2 — jets (know ops, know nothing about models)

lives in `cyb/llm/backend/`. each jet = one op implemented for one backend.

```
cyb/llm/backend/
├── metal/     — MSL kernels, dispatch via aluminum
│   ├── matmul.metal
│   ├── attention.metal
│   ├── rmsnorm.metal
│   └── ... (~48 jets as .metal files)
├── ane/       — MIL programs, dispatch via rane
│   ├── matmul.rs    (generates MIL, calls rane::AneModel)
│   ├── ffn.rs       (generates MIL for fused FFN)
│   └── sdpa.rs      (generates MIL for attention)
├── wgpu/      — WGSL shaders (already 20 files, 89 pipelines)
└── cpu/       — SIMD ops (rmsnorm, rope, attention, sample, embed)
```

a jet knows: "matmul(A, B) on Metal = this MSL shader dispatched via aluminum."
a jet does NOT know: "this matmul is layer 7's Q projection in a Qwen3 model."

### layer 3 — runtime (knows models, knows nothing about hardware)

lives in `cyb/llm/` top level. orchestrates everything.

```
cyb/llm/
├── ir/         — graph IR, atoms, jet registry
├── loader/     — model formats (onnx, gguf, safetensors, ...)
├── generate/   — prefill→decode loop, sampling
├── context/    — context management, cybergraph retrieval
├── schedule/   — op→backend routing, auto-tune
└── trace/      — STARK provability
```

the runtime knows: "Qwen3 has 28 layers, each needs rmsnorm→attention→ffn. use Metal for matmul, CPU for rmsnorm, ANE for embedding."
the runtime does NOT know: how to dispatch a Metal kernel (aluminum does that).

### the test: where does code belong?

```
"dispatch this compute shader on GPU"           → aluminum
"compile this MIL and run on ANE"               → rane
"implement matmul in MSL using aluminum"         → cyb/llm/backend/metal/
"implement matmul in MIL using rane"             → cyb/llm/backend/ane/
"implement matmul in WGSL"                       → cyb/llm/backend/wgpu/
"implement rmsnorm in NEON SIMD"                 → cyb/llm/backend/cpu/
"run 28 layers of Qwen3 in order"                → cyb/llm/generate/
"decide matmul goes to Metal not wgpu"           → cyb/llm/schedule/
"load weights from GGUF"                         → cyb/llm/loader/
"manage KV cache across tokens"                  → cyb/llm/generate/
"build context from cybergraph"                  → cyb/llm/context/
```

### dependencies

```toml
# cyb/llm/Cargo.toml
[dependencies]
aluminum = { path = "../aluminum" }   # layer 1: Metal driver
rane = { path = "../rane" }           # layer 1: ANE driver
wgpu = "24"                           # layer 1: cross-platform GPU
```

aluminum and rane do NOT depend on cyb/llm. cyb/llm depends on them. the arrow points one way.

### what currently violates boundaries

rane currently contains code that belongs in cyb/llm:

| file in rane | should be in | why |
|---|---|---|
| ops/rmsnorm.rs | cyb/llm/backend/cpu/ | CPU op implementation, not ANE driver |
| ops/rope.rs | cyb/llm/backend/cpu/ | CPU op implementation |
| ops/attention.rs | cyb/llm/backend/cpu/ | CPU op implementation |
| ops/sample.rs | cyb/llm/backend/cpu/ | sampling logic |
| ops/embed.rs | cyb/llm/backend/cpu/ | CPU op implementation |
| ops/activation.rs | cyb/llm/backend/cpu/ | CPU op implementation |
| ops/loss.rs | cyb/llm/backend/cpu/ | training op |
| ops/adam.rs | cyb/llm/backend/cpu/ | training op |
| weights.rs | cyb/llm/loader/ | weight management |
| config.rs | cyb/llm/ir/ | model architecture config |
| mil/ffn.rs | cyb/llm/backend/ane/ | ANE jet (uses rane as driver) |
| mil/projection.rs | cyb/llm/backend/ane/ | ANE jet |
| mil/sdpa.rs | cyb/llm/backend/ane/ | ANE jet |

rane keeps: ffi.rs, model.rs, surface.rs, staging.rs, accel.rs, mil/mod.rs (generic MIL builder), probe/. pure driver.

### Nox integration

the llm runtime is a host jet called from [[Nox]] reduction:

```
Nox (control plane, provable)
    │
    ├── pure jets → field arithmetic ([[Trident]], proven)
    │
    └── host jet: infer(model, input)
          │
          └── cyb/llm (layer 3 — runtime)
                │
                ├── schedule: pick backend
                │
                └── cyb/llm/backend/* (layer 2 — jets)
                      │
                      ├── aluminum (layer 1 — Metal)
                      ├── rane (layer 1 — ANE)
                      ├── wgpu (layer 1 — cross-platform)
                      └── CPU SIMD (layer 1 — fallback)
```

[[Trident]] uses aluminum for GPU field arithmetic through the same layer 1 driver. shared hardware access, different jets.

orchestration decisions (which model, what context, which tool) run ON Nox — provable. tensor computation runs THROUGH Nox as a host jet — fast, native GPU, auditable via STARK trace but not itself a Nox reduction.

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

### KV cache compression — TurboQuant

the KV cache is the memory wall. at long contexts it consumes more memory than the model itself:

```
qwen2.5-coder-14b at 128K context:
  F16 KV cache = 48 × 2 × 8 × 128 × 131072 × 2 bytes = 25 GB
  model weights Q4 = 5.9 GB
  KV cache is 4x larger than the model
```

TurboQuant (Google Research, ICLR 2026) compresses KV cache from 16 bits to 2-3 bits with zero accuracy loss, approaching the Shannon limit:

two-stage algorithm:

1. PolarQuant — rotate K/V vectors by a precomputed matrix. after rotation, value distribution becomes predictable (near-uniform). the quantizer is computed once at model load — no calibration data needed. eliminates the 1-2 bits of scale/zero-point overhead that every previous method wastes.

2. QJL (Quantized Johnson-Lindenstrauss) — project the quantization residual to a single sign bit. kills systematic bias in attention scores. compressed attention output is statistically identical to full precision.

critical: naive implementation produces garbage. without proper bias correction in QJL, quantization errors compound and the model becomes unusable. the math must be followed exactly.

impact on cyb-llm:

```
128K context, qwen2.5-coder-14b:
  F16:  25 GB KV cache → does not fit on 16GB machine
  Q8:   12.5 GB        → barely fits, no room for model
  Q4:   6.2 GB          → fits but quality degrades
  Q3 (TurboQuant): 4.7 GB → fits with model (5.9GB) on 16GB
  Q2 (TurboQuant): 3.1 GB → fits with room for tier 0 models
```

128K context on a 16GB laptop. this was impossible before.

implementation in the runtime:

```
// at model load: precompute rotation matrix (PolarQuant)
let rotation = polar_quant::precompute(head_dim);  // one-time, ~1ms

// at each token: compress K/V before storing in Pool slot
let k_compressed = polar_quant::quantize(&k_vector, &rotation, bits=3);
let v_compressed = polar_quant::quantize(&v_vector, &rotation, bits=3);

// at attention: decompress and correct bias (QJL)
let k_restored = qjl::dequantize(&k_compressed, &rotation);
// k_restored is statistically identical to original k_vector
```

new ops in Graph IR:

| op | what | where |
|----|------|-------|
| `KvCompress { bits, rotation }` | PolarQuant + store in Pool slot | after each layer's K/V projection |
| `KvDecompress { bits, rotation }` | dequantize + QJL bias correction | before attention computation |

jets: fused `kv_compress` + `kv_decompress` kernels for Metal/CUDA. the rotation is a matmul — fuses with the K/V projection matmul that already runs.

this integrates with the write-new-only memory model: compressed KV entries are written to new Pool slots (append-only). decompression reads from existing slots (read-only). no mutation.

### paged KV attention
- allocate KV cache in fixed-size blocks (e.g. 256 tokens per block), not contiguous
- blocks are Pool slots from [[unimem]] — acquire/release in ~10ns
- with TurboQuant compression: each block is 5-8x smaller → more blocks fit in memory
- prefix caching: reuse KV blocks for common prompt prefixes across requests

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
  → compress KV cache (f16 → q3 TurboQuant) — zero quality loss, 5x savings
  → if still OOM → compress KV further (q3 → q2) — near-zero loss
  → if still OOM → shed lowest-priority model
  → if still OOM → offload layers to CPU
  → if still OOM → reduce context window
  → never crash
```

### adaptive precision
dynamically switch quantization based on available memory. TurboQuant makes the KV dimension much more aggressive without quality loss:

```
plenty of RAM   → f16 weights, f16 KV cache  (maximum quality)
moderate        → q8 weights,  q3 KV cache   (TurboQuant, zero loss)
heavy pressure  → q4 weights,  q2 KV cache   (TurboQuant, near-zero loss)
extreme         → q4 weights,  q2 KV cache, reduced context
```

the key insight: KV cache compression via TurboQuant is nearly lossless, so the runtime should compress KV aggressively FIRST (free quality), then reduce weight precision (costs quality), then reduce context (costs capability). previous order was wrong — everyone compressed weights before KV cache because KV compression had quality loss. TurboQuant inverts the priority.

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

text → tokens → text. different models use different tokenizers.

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

## inference parameters

every generation call takes a parameter set that controls output behavior:

| parameter | what it does | range | soma defaults |
|-----------|-------------|-------|--------------|
| temperature | controls randomness. 0 = deterministic, 1 = diverse, 2 = chaos | 0.0 — 2.0 | router: 0.0. reasoning: 0.7. creative: 1.0 |
| top_p | nucleus sampling — consider only tokens covering P% of probability mass | 0.0 — 1.0 | 0.9 general. 0.5 for code/JSON |
| top_k | consider only K most probable tokens | 1 — vocab | 40 general. 1 = greedy |
| min_p | discard tokens with probability < min_p × max_probability | 0.0 — 1.0 | 0.05 — removes garbage better than top_p alone |
| repetition_penalty | penalize tokens that already appeared in output | 1.0 — 2.0 | 1.1 for conversation. 1.0 for code |
| max_tokens | hard limit on output length | 1 — context | task-dependent |
| stop | stop sequences — generation halts when any of these appear | strings | `["<|end|>"]` from chat template |
| seed | fix random state for reproducibility | int | always set for STARK provability |

soma presets:

```
router:    { temperature: 0.0, seed: 42 }              — deterministic, provable
code:      { temperature: 0.0, top_p: 0.5, seed: 42 }  — precise, no creativity
reasoning: { temperature: 0.7, min_p: 0.05 }           — balanced
creative:  { temperature: 1.0, top_p: 0.95, min_p: 0.05, repetition_penalty: 1.2 }
```

### sampling pipeline

applied in order after logits computed:

```
raw logits
  → repetition penalty (modify logits of seen tokens)
  → temperature (divide logits by T)
  → top_k (keep only K highest)
  → top_p (keep cumulative prob ≤ P)
  → min_p (drop below threshold)
  → sample from remaining distribution
  → grammar constraint (reject if invalid, resample)
```

grammar-constrained decoding: force output to match a schema (JSON, regex) via finite state machine over token vocabulary. critical for [[soma]] router — must output valid `{"tier": 1, "slot": 3}`, malformed routing = system failure.

## model registry

maps model_type → architecture template + tensor names + tokenizer + chat template. each model family has different tensor naming conventions:

```
qwen3:   model.layers.0.self_attn.q_proj.weight
llama:   model.layers.0.self_attn.q_proj.weight  (same)
mistral: model.layers.0.self_attn.q_proj.weight  (same)
bert:    encoder.layer.0.attention.self.query.weight  (different)
whisper: decoder.layers.0.self_attn.q_proj.weight  (different prefix)
yolo:    model.0.conv.weight  (completely different)
```

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

three access modes: library (embedded in Rust code), daemon (always-on process), CLI (one-shot commands).

### library API (Rust)

```rust
// ── runtime lifecycle ──
let rt = Runtime::new(Config::auto())?;     // detect hardware, init backends
rt.backends()                                // → [Metal, ANE, CPU]
rt.memory_stats()                            // → { total, used, available, per_model }
rt.autotune()                                // benchmark jets on all backends, cache dispatch table
rt.shutdown()                                // unload all models, free GPU memory

// ── model management ──
let model = rt.load("qwen3.5-9b.gguf")?;   // load model, build graph, allocate buffers
rt.unload(model.id())?;                      // free model memory
rt.list_models()                             // → [{ id, name, params, memory, tier }]
rt.hot_swap(old_id, "new_model.gguf")?;      // atomic replace without downtime

// ── text generation (streaming) ──
let stream = model.generate(prompt, params)?;
for token in stream {                        // yields tokens as generated
    print!("{}", token.text);
    if token.is_tool_call() {                // tool call detected mid-stream
        let result = execute_tool(token.tool_call())?;
        stream.inject_tool_result(result)?;  // continue generation with result
    }
}

// ── batch generation ──
let results = model.generate_batch(prompts, params)?;  // concurrent inference

// ── embedding ──
let vector = model.embed("search query")?;   // → Vec<f32>, 768-dim

// ── classification ──
let scores = model.classify(text, &["urgent", "normal", "spam"])?;
                                              // → [("urgent", 0.92), ("normal", 0.07), ...]

// ── multimodal ──
let response = model.generate_with_image(prompt, image_bytes, params)?;
let response = model.generate_with_audio(prompt, audio_bytes, params)?;
let transcript = model.transcribe(audio_bytes)?;        // whisper
let detections = model.detect(image_bytes)?;             // YOLO → [{ class, bbox, confidence }]
let image = model.generate_image(prompt, image_params)?; // flux/wan2.2

// ── LoRA adapters ──
model.load_lora("coding_style.safetensors", alpha: 0.7)?;
model.unload_lora("coding_style")?;
model.list_loras()                           // → [{ name, alpha, params }]

// ── context (see context management section) ──
let context = model.context();
context.set_system("...");
context.inject_graph(query);

// ── observability ──
model.metrics()                              // → { tok_per_sec, prefill_ms, peak_memory, ... }
rt.profile(model.id(), input)?               // → per-op timing breakdown
```

### daemon API

unix socket + JSON protocol. OpenAI-compatible where possible for ecosystem tooling.

```
POST /v1/completions          — text generation (streaming SSE)
POST /v1/embeddings           — vector embedding
POST /v1/classifications      — zero-shot classification
POST /v1/images/generations   — image generation
POST /v1/audio/transcriptions — speech-to-text
POST /v1/detections           — object detection

GET  /v1/models               — list loaded models
POST /v1/models/load          — load model from path
DELETE /v1/models/{id}        — unload model

GET  /v1/health               — { status, memory, models_loaded, backends }
GET  /v1/metrics              — per-model inference metrics
POST /v1/autotune             — trigger benchmark

POST /v1/context/show         — render current context
POST /v1/context/compress     — trigger compression
POST /v1/context/save         — persist session
POST /v1/context/load         — restore session
```

```
soma-runtime serve --config soma.toml --socket /tmp/soma.sock
```

the daemon loads tier 0 models on startup, manages model lifecycle, handles concurrent requests. this is what [[soma]] main loop talks to.

### CLI

```
# inference
soma-runtime run --model qwen3.5-9b.gguf --prompt "hello"
soma-runtime embed --model jina-v5-nano --text "search query"
soma-runtime detect --model yolov11.onnx --image photo.jpg
soma-runtime transcribe --model whisper-small.gguf --audio recording.wav
soma-runtime generate-image --model flux-schnell --prompt "sunset"

# model management
soma-runtime models list
soma-runtime models load path/to/model.gguf
soma-runtime models unload model_id
soma-runtime models info model_id              # params, memory, context, backend

# system
soma-runtime bench --model qwen3.5-9b.gguf    # tok/s, prefill, decode
soma-runtime autotune                           # benchmark all jets on hardware
soma-runtime memory                             # RAM/GPU usage breakdown
soma-runtime backends                           # available: Metal, ANE, CPU

# context
soma-runtime context show --model model_id
soma-runtime context tokens --model model_id
soma-runtime context compress --model model_id
soma-runtime context save session.json
soma-runtime context load session.json
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

## context management

quality of output = quality of context. a 4B model with precise context outperforms 70B with noise in the prompt. managing context is an active process, not passive window filling.

### context structure

```
┌──────────────────────────────────────────────┐
│               context window                  │
│  ┌──────────────────────────────────────────┐ │
│  │ system prompt (identity, rules, format)  │ │ ← static, KV cache reused
│  ├──────────────────────────────────────────┤ │
│  │ graph context (cybergraph retrieval)      │ │ ← dynamic, scored by gravity + links
│  ├──────────────────────────────────────────┤ │
│  │ conversation history                     │ │ ← compressed when grows
│  ├──────────────────────────────────────────┤ │
│  │ tool results                             │ │ ← function call outputs
│  ├──────────────────────────────────────────┤ │
│  │ current input                            │ │ ← user/system message
│  └──────────────────────────────────────────┘ │
│  total tokens ≤ max_position_embeddings       │
└──────────────────────────────────────────────┘
```

### cybergraph retrieval (not RAG)

standard RAG: embed query → cosine similarity search → inject top-K text chunks. a blunt instrument for systems without structure.

soma has [[cybergraph]] — a knowledge graph with typed links, [[tri-kernel]] weights, and provable history in [[bbg]]. retrieval uses the graph, not vectors alone.

four relevance signals:

```
relevance(page, query) =
    α · similarity(embed(query), embed(page))    — semantic proximity
  + β · link_distance(query_context, page)        — graph hops from current focus
  + γ · gravity(page)                             — tri-kernel importance score
  + δ · diffusion(page)                           — temporal activity (recency)
```

retrieval algorithm:

```
1. embed(query) → find seed pages by semantic similarity
2. from seeds, traverse outgoing [[wiki-links]] (1-2 hops)
3. score all candidates by relevance formula
4. rank by score, pack top pages into context budget
5. inject as structured content (frontmatter + body + links), not raw text
```

what this gives over RAG:
- a page about [[energy market]] links to [[sigma]], [[bounty]], [[neuron]] → all related context found via graph, no embedding needed
- high-gravity pages (core concepts) naturally surface → the model gets foundational context
- diffusion score captures "what changed recently" → temporal awareness without date filtering
- retrieval path is recorded in [[bbg]] → provable, auditable
- structured pages with frontmatter → model understands metadata, not just text

### context optimization

| mechanism | what it does | when |
|-----------|-------------|------|
| prefix caching | system prompt KV cache reused across requests — no recomputation | always (same prompt = free prefill) |
| context compression | old conversation summarized by tier 1 model, summary replaces full history | when history > 50% of context window |
| graph retrieval | traverse [[cybergraph]] from query context, rank by gravity × link proximity × diffusion, inject structured pages (not raw chunks) | every request with memory access |
| priority packing | score each context block by relevance, keep highest-scoring, drop rest | when total exceeds window |
| recency bias | most important content at the END of context (models attend more to recent tokens) | always — structure context accordingly |

### overflow handling

when input exceeds model's trained context:

- truncation: drop oldest tokens (simple, lossy)
- sliding window: process in chunks, carry KV cache forward (mistral-style)
- RoPE scaling: extend positional encoding beyond training length (YaRN, NTK-aware)
- the runtime reads `max_position_embeddings` and `rope_scaling` from config.json and applies automatically

### context budget per tier

| tier | typical context | budget strategy |
|------|----------------|-----------------|
| tier 0 (router, intent) | 2-4K | minimal — classify fast, don't waste tokens |
| tier 1 (fast tasks) | 4-8K | task input + brief system prompt |
| tier 2 (reasoning) | 8-32K | graph context + history + detailed system prompt |
| tier 3 (oracle API) | 32-200K | maximum context — send everything relevant |

### context API

three access levels: automatic (runtime handles it), programmatic (Rust API), CLI (debug/manual).

library (Rust):
```rust
let context = model.context();

// inspect
context.tokens()                          // total token count
context.blocks()                          // list blocks with sizes
context.show()                            // render full context as text

// structure
context.set_system("you are a router...") // set/replace system prompt
context.inject_graph(query)               // traverse cybergraph, inject relevant pages
context.add_message(role, content)        // append to conversation history
context.inject_tool_result(name, result)  // add tool output

// manage
context.compress()                        // summarize old history via tier 1 model
context.trim(max_tokens)                  // drop lowest-priority blocks to fit budget
context.clear_history()                   // keep system prompt, clear conversation
context.reset()                           // clear everything

// budget
context.set_budget(Block::System, 500)    // limit system prompt to 500 tokens
context.set_budget(Block::Graph, 2000)    // limit graph context to 2000 tokens
context.set_budget(Block::History, 4000)  // limit conversation history

// persist
context.save("session.json")             // persist conversation state to disk
context.load("session.json")             // restore from disk

// escalation — pass context between tiers
let tier2_context = context.escalate(tier2_model) // carry relevant context to higher tier
                                          // compresses to fit tier 2 budget
```

CLI:
```
soma context show                     # render current context with token counts
soma context tokens                   # total: 3847 / 32768
soma context blocks                   # system: 312, graph: 1200, history: 2100, input: 235
soma context compress                 # trigger compression now
soma context clear history            # keep system, clear rest
soma context budget graph 2000        # set graph context budget
soma context save session.json        # persist
soma context load session.json        # restore
```

automatic behaviors (no commands needed):
- prefix cache: reuse system prompt KV cache between requests — zero config
- auto-compress: when history exceeds budget, compress oldest messages automatically
- auto-trim: when total exceeds window, drop lowest-priority blocks before inference
- escalation context: when routing to higher tier, pack most relevant context into target budget

### context flow in soma main loop

```
input arrives
    │
    ▼
router context (2K budget):
    system: "classify intent, output JSON"
    input: raw message
    → router decides: tier 2, slot: reasoner
    │
    ▼
reasoner context (32K budget):
    system: "you are a reasoning agent..."
    graph: [3 pages from cybergraph, ranked by gravity × link proximity × diffusion]
    history: [last 5 exchanges, compressed]
    tool_results: [previous look() outputs if any]
    input: original message + router classification
    → reasoner generates response, possibly calling tools
    │
    ▼
tool call detected → execute → inject result → continue
```

each tier gets its own context built from shared state. the runtime manages context per-model, not globally — router sees 2K tokens, reasoner sees 32K, from the same conversation.

## tool use

soma models call tools — they don't just generate text. the runtime manages the tool call loop:

```
generate → detect tool call in output → parse → execute → inject result → continue generating
```

### tool call format

```
model output:
  "I need to check the sensor data.
   <tool_call>{"name": "look", "args": {"key": "sensor_3"}}</tool_call>"

runtime:
  1. detect <tool_call> tags
  2. parse JSON
  3. execute look(key="sensor_3") → returns sensor value
  4. inject result into context:
     "<tool_result>{"sensor_3": 42.7, "status": "normal"}</tool_result>"
  5. continue generation with result in context
```

### tool registry

```rust
struct ToolRegistry {
    tools: HashMap<String, ToolSpec>,
}

struct ToolSpec {
    name: String,
    description: String,           // for LLM to understand when to use
    parameters: JsonSchema,         // input validation
    handler: fn(Value) -> Value,    // execution
}
```

tools are injected into the system prompt as JSON schema descriptions. the model learns when and how to call them from the schema. grammar-constrained decoding ensures tool calls are valid JSON.

### soma tool categories

| category | tools | tier |
|----------|-------|------|
| perception | look(bbg_key), listen(audio_stream), see(camera_id) | 0 |
| action | write(bbg_key, value), send(target, message), trade(order) | 1-2 |
| memory | remember(fact), recall(query), forget(key) | 0-1 |
| system | load_model(name), shed_model(name), set_param(key, value) | 0 |

the tool loop is the primary way [[soma]] interacts with the world — through [[bbg]] reads/writes mediated by model decisions.

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

## next-gen jets (emerging architectures)

ops that don't exist yet in the jet registry but are appearing in research and early production. each decomposes into existing atoms — will run slow through interpreter immediately, jets added when architectures mature.

### confirmed emerging (models shipping now)

| jet | atoms decomposition | what it enables | models |
|-----|---------------------|----------------|--------|
| `selective_scan` | read + mul + add + write (recurrent state) | Mamba / State Space Models — linear attention alternative, O(n) not O(n²). 3B Mamba matches 7B transformer | Mamba-2, Jamba, Zamba-2 |
| `linear_attention` | mul + reduce (no softmax) | sub-quadratic attention. MiniMax Lightning Attention, RWKV-6/7 | MiniMax-Text-01, RWKV-7 |
| `ring_attention` | sdpa + distributed reduce | infinite context via distributed attention across devices. each device holds a context chunk | Ring Attention, Striped Attention |
| `tree_attention` | sdpa + tree verify | speculative decoding verification — verify N draft tokens in one forward pass instead of N passes | Medusa, EAGLE-2 |

### research horizon (papers, no production models yet)

| jet | atoms decomposition | what it enables |
|-----|---------------------|----------------|
| `conditional_skip` | cmp + branch | Mixture-of-Depths — skip layers dynamically based on input difficulty. saves 30-50% compute |
| `ode_step` | mul + add + cmp (adaptive) | Neural ODE — continuous-depth networks with adaptive step size |
| `spike` | cmp + write (threshold + reset) | neuromorphic activation — binary spike instead of continuous. extreme efficiency on neuromorphic hardware |
| `hyper_attention` | read + mul + reduce (locality-sensitive hash) | approximate attention via LSH. O(n log n) for very long sequences |
| `tensor_product` | mul + reduce (higher-order) | Tensor Product Attention (TPA) — DeepSeek research, factorized KV heads. 5-10x KV cache compression |

### the atom guarantee

every jet above decomposes into the 8 atoms. when Mamba-3 or RWKV-8 drops tomorrow:

```
1. express as atom composition → runs immediately (slow)
2. profile hot path → write fused jet shader
3. register jet hash → 1000x speedup
4. STARK trace → provable
```

no architecture can surprise the runtime. only speed varies.

see [[soma]] for the model architecture this runtime serves.
