# Completeness audit (2026-04-17)

Spec is **architecture-complete** but **operationally incomplete**.

> **UPDATE 2026-04-17 (2):** CRITICAL 1-7 filled. See changelog at bottom.

A reimplementer can start but would stall at the following. Ordered
by severity — fill in order, especially before any large coding
effort.

## CRITICAL (must fill before reimplementation)

### 1. Graph IR serialization format

`architecture.md` describes a graph executor that "walks the DAG".
`format.md` has no `graph` section layout. `ops.md` defines op math
but not node encoding.

**To fill:** new `ir.md` — Node struct (id, op, inputs, outputs,
attrs), serialization format (binary or TOML), topological walk
rules, shape inference per op.

### 2. KV cache data structure

`ops.md` KvCache says `cache.K[p : p+s] = K_new` but leaves layout
undefined. No struct definition, no memory organization.

**To fill:** extend `ops.md` KvCache section — tensor shape
`[num_layers, kv_heads, max_seq, head_dim]` or similar, append vs
rotate-buffer, lifetime, cleanup.

### 3. Tokenizer BPE algorithm

`format.md` describes vocab TOML but not how to apply merges. No
BPE walk-through, no special-token injection rule.

**To fill:** new `tokenizer.md` — BPE merge algorithm, priority
handling, byte-level vs char-level, special-token preemption,
chat template application.

### 4. Chat template format

`execution.md` calls `apply_chat_template(prompt)`. Spec never says
what a template IS or where it's stored.

**To fill:** section in `format.md` or `tokenizer.md` — template
syntax (Jinja-like? string interpolation?), storage location, per-turn
injection, EOS/BOS rules.

### 5. Multimodal config flattening — examples

`import.md` says "flatten nested VL configs" but `format.md` config
example is single-tower. What keys live under `[architecture]`
vs `[architecture.vision]` is undefined.

**To fill:** add VL example to `format.md` showing post-flatten
layout for qwen2_vl, moondream, Whisper (mel input).

### 6. Fused op auto-detection

`ops.md` says "any backend may implement fused". Does the runtime
detect fusion opportunities? If so, what patterns? If not, who
decides to emit fused?

**To fill:** section in `execution.md` — fusion is compile-time
decision (curated path explicit) OR pattern match during dispatch.
Clarify which.

### 7. K-quant block boundary handling

`quant.md` describes single superblocks. What if tensor K dimension
is not a multiple of 256? Padding rules? Partial blocks?

**To fill:** section in `quant.md` — alignment requirements,
padding vs rejection, K-dim divisibility constraints per format.

## MODERATE (would cause bugs in v1)

8. **RoPE edge cases** — odd head_dim, multi-axis (DiT 3D video),
   unusual rope_theta values.
9. **Causal mask value** — -∞ risks NaN; clarify -1e9 vs -∞ vs
   backend-specific.
10. **Pool op parameters** — kernel, stride, padding, pool mode
    (mentioned but not all specified).
11. **Conv dilation + non-depthwise groups** — only depthwise
    `groups=C_in` covered.
12. **Sampling method config schema** — how to specify Greedy vs
    TopP in `[sampling]` TOML.
13. **GQA expand algorithm** — replication along head axis, exact
    reshape operation.
14. **K-quant matmul tolerance** — tolerance table has F32/F16/Q4
    but Q4_K/Q5_K/Q6_K not itemized.
15. **BertStyle padding mask** — bidirectional attention needs
    padding mask; format and application undefined.
16. **Sharded model runtime loading** — import handles multi-shard
    GGUF; runtime is silent on multi-file `.model` loading.
17. **Adapter storage** — LoRA/adapters mentioned in scope but no
    format or application rules.
18. **Error message format** — error enum defined but no examples
    of actual messages per case.
19. **Numerical stability requirements per op** — which ops must be
    numerically stable, where to clamp/clip.
20. **Prefill-vs-decode backend contract** — is prefill optional
    per backend or required?

## MINOR (slow learning, doesn't block)

21. **FrameAllocator policy** — reuse strategy not specified.
22. **Transpose view-vs-copy criteria** — when is a view possible.
23. **Position cache beyond RoPE** — are causal masks precomputed?
24. **Backend fallback ordering** — multi-backend precedence rules.
25. **Ternary scale storage location** — per-tensor f32 but where.
26. **Position embedding type enumeration** — absolute, relative,
    ALiBi, Kerple — canonical strings.
27. **Error examples per error type** — boilerplate.
28. **Padding byte values in weights section** — zeros or
    undefined.

## Edge cases (undefined)

29. **Empty tensors** (shape `[0]` or `[B, 0, D]`) — behavior.
30. **Max context overflow** — truncate / error / attend-to-partial.
31. **Speculative decoding** — not acknowledged anywhere.

## Future (already scoped out, but gap between v1 and future is large)

32. **Nox VM instruction set** — 18 instructions claimed, none listed.
33. **Trident DSL** — referenced, not specified.
34. **Aruminium crate contract** — honeycrisp depends on it; crate
    API not documented.
35. **unimem** — zero-copy layout referenced; specific IOSurface
    guarantees not stated.
36. **Continuous batching** — explicit "v1 no" but API impact absent.

## Recommended fill order

Before any reimplementation, fill CRITICAL 1-7. Estimate: 2-3 days
writing; saves weeks of rework.

Before v1 ships, fill MODERATE 8-20. Most can be small additions to
existing files.

MINOR and edge cases can be filled on-demand during implementation
— flag with TODO comments, revisit at release.

Future items need their own specs when the consumer appears (Nox VM
when trident lands, continuous batching when serving demands it).

## Verdict

| Criterion | Status |
|---|---|
| Architecture complete | ✓ |
| Math / ops complete | ✓ |
| Quant bit layouts | ✓ (single block only) |
| Family templates | ✓ |
| Tokenizer spec | ✗ **missing** |
| IR format | ✗ **missing** |
| Chat template | ✗ **missing** |
| KV cache struct | ✗ **incomplete** |
| Fused op policy | ✗ **ambiguous** |
| Error examples | ◯ partial |
| Test strategy | ✓ |

Reimplementation blocked on 4 missing pieces: IR format, tokenizer,
chat template, KV cache structure. Others are refinable mid-implementation.

## Changelog

### 2026-04-17 (2) — CRITICAL gaps filled

- ✓ #1 Graph IR → new `ir.md`, binary serialization, op tags,
  payload encoding, walk algorithm, shape inference
- ✓ #2 KV cache → extended `ops.md` KvCache section with struct
  definition, append semantics, lifecycle, memory budget
- ✓ #3 Tokenizer BPE → new `tokenizer.md`, byte-level mapping,
  merge algorithm, special tokens, byte fallback, round-trip
- ✓ #4 Chat template → included in `tokenizer.md` chat templates
  section, Jinja subset, `~~~chat` storage, common formats
- ✓ #5 VL config → added concrete examples to `format.md` for
  qwen2_vl and whisper with nested `[architecture.vision]` /
  `[architecture.audio_encoder]`
- ✓ #6 Fused op policy → extended `execution.md` with curated
  (explicit) vs graph (structural pattern matching at load time)
- ✓ #7 K-quant block boundaries → extended `quant.md` with K dim
  alignment rules, reject-and-fallback policy, no partial blocks

Moderate gaps also addressed in pass:
- RoPE edge cases (odd head_dim error, multi-axis 3D for DiT)
- Causal mask value (-1e4, not -inf)
- Pool stride/padding/mode
- Conv dilation/groups (beyond depthwise)
- Sampling method config schema
- GQA expansion algorithm (repeat_interleave, not tile)
- Ternary scale storage (`.scale` sibling tensor)
- Backend fallback chain (default → honeycrisp → wgpu+rs → CPU)
- FrameAllocator policy (exact-size buckets, per-usage pools)

Remaining: minor gaps (view/copy criteria, error messages, empty
tensors, max context overflow), future items (nox VM, trident,
aruminium contract). Fillable on-demand during implementation.

**Reimplementation is now unblocked.** Start with LlamaStyle curated
+ wgpu+rs backend as the smallest shippable slice.
