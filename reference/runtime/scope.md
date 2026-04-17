# Scope

What cyb-llm runtime MUST run, MAY run, and WILL NOT run.

A "universal, portable" runtime cannot support everything. It must
support a well-defined subset large enough to cover real use, small
enough to verify exhaustively. This file draws the line.

## Design principle

Coverage is a tree, not a list. Each tier inherits from the one
below. Tier N+1 adds capability; Tier N still works without it.
This enables shipping incrementally: Tier 0 working beats Tier 3 broken.

## Tier 0 — decoder-only LLM (MUST)

The 80% of HuggingFace: text-in, text-out autoregressive models.

### Architectures

Canonical transformer decoder with these features:

- Pre-layernorm with RMSNorm (Llama-style) OR post-layernorm with LayerNorm
- Causal self-attention with Grouped Query Attention (GQA, num_heads ≥ kv_heads)
- Rotary Position Embedding (RoPE) — standard or NeoX style
- FFN: SwiGLU (gate, up, down) or GeLU-MLP
- Tied or untied word embeddings
- Optional attention biases on Q, K, V (Qwen2 style)
- Optional per-head QK-norm (Qwen3, DeepSeek-V3 style)

### Covered models (examples)

- Llama 2/3, Mistral, Mixtral (dense only for Tier 0)
- Qwen2, Qwen2.5, Qwen3 (including -Coder variants)
- Gemma 1, Gemma 2 (not Gemma 3/4 — see Tier 1)
- Phi 2, Phi 3, Phi 4
- SmolLM, SmolLM2
- DeepSeek-LLM (not V2/V3 — MoE, see Tier 3)
- StarCoder 2
- MiMo, NuExtract

### Weight formats (per-tensor mixed allowed)

- F32, F16, BF16 (on disk; may dequant at load or GPU)
- Q8_0 (8-bit symmetric blocks of 32)
- Q4_0 (4-bit symmetric blocks of 32)
- Q4_K, Q5_K, Q6_K (K-quants, 256-elem super-blocks)
- Q3_K, Q2_K (low-bit K-quants, for MXFP-like compression)

### Context length

- Up to 32K tokens during generation
- Prefill supports same max as decode
- Attention tiling required beyond 2048 (workgroup memory limit)

### Sampling

- Greedy (temperature=0)
- Top-K, Top-P (temperature > 0)
- Min-P optional

### Tokenization

- Byte-level BPE (GPT-2, Qwen, Mistral family)
- SentencePiece (Llama, Gemma)
- Special token registration + detection

## Tier 1 — encoders + exotic decoders (SHOULD, next)

### Encoder-only

Unlocks classification, embedding, zero-shot, retrieval.

- BERT, RoBERTa, DeBERTa v2/v3
- ModernBERT
- Jina, e5, bge embedding models
- Absolute position embeddings
- Bidirectional attention (no causal mask)
- CLS pooling, mean pooling
- Classification head (sequence, token)

Covered: our tier-0 soma models (deberta-zeroshot, modernbert, granite-hap, jina).

### Decoder extensions for Gemma 3/4

- Mixed sliding_window + full_attention layer types
- GELU activation (instead of SiLU)
- Final logit softcapping
- attention_k_eq_v (shared K=V projections)

Covered: gemma-3, gemma-4.

### Weight formats (additional)

- IQ2, IQ3, IQ4 (imatrix quants) — optional, for space-constrained
- Ternary (BitNet 1.58-bit)

## Tier 2 — sequence-to-sequence + multimodal (MAY, later)

### Encoder-decoder

- T5, FlanT5, mT5 (relative position bias, different attention)
- BART, Whisper

### Multimodal (vision-language)

- LLaVA, Qwen2-VL, Moondream
- Vision encoder (ViT or similar) → projection → LLM
- Image token merging

### Audio

- Whisper (ASR)
- BEATs (audio encoder)
- XTTS, Piper (TTS)

## Tier 3 — advanced architectures (WILL NOT for now)

Explicitly out of scope for current universality claim:

- Sparse MoE (Mixtral, DeepSeek-V2/V3, Qwen-MoE)
- State-space models (Mamba, Mamba2, RWKV)
- Diffusion (Stable Diffusion)
- Training / fine-tuning
- Continuous batching (multi-request)

These are future work. Tier 0 + Tier 1 is the "universal, portable"
minimum. Models outside Tier 0/1 must either fall back to CPU
reference or be explicitly rejected with a clear error.

## Acceptance criteria for "universal, portable"

Runtime may claim universality when:

1. Any Tier 0 model imports without manual intervention (one command)
2. Any Tier 0 model produces correct output (verified against llama.cpp golden values per test.md)
3. Any Tier 1 model produces correct output
4. Out-of-scope models emit a clear, actionable error ("unsupported:
   sparse MoE layer in layers.0.mlp.experts") — not silent corruption
5. Runtime passes test suite on all supported backends:
   wgpu, metal (macOS), cpu (any OS)
6. Speed is within 2x of llama.cpp on any Tier 0 model

Today we fail #2 (forward pass produces garbage for Qwen3 despite
correct weights). Fix must come from spec-driven correctness, not
symptom-chasing.

## Decision log

- 2026-04-17: Tier 0 = dense decoder-only LLMs with Llama-family
  features + QK-norm. Covers ~80% of HF. Rationale: smallest coherent
  scope that's useful alone.
- 2026-04-17: Gemma 3/4 moved to Tier 1. Rationale: adds three
  architectural features (softcapping, sliding window, K=V) that
  all need spec + dispatch, cannot be bolted on.
- 2026-04-17: MoE explicitly out of Tier 0. Rationale: experts routing
  is orthogonal to base transformer and would balloon scope.
