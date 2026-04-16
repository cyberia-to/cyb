# Runtime Universality Plan

Status: approved
Created: 2026-04-16
Updated: 2026-04-16

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

### 3. Coder-14b garbled output — IN PROGRESS

New findings (2026-04-16):
- WGPU: logits range normal (-17..15), but argmax picks wrong tokens.
  Output "Rif>Mainvos" or "PKG" instead of "4".
- Metal: NaN logits from pos=0. Argmax=0 every step. Output "!!!!!".
- Ollama: correct output "2+2 = 4" from same GGUF.
- qwen3-0.6b works correctly on both backends (225 tok/s Metal).
- Difference: qwen3-0.6b uses Q4 encoding. Coder-14b uses Q4_K encoding.
- Coder-14b also uses Q4_K GPU embed path (vocab 152K × hidden 5120 = 3.1GB > 2GB threshold).

Suspects:
1. Q4_K matvec at large dimensions (5120 hidden) — may have precision/indexing bug
2. Q4_K embed GPU dequant may produce wrong values
3. GGUF shape convention: embed stored as [hidden, vocab] vs [vocab, hidden]
   — physical layout IS [vocab, hidden] (confirmed), but metadata says [5120, 152064]

Next step: add debug dump after embed + after layer 0 for coder-14b.
Compare first 8 values with reference (llama.cpp/ggml debug output).

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
