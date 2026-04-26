# Architecture templates

Each curated family is one graph template. Models within a family
share the template; their `config` section parameterizes it.
Adding a new model to an existing family requires zero code change
— only a config update and sometimes a tensor-naming entry in
[import.md](../../import/specs/import.md).

## LlamaStyle

Covers: Llama 2/3, Mistral, Qwen 2/2.5/3, Phi 2/3/4, Gemma 1/2,
SmolLM/SmolLM 2, DeepSeek-LLM dense, StarCoder 2, MiMo, NuExtract,
Yi.

### Template

```
tokens = tokenizer.encode(input)
h = embed[tokens]                         # [seq, hidden]

for layer in 0..num_hidden_layers:
    # Attention
    norm1 = RmsNorm(h, layer.input_layernorm.weight, eps)
    q = norm1 @ layer.self_attn.q_proj.weight^T   # [seq, num_heads * head_dim]
    k = norm1 @ layer.self_attn.k_proj.weight^T   # [seq, kv_heads * head_dim]
    v = norm1 @ layer.self_attn.v_proj.weight^T   # [seq, kv_heads * head_dim]

    if has_attention_bias:               # Qwen2
        q += layer.self_attn.q_proj.bias
        k += layer.self_attn.k_proj.bias
        v += layer.self_attn.v_proj.bias

    # reshape: q → [seq, num_heads, head_dim], k/v → [seq, kv_heads, head_dim]

    if has_qk_norm:                       # Qwen3, DeepSeek-V3
        q = RmsNorm(q, layer.self_attn.q_norm.weight, eps)  # per-head
        k = RmsNorm(k, layer.self_attn.k_norm.weight, eps)

    q = Rope(q, pos, head_dim, rope_theta)
    k = Rope(k, pos, head_dim, rope_theta)

    # KV cache append, get full cache up to current position
    full_k, full_v = KvCache.append(k, v)

    attn_out = Sdpa(q, full_k, full_v, num_heads, kv_heads, head_dim, causal=true)
    # [seq, num_heads * head_dim]

    attn_out = attn_out @ layer.self_attn.o_proj.weight^T

    h = h + attn_out                      # residual

    # FFN
    norm2 = RmsNorm(h, layer.post_attention_layernorm.weight, eps)
    gate = norm2 @ layer.mlp.gate_proj.weight^T
    up   = norm2 @ layer.mlp.up_proj.weight^T
    ffn  = (Silu(gate) ⊙ up) @ layer.mlp.down_proj.weight^T

    h = h + ffn                           # residual

h = RmsNorm(h, model.norm.weight, eps)
logits = h[-1] @ lm_head.weight^T         # decode: last token only
# lm_head.weight may be tied to embed_tokens.weight

token = Sample(logits, sampling_method)
```

### Config

```toml
[architecture]
hidden_size = 1024                        # or model-specific
num_attention_heads = 16
num_key_value_heads = 8                   # GQA; = num_heads for MHA
num_hidden_layers = 28
intermediate_size = 3072                  # FFN middle dim
vocab_size = 151936
max_position_embeddings = 40960
rope_theta = 1000000                      # Qwen2 has 10000, Qwen3 has 1000000
rms_norm_eps = 1e-6
tie_word_embeddings = true                # small models tie; large models don't

[tokenizer]
eos_token_ids = [151645, 151643]
```

### Variant flags

- `has_attention_bias`: detected from presence of `q_proj.bias` tensors
  (Qwen2 has them; Llama, Mistral don't).
- `has_qk_norm`: detected from presence of `q_norm.weight` tensors
  (Qwen3, DeepSeek-V3 have them).

## LlamaStyle+ (Gemma 3/4 extensions)

Adds to LlamaStyle:

### Embedding scale (Gemma 1/2/3/4)

```
h = embed[tokens] * sqrt(hidden_size)
```

Applied on every Gemma generation. Llama/Qwen/Mistral do not scale.
Controlled by `FamilyProfile::scaled_embeddings`.

### RMSNorm weight encoding (Gemma 2/3)

Gemma 2/3 store norm weights as offsets from 1 (`w_stored = w_true − 1`). At
load time each norm tensor is adjusted to `w_true = w_stored + 1` so the
runtime uses the standard `w * x / rms` formula unchanged. Controlled by
`FamilyProfile::rmsnorm_plus_one`. Gemma 4 uses standard encoding.

### Pre-residual norms on attention and FFN output (Gemma 2/3/4)

LlamaStyle has bare residuals. Gemma 2/3/4 apply an extra RMSNorm to each
sublayer output *before* adding to the residual stream:

```
# After attention output projection:
attn_out = RmsNorm(attn_out, layer.post_attention_norm.weight, eps)
h = h + attn_out

# After FFN down projection:
ffn_out = RmsNorm(ffn_out, layer.post_ffw_norm.weight, eps)
h = h + ffn_out
```

Both are optional tensors — present/absent determined by tensor existence.

### V-norm per head (Gemma 4)

Before writing V to the KV cache, each head vector is divided by its own RMS
(no learned scale — pure RMS normalise):

```
for h in 0..kv_heads:
    v[h] = v[h] / sqrt(mean(v[h]²) + eps)
```

Applied on every layer kind (sliding and full alike). Controlled by
`FamilyProfile::v_norm_per_head`.

### Layer residual scalar (Gemma 4)

After both residual adds within a layer:

```
h = (h + ffn_out) * layer_output_scale     # layer_output_scale is a scalar [1] tensor
```

`layer_output_scale` is trained per-layer (HF: `self.layer_scalar`). Without it,
activations grow without bound through the residual stream.

### Sliding window attention

Some layers use full attention, others restrict to a window of
recent tokens:

```
if layer.attention_type == "sliding":
    mask[i, j] = -inf if (j < i - window_size) else 0
    # in addition to causal mask
```

Layer type pattern is per-layer, encoded in `config.layer_types`. Two
layer kinds: `sliding` and `full`. Window size in `config.sliding_window`
(typical: 1024).

### Per-layer attention dimension switching (Gemma 4)

Gemma 4 differs from Gemma 3: full-attention layers use a different
head dimension and KV-head count than sliding-attention layers.

| Layer kind | head_dim source | kv_heads source |
|---|---|---|
| `sliding` | `config.head_dim` | `config.num_key_value_heads` |
| `full` | `config.global_head_dim` | `config.num_global_key_value_heads` |

The number of query heads (`num_attention_heads`) is constant across layers.
For Gemma-4-31b: 32 q-heads everywhere; sliding layers use 256 head_dim ×
16 kv_heads, full layers use 512 head_dim × 4 kv_heads. Q/K/V projection
shapes follow per-layer dims, so weight loading and forward both branch
on `layer_types[i]`.

Gemma 3 omits the `global_*` fields — every layer uses one shape.

### Per-layer RoPE (Gemma 4)

Full-attention and sliding-attention layers use independent RoPE
configurations:

| Layer kind | rope_theta | rope_dim |
|---|---|---|
| `sliding` | `config.rope_theta` | full `head_dim` |
| `full` | `config.rope_theta_full` | `head_dim × config.partial_rotary_factor_full` |

Gemma-4-31b: sliding uses θ=10⁴ over the full 256-dim head; full uses θ=10⁶
over the first 128 of 512 dims (`partial_rotary_factor=0.25`), with the
remaining 384 dims passed through unrotated. Wrong RoPE on either kind
yields fluent-but-incoherent output (model emits next tokens but loses
coherence within one full layer's attention).

`Op::Rope` carries a `rope_dim` field for partial rotary; defaults to
`head_dim` so non-Gemma-4 callers stay unchanged.

### GELU activation instead of SiLU

```
ffn = (Gelu_tanh(gate) ⊙ up) @ W_down^T
```

Selected by `config.hidden_activation`. Canonical names:

| Name | Op |
|---|---|
| `silu` (default) | `Silu` |
| `gelu_pytorch_tanh` | `Gelu { approximate: true }` |
| `gelu` | `Gelu { approximate: false }` |

### Final logit softcapping

```
logits = tanh(logits / cap) * cap        # cap = config.final_logit_softcapping
```

Clamps logits to [-cap, cap], prevents runaway softmax. Applied once
after the LM head, before sampling. Skip when the field is absent or 0.

### Attention K=V shared projection

When `config.attention_k_eq_v` is true, the K and V projections share
the same weights. The runtime sees two tensors named `k_proj.weight` and
`v_proj.weight` with identical bytes — the `import` crate is responsible
for splitting any fused `kv_proj` source into the two canonical names so
the runtime stays one codepath.

The shared-bytes case still uses GQA shapes: `kv_proj` ≡ `k_proj` ≡
`v_proj` with shape `[kv_dim, hidden]`, where `kv_dim` follows the
per-layer rule above.

## MoEStyle

Covers: Mixtral, DeepSeek-V2/V3, Qwen-MoE.

Template = LlamaStyle, but FFN is:

```
router_logits = norm2 @ layer.mlp.gate.weight^T        # [seq, num_experts]
topk_logits, topk_idx = TopK(router_logits, top_k)      # [seq, top_k]
weights = Softmax(topk_logits)

ffn = zeros_like(norm2)
for i in 0..top_k:
    expert_idx = topk_idx[:, i]
    expert_out = per-token dispatch to:
        W_gate = experts[expert_idx].gate_proj.weight
        W_up   = experts[expert_idx].up_proj.weight
        W_down = experts[expert_idx].down_proj.weight
        SwiGlu(norm2, W_gate, W_up, W_down)
    ffn += weights[:, i] * expert_out
```

Additional primitive needed: `RoutedMatmul` or equivalent for sparse
expert dispatch. Until added, MoE models run in graph executor path
using elementwise dispatch (slow but correct).

## BertStyle

Covers: BERT, RoBERTa, DeBERTa v2/v3, ModernBERT, Jina, e5, bge.

### Template

```
tokens = tokenizer.encode(input)
h = embeddings.word_embeddings[tokens]                 # [seq, hidden]
h += embeddings.position_embeddings[positions]          # learned, absolute
if has_token_type:
    h += embeddings.token_type_embeddings[segment_ids]
h = LayerNorm(h, embeddings.LayerNorm.weight, embeddings.LayerNorm.bias, eps)

for layer in 0..num_hidden_layers:
    # Attention (bidirectional, no causal mask)
    q = h @ attention.self.query.weight^T + attention.self.query.bias
    k = h @ attention.self.key.weight^T + attention.self.key.bias
    v = h @ attention.self.value.weight^T + attention.self.value.bias

    attn_out = Sdpa(q, k, v, num_heads, num_heads, head_dim, causal=false)
    attn_out = attn_out @ attention.output.dense.weight^T + attention.output.dense.bias

    h = LayerNorm(h + attn_out,
                  attention.output.LayerNorm.weight,
                  attention.output.LayerNorm.bias,
                  eps)                                   # post-norm

    # FFN (GELU-MLP, not SwiGLU)
    ff = Gelu_erf(h @ intermediate.dense.weight^T + intermediate.dense.bias)
    ff = ff @ output.dense.weight^T + output.dense.bias

    h = LayerNorm(h + ff, output.LayerNorm.weight, output.LayerNorm.bias, eps)

# Pooling (task-dependent)
if task == "classification":
    pooled = pooler(h[:, 0])                            # CLS token
    logits = pooled @ classifier.weight^T + classifier.bias
elif task == "mean-embedding":
    pooled = mean(h, dim=seq)
elif task == "masked-lm":
    logits = h @ cls.predictions.decoder.weight^T
```

### Config

```toml
[architecture]
hidden_size = 768
num_attention_heads = 12
num_hidden_layers = 12
intermediate_size = 3072
vocab_size = 30522
max_position_embeddings = 512
type_vocab_size = 2                                     # token types, 0 if absent
layer_norm_eps = 1e-12                                   # note: Layer, not RMS
position_embedding_type = "absolute"                     # or "relative"
```

### Variants

- **DeBERTa-v2/v3**: relative position bias instead of absolute.
  Attention uses `RelativePosEmbedding` op added to scores.
- **ModernBERT**: uses RoPE + RMSNorm (looks more like LlamaStyle).
  May dispatch to LlamaStyle variant instead of BertStyle.
- **Jina, e5, bge**: standard BERT for embeddings; mean-pool output.

## T5Style

Covers: T5, FlanT5, mT5, BART, mBART, Marian, M2M.

Encoder + decoder with cross-attention. RelativePosEmbedding in
encoder attention. LayerNorm is applied in pre-norm fashion (vs BERT
post-norm).

```
# Encoder
h_enc = embed[input_tokens]
for layer in encoder_layers:
    h_enc = h_enc + SelfAttention(LayerNorm(h_enc), relative_bias)
    h_enc = h_enc + FFN(LayerNorm(h_enc))

# Decoder (autoregressive with cross-attn)
h_dec = embed[decoder_tokens]
for layer in decoder_layers:
    h_dec = h_dec + SelfAttention(LayerNorm(h_dec), relative_bias, causal=true)
    h_dec = h_dec + CrossAttention(LayerNorm(h_dec), h_enc)
    h_dec = h_dec + FFN(LayerNorm(h_dec))
```

## WhisperStyle

Covers: Whisper tiny/base/small/medium/large/v3.

Input is Mel spectrogram, not tokens:

```
# Preprocessing
mel = audio_to_mel(audio_samples, n_mels=80 or 128)     # [n_mels, n_frames]

# Conv stem
h = Conv1d(mel, filter_width=3, stride=1) + GELU
h = Conv1d(h,   filter_width=3, stride=2) + GELU        # downsamples
h = h + sinusoidal_position_embedding

# Encoder: self-attention only, bidirectional
for layer in encoder_layers:
    h = h + SelfAttention(LayerNorm(h))
    h = h + FFN(LayerNorm(h))
h_enc = LayerNorm(h)

# Decoder: causal + cross-attn to h_enc
h_dec = token_embed[decoder_tokens] + position_embed[pos]
for layer in decoder_layers:
    h_dec = h_dec + SelfAttention(LayerNorm(h_dec), causal=true)
    h_dec = h_dec + CrossAttention(LayerNorm(h_dec), h_enc)
    h_dec = h_dec + FFN(LayerNorm(h_dec))

logits = h_dec @ token_embed^T                          # tied
```

## ViTStyle

Covers: ViT, DeiT, CLIP vision, SigLIP vision, DINO, PaliGemma vision.

```
# Patch embed: Conv2d with kernel=stride=patch_size
patches = Conv2d(image, kernel=patch_size, stride=patch_size)
          # [B, hidden, H/p, W/p]
patches = flatten and transpose to [B, num_patches, hidden]

h = concat([cls_token, patches]) + position_embed       # learned abs

for layer in encoder_layers:
    h = h + SelfAttention(LayerNorm(h))                 # pre-norm or post-norm
    h = h + FFN(LayerNorm(h))

h = LayerNorm(h)

if task == "classification":
    logits = h[:, 0] @ head.weight^T                    # CLS
else:
    features = h
```

## CNNStyle

Covers: YOLO, ResNet, ESRGAN, RealESRGAN, SwinIR.

Pure convolutional. Structure is model-specific; no attention
(except SwinIR which has windowed attention; dispatch to hybrid).

```
h = Conv2d + BatchNorm + ReLU        # stem
for block in backbone:
    residual = h
    h = block(h)
    h = h + residual
h = head(h)                           # model-specific: bbox, class, etc.
```

## UNetDiffusion

Covers: Stable Diffusion 1.5, 2, XL, 3.

```
# Timestep embed
t_emb = SinusoidalEmbed(t, embed_dim)
t_emb = MLP(t_emb)

# Text embed (from CLIP/T5, preprocessed)
# Input: latent from VAE, [B, 4, H/8, W/8]

# Encoder (downsampling)
skips = []
for down_block in encoder:
    h = ResBlock(h, t_emb)               # GroupNorm + SiLU + Conv2d + skip
    if has_cross_attn:
        h = h + CrossAttn(LayerNorm(h), text_emb)
    skips.push(h)
    h = Downsample(h)

# Middle
h = ResBlock + CrossAttn + ResBlock

# Decoder (upsampling with skips)
for up_block in decoder:
    h = Upsample(h)
    h = concat(h, skips.pop())
    h = ResBlock(h, t_emb)
    if has_cross_attn:
        h = h + CrossAttn(LayerNorm(h), text_emb)

out = Conv2d(GroupNorm(h))               # predicted noise
```

ResBlock internals:
```
h = Conv2d(Silu(GroupNorm(x)))
h = h + Linear(Silu(t_emb))              # time conditioning
h = Conv2d(Silu(GroupNorm(h)))
h = h + x                                # residual
```

## DiTDiffusion

Covers: Flux, SD3-medium, Hunyuan-Video, Mochi, Wan 2.2, LTX.

```
# Patchify
patches = PatchEmbed(latent, patch_size)  # or Conv3d for video

# Timestep + text conditioning produces per-layer modulation
t_emb = SinusoidalEmbed(t) + text_emb_pooled
mods = [MLP_per_layer(t_emb) for layer in blocks]  # shift, scale, gate

h = patches
for i, block in enumerate(blocks):
    shift, scale, gate = mods[i]
    # AdaLN-modulated attention
    h = h + gate * SelfAttention(AdaLN(h, scale, shift))
    h = h + gate * FFN(AdaLN(h, scale, shift))

h = FinalLayer(h, mods[-1])
out = Unpatchify(h)
```

## TTSStyle

Covers: XTTS v2, Piper, VITS, MeloTTS.

Three-stage pipeline:

```
# 1. Text encoder: transformer or conv
text_emb = text_encoder(text_tokens)

# 2. Duration + pitch predictor
durations = DurationPredictor(text_emb)
pitch = PitchPredictor(text_emb)

# 3. Flow (normalizing flow, invertible)
z = Flow(latent_noise, conditioned_on=text_emb, durations)

# 4. Vocoder (mel → audio via HiFi-GAN style)
mel = z_to_mel(z)
audio = Vocoder(mel)   # ConvTranspose1d stacks with multi-scale discriminators
```

Flow primitive: `FlowStep` — coupling layer, invertible.
Vocoder: Conv1d + ConvTranspose1d + LeakyReLU chains.

## Dispatch priority

When multiple templates could apply (e.g. ModernBERT looks like
both BertStyle and LlamaStyle), dispatch priority is:

1. Exact `model_type` match in config
2. Architecture-specific tensor presence (e.g. `q_norm.weight` → QK-norm variant)
3. Fallback to graph executor

Ambiguity at this layer is a config/import bug — [import.md](../../import/specs/import.md)
must disambiguate during import.
