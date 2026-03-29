# LLM Runtime — Full Implementation Plan

Based on `graph/llm.md` spec, current cyb/inference codebase, trident GPU infrastructure, and rane (ANE).

## Current State

**What exists:**
- 9 WGSL shaders (q4_matmul, f32_matmul, attention, rms_norm, rope, elementwise, kv_cache, argmax, embed)
- wgpu pipeline manager with single-pass dispatch, frame allocator, subgroup ops (patched naga)
- ONNX model loader with Q4 packed weights, external data via mmap
- Working forward pass: Qwen3 0.6B (44 tok/s), Llama 3.2 1B (42 tok/s)
- ~2500 LOC total (Rust + WGSL)

**What doesn't exist:**
- Graph IR (ops are hardcoded in model.rs forward pass)
- Multi-format loading (only ONNX)
- Multi-backend dispatch (only wgpu)
- Scheduler / orchestrator
- Provability traces
- Memory tiers
- Any model beyond decoder-only transformers

## Architecture Target

```
.onnx / .safetensors / .gguf / .bin
              ↓
        Model Loader              ← Phase 1
              ↓
        Graph IR (typed DAG)      ← Phase 2
              ↓
        Op Fusion Pass            ← Phase 3
              ↓
    ┌─────────┴─────────┐
    │    Op Registry     │        ← Phase 4
    │  op × dtype × hw   │
    │    → kernel        │
    └─────────┬─────────┘
              ↓
    ┌─────────┴─────────────────────────────┐
    │           Backend Layer                │
    │  Metal │ wgpu │ CUDA │ CPU │ ANE      │  ← Phase 5-9
    └───────────────────────────────────────┘
```

## Phase 0: Restructure (1 week)

Current `inference/` is a prototype. Restructure into the target architecture.

### Crate structure
```
cyb/llm/                          ← new crate (replaces inference/)
├── Cargo.toml
├── src/
│   ├── lib.rs                    ← public API: load_model(), Model::generate()
│   ├── loader/
│   │   ├── mod.rs                ← format detection + dispatch
│   │   ├── onnx.rs              ← ONNX protobuf → Graph IR (from current graph/mod.rs)
│   │   ├── safetensors.rs       ← safetensors → Graph IR
│   │   └── gguf.rs              ← GGUF → Graph IR
│   ├── ir/
│   │   ├── mod.rs               ← Graph IR types
│   │   ├── graph.rs             ← DAG: nodes, edges, typed tensors
│   │   ├── ops.rs               ← ~30 op enum (Matmul, RmsNorm, Rope, Attention, ...)
│   │   ├── dtype.rs             ← F32, F16, Q8, Q4, Ternary, per-tensor
│   │   └── fusion.rs            ← fusion passes (norm+matmul, skip+norm, swiglu)
│   ├── backend/
│   │   ├── mod.rs               ← Backend trait + detection
│   │   ├── wgpu/
│   │   │   ├── mod.rs           ← WgpuBackend: pipeline cache, frame alloc
│   │   │   ├── shaders/         ← current 9 WGSL shaders
│   │   │   └── dispatch.rs      ← single-pass dispatch logic
│   │   ├── metal/
│   │   │   ├── mod.rs           ← MetalBackend via objc2-metal / rmetal
│   │   │   └── shaders/         ← .metal shaders (ported from llama.cpp)
│   │   ├── cpu/
│   │   │   └── mod.rs           ← SIMD fallback (NEON/AVX2)
│   │   └── ane/
│   │       └── mod.rs           ← Apple Neural Engine via rane
│   ├── schedule/
│   │   ├── mod.rs               ← Scheduler: op → backend assignment
│   │   ├── memory.rs            ← Memory tiers, KV cache management
│   │   └── autotune.rs          ← Runtime profiling + kernel selection
│   ├── generate/
│   │   ├── mod.rs               ← Autoregressive loop
│   │   ├── sampler.rs           ← top-p, top-k, temperature, repetition penalty
│   │   ├── kv_cache.rs          ← Paged KV cache, quantization, prefix caching
│   │   └── speculative.rs       ← Speculative decoding (draft + verify)
│   ├── hub/
│   │   └── mod.rs               ← HuggingFace download (current code)
│   └── trace/
│       └── mod.rs               ← Provability: (op_id, input_hashes, output_hash, timing_ns)
└── vendor/
    └── naga/                     ← patched naga with subgroups (current)
```

### Migration from inference/ to llm/
1. Move runtime/shaders/ → llm/src/backend/wgpu/shaders/
2. Move runtime/model.rs → split into llm/src/ir/ + llm/src/backend/wgpu/dispatch.rs
3. Move runtime/ops.rs → llm/src/backend/wgpu/dispatch.rs
4. Move runtime/pipelines.rs → llm/src/backend/wgpu/mod.rs
5. Move runtime/alloc.rs → llm/src/backend/wgpu/mod.rs
6. Move graph/mod.rs → llm/src/loader/onnx.rs
7. Move hub/ → llm/src/hub/
8. Move generate/ → llm/src/generate/
9. Keep inference/ as compatibility wrapper that uses llm/

## Phase 1: Model Loaders (2 weeks)

### 1.1 ONNX loader (done)
- [x] Protobuf parsing
- [x] Q4 packed weight loading
- [x] External data via mmap
- [ ] Full op coverage (currently ~20 ops, need ~30)
- [ ] Validation: shape inference, dtype checking

### 1.2 Safetensors loader
- Parse safetensors header (JSON metadata + byte offsets)
- mmap tensor data (zero-copy)
- Detect model architecture from config.json:
  - `architectures: ["Qwen2ForCausalLM"]` → Qwen
  - `architectures: ["LlamaForCausalLM"]` → Llama
  - `architectures: ["MistralForCausalLM"]` → Mistral
- Map weight names → Graph IR nodes
- Support: fp16, bf16, f32 dtypes
- **This unlocks ALL HuggingFace models** (most are safetensors now)

### 1.3 GGUF loader
- Parse GGUF header (metadata key-value pairs)
- Read quantized tensor blocks (Q4_0, Q4_1, Q5_0, Q5_1, Q8_0, F16, F32)
- Map GGUF tensor names → Graph IR
- **This unlocks llama.cpp ecosystem** (Ollama, LM Studio models)

### 1.4 Architecture templates
Pre-built Graph IR templates for common architectures:
- `LlamaDecoder` — Llama 2/3, CodeLlama, Mistral, Qwen
- `GPTDecoder` — GPT-2, GPT-J, GPT-NeoX
- `BertEncoder` — BERT, DeBERTa, RoBERTa (Phase 4)
- `WhisperEncoderDecoder` — Whisper (Phase 5)
- `UNetDiffusion` — Stable Diffusion (Phase 7)

## Phase 2: Graph IR (2 weeks)

### 2.1 Core types
```rust
struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,    // tensor flow between nodes
    weights: WeightStore, // typed tensor storage
}

struct Node {
    id: NodeId,
    op: Op,
    inputs: Vec<TensorId>,
    outputs: Vec<TensorId>,
    attrs: HashMap<String, Attr>,
}

enum Op {
    Matmul { dtype_a: DType, dtype_b: DType },
    RmsNorm { eps: f32 },
    LayerNorm { eps: f32 },
    Rope { head_dim: u32, max_seq: u32 },
    Attention { num_heads: u32, kv_heads: u32, head_dim: u32 },
    Add, Mul, Silu, Gelu, Sigmoid, Softmax,
    Embed { vocab: u32, dim: u32 },
    Conv2d { ... },
    // ... ~30 total
}

enum DType { F32, F16, BF16, Q8, Q4, Q4_1, Ternary }

struct TensorMeta {
    shape: Vec<usize>,
    dtype: DType,
    layout: Layout, // row-major, col-major, block-quantized
}
```

### 2.2 Fusion passes
Pattern-match on Graph IR and merge nodes:
- `RmsNorm → Matmul` → `FusedNormMatmul` (when norm feeds exactly 1 matmul)
- `Add → RmsNorm` → `FusedSkipNorm` (residual + normalize)
- `Matmul → Sigmoid → Mul → Matmul → Mul` → `SwiGLU` (gate+up+silu+mul)
- `Matmul → Scale → Mask → Softmax → Matmul` → `FlashAttention`
- `Matmul + Matmul + Matmul` (same input) → `FusedQKV` (concatenated weights)

### 2.3 Memory planning
- Static analysis: compute peak memory for each execution order
- Buffer reuse: identify non-overlapping lifetimes, share allocations
- KV cache budget: reserve memory for target context length

## Phase 3: Op Registry + Backend Trait (1 week)

```rust
trait Backend {
    fn name(&self) -> &str;
    fn supports(&self, op: &Op, dtypes: &[DType]) -> bool;
    fn dispatch(&self, op: &Op, inputs: &[&Buffer], output: &mut Buffer);
    fn sync(&self); // wait for all pending ops
}

struct OpRegistry {
    // (op_pattern, dtype_combination) → Vec<(Backend, priority)>
    dispatch_table: HashMap<OpKey, Vec<(BackendId, Priority)>>,
}
```

## Phase 4: Backend — wgpu (done, polish)

Current state: 9 shaders, 44 tok/s. Polish:
- [ ] Add missing ops: LayerNorm, BatchNorm, GroupNorm, Conv2d, Conv1d, Gelu, Sigmoid
- [ ] Q8 matmul shader (higher quality than Q4)
- [ ] Ternary matmul shader (BitNet — add/subtract only, no multiply)
- [ ] Flash attention variant (tiled, for prefill with long sequences)
- [ ] Prefill optimization: batch matmul instead of per-position loop

## Phase 5: Backend — Metal Native (3 weeks)

The 4x gap closer. Native Metal shaders via `objc2-metal` or `rmetal`.

### 5.1 Metal device setup
```rust
use objc2_metal::{MTLDevice, MTLCommandQueue, MTLComputePipelineState};
// OR
use rmetal::{Device, CommandQueue, ComputePipeline};
```

### 5.2 Port hot path shaders from llama.cpp
- `kernel_mul_mv_q4_0_f32` → Metal native Q4 matmul with simdgroup_matrix_multiply_accumulate
- `kernel_rms_norm_fuse_impl` → Metal RMSNorm with simd_sum
- `kernel_rope_neox` → Metal RoPE
- `kernel_flash_attn_ext` → Metal Flash Attention with threadgroup memory
- `kernel_soft_max_4` → Metal Softmax with SIMD

### 5.3 Integration
- Auto-detect: Metal available → use Metal for hot path (matmul, attention, rope, norm)
- wgpu for everything else (elementwise, embed, argmax — not worth native)
- Zero-copy buffer sharing between Metal and wgpu (shared GPU memory on Apple Silicon)

### 5.4 Expected performance
- Q4 matmul: 4-5x faster (simdgroup_matrix + SIMD reduction)
- Flash attention: 2-3x faster (threadgroup memory tiling)
- Total: 44 tok/s → 150-180 tok/s (parity with ollama)

## Phase 6: Backend — CPU SIMD (1 week)

Fallback for systems without GPU, and for small ops.

- NEON (ARM): `std::arch::aarch64::*` — Q4 dot product, RMSNorm, RoPE
- AVX2 (x86): `std::arch::x86_64::*` — same ops
- Rayon for thread parallelism across output rows
- Used for: prefill (large batch matmul), or when GPU not available

## Phase 7: Backend — ANE (2 weeks)

Apple Neural Engine — 3-5W power, leaves GPU free.

### 7.1 rane integration
- `rane` crate provides pure Rust ANE access (no CoreML, no objc)
- Compile model subgraph to ANE program
- Constraints: dimensions must be multiples of 128, limited op set

### 7.2 Strategy
- ANE for tier 0 models (always-on, low power)
- GPU for tier 1-2 models (generative, throughput)
- Hybrid: attention on GPU, FFN on ANE

## Phase 8: Backend — CUDA (2 weeks)

For NVIDIA server deployment.

- `cudarc` crate (production, 3.1M downloads)
- cuBLAS for matmul, custom CUDA kernels for attention/norm
- Tensor Core support (FP16/INT8 matmul)

## Phase 9: Scheduler (2 weeks)

### 9.1 Backend selection
```rust
fn select_backend(op: &Op, shape: &[usize], available: &[Backend]) -> BackendId {
    // 1. Filter by support
    // 2. Estimate cost per backend (lookup table from autotune)
    // 3. Check memory availability
    // 4. Return lowest-cost option
}
```

### 9.2 Autotune
- On first run: benchmark each kernel variant on actual hardware
- Cache results: `~/.cache/cyb-llm/autotune-{device_id}.json`
- Re-tune on device change or model change

### 9.3 Memory management
```rust
enum MemoryTier {
    Resident,  // always in GPU memory (tier 0 models)
    Cached,    // loaded on first use, LRU eviction
    Streamed,  // loaded per-inference, freed after
}
```

- Graceful degradation: OOM → drop KV cache precision → shed lowest model → offload to CPU
- Adaptive precision: plenty of RAM → f16; pressure → q8; heavy → q4

## Phase 10: Generation Engine (2 weeks)

### 10.1 KV Cache
- Paged allocation (fixed-size blocks, no fragmentation)
- KV cache quantization (f16 → q8 under memory pressure)
- Prefix caching (reuse KV for shared prompt prefixes)

### 10.2 Sampling
- top-p, top-k, temperature
- Repetition penalty, frequency penalty
- Min-p sampling
- Structured output (JSON mode, grammar-constrained)
- GPU-side top-k (read only K values instead of vocab_size)

### 10.3 Speculative decoding
- Small model (draft) generates N candidate tokens
- Large model (verify) validates in one forward pass
- 2-3x speedup for autoregressive generation

### 10.4 Multi-token prediction
- MiMo / Step 3.5 Flash: 2-3 tokens per forward pass
- Variable output length per step

## Phase 11: Orchestration (2 weeks)

### 11.1 Multi-model
- Priority queue: tier 0 preempts tier 1-2
- Memory pressure → shed lowest priority
- Hot-swap: load new weights while old model serves

### 11.2 Model composition
- Pipeline: whisper → LLM → TTS
- Zero-copy tensor passing between models (shared GPU buffers)

### 11.3 Continuous batching
- New requests join mid-batch
- Iteration-level scheduling (vLLM-style)

## Phase 12: Provability (1 week)

Every op execution → trace entry:
```rust
struct TraceEntry {
    op_id: u32,
    input_hashes: Vec<[u8; 32]>, // blake3
    output_hash: [u8; 32],
    timing_ns: u64,
}
```
STARK-compatible execution record. Verifier replays and confirms.

## Timeline

| Phase | What | Duration | Dependency |
|-------|------|----------|------------|
| 0 | Restructure crate | 1 week | — |
| 1 | Safetensors + GGUF loaders | 2 weeks | Phase 0 |
| 2 | Graph IR + fusion | 2 weeks | Phase 0 |
| 3 | Op registry + backend trait | 1 week | Phase 2 |
| 4 | wgpu polish (missing ops) | 1 week | Phase 3 |
| 5 | **Metal native** | 3 weeks | Phase 3 |
| 6 | CPU SIMD | 1 week | Phase 3 |
| 7 | ANE via rane | 2 weeks | Phase 3 + rane |
| 8 | CUDA via cudarc | 2 weeks | Phase 3 |
| 9 | Scheduler + autotune | 2 weeks | Phase 3-8 |
| 10 | Generation engine | 2 weeks | Phase 9 |
| 11 | Orchestration | 2 weeks | Phase 10 |
| 12 | Provability traces | 1 week | Phase 10 |

**Critical path: Phases 0 → 2 → 3 → 5 (Metal)**

After Phase 5: one binary, 150+ tok/s on Apple Silicon, portable to all platforms.
After Phase 8: all major GPU vendors covered.
After Phase 11: production model scheduler for soma.

## Консерны

1. **rmetal maturity** — haven't seen the crate. If it's early, may need objc2-metal directly. Fallback: raw `objc2::msg_send!` calls to Metal C API.

2. **rane maturity** — ANE access is undocumented Apple private API. rane may break between macOS versions. Mitigation: ANE is optimization, not requirement. wgpu+Metal covers all cases.

3. **GGUF format complexity** — many quantization types (Q4_0, Q4_1, Q5_0, Q5_1, Q8_0, IQ variants). Start with Q4_0 + Q8_0, add others incrementally.

4. **Graph IR vs hardcoded forward pass** — current model.rs has the forward pass hardcoded for Qwen/Llama. Graph IR adds flexibility but ~10% overhead from indirection. Mitigation: keep hardcoded "fast paths" for common architectures, Graph IR for everything else.

5. **Memory management complexity** — paged KV cache + multi-model + graceful degradation = complex state machine. Start simple (current approach), add tiers incrementally.

6. **STARK trace overhead** — hashing every tensor adds latency. Make it opt-in, not default. Zero overhead when disabled.
