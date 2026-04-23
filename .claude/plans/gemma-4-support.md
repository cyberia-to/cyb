# Gemma-4-31b runtime support

Manifest model #4. Currently fails at load (layer 5, full_attention).
This plan closes the gap so all four manifest models pass `mr status`.

## Reference (per `reference/runtime/`)

Existing spec in `arch.md` §LlamaStyle+ already names four Gemma 3/4 extensions:
sliding window, GELU, final logit softcapping, K=V shared projection.
That's the spec for **Gemma 3** — it does not yet describe Gemma-4's per-layer
dimension switching.

## Reference gaps

Discovered from `~/llm/gemma-4-31b-import/config.json`:

| Gap | Field | Spec impact |
|---|---|---|
| Per-layer attention dim | `global_head_dim: 512` (vs `head_dim: 256`) | `arch.md` §LlamaStyle+ must add: full_attention layers use a different head_dim than sliding_attention layers |
| Per-layer kv_heads | `num_global_key_value_heads: 4` (vs `num_key_value_heads: 16`) | same spec section |
| Layer type list | `layer_types: [sliding, sliding, sliding, sliding, sliding, full, ...]` | `format.md` config.toml schema must enumerate this field |
| Activation flag | `hidden_activation: "gelu_pytorch_tanh"` | `arch.md` activation table needs this canonical name |
| Softcapping value | `final_logit_softcapping: 30.0` | `arch.md` already mentions the formula; spec the field name |
| K=V detection | `attention_k_eq_v: true` flag, no separate kv_proj tensor name in our import | `import.md` must say: when this flag is set, mi splits the fused kv weight into two identical tensors named k_proj/v_proj at pack time, OR keeps fused as kv_proj.weight (decision needed) |

Each gap = one paragraph in the relevant reference file. Spec edit is one
commit per file, separate from the implementation.

## Decision: extend `llama_style` vs new `gemma4_style`

Spec (`scope.md`) says "These add variant flags to LlamaStyle, not a new
family." Keeping that contract.

Cost: `LlamaConfig` grows ~6 fields, `LayerWeights` gains an optional
`kv_proj`, `forward_layer` reads per-layer dim from a `Vec<LayerKind>`.
Total ~80 lines added to llama_style; no new files.

Rejected: a new `gemma4_style/` family. Would duplicate ~400 lines of
weight loading and forward orchestration. Per "kill the zoo": extend.

## Implementation phases

### 1. Spec edits (one session, one file at a time)

- [ ] `arch.md` §LlamaStyle+ — add per-layer dim switching (`global_head_dim`, `num_global_key_value_heads`, `layer_types`)
- [ ] `arch.md` activation table — note `gelu_pytorch_tanh`
- [ ] `format.md` config.toml schema — add `layer_types`, `global_head_dim`, `num_global_key_value_heads`, `final_logit_softcapping`, `attention_k_eq_v`, `hidden_activation`, `sliding_window`
- [ ] `import.md` — specify K=V tensor naming convention (decision: mi splits fused kv into k_proj+v_proj that share underlying bytes; runtime sees two tensors, no special path)

### 2. Import (mi) — decode Gemma-4 GGUF (one session)

- [ ] Detect `gemma4` model_type from config.json
- [ ] Pack `layer_types`, `global_head_dim`, `num_global_key_value_heads`, `final_logit_softcapping`, `sliding_window`, `hidden_activation`, `attention_k_eq_v` into config.toml
- [ ] If `attention_k_eq_v`, ensure k_proj and v_proj tensors are both present in the .model (split fused kv if GGUF stored it as one)
- [ ] Re-import gemma-4-31b → produce gemma-4-31b.model that passes our load

### 3. Runtime (mr/llama_style) — Gemma-4 forward (one session)

`config.rs`:
- [ ] Parse `layer_types: Vec<LayerKind>` (Sliding | Full)
- [ ] Parse `global_head_dim`, `num_global_key_value_heads`, `sliding_window`, `final_logit_softcapping`, `hidden_activation`
- [ ] Add helper `head_dim_for(layer_idx)` and `kv_heads_for(layer_idx)`

`weights.rs`:
- [ ] Per-layer head_dim/kv_heads → per-layer q_proj/k_proj/v_proj shapes
- [ ] Validation against the per-layer expected shape

`forward.rs`:
- [ ] Read per-layer head_dim and kv_heads inside `forward_layer`
- [ ] Sliding-window mask: when `LayerKind::Sliding`, scores positions outside window get -inf
- [ ] Activation dispatch: `Op::Gelu { approximate: true }` for Gemma, `Op::Silu` for default
- [ ] After lm_head: if `final_logit_softcapping` set, `logits = tanh(logits / cap) * cap`

### 4. Verification

- [ ] `mr status` shows gemma-4-31b row with non-`✗` cells
- [ ] `mr run gemma-4-31b --prompt "Hello" --backend cpu --max-tokens 16` produces coherent text
- [ ] All existing tier-3 tests (qwen3, qwen2.5-coder-1.5b) still pass
- [ ] qwen3 HF golden still matches (Gemma changes must not regress LlamaStyle)
- [ ] `cpu/` portability audit clean (no new arch-specific code)

## Out of scope

- Gemma-4 vision tower (text-only path first; multimodal is a separate plan)
- Speed optimisation — gemma-4-31b is 31B params; honest tok/s on cpu will be ~1
- Gemma-3 specifically — same family, but no model in our manifest

## Acceptance

`mr status` row for gemma-4-31b shows ✓ on at least cpu backend, with the
existing three models (qwen3, coder-1.5b, coder-14b) still ✓. That's the
4-of-4 manifest goal stated in `cyb-mvp.md` Phase 0.
