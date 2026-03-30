# cyb-llm architecture

pure Rust inference runtime. loads any model format, runs on any GPU via WGSL compute shaders.

## pipeline

```
model file (.onnx / .safetensors / .gguf / .bin / .npz)
      ↓
   loader (format-specific parser)
      ↓
   Graph IR (typed DAG: nodes + edges + weights)
      ↓
   optimizer (fusion, dead elimination, constant folding, memory planning)
      ↓
   executor (walks DAG, dispatches each node)
      ↓
   jet registry (formula hash → fused GPU kernel)
      ↓
   backend (wgpu / Metal / CUDA / CPU)
      ↓
   GPU compute (WGSL shaders / .metal / CUDA kernels)
```

## key design decisions

### atom/jet system

every neural network operation decomposes into 8 atoms: `mul`, `add`, `cmp`, `exp`, `read`, `write`, `reduce`, `slide`. this is the reference interpreter — always correct, any backend, ~1000x slow.

jets are fused GPU kernels that replace recognized atom compositions. formula hashing: same atom sequence → same hash → same jet. 51 jets registered covering matmul, attention, normalization, activation, convolution, spatial ops, adapters.

the guarantee: unknown op → decompose to atoms → runs (slow). write a jet → 1000x speedup. no architecture can break the runtime.

### graph executor vs hardcoded forward pass

two execution paths coexist:

1. **hardcoded path** (model.rs) — hand-optimized forward pass for transformer decoders. single compute pass for all 28 layers. 44 tok/s Qwen3, 70 tok/s SmolLM2. used for LLM decode.

2. **graph executor** (executor.rs) — universal DAG walker. takes any Graph IR, dispatches ops via jet registry. supports any architecture template. slightly slower due to indirection but works for any model.

both paths use the same shaders, same pipelines, same frame allocator.

### single compute pass

the critical optimization: all GPU dispatches for one decode step go into ONE `wgpu::ComputePass`. this reduces pass overhead from ~500 separate passes to 1.

achieved by pre-creating all bind groups before opening the pass, then dispatching 600+ commands within it. different pipelines switch inside the same pass.

### frame allocator

GPU buffer allocation is expensive (~0.05ms per `create_buffer`). the frame allocator pools buffers by size. after warmup, zero new GPU allocations — all buffers reused from pool.

### naga subgroups patch

WGSL subgroup operations (`subgroupAdd`, `subgroupMax`) give instant SIMD reduction on Metal (maps to `simd_sum`). naga doesn't support `enable subgroups;` — we vendor naga and patch one file to enable it. all reduction shaders use subgroups: matmul, norm, attention, argmax.

## file layout

```
llm/
├── src/
│   ├── lib.rs                      — public API
│   ├── main.rs                     — CLI (run, list, info, download)
│   ├── ir/
│   │   ├── graph.rs                — Graph IR: Node, TensorMeta, Shape, Residency
│   │   ├── ops.rs                  — Op enum (~48 operations)
│   │   ├── dtype.rs                — DType: F32/F16/BF16/Q4/Q8/Ternary/...
│   │   ├── atoms.rs                — 8 atoms + decompose() + reference interpreter
│   │   ├── jets.rs                 — JetRegistry: formula hash → jet name
│   │   ├── executor.rs             — GraphExecutor: DAG → GPU dispatch
│   │   ├── fusion.rs               — graph optimization passes
│   │   └── templates.rs            — architecture templates (6 architectures)
│   ├── loader/
│   │   ├── onnx.rs                 — ONNX protobuf parser
│   │   ├── safetensors.rs          — safetensors JSON+mmap parser
│   │   ├── gguf.rs                 — GGUF binary parser (Q4_0/Q8_0 + K-quants)
│   │   ├── bin.rs                  — fasttext/raw binary
│   │   └── mlx.rs                  — numpy NPZ parser
│   ├── backend/
│   │   ├── mod.rs                  — Backend trait
│   │   └── wgpu_backend/
│   │       ├── mod.rs              — WgpuBackend: device init, subgroups
│   │       ├── pipelines.rs        — 50 compute pipelines
│   │       ├── dispatch.rs         — 57 dispatch functions
│   │       ├── alloc.rs            — frame allocator
│   │       ├── model.rs            — NativeModel (hardcoded fast path)
│   │       └── shaders/            — 20 WGSL files, 64 compute kernels
│   ├── generate/
│   │   ├── mod.rs                  — TextGenerator + autoregressive loop
│   │   └── sampler.rs              — top-p, top-k, repetition penalty
│   └── hub/
│       └── mod.rs                  — HuggingFace model download
├── proto/
│   └── onnx.proto3                 — ONNX protobuf definition
├── docs/
│   └── architecture.md             — this file
└── specs/
    └── reference.md                — reference specification
```

## shader inventory

| file | kernels | purpose |
|------|---------|---------|
| q4_matmul.wgsl | 1 | Q4 dequant+matmul, NR=4, subgroup reduction |
| q8_matmul.wgsl | 1 | Q8 signed int8 matmul |
| f32_matmul.wgsl | 1 | full precision matmul |
| f16_matmul.wgsl | 1 | f16-packed weight matmul |
| ternary_matmul.wgsl | 1 | BitNet {-1,0,+1} add/sub only |
| attention.wgsl | 1 | fused QK^T + softmax + AV, one workgroup per head |
| rms_norm.wgsl | 1 | RMSNorm with subgroup reduction |
| layernorm.wgsl | 1 | LayerNorm (mean + var + scale + bias) |
| norm.wgsl | 4 | BatchNorm, GroupNorm, InstanceNorm, AdaLN |
| rope.wgsl | 1 | rotary position embeddings |
| elementwise.wgsl | 16 | add/sub/mul/div/relu/sigmoid/tanh/clamp/... |
| activations.wgsl | 6 | GeGLU, SwiGLU, GLU, PReLU, GELU variants |
| kv_cache.wgsl | 2 | KV append + GQA head expansion |
| conv.wgsl | 7 | Conv1d/2d/3d, transpose, causal, depthwise, pool |
| spatial.wgsl | 7 | interpolate(3), pixel_shuffle/unshuffle, patch_embed, unpatchify |
| special.wgsl | 7 | sinusoidal, noise_schedule, flow_step, pos_embed, quantize, cross/flash attention |
| adapter.wgsl | 2 | LoRA apply, Kronecker product |
| argmax.wgsl | 1 | GPU-side argmax with subgroup reduction |
| fused_norm_q4.wgsl | 1 | fused RMSNorm + Q4 matmul |
| fused_skip_norm.wgsl | 1 | fused skip connection + RMSNorm |

## performance

| model | params | format | tok/s | backend |
|-------|--------|--------|-------|---------|
| SmolLM2 | 135M | safetensors BF16 | 70 | wgpu |
| Qwen3 | 0.6B | ONNX Q4 | 44 | wgpu |
| Llama 3.2 | 1.0B | ONNX Q4 | 42 | wgpu |

comparison with ollama (llama.cpp + native Metal):
- SmolLM2: 70 vs 72 tok/s = **parity**
- Qwen3 0.6B: 44 vs 198 tok/s = 4.5x gap (wgpu overhead)
- Llama 3.2 1B: 42 vs 153 tok/s = 3.6x gap

the gap is wgpu→Metal translation overhead. native Metal backend (Phase 5) closes it.

## supported architectures

| template | models | status |
|----------|--------|--------|
| transformer_decoder | Llama, Qwen, Mistral, SmolLM, Phi, DeepSeek | working (44-70 tok/s) |
| transformer_encoder | BERT, DeBERTa, RoBERTa | shaders ready, no forward pass |
| encoder_decoder | Whisper | template + shaders, no forward pass |
| diffusion_dit | Flux, SD3, Wan2.2 | template + shaders, no forward pass |
| cnn_detector | YOLO | template + shaders, no forward pass |
| moe_decoder | Mixtral, DeepSeek-MoE | template + shaders, no forward pass |

## quantization

per-tensor quantization — different layers can use different types:

| format | bits | shader | notes |
|--------|------|--------|-------|
| F32 | 32 | f32_matmul | full precision |
| F16 | 16 | f16_matmul | half precision, f16→f32 in shader |
| Q8 | 8 | q8_matmul | signed int8, per-block scale |
| Q4 | 4 | q4_matmul | 4-bit nibbles, per-block scale |
| Ternary | 1.6 | ternary_matmul | BitNet {-1,0,+1}, add/sub only |

GGUF K-quants (Q2_K through Q6_K) converted to Q4 at load time.
