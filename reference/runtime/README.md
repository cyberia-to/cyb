# cyb-llm runtime specification

Canonical spec for the cyb-llm runtime. Defines what models we run,
how we represent them, how we execute them.

Every backend (wgpu, metal, cpu, ane) is verified against this spec.
Code disagreeing with spec is a bug. Spec disagreeing with reality
is a spec bug — fix spec first, then code.

## Files

- [scope.md](scope.md) — model types in/out of scope, with rationale
- [tensor.md](tensor.md) — shape conventions, memory layout, dtype
- [quant.md](quant.md) — exact bit layouts for Q4_0/Q4_K/Q5_K/Q6_K/Q8/ternary
- [ops.md](ops.md) — math for RMSNorm, RoPE, attention, SwiGLU, softmax
- [arch.md](arch.md) — architecture definitions: Qwen2, Qwen3, Gemma4, etc.
- [format.md](format.md) — .model file layout, tensor index, sections
- [import.md](import.md) — GGUF → .model, invariants preserved
- [execution.md](execution.md) — forward pass contract, what backends must produce
- [test.md](test.md) — three-tier test strategy: op, layer, e2e

## Source of truth

When code, docs, and spec disagree: spec is authoritative. If spec
is wrong, update spec first (one commit), then propagate to code
(separate commit).

Reference implementations (llama.cpp, HuggingFace transformers) are
not authoritative — they have bugs too. But they are practical
ground truth for golden tests: if llama.cpp and HF both produce Y
from input X, and our runtime produces Z, we have a bug.
