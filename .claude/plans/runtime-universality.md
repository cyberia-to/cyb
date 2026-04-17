# Runtime Universality Plan

Status: approved
Created: 2026-04-16
Updated: 2026-04-17

Make any GGUF model work out of the box. No model-specific workarounds.

## Progress

### 1. Buffer >4GB / large-vocab embed — DONE

WGPU: Q4_K embed shader (q4k_embed.wgsl) — keeps raw Q4_K on GPU, dequant
per-token. Decode + prefill both wired. Committed a26e2c42.

Metal: CPU Q4_K dequant per-token (dequant_q4k_row_to_f16). Avoids 2.8GB
f16 table upload. Committed 1bb4cf00.

File loader: mmap for >1GB files. Was reading 17GB twice (34GB total).
Now mmap + per-tensor copy. Header scan up to 32MB (gemma-4 has 18.4MB
text header due to 60-layer tensor index). Committed 1bb4cf00.

Metal matvec_q4k: fixed dmin handling + get_scale_min_k4. Previous version
ignored dmin entirely. Committed a26e2c42.

### 2. Attention shared memory limit — TODO

`scores: array<f32, 2048>` in attention.wgsl limits max_seq to 2048.
Fix: tiled attention or storage buffer.

### 3. Generation garbled — RUNTIME HAS REAL BUG IN FORWARD PASS

**Status: unresolved. Earlier abliteration theory REFUTED.**

Smoking gun: test_ollama_gguf_direct loads ollama's Q6_K
`huihui_ai/qwen3-abliterated:0.6b` GGUF (same model ollama runs
correctly) through our runtime. Output: argmax=2423 after
`<|im_start|>`, logit[151645]=-4.04. Garbage.

**Same abliterated model:**
- Through ollama → correct "2 + 2 = 4" with thinking
- Through our runtime → `|im_end|>` pieces or random tokens

**All eliminated:**
- ✗ Q4_0 quantization loss (Q6_K also garbage)
- ✗ Abliteration damage (ollama handles it fine)
- ✗ Import bug (direct GGUF load also garbage)
- ✗ Matmul (unit tests pass at full scale 152064×5120 @ 3e-7)
- ✗ LM head (Q6_K lm_head verified against CPU)
- ✗ Subgroup UB (fixed, no behavior change)
- ✗ Special token embed norms (ollama's 0.48 norm works correctly)

**Bug must be in forward chain:**
- Attention decode shader
- RoPE (position indexing, cos/sin cache)
- Qwen3 QK-norm (q_norm/k_norm per-head, Qwen3-specific)
- Layer residual connections
- KV cache write/read race conditions

**Highest-suspicion: QK-norm.**
Qwen3 has `q_norm` and `k_norm` weights that Qwen2 doesn't. Our
code detects them (`has_qk_norm`) and applies rms_norm per-head.
If this is buggy, Q and K projections feed wrong values into
attention, and outputs degrade systematically. Qwen2.5-coder-14b
also has qk-norm → would affect both models. Worth investigating
first.

**Tests available:**
- test_ollama_gguf_direct — reproduces bug with known-good weights
- test_compare_our_embed_vs_ollama_gguf — weight comparison
- DEBUG_LAYERS=1 — per-layer hidden dump
- DEBUG_TOKEN_IDS=1 — per-token ID dump


- Reverted to commit 6bec0b97 (before recent Q4_K/mmap/subgroup work):
  qwen3-0.6b still garbled. Confirms not caused by recent changes.
- qwen3-0.6b on `"2+2="`: predicts `<|im_end|>` immediately instead
  of `<think>...2+2=4...`. Metal 218 tok/s, but wrong output.
- Ollama with same GGUF: correct "2+2 = 4" with thinking trace.

**What's verified correct (unit tests all pass):**
- Q4_K matmul: 3e-7 precision, small and full-size (152064×5120 lm_head)
- Q6_K matmul: same precision at lm_head scale
- Q5_K, Q3_K, Q2_K matmul shaders
- RMS norm matches CPU reference (e2e layer0 test)
- Embed: CPU f32 and GPU Q4_K paths both tested

**Evidence of correct runtime (all unit tests pass):**
- Not Q4_K matmul (tested full scale)
- Not Q6_K lm_head (test_q6k_lm_head_real passes)
- Not subgroup reduction UB (fixed in 7bc31e1e)
- Not model quant format mismatch
- Hidden states normal range (no NaN/INF) through all layers

**Root cause: abliteration zeros special token directions.**
test_special_token_embed_norms shows special tokens (151644, 151645,
151665) in qwen3-0.6b-abl have embed norm ~0.4x regular tokens
(0.94). With tied weights, argmax systematically under-scores these
→ model emits pieces.

DEBUG_LAYERS=1 and DEBUG_TOKEN_IDS=1 env vars available for future
debugging (committed).

### 4. Gemma-4 architecture — BLOCKED on transpose

Gemma-4 loads but panics at layer 5: transpose_blocks index out of range.
Slice len=6193152 but tried to read 6193296 (diff = 144 = 1 Q4_K block).
Some tensor shape does not divide cleanly by Q4_K block size.

Also needs:
- GELU activation (vs SiLU)
- final_logit_softcapping
- Mixed sliding_window / full_attention layer types
- attention_k_eq_v (shared K/V projections)

## Implementation priority

1. ~~Q4_K embed shader~~ DONE
2. **Coder-14b debug** — find divergence point vs reference
3. **Gemma-4 tensor shapes** — fix transpose_blocks for non-aligned tensors
4. **Tiled attention** — unblocks long context
5. **Gemma-4 arch features** — GELU, softcapping, sliding window
