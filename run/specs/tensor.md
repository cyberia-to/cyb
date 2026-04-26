# Tensor conventions

Foundational conventions for tensors flowing through the runtime.
Every op in [ops.md](ops.md) obeys these. Every backend in
[architecture.md](architecture.md) stores tensors this way.

## Shape

Shape is an ordered list of non-negative integers, most-significant
dimension first. Rank = number of dimensions.

```
scalar:     shape = []              rank = 0
vector:     shape = [N]             rank = 1
matrix:     shape = [rows, cols]    rank = 2
token hidden: shape = [batch, seq, hidden]  rank = 3
attention: shape = [batch, heads, seq, head_dim]  rank = 4
image:      shape = [batch, channels, height, width]  rank = 4
video:      shape = [batch, channels, frames, height, width]  rank = 5
```

Names in shape documentation are hints; shape is integers.

## Layout

Row-major (C-style, same as NumPy/PyTorch default, HuggingFace,
GGUF). The last dimension is contiguous in memory. Strides derive
from shape:

```
stride[rank-1] = 1
stride[i] = stride[i+1] * shape[i+1]  for i < rank-1
```

Offset of element at index `[i0, i1, ..., i_{r-1}]` in element units:

```
offset = sum(i_k * stride[k] for k in 0..rank)
```

No transposed layouts. No column-major. If a tensor logically needs
reshaping, `Transpose` or `Permute` produces a new tensor.

## Dtype

Element types supported:

| Dtype | Bits | Use |
|---|---|---|
| F32 | 32 | CPU reference, small weights, position caches |
| F16 | 16 | GPU activations, KV cache, mid-precision weights |
| BF16 | 16 | HF-native weight storage, converted at import |
| I8 | 8 | Q8 residual, indexing |
| U8 | 8 | Quantized nibble containers, ternary blocks |
| Bool | 8 | Masks (attention mask, padding) |
| Q4_0 | ~4.5 | Legacy Q4 weight, block of 32 |
| Q8_0 | ~8.5 | Q8 weight, block of 32 |
| Q2_K, Q3_K, Q4_K, Q5_K, Q6_K | 2-6 | K-quant weights, superblock of 256 |
| Ternary | 1.58 | BitNet weights |

Floating-point types follow IEEE 754. Quantized types have exact
byte layouts defined in [quant.md](quant.md).

## Broadcasting

Element-wise ops (Add, Mul, Sub, Div, and their fused variants) follow
NumPy broadcasting rules:

1. Right-align shapes.
2. Each aligned dim must be equal, or one of them is 1.
3. Size-1 dims virtually repeat to match the other side.

```
[B, S, H] + [H]           →  [B, S, H]   (bias add)
[B, S, H] + [B, 1, H]     →  [B, S, H]   (per-batch bias)
[B, H, S, S] + [S, S]     →  [B, H, S, S] (attention mask)
[B, H, S, D] * [B, H, 1, D] → [B, H, S, D] (rope, scalar per pos)
```

Broadcasting never allocates new tensors. The iteration pattern is
strided. Storage remains what each operand has.

## Tensor identity

Tensors in the graph are named. Names are strings, typically from HF
convention:

```
model.embed_tokens.weight            — token embedding
model.layers.{i}.input_layernorm.weight
model.layers.{i}.self_attn.q_proj.weight
model.layers.{i}.self_attn.k_proj.weight
model.layers.{i}.self_attn.v_proj.weight
model.layers.{i}.self_attn.o_proj.weight
model.layers.{i}.self_attn.q_norm.weight   — Qwen3 QK-norm
model.layers.{i}.self_attn.k_norm.weight   — Qwen3 QK-norm
model.layers.{i}.post_attention_layernorm.weight
model.layers.{i}.mlp.gate_proj.weight
model.layers.{i}.mlp.up_proj.weight
model.layers.{i}.mlp.down_proj.weight
model.norm.weight
lm_head.weight                        — or tied to embed_tokens
```

[import.md](../../import/specs/import.md) defines how other conventions (GGUF `blk.i.attn_q`,
Whisper, DiT, ...) map to this canonical set.

## Weight matrix convention

For a linear layer computing `y = x @ W^T` (PyTorch convention):

- Input activation `x` has shape `[..., K]`
- Weight `W` has shape `[N, K]` (output first, input second)
- Output `y` has shape `[..., N]`

This matches HuggingFace, PyTorch, GGUF (GGUF stores
`[in_features, out_features]` in its metadata but physical layout
is also `[N, K]` row-major).

When ambiguous: weight's outermost dimension equals the op's output
dimension. `q_proj.weight` with shape `[4096, 1024]` means
"hidden 1024 → 4096 Q dimension". Token embed `[vocab, hidden]`.

## Scalars

Per-tensor scalars (scale, zero-point, d, dmin in quantized blocks)
are metadata, not separate tensors. They're embedded in the
quantized format. [quant.md](quant.md) specifies where.

## Errors

These are bugs and must be caught at tensor operation boundaries:

- Shape mismatch in binary op that isn't broadcast-compatible
- Dtype mismatch when op requires same dtype
- Rank too low / too high for op expectation
- Non-contiguous tensor passed to op that requires contiguous
- NaN/Inf in intermediate tensor when numerical stability expected

Each is a specific error, not silent corruption.
