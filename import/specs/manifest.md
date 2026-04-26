# import manifest

`import/manifest.rs` is the single source of truth for which models are
**verified-correct + fast** in MVP scope. Membership is the discipline
that keeps the runtime moat from sliding into a model zoo.

## MVP target — 4 models

| # | Model | Family | Role | Why in scope |
|---|-------|--------|------|--------------|
| 1 | `qwen3-0.6b-abl` | LlamaStyle (qk_norm) | router | always-on classifier; smallest path to verified correctness |
| 2 | `qwen2.5-coder-1.5b-abl` | LlamaStyle (attn_bias) | code (small) | second LlamaStyle variant — proves runtime handles both |
| 3 | `qwen2.5-coder-14b-abl` | LlamaStyle | code (large) | large-model load path (fused Q4_K matmul) |
| 4 | `gemma-4-31b` | LlamaStyle+ | general | exercises softcapping, sliding window, K=V — the long-tail features |

Source: [.claude/plans/cyb-mvp.md](../../.claude/plans/cyb-mvp.md) §"Manifest scope".

## ModelSpec

The live struct is `ModelSpec` in [`import/manifest.rs`](../manifest.rs).
Each entry binds:

- a name on disk (`~/llm/{name}.model`),
- an HF repo (the source of weights and tokenizer),
- an optional ollama tag (the comparison baseline for benchmarks),
- a `Role` (Router / Specialist / General — see below),
- a Q4 RAM target in megabytes,
- a one-line rationale.

`Role` partitions the soma routing layer:

| Role | Loaded | Purpose |
|---|---|---|
| `Router` | always | classify queries, dispatch to specialists |
| `Specialist` | on-demand | domain-specific (code, vision, …) |
| `General` | on-demand | broad capability fallback |

## Membership criteria

A model joins the manifest when **all** of the following hold:

1. Source format is one `import` accepts ([import.md](import.md) §Inputs).
2. Architecture maps onto a known family
   ([run/specs/arch.md](../../run/specs/arch.md)).
3. There is a planned role in the soma (router / specialist / general)
   that no current member covers, OR it exercises a runtime variant
   not covered by the existing four (e.g. attn_bias, K=V, softcapping).
4. We have an ollama tag (or HF leaderboard slot) for honest
   benchmarking.
5. Q4 RAM fits the per-role target:

   | Role | Q4 RAM target | Hardware envelope |
   |---|---|---|
   | Router | ≤ 1 GB | runs anywhere; M1 Air 8 GB OK |
   | Specialist (small) | ≤ 4 GB | M1 Pro 16 GB |
   | Specialist (large) | ≤ 12 GB | M1 Pro 16 GB (with headroom) |
   | General | ≤ 24 GB | M1 Max 32 GB |

   Exceeding the target = an architectural exception, documented in
   `notes`. Today's general entry (`gemma-4-31b`, 18 GB) sets the
   M1 Max envelope; tightening this requires either heavier quant
   or KV-compression work in `run/`.

Models added without an honest reason to be in scope dilute the
discipline. Per [.claude/plans/cyb-mvp.md](../../.claude/plans/cyb-mvp.md):
"new families enter the manifest only after the existing four are
perfect and fast."

## Promotion: experiment → manifest

Out-of-manifest models live in `~/llm/` alongside manifest models;
nothing prevents loading them. They simply don't get the
"verified-correct + fast" guarantees. To promote:

1. Demonstrate the variant exercises a runtime path no current member
   does (or covers a soma role gap).
2. Open a manifest entry with the rationale in `notes`.
3. Drive it to verified-correct on all backends before merging.

## Demotion: drop from manifest

A manifest entry is removed when its rationale collapses (e.g. the
runtime path it exercised is now covered better by another member, or
the upstream model was pulled). Demotion is a deliberate decision, not
silent attrition.
