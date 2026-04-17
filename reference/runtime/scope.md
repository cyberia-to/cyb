# Scope

What models cyb-llm runtime runs. Separate from *how* it runs them
(that is [architecture.md](architecture.md)).

## Principle

A runtime "for intelligent beings" needs full modality coverage:
language, vision, audio, speech, generation. Chat alone is a tiny slice.

Scope is the commitment: every modality below has at least one model
running end-to-end with correct output. Growth is adding model
families within these modalities, or adding a modality when first
consumer lands.

## Modality coverage (v1)

| Modality | Example models | Execution path |
|---|---|---|
| Text generation (LLM) | Llama 3, Qwen 3, Mistral, Phi 4, Gemma 2 | Curated: LlamaStyle |
| Text generation (MoE) | Mixtral, DeepSeek-V3, Qwen-MoE | Curated: MoEStyle |
| Text encoder | BERT, DeBERTa, ModernBERT | Curated: BertStyle |
| Text embedding | Jina, e5, bge | Curated: BertStyle |
| Seq2seq | T5, BART | Curated: T5Style |
| Speech recognition | Whisper | Curated: WhisperStyle |
| Text-to-speech | XTTS, Piper, VITS | Curated: TTSStyle |
| Vision (transformer) | ViT, CLIP, SigLIP | Curated: ViTStyle |
| Vision (CNN) | YOLO, ResNet, ESRGAN | Curated: CNNStyle |
| Image generation (UNet) | Stable Diffusion 1.5/XL/3 | Curated: UNetDiffusion |
| Image generation (DiT) | Flux, SD3-medium | Curated: DiTDiffusion |
| Video generation | Hunyuan, Mochi, Wan | Curated: DiTDiffusion |
| Audio generation | Stable Audio | Curated: DiTDiffusion |
| Multimodal (VL) | LLaVA, Qwen-VL, Moondream, PaliGemma | Curated: hybrid (ViT + LLM) |
| Research / new | anything expressible as IR graph | Graph executor |

Graph executor is the catch-all. If a model's computation graph uses
supported IR ops, it runs correctly on graph path regardless of
whether a curated codepath exists. Research models, novel
architectures, one-off experiments all work — just slower.

## Model families (curated)

Each family is one hand-written codepath. New model within the family
typically requires zero code change — just config parsing.

### LlamaStyle (pre-norm RMSNorm + GQA + RoPE + SwiGLU)

Llama 2, Llama 3, Mistral, Qwen 2, Qwen 2.5, Qwen 3, Phi 2/3/4,
Gemma 1, Gemma 2, SmolLM, SmolLM 2, DeepSeek-LLM, StarCoder 2, MiMo,
NuExtract, Yi.

Variants handled by config:
- Optional attention biases on Q/K/V (Qwen2)
- Optional per-head q_norm/k_norm (Qwen3, DeepSeek-V3)
- Tied vs untied word embeddings
- RoPE theta, head_dim, num_heads, num_kv_heads per config

Gemma 3/4 extends this with: sliding window alternating layers,
GELU activation, final logit softcapping, K=V shared projections.
These add variant flags to LlamaStyle, not a new family.

### MoEStyle (LlamaStyle + routed FFN)

Mixtral 8x7B/8x22B, DeepSeek-V2/V3 (256 experts), Qwen-MoE.
Adds `RoutedMatmul` primitive (experts selection + sparse dispatch).

### BertStyle (bidirectional + learned position + CLS)

BERT, RoBERTa, DeBERTa v2/v3, ModernBERT, Jina v2/v3, e5, bge.
Classification head, MLM head, sentence embeddings.

### T5Style (encoder + decoder with cross-attn, relative position bias)

T5, FlanT5, mT5, BART, mBART, Marian, M2M.
Relative position bias primitive, encoder-decoder orchestration.

### WhisperStyle (conv stem + encoder + causal decoder with cross-attn)

Whisper tiny/base/small/medium/large/v3.
Mel spectrogram input, conv1d stem, bidirectional encoder, causal
decoder attending to encoder output.

### ViTStyle (patch embed + transformer + pool)

ViT, DeiT, CLIP vision, SigLIP vision, DINO, PaliGemma vision tower.
Patch embed = Conv2d with kernel=stride=patch_size. Rest is
transformer encoder.

### CNNStyle (Conv2d + BatchNorm + Pool)

YOLO v5/v8/v10, ResNet 50/101, ESRGAN, RealESRGAN, SwinIR.
Pure convolutional networks, no attention.

### UNetDiffusion (Conv2d + ResBlocks + cross-attn + timestep)

Stable Diffusion 1.5, 2, XL, 3. Latent diffusion with VAE.
ResBlock = GroupNorm + SiLU + Conv2d + skip. Cross-attn from prompt.

### DiTDiffusion (PatchEmbed + AdaLN + SDPA + MLP)

Flux, SD3-medium, Hunyuan-Video, Mochi, Wan 2.2, LTX.
Diffusion transformer over patches. Video = 3D patches.
Adaptive layer norm modulated by timestep + text embedding.

### TTSStyle (text encoder + flow + vocoder)

XTTS v2, Piper, VITS, MeloTTS, Parler-TTS.
Text-to-mel via transformer + normalizing flow; mel-to-audio via
HiFi-GAN-style vocoder with transposed convolutions.

## Weight formats

Every family accepts any per-tensor mix of:

- F32, F16, BF16 (disk or GPU)
- Q8_0 (8-bit symmetric, block 32)
- Q4_0 (4-bit symmetric, block 32)
- Q4_K, Q5_K, Q6_K (K-quant superblocks of 256)
- Q3_K, Q2_K (low-bit K-quants)
- Ternary (BitNet 1.58-bit)

K-quant preferred for new imports. Q4_0 kept for backwards compat.

## Backends

| Backend | Target | When available |
|---|---|---|
| cpu | reference, slow, always correct | v1 |
| wgpu | portable GPU (Windows, Linux, Android, Web) | v1 |
| metal | Apple GPU with aruminium zero-copy | v1 |
| ane | Apple Neural Engine via MIL | v2 |
| cuda | NVIDIA | future |

A model runs on a backend if the backend implements every op the
model's graph uses. Missing op falls back to CPU for that op
(see [architecture.md](architecture.md)).

## Out of scope (v1)

Explicitly rejected, with rationale:

- **Training / fine-tuning.** Different system — gradients, optimizer,
  data pipeline. Use PyTorch / HF trainer.
- **Distributed execution.** Single-node focus. Multi-GPU later if
  flagship models require it.
- **Continuous batching.** Scheduler concern, not runtime. Added
  when serve-at-scale becomes relevant.
- **Custom per-model kernels.** If a paper ships a new op that can't
  compose from existing IR, we reject until the primitive is added
  through the IR versioning process.

## Acceptance criteria for v1

Runtime may claim "full spectrum" when:

1. At least one model from each modality row runs correctly end-to-end
   with golden test values matching F32 reference within dtype tolerance.
2. Every curated family has at least one model in v1 test suite,
   tested on every backend.
3. Graph path runs at least one model NOT covered by curated
   (proves fallback works).
4. Any supported model imports in one command (`cyb-llm fetch MODEL`),
   runs in one command (`cyb-llm run MODEL "prompt"`).
5. Any unsupported model produces a clear, actionable error — never
   silent corruption.

Today we fail on #2 (Qwen3 generates garbage despite correct weights).
Proof: [test_ollama_gguf_direct](../../llm/src/backend/wgpu/model.rs)
loads the same abliterated model ollama runs correctly and produces
garbage. Fix must come from spec-driven correctness rework.

## Versioning

Scope is versioned with the runtime. Adding a modality or model
family is a minor version bump. Removing one is a breaking change
(major). Spec and runtime move together — no drift.

## Decision log

- 2026-04-17: Modality-based scope replaces the "tiered feature"
  scope. We commit to text, encoders, seq2seq, ASR, TTS, vision,
  CNN, diffusion (UNet + DiT), video, multimodal. No tier is "future
  only" — every modality has at least one runner in v1.
- 2026-04-17: Graph executor is the safety net for anything not
  covered by curated families. It's not "legacy" — it's the
  universality promise.
- 2026-04-17: Training out of scope. cyb-llm is inference-only.
