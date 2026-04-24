# Reality check (2026-04-24)

What runs today vs what the spec targets. Updated each session.

## Manifest models (in-scope for run/)

The 4 models in `manifest.rs` are the acceptance criterion.

| Model | Family | Load | Run | Output | tok/s (CPU) |
|---|---|---|---|---|---|
| qwen3-0.6b-abl | decoder/baseline | ✓ | ✓ | ✓ verified correct | ~9 |
| qwen2.5-coder-1.5b-abl | decoder/baseline | ✓ | ✓ | ✓ verified correct | ~4 |
| qwen2.5-coder-14b-abl | decoder/baseline | ✓ | ✓ | ✓ verified correct | ~0.4 |
| gemma-4-31b | decoder/gemma4 | ✓ | ✓ | ✓ verified correct | ~0.4 |

All 4 produce sensible factual answers on "What is the sun?" and math prompts.
qwen3-0.6b passes HF per-op activation golden regression (tier3_goldens).

## Full inventory (broader soma model set — future targets)

| Model | Type | Modality | Status | Blocker |
|---|---|---|---|---|
| qwen2.5-0.5b-abl | qwen2 | LLM | not in manifest | out of scope |
| qwen2.5-coder-1.5b-abl | qwen2 | LLM | ✓ works | — |
| qwen2.5-coder-14b-abl | qwen2 | LLM | ✓ works | — |
| qwen3-0.6b-abl | qwen3 | LLM | ✓ works | — |
| gemma-4-31b | gemma4 | LLM | ✓ works | — |
| deepseek-r1-8b-abl | qwen3 | LLM | loads, quality unknown | needs verify |
| nuextract-1.5 | phi3 | LLM | err | tensor naming mismatch |
| smollm2-360m | llama | LLM | err | tensor naming mismatch |
| bitnet-2b | bitnet | LLM | loads, quality unknown | needs verify |
| mimo-7b-rl | mimo | LLM | err | Q8 dtype gap |
| qwen2.5-vl-7b-abl | qwen2_vl | VL | err | nested config + VL arch |
| whisper-small | whisper | ASR | err | WhisperStyle arch missing |
| deberta-zeroshot | deberta-v2 | encoder | err | BertStyle arch missing |
| modernbert | modernbert | encoder | err | BertStyle arch missing |
| jina-v5-nano | eurobert | encoder | err | BertStyle arch missing |
| granite-hap-125m | roberta | encoder | err | BertStyle arch missing |
| granite-hap-38m | roberta | encoder | err | BertStyle arch missing |
| beats / glotlid / piper / xtts / yolo | — | — | — | binary, not .model format |

## Failure taxonomy (future targets)

### A. Tensor naming gaps
smollm2, nuextract: use different naming convention.
Fix: import.md normalization schema.

### B. Weight dtype gaps
mimo-7b-rl: Q8 dispatch not wired through load path.
Fix: quant.md enumerate per-backend dtype support.

### C. Config parsing gaps
VL models: `hidden_size` under `text_config.hidden_size`, not root.
Fix: format.md canonical config schema.

### D. Missing arch families
Encoder-only (BERT family), Whisper, VL hybrid — no curated codepath.
Fix: arch/encoder/, arch/encoder_decoder/ — future milestones.

## Path forward

1. **Honeycrisp backend** — accelerate existing 4 manifest models on Apple Silicon.
2. **Import normalization** — unblock tensor-naming and config gaps without new archs.
3. **BertStyle codepath** — unblock 5 encoder models (arch/encoder/).
4. **WhisperStyle** — arch/encoder_decoder/.
5. **VL** — multimodal arch.
