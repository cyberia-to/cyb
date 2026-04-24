# Operations

Math definitions for every op in the runtime. Every backend
implements the math below within its dtype tolerance. The CPU
reference library in wgpu+rs implements all of them in f32 — that's
the correctness authority.

Notation: `x`, `y`, tensors. `x[i]`, element access. `⊙`, element-wise
multiply. `@`, matrix multiply. `W`, weight.

## 1. Linear algebra

### Matmul

```
Matmul(x, W) := x @ W^T
  x: shape [..., K]
  W: shape [N, K]
  y: shape [..., N]
  y[..., i] = sum over k=0..K of x[..., k] * W[i, k]
```

All higher-level ops (attention, FFN) reduce to Matmul + elementwise.

### Add, Mul, Sub, Div

Elementwise with broadcasting ([tensor.md](tensor.md#broadcasting)).

### Transpose, Permute, Reshape

Logical rearrangement. Output is a view (same data, new shape/stride)
when possible, otherwise allocates and copies.

### Concat, Split, Chunk

`Concat` glues along one axis. `Split` with explicit sizes, `Chunk`
with equal parts. Shape must match on all other axes.

### Clamp, NanToNum

Numerical stability.
```
Clamp(x, lo, hi): y[i] = min(max(x[i], lo), hi)
NanToNum(x, nan=0, posinf=F32_MAX, neginf=F32_MIN):
  y[i] = x[i] if finite else replacement
```

### Argmax

Index of maximum along axis. Used in greedy decoding.

## 2. Normalization

### RmsNorm

Root-Mean-Square norm, Llama-family.

```
RmsNorm(x, g, ε):
  # x: [..., D], g: [D] (learned gain), ε: small scalar
  rms = sqrt(mean(x^2) + ε)             # mean over last dim only
  y = (x / rms) ⊙ g
```

**Critical:** ε is added to the mean of squares, **before** the square
root. Common bug: adding ε after sqrt (different numerical behavior
for small x).

Tolerance: F32 1e-6, F16 1e-3.

### LayerNorm

Standard layer normalization.

```
LayerNorm(x, g, b, ε):
  # x: [..., D], g: [D] gain, b: [D] bias
  μ = mean(x)
  σ² = mean((x - μ)²)
  y = (x - μ) / sqrt(σ² + ε) ⊙ g + b
```

### BatchNorm, GroupNorm, InstanceNorm

Same structure as LayerNorm, different reduction axis set:

- **BatchNorm**: reduce over [B, ..., spatial], per-channel
- **GroupNorm**: reduce over group of channels + spatial
- **InstanceNorm**: reduce over spatial only, per [B, C]

### AdaLN

Adaptive layer norm, DiT family. Scale and shift are modulated by
an external conditioning signal:

```
AdaLN(x, scale, shift, ε):
  # Scale and shift are produced by a separate MLP from timestep/text
  y_norm = LayerNorm(x, 1, 0, ε)         # no learned g/b
  y = y_norm ⊙ (1 + scale) + shift
```

Variant `AdaLN-Zero`: the conditioning is initialized to produce
zero output, i.e. y = x + residual · gate.

## 3. Position encoding

### Rope (Rotary Position Embedding)

Standard NeoX-style pairing (first half of head_dim with second half):

```
Rope(x, pos, head_dim, base):
  # x: [..., num_heads, head_dim]
  # pos: current sequence position(s)
  # base: rope_theta (typically 10000 or 1000000)
  # head_dim must be even; if odd, validation fails at load time
  half = head_dim / 2
  for j in 0..half:
    θ = pos / base^(2*j / head_dim)
    c, s = cos(θ), sin(θ)
    x1, x2 = x[..., j], x[..., j + half]
    y[..., j]       = x1 * c - x2 * s
    y[..., j+half]  = x1 * s + x2 * c
```

Alternative pairing (standard, not NeoX): consecutive pairs
`(x[2j], x[2j+1])`. Choice is per-model; Qwen/Llama use NeoX. Set by
the architecture template ([arch.md](arch.md)). Families document
which pairing they use.

Cos/sin cache: precompute `cos[pos, j]` and `sin[pos, j]` for all
positions up to max_seq. Per-model `base` (rope_theta) parameter.

**Edge cases:**
- `head_dim` must be even — validated at load, error if odd.
- `pos=0` produces `θ=0 → cos=1, sin=0` → identity rotation. Correct.
- `pos > max_position_embeddings` is an implementation choice:
  extrapolate cos/sin formula (may produce wrong results) OR error.
  Spec: error with `ContextOverflowError`.
- `base < 1.0` or `base > 1e9` → warn but permit; extreme values
  may produce numerical issues.

**Multi-axis RoPE (3D for DiT video):**

For video/image DiT, position is a vector `[t, h, w]`. head_dim is
split into sub-ranges per axis:

```
# dim_per_axis: e.g. [t_dim, h_dim, w_dim] summing to head_dim
# All must be even.
for axis, (pos_axis, dim_axis) in enumerate(zip(pos_vec, dims_per_axis)):
    offset = sum(dims_per_axis[0..axis])
    Rope_on_slice(x[..., offset : offset + dim_axis], pos_axis, dim_axis, base)
```

Each axis gets an independent RoPE over its own sub-range. `base`
may differ per axis (configured per-model).

### SinusoidalEmbed

Diffusion timestep embedding.

```
SinusoidalEmbed(t, dim):
  # t: scalar timestep, dim: embedding dimension
  half = dim / 2
  for j in 0..half:
    freq = exp(-j * log(10000) / half)
    y[2j]     = sin(t * freq)
    y[2j + 1] = cos(t * freq)
```

### RelativePosEmbedding

T5-style learned relative position bias. Adds to attention scores.

### PosEmbed, TokenEmbed

Lookup from a learned embedding table. `y = W[id]`.

## 4. Activation

### Silu (Swish-1)

```
Silu(x) := x * sigmoid(x) = x / (1 + exp(-x))
```

### Gelu

Two variants. Models specify which.

```
Gelu_erf(x)  := x * 0.5 * (1 + erf(x / sqrt(2)))       # exact
Gelu_tanh(x) := 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))
```

BERT-family uses `Gelu_erf`. GPT-2, Gemma use `Gelu_tanh`. Spec per-model.

### Relu, LeakyRelu, PRelu, Sigmoid, Tanh

Standard element-wise.

### Softmax

Numerically stable (subtract max):
```
Softmax(x, dim):
  m = max(x, dim)
  e = exp(x - m)
  y = e / sum(e, dim)
```

Without the max subtraction, large x produces Inf/NaN.

### SwiGlu

Gated feed-forward. Llama/Qwen/Mistral FFN.

```
SwiGlu(x, W_gate, W_up, W_down):
  gate = x @ W_gate^T
  up   = x @ W_up^T
  y    = (Silu(gate) ⊙ up) @ W_down^T
```

### GeGlu

GELU-gated variant (some encoder-decoder models).

```
GeGlu(x, W_gate, W_up, W_down):
  gate = x @ W_gate^T
  up   = x @ W_up^T
  y    = (Gelu(gate) ⊙ up) @ W_down^T
```

### Glu

Sigmoid-gated (Stable Audio Conformer and similar).

## 5. Attention

### Sdpa (Scaled Dot-Product Attention)

Standard causal or non-causal attention, possibly with Grouped Query
Attention (GQA). Optional additive mask input.

```
Sdpa(Q, K, V, num_heads, kv_heads, head_dim, causal, mask=None):
  # Q: [B, num_heads, Sq, head_dim]
  # K, V: [B, kv_heads, Sk, head_dim]
  # mask (optional): [B, 1, Sq, Sk] or [Sq, Sk] additive f32 mask
  # if kv_heads < num_heads, expand K, V (see GQA below)
  scale = 1 / sqrt(head_dim)
  scores = Q @ K^T * scale              # [B, num_heads, Sq, Sk]
  if causal:
    scores[..., i, j] += causal_mask[i, j]
  if mask is not None:
    scores += broadcast(mask, [B, num_heads, Sq, Sk])
  probs = Softmax(scores, dim=-1)
  y = probs @ V                         # [B, num_heads, Sq, head_dim]
```

**Scale is divided, not multiplied.** Some implementations bake it
into Q; equivalent but spec here uses explicit scale.

**Causal mask value:** use `-1e4` (F16-safe large negative),
NOT `-inf`. Reasons:
- `-inf` in F16 softmax can produce NaN if a whole row is masked
  (all rows should have at least one unmasked entry, but defensively
  `-1e4` + at least one `0` survives)
- F16 overflow during intermediate accumulation is avoided
- After softmax, `exp(-1e4) ≈ 0` to F16 precision — effectively the
  same as `-inf`

**Mask shapes:** attention supports three mask patterns:
- **Causal** (`causal=true`): implicit lower-triangular mask added
  inside the kernel, no input needed
- **Padding mask**: `[B, 1, 1, Sk]` additive, -1e4 for padded positions
- **Generic**: `[B, num_heads, Sq, Sk]` additive

Shape `[Sq, Sk]` or `[1, Sq, Sk]` etc. broadcast along missing dims.

### GQA expansion (when num_heads > kv_heads)

When `num_heads > kv_heads`, each KV head is shared by
`repeat = num_heads / kv_heads` Q heads. Expansion is logical
(no copy):

```
# K has shape [B, kv_heads, Sk, head_dim]
# Expand to [B, num_heads, Sk, head_dim] via repeat_interleave:
K_expanded[b, h, s, d] = K[b, h / repeat, s, d]
# Where h / repeat is integer division.
```

**Must be `repeat_interleave` (groups of `repeat` consecutive Q
heads share one KV head), NOT `tile` (strided sharing).**

Backends may implement expansion as a virtual view (no memory copy)
or as a physical expand. Output must match.

`num_heads % kv_heads == 0` is required; non-integer ratio is invalid.

### SdpaCross

Cross attention: Q from decoder, K/V from encoder.

```
SdpaCross(Q_dec, K_enc, V_enc, num_heads, head_dim):
  # Q: [B, num_heads, Sq, head_dim]
  # K,V: [B, num_heads, Se, head_dim]     (encoder output)
  # No causal mask.
  scale = 1 / sqrt(head_dim)
  probs = Softmax(Q @ K^T * scale, dim=-1)
  y = probs @ V
```

### SdpaWindow

Windowed attention (Swin, Mamba-2 attention step).

Each query attends only to keys within a local window. Implementation
reshapes [Sq] into [num_windows, window_size] and runs attention
inside each window.

### FlashAttention

Same math as Sdpa, different memory access pattern (tiled, avoids
materializing full [Sq, Sk] score matrix). Output must match Sdpa
within ε (verification requirement). For decode (Sq=1), FlashAttention
is equivalent to Sdpa.

### KvCache

Stateful append. One cache per (conversation, layer).

Data structure:
```rust
pub struct KvCache {
    /// [num_layers] — one entry per transformer layer
    pub layers: Vec<LayerKvCache>,
    /// Current position (next write offset). Shared across layers.
    pub past_seq_len: usize,
    /// Maximum sequence length this cache supports.
    pub max_seq: usize,
}

pub struct LayerKvCache {
    /// Shape [kv_heads, max_seq, head_dim] — row-major, contiguous.
    /// Positions 0..past_seq_len are valid; past_seq_len..max_seq are
    /// uninitialized (not read by attention).
    pub k: Tensor,
    pub v: Tensor,
}
```

Append op:
```
KvCache.append(layer_idx, K_new, V_new):
    # K_new, V_new: [kv_heads, s, head_dim], s = seq_len of this step
    L = self.layers[layer_idx]
    p = self.past_seq_len
    L.k[kv, p:p+s, :] = K_new[kv, :, :]    # write per head
    L.v[kv, p:p+s, :] = V_new[kv, :, :]
    # past_seq_len updated only after ALL layers have appended
    # (single update per forward pass, at end)
```

Read for attention (layer `i`, current step):
```
K_full = self.layers[i].k[:, 0:p+s, :]     # slice, view
V_full = self.layers[i].v[:, 0:p+s, :]
# passed to Sdpa
```

**Lifecycle rules:**

1. Cache is allocated once at first forward call, sized to
   `max_seq = config.max_position_embeddings` (capped to a practical
   limit like 32K to control memory).
2. Each decode step appends `s=1`; prefill step may append any `s`.
3. `past_seq_len` advances by `s` at the **end** of a forward call,
   after all layers appended successfully.
4. `reset_kv_cache()` sets `past_seq_len = 0`. Does NOT zero memory
   (uninitialized positions aren't read).
5. If `past_seq_len + s > max_seq` at the start of a forward,
   `ContextOverflowError` is returned. No silent truncation.

**Memory:** for `hidden=4096, num_layers=32, kv_heads=8,
head_dim=128, max_seq=8192, dtype=F16`, one cache is
`32 × 2 × 8 × 8192 × 128 × 2 bytes ≈ 1 GiB`. Budget accordingly.

### QK-norm (Qwen3, DeepSeek-V3)

Applied BEFORE Rope, inside the attention forward:

```
Q = x @ W_q^T                # [B, Sq, num_heads, head_dim]
K = x @ W_k^T                # [B, Sq, kv_heads, head_dim]
Q = RmsNorm(Q, g_q, ε)       # per-head — gain shape [head_dim]
K = RmsNorm(K, g_k, ε)       # per-head
Q = Rope(Q, pos, head_dim, rope_theta)
K = Rope(K, pos, head_dim, rope_theta)
# then Sdpa(Q, K, V, ...)
```

**Critical:** the RmsNorm is applied **per head** — the reduction is
over head_dim, not over (num_heads × head_dim). Each head's vector
of length head_dim gets normalized independently, then multiplied
element-wise by the gain vector of shape [head_dim].

Tolerance: same as RmsNorm.

## 6. Convolution

### Conv1d, Conv2d, Conv3d

Standard convolution with stride, padding, dilation, groups. For Conv2d:

```
Conv2d(x, W, bias, kernel, stride, padding, dilation, groups):
  # x: [B, C_in, H, W]
  # W: [C_out, C_in/groups, kH, kW]
  # bias: [C_out] or None
  # output: [B, C_out, H', W']
  H' = (H + 2*pH - dH*(kH-1) - 1) / sH + 1
  W' = (W + 2*pW - dW*(kW-1) - 1) / sW + 1
  for c_out in 0..C_out:
    g = c_out / (C_out / groups)              # which group
    for cin in 0..C_in/groups:
      for ki in 0..kH:
        for kj in 0..kW:
          y[b, c_out, i, j] += x[b, g*(C_in/groups) + cin,
                                 i*sH + ki*dH - pH,
                                 j*sW + kj*dW - pW] * W[c_out, cin, ki, kj]
    y[b, c_out, i, j] += bias[c_out]
```

Out-of-bounds reads (from padding): zero (zero-padding). Other
padding modes (replicate, reflect) are separate ops or flags.

`groups` cases:
- `groups = 1`: standard convolution
- `groups = C_in = C_out`: depthwise (each channel independent)
- `groups = k` (intermediate): grouped convolution (ResNeXt-style)

Constraint: `C_in % groups == 0` and `C_out % groups == 0`.

### ConvTranspose2d

Learned upsampling (inverse strides).

### CausalConv1d

Replicate-pad left so output index `t` depends only on input ≤ t.
Video models (Wan, Hunyuan) use this.

### DepthwiseConv

`groups = C_in`. Each input channel has its own filter.

### Pool

Max or average pooling over spatial window.

```
Pool(x, mode, kernel, stride, padding):
  # mode: max | avg
  # x: [B, C, H, W], kernel: (kH, kW), stride: (sH, sW), padding: (pH, pW)
  H' = (H + 2*pH - kH) / sH + 1
  W' = (W + 2*pW - kW) / sW + 1
  y[b, c, i, j] = REDUCE over (ki in 0..kH, kj in 0..kW):
      x[b, c, i*sH + ki - pH, j*sW + kj - pW]
    where REDUCE = max for mode=max, sum/(kH*kW) for mode=avg
  # Out-of-bounds reads (from padding): -inf for max, 0 for avg
```

Defaults: stride = kernel (non-overlapping), padding = 0.

## 7. Spatial

### Interpolate

Nearest/bilinear/area resizing.

### PixelShuffle / PixelUnshuffle

```
PixelShuffle(x, r):
  # [B, C*r^2, H, W] → [B, C, H*r, W*r]
PixelUnshuffle(x, r):
  # [B, C, H*r, W*r] → [B, C*r^2, H, W]
```

### PatchEmbed

Conv2d with kernel=stride=patch_size, mapping image to sequence of
patch embeddings. Used by ViT, DiT.

### Unpatchify

Inverse of PatchEmbed.

## 8. Sampling

### Sample

Unified sampling op accepting method config.

```
Sample(logits, method):
  match method:
    Greedy:
      return argmax(logits)
    Temperature(t):
      probs = softmax(logits / t)
      return sample_categorical(probs)
    TopK(k, t):
      top_values, top_indices = topk(logits, k)
      probs = softmax(top_values / t)
      return top_indices[sample_categorical(probs)]
    TopP(p, t):
      sorted_logits, sorted_idx = sort_desc(logits)
      sorted_probs = softmax(sorted_logits / t)
      cumsum = cumulative_sum(sorted_probs)
      keep_mask = cumsum <= p   # include exactly up to p
      # always keep first token (handles case where top token alone > p)
      keep_mask[0] = true
      filtered_probs = sorted_probs * keep_mask
      filtered_probs /= sum(filtered_probs)   # renormalize
      return sorted_idx[sample_categorical(filtered_probs)]
    MinP(p, t):
      # keep tokens with post-softmax prob >= p * max(probs)
      probs = softmax(logits / t)
      threshold = p * max(probs)
      keep_mask = probs >= threshold
      probs = probs * keep_mask
      probs /= sum(probs)
      return sample_categorical(probs)
```

Config schema in `[sampling]` TOML ([format.md](format.md)):

```toml
[sampling]
method = "top_p"              # greedy | temperature | top_k | top_p | min_p
temperature = 0.6
top_p = 0.95                  # for top_p
top_k = 40                    # for top_k
min_p = 0.05                  # for min_p
seed = 0                      # 0 = non-deterministic; nonzero = reproducible
```

Defaults: `method = "greedy"`, `temperature = 1.0` if not specified.

`sample_categorical` draws one index from a probability distribution
using a seeded RNG (Xorshift or PCG). Same seed + same distribution
= same output (determinism for reproducibility).

## 9. Quantize / Dequantize

Convert between dtypes. See [quant.md](quant.md) for exact formats.

```
Quantize(x, dtype):    # e.g. F32 → Q4_K
Dequantize(x, source): # e.g. Q4_K → F32
```

Quantized matmul is conceptually `Dequantize(W) ⊙ x` but implemented
as a fused kernel that reads quantized bytes and produces f32/f16 output
without materializing the dequantized weight.

## 10. Fused ops

All fused ops are performance optimizations. Their output must match
the corresponding unfused composition within ε.

- **FusedNormMatmul(x, norm_w, W) = Matmul(RmsNorm(x, norm_w, ε), W)**
- **FusedSkipNorm(x, skip, norm_w) = RmsNorm(x + skip, norm_w, ε), x + skip** (returns both)
- **FusedSwiGlu(x, W_gate, W_up) = Silu(x @ W_gate^T) ⊙ (x @ W_up^T)**

These are NOT new semantics — they must numerically match the
unfused equivalent to within 1e-4 (F32) or 1e-2 (F16). Any backend
may implement them as fused or unfused; choice is a performance
decision.

## Tolerance summary

| Context | F32 | F16 | Q4 |
|---|---|---|---|
| Single op output | 1e-6 | 1e-3 | 1e-2 |
| Layer composition | 1e-5 | 1e-3 | 1e-2 |
| Full forward (hundreds of ops) | 1e-4 | 1e-2 | 5e-2 |

See [test.md](test.md) for how these are verified.
