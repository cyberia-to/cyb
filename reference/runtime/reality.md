# Reality check (2026-04-17)

What runs today vs what the spec targets. Updated each session.

## Full inventory (26 models)

| Model | Type | Modality | Load | Run | Reality |
|---|---|---|---|---|---|
| qwen2.5-0.5b-abl | qwen2 | LLM | ✓ | ✓ | garbled: "桨吾" |
| qwen2.5-coder-1.5b-abl | qwen2 | LLM | ✓ | ✓ | plausible: "Hello!" |
| qwen2.5-coder-14b-abl | qwen2 | LLM | ✗ timeout | — | load >30s (big model) |
| qwen3-0.6b-abl | qwen3 | LLM | ✓ | ✓ | fragmented: "<\|im_end" pieces |
| deepseek-r1-8b-abl | qwen3 | LLM | ✓ | ✓ | runs, quality unknown |
| mimo-7b-rl | mimo | LLM | ✗ | err | Q8 dtype not supported on Metal |
| nuextract-1.5 | phi3 | LLM | ✗ | err | tensor naming mismatch |
| smollm2-360m | llama | LLM | ✗ | err | tensor naming mismatch |
| bitnet-2b | bitnet | LLM | ✓ | ✓ | runs, quality unknown |
| gemma-4-31b | gemma4 | LLM | ✗ timeout | — | panics at layer 5 (shape) |
| qwen2.5-vl-7b-abl | qwen2_vl | VL | ✗ | err | Q8 dtype + VL config not parsed |
| qwen3.5-4b-abl | qwen3_5_vl | VL | ✗ | err | nested config (hidden_size under text_config) |
| moondream2 | moondream | VL | ✗ | err | same nested config |
| wan22-video | wan | video | ✗ | err | same nested config |
| whisper-small | whisper | ASR | ✗ | err | no embed_tokens (arch mismatch) |
| deberta-zeroshot | deberta-v2 | encoder | ✗ | err | no embed_tokens (BERT naming) |
| modernbert | modernbert | encoder | ✗ | err | same |
| jina-v5-nano | eurobert | encoder | ✗ | err | same |
| granite-hap-125m | roberta | encoder | ✗ | err | same |
| granite-hap-38m | roberta | encoder | ✗ | err | same |
| beats | ? | audio | — | — | binary (not text .model) |
| glotlid | ? | text | — | — | binary |
| piper-tts | ? | TTS | — | — | binary |
| xtts-v2 | ? | TTS | — | — | binary |
| yolo11n | ? | CNN | — | — | binary |
| moondream2 | moondream | VL | ✗ | err | missing hidden_size |

## Summary

- **5/26** models load and run (19%)
- **0/26** produce verified-correct output
- **2/26** produce plausible-looking output (coder-1.5b, not verified)

## Failure taxonomy

Not bugs — structural gaps between current code and spec:

### A. LLM bugs (runtime has codepath but output wrong) — 3 models

qwen3-0.6b, qwen2.5-0.5b, qwen2.5-coder-14b (times out).
Root cause: unknown, under investigation. Unit tests of individual
ops pass; composition fails. See [architecture.md](architecture.md)
correctness invariants.

### B. Tensor naming gaps — 3 models

smollm2 (llama), nuextract (phi3), and likely others.
`model.layers.0.self_attn.q_proj.weight` missing. These architectures
use subtly different naming that our import didn't normalize.

Fix: [import.md](import.md) should define normalization to a
canonical tensor-naming schema.

### C. Weight dtype gaps — 2 models

mimo-7b-rl, qwen2.5-vl-7b: Metal doesn't handle Q8 weights.
Q8 dispatch exists in code but not wired through load path.

Fix: [quant.md](quant.md) must enumerate every dtype and state
which backend implements which.

### D. Config parsing gaps — 4 models

All VL (qwen2_vl, qwen3_5_vl, moondream, wan): nested config
structure. `hidden_size` lives under `text_config.hidden_size`, not
at root. Our runtime reads root only.

Fix: [format.md](format.md) must define canonical config schema
(flat or nested), import normalizes.

### E. Missing model families — 6+ models

- Encoder-only BERT family (deberta, modernbert, jina, granite×2) —
  no BertStyle codepath + Metal backend assumes embed_tokens.
- Whisper — no WhisperStyle.
- VL models — no hybrid codepath.
- Binary models (beats, glotlid, piper, xtts, yolo) — unclear
  if even well-formed; skipped for now.

Fix: each family gets a curated codepath per
[arch.md](arch.md) templates. OR graph executor runs them (graph
path not yet wired through import).

### F. Import-time slow / broken — 2 models

qwen2.5-coder-14b (timeout), gemma-4-31b (panics at layer 5).
Both are big models with Q4_K quant; likely need mmap path that
was added recently. gemma-4 has tensor shape irregularity.

Fix: [import.md](import.md) + [execution.md](execution.md) cover
large-model loading explicitly.

## Path of least resistance

If we want to move from 5/26 to 26/26, in priority order:

1. **Fix LLM correctness bug** (category A) — unblocks 5 existing runs.
   Spec-first: write [ops.md](ops.md) + [arch.md](arch.md) for LlamaStyle
   precisely, then reconcile code.

2. **Import normalization** (B + C + D) — unblocks 9 more models
   without new codepaths. Specs: [format.md](format.md),
   [import.md](import.md), [quant.md](quant.md).

3. **Add BertStyle codepath** — unblocks 5 encoder models.
   Specs: [arch.md](arch.md) BertStyle template.

4. **Wire graph executor through import** — unblocks remainder as
   correctness-first fallback.

5. **Curated codepaths for ASR, VL, TTS, diffusion** — v1 modality
   coverage. Weeks of work but each unblocks a modality.

## Notes

- Binary .model files (beats, piper, xtts, glotlid, yolo) need
  separate inspection — skipped in this check.
- Output correctness was not validated for any run; all "✓ run"
  cases need golden-value verification per [test.md](test.md).
- Times are approximate; 30s timeout arbitrary.
- Reality check runs again after each significant code change via
  `cyb-llm status` + this document.
