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
the architecture template ([arch.md](arch.md)).

Cos/sin cache: precompute `cos[pos, j]` and `sin[pos, j]` for all
positions up to max_seq. Per-model `base` (rope_theta) parameter.

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
Attention (GQA).

```
Sdpa(Q, K, V, num_heads, kv_heads, head_dim, causal):
  # Q: [B, num_heads, Sq, head_dim]
  # K, V: [B, kv_heads, Sk, head_dim]
  # if kv_heads < num_heads, each KV head is shared by (num_heads / kv_heads)
  # Q heads (expand K, V by replication)
  scale = 1 / sqrt(head_dim)
  scores = Q @ K^T * scale              # [B, num_heads, Sq, Sk]
  if causal:
    scores[..., i, j] = -inf for j > i
  probs = Softmax(scores, dim=-1)
  y = probs @ V                         # [B, num_heads, Sq, head_dim]
```

**Scale is divided, not multiplied.** Some implementations bake it
into Q; equivalent but spec here uses explicit scale.

**Causal mask** applies BEFORE softmax. Masked entries get -∞ so they
contribute 0 after softmax.

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

Stateful append. For position `p` with seq_len `s`:

```
cache.K[p : p+s] = K_new    # [B, kv_heads, s, head_dim]
cache.V[p : p+s] = V_new
attn uses cache.K[0 : p+s] and cache.V[0 : p+s]
```

Lifecycle: persistent buffer pre-allocated to max_seq; zero
allocations during decode.

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

Standard convolution. For Conv2d:

```
Conv2d(x, W, bias, kernel, stride, padding, groups):
  # x: [B, C_in, H, W], W: [C_out, C_in/groups, kH, kW]
  # output: [B, C_out, H', W']
  y[b, c, i, j] = sum over (ki, kj, cin) of
                  x[b, cin + c_group*C_in/groups, i*sH + ki - pad, j*sW + kj - pad]
                  * W[c, cin, ki, kj]
                  + bias[c]
```

### ConvTranspose2d

Learned upsampling (inverse strides).

### CausalConv1d

Replicate-pad left so output index `t` depends only on input ≤ t.
Video models (Wan, Hunyuan) use this.

### DepthwiseConv

`groups = C_in`. Each input channel has its own filter.

### Pool

Max or average pooling over spatial window.

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
    Greedy: argmax(logits)
    Temperature(t): softmax(logits / t), then sample
    TopK(k, t): keep top-k, renormalize softmax, sample
    TopP(p, t): keep smallest set with cumulative prob ≥ p, renormalize, sample
    MinP(p, t): keep tokens with prob ≥ p*max_prob, renormalize, sample
```

All above are reductions to a single token id.

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
