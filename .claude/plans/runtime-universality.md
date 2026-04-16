# Runtime Universality Plan

Status: approved
Created: 2026-04-16

Make any GGUF model work out of the box. No model-specific workarounds.

## Verified issues

### 1. Buffer >4GB crash (blocks gemma-4-31b)

wgpu max_buffer_size = 4GB. Any model with vocab × hidden × 4 > 4GB crashes.

Gemma-4: vocab=262144 × hidden=5376 × 4 = 5.6GB → crash.

Fix: keep embed as quantized on GPU. Do Q4_K embed lookup in a compute shader
instead of dequanting 5.6GB to f32. Same for lm_head — keep as quantized,
use Q4_K matvec for logit computation.

### 2. Attention shared memory limit (future-proofs to long context)

`scores: array<f32, 2048>` in attention.wgsl limits max_seq to 2048.
Models with context >2048 silently corrupt. Gemma-4 has 262K context.

Fix: tile attention over sequence — process chunks of 2048, accumulate partial
softmax. Or use dynamic allocation via storage buffer instead of workgroup mem.

### 3. Coder-14b garbled output (unknown compute bug)

Everything individually verified correct: Q4_K shader, Q6_K shader, embed,
tokenizer, GQA expansion, format chain. But 48-layer forward produces garbage.
Ollama outputs correct "Four" from same GGUF.

Suspected: subtle numerical issue in one of the intermediate kernels (RoPE,
attention, skip_norm) that only manifests at larger dimensions. Need
layer-by-layer comparison with a known-good reference.

Debug approach:
- Export intermediate tensors from ollama/llama.cpp for first token
- Compare with our GPU output after each layer
- Find exact divergence point

### 4. Gemma-4 architecture support

Gemma-4 needs features not in the current transformer template:
- Mixed sliding_window / full_attention layer types
- GELU activation (vs SiLU)
- final_logit_softcapping
- attention_k_eq_v (shared K/V projections)

Fix: parse these from config, add conditional dispatch.

## Implementation priority

1. **Q4_K embed shader** — unblocks gemma-4 (and any large-vocab model)
2. **Layer-by-layer debug** — find coder-14b compute bug
3. **Tiled attention** — unblocks long context
4. **Gemma-4 arch features** — activations, softcapping, sliding window
