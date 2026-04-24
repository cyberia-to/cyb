# Test strategy

Three-tier verification. Each tier has specific coverage, tolerance,
and speed budget. A runtime claims correctness only when all three
tiers pass for a given model + backend combination.

## Tier 1: Op unit tests

Each op from [ops.md](ops.md) has a dedicated test with golden values
that any implementation must match.

### Structure

```
tests/ops/<op_name>/
    inputs.json          # fixed seeded inputs
    expected.json        # golden outputs from reference (HF PyTorch f32)
    test.rs              # loads inputs, runs op on each backend, compares
```

### Golden value provenance

Reference = HuggingFace PyTorch f32. One-time dump into `expected.json`
by a Python script (`reference_dump.py` in the repo). Committed to
source control.

Cross-check against llama.cpp f32 where the op exists there (matmul,
rmsnorm, rope, sdpa). Discrepancies between HF and llama.cpp are
resolved case-by-case and documented inline.

### Tolerance

Per-op, per-dtype:

| Op | F32 | F16 | Q4_K |
|---|---|---|---|
| Matmul | 1e-6 | 1e-3 | 1e-2 |
| RmsNorm | 1e-6 | 1e-3 | N/A (not quantized) |
| Rope | 1e-6 | 1e-3 | N/A |
| Sdpa | 1e-5 | 1e-2 | N/A (activations) |
| Softmax | 1e-6 | 1e-3 | N/A |
| Silu / Gelu | 1e-6 | 1e-3 | N/A |
| Quantize round-trip | see [quant.md](quant.md#tolerance) | | |

### Size

Op tests use small tensors (typically N=K=D=32 or 64). Execution is
milliseconds. Entire Tier 1 suite runs in < 30 seconds.

### Must-run

Every backend that claims to implement an op runs Tier 1 for that op.

## Tier 2: Layer composition tests

Verify that ops composed into a layer produce outputs matching the
reference. Catches bugs that don't surface in isolated ops — e.g.
wrong KV cache read order, RoPE applied to wrong tensor half,
skip connection before/after norm.

### Structure

Per family (LlamaStyle, BertStyle, etc.), one test per layer
primitive:

```
tests/layers/decoder/
    attention_layer.rs       # embed → norm → QKV → rope → sdpa → o_proj → skip
    ffn_layer.rs             # norm → gate/up → silu+mul → down → skip
    full_block.rs            # attention_layer + ffn_layer
    with_qk_norm.rs          # Qwen3 variant
    with_gqa.rs              # GQA with num_heads > kv_heads
```

### Golden values

Dumped from HuggingFace transformers running in f32. For each test:
given specific weight values (small ones, random seeded) and input
activation, run the layer and save output.

Implementations on each backend must match to within tolerance.

### Tolerance

Layer composition accumulates op errors. Tolerance scales with
expected op count:

| Layer type | F32 | F16 |
|---|---|---|
| Single op | 1e-6 | 1e-3 |
| Attention layer (~7 ops) | 1e-5 | 5e-3 |
| Full decoder block (~15 ops) | 5e-5 | 1e-2 |

### Size

Layer tests use realistic dimensions (e.g. hidden=1024, num_heads=16,
head_dim=64). Single forward over 1-4 tokens. Runs in seconds.

## Tier 3: Model golden tests

End-to-end. For each supported model, a small set of prompts with
known-correct outputs.

### Structure

```
tests/models/<model_name>/
    config.toml              # redundant with .model but explicit
    goldens.json             # [{prompt, expected_tokens, expected_text, tolerance}]
    test.rs                  # loads, runs, compares
```

### Prompts

Fixed canonical prompts designed to:

1. **Trigger all sub-architectures**: math (exercises reasoning),
   special tokens (tests EOS emission), code (tests long generation),
   language-switching (tests full vocab).

2. **Have deterministic greedy outputs**: temperature=0, golden is
   the exact token sequence.

Example:
```json
[
  {
    "prompt": "What is 2+2? /no_think",
    "expected_tokens": [17, 488, 220, 17, 284, 220, 19, 13],
    "expected_text": "2 + 2 = 4.",
    "tolerance_tokens": "exact"
  },
  {
    "prompt": "Translate 'hello' to French: ",
    "expected_contains": ["bonjour", "Bonjour", "Salut"],
    "tolerance_tokens": "contains_one_of"
  }
]
```

### Correctness source

Reference runs:
1. HuggingFace transformers f32 (primary)
2. llama.cpp with matching GGUF (cross-check)

Both must agree on the golden. Divergence documented and investigated.

### Tolerance

Exact token match for greedy (temperature=0) is the goal. If reference
tokenization differs across libraries, fallback to "contains_one_of"
with enumerated acceptable outputs.

For temperature > 0, tests are not deterministic → skipped in this tier.

### Coverage requirement

v1 acceptance ([scope.md](scope.md#acceptance-criteria-for-v1)): at
least one model per modality in Tier 3 tests, passing on every
supported backend.

## Tier 0: Import round-trip

Before Tiers 1-3 make sense, imports must be faithful.

### Structure

For each source-format / model combination:

1. Import source → `.model`.
2. Read back `.model`, dequantize tensors.
3. Dequantize source tensors (GGUF → f32 via llama.cpp, safetensors →
   f32 via Python).
4. Compare tensor-by-tensor within [quant.md](quant.md#tolerance).

### Passes when

Every imported tensor matches source-dequantized within per-dtype
tolerance. No tensor dropped, no name mismatch, no shape mismatch.

## CI structure

Each commit runs:

- All Tier 0 (import round-trip) — must pass
- All Tier 1 (op unit tests) — must pass on CPU reference and default
  backend for the platform (wgpu+rs everywhere, honeycrisp on macOS)
- Tier 2 for curated families with implementation — must pass
- Tier 3 for models in a "fast" subset (small ones, <1B params) —
  must pass

Nightly / release runs:

- Tier 3 full: every model in manifest, every backend
- Stress test: same prompt 1000× with temperature=0 — must produce
  bit-identical output (catches nondeterminism)

## Proving a bug

When a bug is suspected, reproduce by narrowing through tiers:

1. Does Tier 3 pass? No → identify divergence point.
2. Run Tier 2 for the layer containing the divergence. Which layer
   first shows off-by-ε values?
3. Run Tier 1 for the ops in that layer. Which op produces wrong output?
4. Fix that op. Re-run all tiers.

A bug that doesn't show in Tier 1 or Tier 2 but shows in Tier 3 is
usually a composition bug (wrong buffer wiring, wrong op order, state
pollution across forward calls) — Tier 2 should grow a new test that
reproduces it.

## Coverage tracking

`cyb-llm test --report` produces:

```
Tier 0: import/gguf/qwen3-0.6b  ✓   (312 tensors, max_diff 1.4e-2)
Tier 1: op/rmsnorm              ✓   (f32 ε<1e-6, f16 ε<9e-4)
Tier 1: op/sdpa                 ✓
Tier 2: layers/decoder/attention_layer  ✗  at qk_norm output (first divergence 3.2e-2)
Tier 3: models/qwen3-0.6b       ✗  (divergence at token 4)

Summary: 5/26 models passing Tier 3.
```

Regressions in report → block release.

## Running locally

```
cyb-llm test                     # all tiers, current backend
cyb-llm test --tier 1            # just op tests
cyb-llm test --backend wgpu+rs   # verify portable path
cyb-llm test --model qwen3-0.6b  # specific model, all tiers
```

## Responsibility

- **Runtime author** writes Tier 1 + Tier 2 when adding an op or
  family.
- **Import author** writes Tier 0 when adding a source format mapping.
- **Model onboarding** adds Tier 3 goldens per model.

A PR that adds a feature but not tests is incomplete.
