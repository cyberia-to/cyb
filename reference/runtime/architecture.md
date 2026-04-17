# Architecture

cyb-llm runtime executes models via three parallel paths. Each path
trades off speed, universality, and maintenance differently. Together
they cover the full spectrum from commodity chat to frontier research
without compromise on correctness.

## Three paths

```
                     import (.model file)
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
         ┌────────┐   ┌─────────┐   ┌───────┐
         │curated │   │  graph  │   │  nox  │
         │fast    │   │ correct │   │future │
         └───┬────┘   └────┬────┘   └───┬───┘
             └────────┬────┴────────────┘
                      ▼
              backend kernels
            (wgpu / metal / cpu)
```

### 1. Curated path — hand-optimized Rust per family

Hand-written forward() in Rust, dispatched by architecture family.
Reads config + weights from `.model`, calls backend kernels directly.

Speed: 1.5-2× llama.cpp on supported families.
Maintenance: ~10-15 family codepaths covering 95% of HF usage.

Families (v1):
- `LlamaStyle` — Llama 2/3, Mistral, Qwen2/3, Phi, Gemma 1/2, SmolLM, DeepSeek-dense, StarCoder, MiMo, NuExtract
- `MoEStyle` — Mixtral, DeepSeek-V2/V3, Qwen-MoE
- `BertStyle` — BERT, RoBERTa, DeBERTa, ModernBERT, Jina embed
- `T5Style` — T5, BART, M2M
- `WhisperStyle` — Whisper encoder-decoder with cross-attn
- `ViTStyle` — ViT, CLIP, SigLIP, PaliGemma
- `CNNStyle` — YOLO, ResNet, ESRGAN
- `UNetDiffusion` — SD 1.5/XL/3
- `DiTDiffusion` — Flux, Hunyuan, Mochi, Wan (video)
- `TTSStyle` — XTTS, Piper, VITS

### 2. Graph path — universal IR walker

Generic executor. Reads graph from `.model` (IR nodes), walks the DAG,
dispatches each op to a backend kernel. No architecture-specific code.

Speed: 1× llama.cpp (30-50% overhead from dispatch + shape inference).
Maintenance: single executor, grows by adding IR ops when a new dataflow
pattern requires it.

Coverage: anything expressible as a composition of IR ops
(`llm/src/ir/ops.rs` has 50+: Matmul, Conv1d/2d/3d, AdaLN, KvCompress,
FlowStep, PatchEmbed, ...). Research models, new architectures,
uncommon layer compositions all run here.

### 3. Nox path — compiled program section (future)

Each `.model` has a `program` section describing the entire pipeline
(tokenize → forward → sample → decode) in trident. The trident
compiler lowers this to nox VM assembly (18 instructions) specialized
for target hardware. Runtime executes nox.

Speed: approaches curated (compiler knows the architecture, can specialize).
Maintenance: zero — compiler generates codepaths automatically.
Timeline: nox/trident mature sometime after v1.

## Dispatch

Import reads `.model` and chooses path:

```
model_type in known_families?
  yes → curated (fastest)
  no  → check graph section present?
    yes → graph executor
    no  → error: "unrecognized model_type, no graph section; try reimport"
```

Explicit override via CLI: `--path=curated|graph|nox`.

Graph is the safety net. A curated path can fail or be missing;
graph always runs if the IR ops are supported.

## Correctness invariants

These must hold across all paths:

1. **Bit-for-bit determinism per path + backend + dtype.**
   Same input, same path, same backend → identical output.

2. **ε-equivalence across paths.**
   Curated and graph for the same model must produce outputs within
   a tolerance (F32: 1e-5, F16: 1e-3, Q4: 1e-2). Any larger divergence
   is a bug in curated (graph is the reference for correctness).

3. **CPU reference is ground truth.**
   Every IR op has a CPU implementation. That implementation matches
   llama.cpp / HF transformers within ε for standard ops. GPU kernels
   verified against CPU.

4. **No silent corruption.**
   If a path can't run a model, it errors with the specific cause
   ("Op::Scan not implemented in graph executor", "model_type `mamba`
   has no curated codepath").

## Layer stack

```
┌──────────────────────────────────────────────┐
│  CLI / API (cyb-llm run, serve)              │
├──────────────────────────────────────────────┤
│  Dispatcher (curated vs graph vs nox)        │
├──────────────────────────────────────────────┤
│  Curated family codepaths  │ Graph executor  │
│  (LlamaStyle, DiTDiffusion,│ (IR walker)     │
│   ...)                     │                 │
├──────────────────────────────────────────────┤
│  Ops layer (IR operations, CPU reference)    │
├──────────────────────────────────────────────┤
│  Backend kernels (wgpu / metal / cpu / ane)  │
├──────────────────────────────────────────────┤
│  Memory (FrameAllocator, mmap)               │
├──────────────────────────────────────────────┤
│  Storage (.model format, content-addressed)  │
└──────────────────────────────────────────────┘
```

## Backend contract

A backend declares which IR ops it implements. The graph executor
queries the backend at dispatch time:

```rust
trait Backend {
    fn supports(&self, op: &Op) -> bool;
    fn execute(&self, op: &Op, inputs: &[Tensor]) -> Result<Vec<Tensor>>;
}
```

Missing op falls back to CPU. CPU always implements every op. This
means **any graph runs on any backend** — at worst with CPU fallback
for unsupported ops.

Curated paths bypass this — they know which ops they need and call
them directly. A curated path that needs an op the backend doesn't
have fails at init time with a specific error.

## Evolution

The three paths are not eternal. They converge:

**Today (v1):**
- Curated handles top 10 families hand-written
- Graph handles everything else, slower
- Nox not yet usable

**Mid-term:**
- Curated stays for hot paths (big LLMs where 2× matters)
- Graph path improved via better op fusion, matches curated on
  medium-size models
- Nox compiles simple program sections, experimental

**Long-term:**
- Nox produces codepaths that match or exceed curated
- Curated shrinks to just the 2-3 flagships that benefit from manual
  tuning
- Graph stays as correctness reference and research fallback

The spec accommodates this evolution: curated and graph are
implementation strategies, not architectural boundaries. The
contract is stable; strategies change.

## What lives where

| Concern | Curated | Graph | Nox |
|---|---|---|---|
| Model architecture | Rust code | IR nodes | trident source |
| Tokenizer | In codepath | In codepath | In program |
| Chat template | In codepath | Config | In program |
| Sampling | In codepath | Config | In program |
| KV cache | Manual | Manual | Compiler |
| Specialization | Hand | Backend kernel | Compiler target |

## Decision log

- 2026-04-17: Hybrid architecture approved — curated for speed,
  graph for universality, nox for the future. Correctness is the
  contract; implementation strategy is flexible.
- 2026-04-17: Graph executor resurrected from "not used" state. It
  is the safety net for everything curated doesn't cover. Must
  work correctly even when slow.
- 2026-04-17: CPU reference is the correctness authority. Every
  op has a CPU implementation; GPU kernels verified against it.
- 2026-04-17: Backend contract: CPU fallback for unsupported ops.
  Any graph runs on any backend, at worst slowly.
