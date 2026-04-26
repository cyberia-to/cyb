# import graph IR embedding

When and how the binary IR graph is embedded into a `.model` file at
import time.

## Why embed

The graph is derivable from the config plus an arch template, but
embedding it at import time means consumers (runtime, graph
compiler, browser) read it directly instead of re-running the
template each load.

The embedded graph is the executor-compatible template (one matmul
per QKV projection), not the merged-QKV variant. It round-trips
through `run::format::read_model_file` → `LoadedModel::graph()`.

## When emitted

After tensors and config are normalized, `import` attempts:

```
let metas = run::format::parse_tensors_toml(&tensors_toml)?;
let cfg   = run::arch::decoder::config::LlamaConfig::parse(&config_toml, &metas)?;
let graph = run::ir::family_graph(&cfg)?;
```

The `~~~graph` section is emitted iff **all three** succeed:

1. `parse_tensors_toml` — already-normalized tensor index parses.
2. `LlamaConfig::parse` — config matches the LlamaStyle schema.
   This is the real gate: a config that isn't LlamaStyle (MoE, DiT,
   encoder-only, vision tower, …) fails to parse here and the graph
   emission is skipped. The tensor list passed to `parse` lets it
   detect `has_qk_norm` / `has_attn_bias` and infer `head_dim` when
   the config omits it (Qwen3 has `head_dim=128` independent of
   `hidden_size / num_heads`).
3. `family_graph` — produces a `Graph` from the parsed config. Today
   it always returns `Some` for any `LlamaConfig`; the `Option`
   leaves room for future model-type gating once non-LlamaStyle
   templates land.

If any step fails, `import` prints a one-line skip notice and writes
the `.model` without a `~~~graph` section. The runtime continues to
work because the curated forward path doesn't need it.

## Section layout

```
[[files]]
name = "graph"
format = "hex"

~~~graph
<lowercase hex of the binary IR>
```

Position: between `~~~config` and `~~~tensors`. Hex encoding keeps the
section text-safe (the file is mostly text up to `~~~weights\n`); the
binary form is canonical-bytes serialization defined in
[run/specs/ir.md](../../run/specs/ir.md).

## Implementation status (LlamaStyle+ faithfulness gap)

`TransformerConfig::from_llama` flattens the LlamaConfig down to the
LlamaStyle base. LlamaStyle+ extras (final logit softcapping, sliding
window, layer kinds, K=V projection sharing, query pre-attention
scalar, global head dim, RoPE-full theta, partial rotary factor) are
**not** carried into the embedded graph today.

For models that use those features (Gemma 3, Gemma 4), the embedded
graph approximates the LlamaStyle base and would diverge from the
actual runtime forward path. Two paths to close this:

1. Extend `TransformerConfig` + `transformer_decoder_for_exec` to
   carry the LlamaStyle+ fields and emit corresponding ops.
2. Gate `family_graph` on `model_type` so unsupported families return
   `None` until (1) lands.

Path (2) is the honest stopgap; path (1) is the real fix. Until one
ships, `import` continues to emit best-effort graphs only for plain
LlamaStyle (qk_norm and attn_bias variants).

## Round-trip test

`import/tests/graph_section_roundtrip.rs` covers the writer ↔ reader
seam: a `.model` written with `Some(hex)` round-trips through
`read_model_file` to identical bytes; a `.model` written with `None`
has no graph section.

## Related specs

- [run/specs/ir.md](../../run/specs/ir.md) — binary IR encoding
- [run/specs/arch.md](../../run/specs/arch.md) — graph templates per family
- [import.md](import.md) — the larger import contract this fits inside
