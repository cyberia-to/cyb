# import graph IR

How the runtime gets a Graph IR for execution under
[`cyb/cyb-model`](https://cyber.page/cyb/cyb-model).

## Where the graph comes from

The canonical `.model` does **not** embed a graph section. The graph
is rebuilt at runtime load time from the config + arch template:

```
GraphRunner::from_loaded(lm, llama_cfg, backend)
  -> family_graph(llama_cfg)        // template by family
  -> GraphExecutor::prepare(graph, build_weight_map(lm), backend)
```

Rebuilding from config keeps canonical files small and avoids a
parallel-source-of-truth drift between the embedded graph and what
the template would produce today.

`mr run --path=graph <model>` forces this code path. The default
`mr run` uses the curated forward path (faster); `--path=graph` is
the verification reference.

## Template

`run::ir::family_graph(LlamaConfig)` returns a `Graph` for a parsed
LlamaStyle config. Today's template (`transformer_decoder_for_exec`)
carries:

- per-layer `RmsNorm → QKV matmul → (optional bias add) → reshape →
  (optional QK norm) → RoPE → KvCache → SDPA → o_proj matmul →
  residual add`
- `RmsNorm → gate/up matmul → activation → mul → down_proj matmul →
  residual add`
- final `RmsNorm → lm_head matmul → logits`

Two config-driven branches are wired:

| Field | Effect |
|---|---|
| `has_qk_norm` | inserts `RmsNorm` on Q and K **after** reshape, **before** RoPE |
| `has_attn_bias` | inserts `Add` of `q_proj.bias` / `k_proj.bias` / `v_proj.bias` after each matmul |

Tied embeddings are handled in `build_weight_map` — when the source
omits `lm_head.weight`, it's aliased to `model.embed_tokens.weight`.

## Implementation status

LlamaStyle+ extras (final logit softcapping, per-layer sliding/full
attention window, K=V shared projection, per-layer attention head dim
switching, RoPE-full / partial-rotary-factor, per-layer scalar on
residual output) are **not yet emitted** by the template.

For models that use those features (Gemma 3, Gemma 4), the graph
executor produces approximation output relative to the curated
forward path. The curated path supports them; the graph path does
not yet. Closing path: extend `TransformerConfig::from_llama` to
carry the extras and emit corresponding `Op` variants in the
template.

## Verification

End-to-end on cpu graph backend (`mr run --path=graph`):

| Model | Graph status |
|---|---|
| `qwen3-0.6b.canonical` (qk_norm, tied) | sensical |
| `qwen2.5-coder-1.5b.canonical` (attn_bias, tied) | sensical |
| `qwen2.5-coder-14b-abl.canonical` (attn_bias) | template-equivalent to 1.5b; RAM-pressured to verify directly |
| `gemma-4-31b.canonical` (LlamaStyle+) | not yet — pending template extras |

## Related specs

- [run/specs/ir.md](../../run/specs/ir.md) — binary IR encoding
- [run/specs/arch.md](../../run/specs/arch.md) — graph templates per family
- [import.md](import.md) — the larger import contract
