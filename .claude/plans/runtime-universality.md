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

### 3. Coder-14b garbled output — GENERATION BROKEN FOR ALL MODELS

**Critical finding (2026-04-17): not coder-14b-specific. All models
produce garbled generation. Pre-existing bug.**

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

**What's broken:**
Full forward produces coherent tokens but semantically wrong. For
qwen3 "2+2=": first gen token `<` (correct — Qwen3 uses `<think>`),
then `|` (wrong — should be `think`). Off-track after 1 correct token.

Logits normal range, no NaN/INF. Top-5 close (flat distribution).
Signature of broken compute in attention/RoPE/KV cache, NOT in matmul.

**Findings ruled out:**
- Not Q4_K matmul (tested full scale)
- Not Q6_K lm_head (test_q6k_lm_head_real passes)
- Not subgroup reduction UB (fixed in 7bc31e1e, didn't change behavior)
- Not model quant format mismatch (prefill per-layer fix didn't help,
  plus generate() uses decode path not prefill)

**Next investigation (not attempted):**
1. Layer-by-layer hidden state vs llama.cpp reference for identical input
2. Attention_decode shader at head_dim=64/128 — check scores array access
3. RoPE cache indexing for generation positions (pos > prompt_len)
4. KV cache persistence: is write-then-read consistent across forward calls?
5. Does prefill path (seq_len > 1) give same output as decode (seq_len = 1)
   for same tokens? If not, which is correct?

DEBUG_LAYERS=1 env var dumps layer 0-2 + last 2 hidden states (committed).

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
