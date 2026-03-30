# cyb-llm reference specification

version: 0.1.0
status: implementation in progress
parent spec: `graph/llm.md`

## 1. scope

universal inference runtime for neural networks. loads any model format, runs on any hardware. replaces llama.cpp, ONNX Runtime, MLX, PyTorch inference.

target: one `cargo build` → binary that runs LLMs, ASR, diffusion, detection, TTS.

## 2. model formats

### 2.1 supported formats

| format | extension | parser | status |
|--------|-----------|--------|--------|
| ONNX | .onnx | protobuf (prost) | implemented |
| safetensors | .safetensors | JSON header + mmap | implemented |
| GGUF | .gguf | binary parser | implemented (K-quants via conversion) |
| fasttext/bin | .bin | binary header + f32 | implemented |
| numpy/MLX | .npz | ZIP + .npy parser | implemented |

### 2.2 format detection

by extension first, then magic bytes:
- `GGUF` (4 bytes) → GGUF
- `PK\x03\x04` → NPZ
- `\x93NUMPY` → NPY
- protobuf attempt → ONNX
- JSON header parse → safetensors

### 2.3 convergence

all formats produce the same `Graph` struct: `Vec<Node>` + `HashMap<String, WeightData>`. the runtime is format-agnostic after loading.

## 3. graph IR

### 3.1 types

```
Graph { nodes: Vec<Node>, tensors: HashMap<TensorId, TensorMeta>, weights: HashMap<String, WeightData> }
Node { id: usize, op: Op, inputs: Vec<TensorId>, outputs: Vec<TensorId>, attrs: HashMap<String, AttrValue>, backend_hint: Option<BackendHint> }
TensorMeta { shape: Shape, dtype: DType, residency: Residency }
WeightData { data: Vec<u8>, shape: Vec<usize>, dtype: DType }

Shape = Vec<Dim> where Dim = Fixed(usize) | Dynamic(String)
Residency = Resident | Cached | Streamed
BackendHint = Metal | Wgpu | Cuda | Cpu | Ane
DType = F32 | F16 | BF16 | I8 | U8 | Bool | Q8 | Q4 | Q4_1 | Ternary | Q2_K | Q3_K | Q4_K | Q5_K | Q6_K
```

### 3.2 operations (~48 ops)

#### core linear algebra
`Matmul`, `Add`, `Mul`, `Sub`, `Div`, `Transpose`, `Reshape`, `Permute`, `Concat`, `Split`, `Chunk`, `Clamp`, `NanToNum`

#### attention
`Sdpa { num_heads, kv_heads, head_dim, causal }`, `SdpaCross { num_heads, head_dim }`, `SdpaWindow { num_heads, head_dim, window_size }`, `KvCache`, `Rope { head_dim, base }`, `SinusoidalEmbed { dim }`, `RelativePosEmbedding { num_buckets }`

#### normalization
`RmsNorm { eps }`, `LayerNorm { eps }`, `BatchNorm { eps, momentum }`, `GroupNorm { num_groups, eps }`, `InstanceNorm { eps }`, `AdaLN`

#### activation
`Silu`, `Gelu { approximate }`, `GeGlu`, `SwiGlu`, `Glu`, `Relu`, `LeakyRelu { slope }`, `PRelu`, `Sigmoid`, `Tanh`, `Softmax { dim }`

#### convolution
`Conv1d`, `Conv2d`, `Conv3d`, `ConvTranspose2d`, `CausalConv1d`, `DepthwiseConv`, `Pool { mode, kernel }`

#### spatial
`Interpolate { mode, scale }`, `PixelShuffle`, `PixelUnshuffle`, `PatchEmbed`, `Unpatchify`

#### embedding
`TokenEmbed`, `PosEmbed`

#### special
`NoiseSchedule`, `FlowStep`, `Quantize`, `Dequantize`, `Sample { method }`

#### adapter
`LoraApply { rank, alpha }`, `Kron`, `MatrixInverse`

#### fused (recognized compositions)
`FusedNormMatmul`, `FusedSkipNorm`, `FusedSwiGlu`, `FlashAttention`

### 3.3 graph optimizations

applied in order:
1. `topological_sort` — ensure execution order respects data dependencies
2. `eliminate_dead_nodes` — remove nodes whose outputs are unused
3. `constant_fold` — execute constant-input nodes on CPU, replace with weights
4. `fuse_norm_matmul` — merge RmsNorm → Matmul when norm has single consumer
5. `fuse_skip_norm` — merge Add → RmsNorm into single kernel
6. `fuse_swiglu` — merge Sigmoid → Mul → Mul pattern
7. `memory_plan` — assign buffer lifetimes, compute peak memory

### 3.4 architecture templates

templates generate complete Graph IR from config. weight tensor names match loader conventions.

| template | function | config params |
|----------|----------|---------------|
| transformer_decoder | `transformer_decoder(config)` | hidden, heads, kv_heads, layers, ffn, vocab, eps, rope_theta |
| transformer_encoder | `transformer_encoder(config)` | same, no KV cache |
| encoder_decoder | `encoder_decoder(enc, dec)` | encoder + decoder configs |
| diffusion_dit | `diffusion_dit(config)` | depth, heads, hidden, patch_size, num_classes |
| cnn_detector | `cnn_detector(config)` | backbone_channels, num_classes, anchors |
| moe_decoder | `moe_decoder(config)` | base decoder + num_experts, top_k |

## 4. atom/jet system

### 4.1 eight atoms

| atom | type | what |
|------|------|------|
| `Mul` | arithmetic | multiply two values |
| `Add` | arithmetic | add two values |
| `Cmp(Max\|Min\|Lt\|Gt)` | logic | compare |
| `Exp` | transcendental | exponential |
| `Read` | memory | indexed lookup |
| `Write` | memory | indexed store |
| `Reduce(Sum\|Max\|Mean)` | aggregation | collapse dimension |
| `Slide(1D\|2D\|3D)` | pattern | windowed access |

every Op decomposes into atoms. exhaustive match — no fallback.

### 4.2 decomposition examples

```
matmul    = Slide(1D) + Mul + Reduce(Sum)
softmax   = Exp + Reduce(Sum) + Mul
relu      = Cmp(Max)
rmsnorm   = Mul + Reduce(Sum) + Exp + Mul
conv2d    = Slide(2D) + Mul + Reduce(Sum)
embedding = Read
kv_cache  = Write + Read
```

### 4.3 jet registry

formula hash: deterministic u64 from atom sequence. same atoms → same hash → same jet.

51 registered jets. lookup by hash or by Op variant. jet found → dispatch fused GPU kernel. not found → execute atoms on CPU (Level 0 guarantee).

### 4.4 three-level guarantee

```
level 0: atoms      — always correct, any backend, ~1000x slow (CPU interpreter)
level 1: jets       — fused GPU kernel, hardware-specific, ~1000x fast
level 2: STARK      — trace (op_id, input_hashes, output_hash, timing_ns), verifiable
```

## 5. execution

### 5.1 graph executor

`GraphExecutor::execute_decode(graph, tokens, weights, kv_cache, past_seq, cos, sin) → logits`

walks topologically sorted nodes. for each node:
1. resolve input buffers (from weights or previous outputs)
2. allocate output buffer (frame allocator)
3. look up jet for this Op
4. if jet exists: prepare bind group + dispatch command
5. if no jet: atom interpreter on CPU
6. store output buffer for downstream nodes

all dispatches collected into single compute pass.

### 5.2 hardcoded fast path

`NativeModel::forward(tokens) → logits`

hand-optimized for transformer decoders. 1000 lines of Rust. pre-computed params, pre-created bind groups where possible. single compute pass for all 28 layers.

44 tok/s Qwen3 0.6B, 70 tok/s SmolLM2 135M.

### 5.3 when to use which

| scenario | path | why |
|----------|------|-----|
| LLM decode (Llama/Qwen/Mistral) | hardcoded | maximum speed, pre-optimized |
| new architecture | graph executor | works immediately from template |
| research / custom model | graph executor | define graph in code, run |
| production after profiling | hardcoded | port hot path to hand-optimized |

## 6. backend

### 6.1 wgpu backend (implemented)

20 WGSL shader files, 64 compute kernels, 50 pipelines.

features:
- subgroup operations (naga patch) for SIMD reduction
- frame allocator for zero-allocation decode
- single compute pass for all ops
- per-tensor quantization dispatch (F32/F16/Q4/Q8/Ternary)

### 6.2 Metal backend (in progress)

native Metal shaders for hot path: matmul, attention, rope, conv2d.
`simdgroup_matrix_multiply_accumulate` for tensor core access.
expected 2-5x speedup over wgpu on Apple Silicon.

### 6.3 other backends (planned)

| backend | crate | status |
|---------|-------|--------|
| CUDA | cudarc | planned |
| CPU SIMD | std::arch | planned |
| ANE | rane | planned |
| WebGPU | wgpu → WASM | untested |

## 7. quantization

### 7.1 per-tensor dtype

each weight tensor has its own DType. one model can mix Q4 attention + F16 embedding + Q8 output.

### 7.2 matmul dispatch

```
weight.quant_format → shader selection:
  F32     → f32_matmul.wgsl
  F16     → f16_matmul.wgsl (f16→f32 in shader)
  Q4      → q4_matmul.wgsl (dequant + matmul, NR=4, subgroups)
  Q8      → q8_matmul.wgsl (signed int8)
  Ternary → ternary_matmul.wgsl (add/sub only, no multiply)
```

### 7.3 GGUF K-quant support

Q2_K, Q3_K, Q4_K, Q5_K, Q6_K converted to Q4 at load time via dequant→requant. quality loss from round-trip, but models load and run.

### 7.4 runtime conversion

GPU: `quantize_f32_to_q4()`, `quantize_f32_to_q8()` via compute shader.
CPU: `dequantize_q4_to_f32()`, `dequantize_q8_to_f32()` for testing.

## 8. generation

### 8.1 autoregressive loop

```
encode prompt → prefill (all tokens) → decode loop (one token at a time)
  each step: forward() → logits → sample → append token → repeat
```

### 8.2 sampling

applied in order: repetition_penalty → temperature → top_k → top_p → sample.

greedy mode: GPU-side argmax (reads 4 bytes instead of vocab_size × 4 bytes).

### 8.3 KV cache

simple append (not paged). per-layer key/value buffers grow with sequence length.

## 9. CLI

```
cyb-llm run <model> -p "prompt" -n 100 -t 0.7    — generate text
cyb-llm run /path/to/model.safetensors -p "hi"    — local file
cyb-llm list                                        — cached models
cyb-llm info /path/to/model.gguf                   — show model info
cyb-llm download <hf_id>                            — download from HF
```

## 10. testing

22 unit tests covering:
- atom decomposition (all ops decompose, layout ops are empty)
- atom interpreter correctness (mul, add, exp, relu, reduce)
- jet registry (deterministic hash, collision avoidance, lookup)
- architecture templates (weight names, attrs, topo sort)

## 11. metrics

| metric | value |
|--------|-------|
| Rust LOC | ~11,000 |
| WGSL LOC | ~3,300 |
| shader files | 20 |
| compute kernels | 64 |
| registered jets | 51 |
| supported ops | 48 |
| model formats | 5 |
| quant types | 5 (+ 5 K-quant via conversion) |
| architecture templates | 6 |
| unit tests | 22 |
