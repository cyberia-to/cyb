# Import contract

Converting external model formats (GGUF, safetensors, HF PyTorch,
MLX, ONNX) to canonical `.model` files.

Import is the boundary where external variance becomes internal
normalization. After import, every `.model` follows [format.md](format.md)
and [tensor.md](tensor.md) conventions exactly — the runtime never
deals with source-format quirks.

## Inputs

Accepted source formats:

| Format | Source | Priority |
|---|---|---|
| GGUF | llama.cpp, ollama | High (well-defined) |
| safetensors | HuggingFace | High (HF-native) |
| HF PyTorch (.bin) | legacy HF | Medium (unsafe pickle; prefer safetensors) |
| MLX | Apple MLX | Medium |
| ONNX | generic | Low (metadata varies) |

An import reads source, validates, normalizes, and writes a `.model`.

## Output invariants

After import, the `.model` file MUST satisfy:

1. **Canonical tensor names** — HF naming scheme
   (see [tensor.md](tensor.md#tensor-identity)).
2. **Canonical shapes** — row-major, `[out, in]` for weight matrices,
   `[vocab, hidden]` for embeddings.
3. **Canonical dtypes** — no BF16 in weights (converted to F16).
   Q4_0 normalized to Q4_K where possible.
4. **Canonical config** — flat schema matching [format.md](format.md).
5. **Special tokens registered** — every `<|...|>` pattern in vocab
   appears as added token with correct ID.
6. **All tensors required by declared `model_type` present** —
   import fails with explicit error otherwise.
7. **Non-weight metadata (sampling, chat template, license) inline**
   — no external files referenced.

Violation of any = import bug, fix import, do not work around in runtime.

## Tensor name normalization

Source-to-canonical mapping. Apply by regex substitution.

### GGUF → canonical

```
token_embd.weight                 → model.embed_tokens.weight
output_norm.weight                → model.norm.weight
output.weight                     → lm_head.weight

blk.{i}.attn_norm.weight          → model.layers.{i}.input_layernorm.weight
blk.{i}.attn_q.weight             → model.layers.{i}.self_attn.q_proj.weight
blk.{i}.attn_k.weight             → model.layers.{i}.self_attn.k_proj.weight
blk.{i}.attn_v.weight             → model.layers.{i}.self_attn.v_proj.weight
blk.{i}.attn_output.weight        → model.layers.{i}.self_attn.o_proj.weight
blk.{i}.attn_q.bias               → model.layers.{i}.self_attn.q_proj.bias
blk.{i}.attn_k.bias               → model.layers.{i}.self_attn.k_proj.bias
blk.{i}.attn_v.bias               → model.layers.{i}.self_attn.v_proj.bias
blk.{i}.attn_q_norm.weight        → model.layers.{i}.self_attn.q_norm.weight
blk.{i}.attn_k_norm.weight        → model.layers.{i}.self_attn.k_norm.weight
blk.{i}.ffn_norm.weight           → model.layers.{i}.post_attention_layernorm.weight
blk.{i}.ffn_gate.weight           → model.layers.{i}.mlp.gate_proj.weight
blk.{i}.ffn_up.weight             → model.layers.{i}.mlp.up_proj.weight
blk.{i}.ffn_down.weight           → model.layers.{i}.mlp.down_proj.weight
```

### HF safetensors → canonical

Many models already use HF naming directly. Apply identity for
decoder-only. For encoder models:

```
embeddings.word_embeddings.weight → model.embed_tokens.weight
embeddings.LayerNorm.weight       → model.norm.weight     (preserve)
encoder.layer.{i}.*               → model.layers.{i}.*     (position swap)
```

When an imported model has non-standard names (e.g., some Phi
variants use `lm_head.linear.weight`), the import layer must
recognize and rename.

**Missing mapping = import failure with actionable error**:
`"Unknown tensor 'foo.bar.weight' in source; add mapping to
import.md GGUF→canonical section"`.

### K=V shared projection (Gemma 3/4)

When the source declares K and V projections share weights
(`attention_k_eq_v: true` in HF config, or a single fused tensor in
GGUF), import emits **two canonical tensors** with identical bytes:

```
model.layers.{i}.self_attn.k_proj.weight
model.layers.{i}.self_attn.v_proj.weight   # same bytes as k_proj
```

The runtime sees the standard layout and stays one codepath. The
duplication cost (~kv_dim × hidden × bytes_per_elem per layer) is
amortised against the simpler runtime — the alternative (a fused
`kv_proj` tensor and a runtime-side split) would fork every backend's
weight loading.

The import sets `attention_k_eq_v = true` in the config so deduplicating
storage is a possible later optimisation (load once, alias both names).

## Shape normalization

GGUF stores weight matrices as `[in_features, out_features]` in
metadata, but physical layout is row-major by first dim = `[out, in]`.
Import records the CANONICAL shape `[out, in]` in `.model` tensors
section, matching physical layout.

Embed table: canonical shape is `[vocab, hidden]`, row-major.

Inconsistent sources must be detected: if an imported tensor's
declared shape × dtype ≠ byte count in source, raise error.

## Dtype normalization

```
BF16 → F16                       # always; BF16 not kept at runtime
Q4_0 → Q4_K (optional)           # higher precision; lossless upgrade
Q4_1 → Q4_K                      # Q4_1 deprecated
IQ2/IQ3/IQ4 → corresponding K-quant
```

The normalization is computed at import time, not runtime. Runtime
kernels operate on the canonical dtype set only.

Rationale for Q4_0 → Q4_K: empirically 25% lower RMSE per weight
for the same storage. K-quants are always preferred.

## Config normalization

### Flatten nested VL configs

HuggingFace VL models have:
```json
{
  "model_type": "qwen2_vl",
  "text_config": {"hidden_size": 1536, ...},
  "vision_config": {"hidden_size": 1280, ...}
}
```

Import writes:
```toml
model_type = "qwen2_vl"

[architecture]
hidden_size = 1536                # from text_config
... other text fields ...

[architecture.vision]
hidden_size = 1280
... other vision fields ...
```

The root `[architecture]` is the text tower; vision tower has its
own subsection. Curated VL codepath reads both.

### RMS norm epsilon convention

Source files may encode eps as:
- `rms_norm_eps = 0.000001` (direct)
- `rms_norm_eps = 1000000` (inverted, some older imports)

Import canonicalizes to direct form:
```
if value >= 1.0:
    canonical_eps = 1.0 / value
else:
    canonical_eps = value
```

`.model` config always stores direct form.

### Rope theta

Source may be f64; store as f32 in config. Typical values: 10000,
500000, 1000000, 10000000.

## Special tokens

Extract from source's tokenizer metadata:
- GGUF: `tokenizer.ggml.tokens` array + `tokenizer.ggml.eos_token_id` etc.
- HF: `tokenizer.json` + `special_tokens_map.json`

Every token matching `<|.+|>` (and a few common patterns like `[CLS]`,
`[MASK]`) is registered as an added/special token in the vocab section.

Import verifies that the tokenizer can re-tokenize canonical inputs
to produce the EOS token id(s) specified in config.

## Validation before writing

Before writing the `.model`, import runs checks:

1. **Tensor completeness** for declared `model_type`. E.g. LlamaStyle
   requires `model.embed_tokens.weight`, `model.norm.weight`,
   `lm_head.weight` (or `tie_word_embeddings=true`), and per-layer
   set. Missing = fail.

2. **Shape consistency** against config. Embed must be
   `[vocab_size, hidden_size]`. Q proj must be
   `[num_heads * head_dim, hidden_size]`. Any mismatch = fail.

3. **Dtype uniformity within tensor**. Mixed dtypes per tensor not
   supported (mixed per-tensor across the model is fine).

4. **Weight count** declared in config matches sum of tensor element
   counts.

5. **Round-trip token test**: encode → decode a known string (e.g.
   `"hello world"`) should reproduce input (within tokenizer's
   lossiness).

Failed validation: import exits with list of issues. No partial
`.model` is written.

## Multi-shard GGUF

Large models (13B+) come as multiple GGUF shards (`.gguf.00001-of-N`).
Import reads all shards, concatenates their tensor tables, writes
single `.model`.

## Import command

```
cyb-llm import SOURCE_DIR [--out OUTPUT.model] [--precision q4k|q6k|f16]
cyb-llm fetch HF_REPO [--out OUTPUT.model]      # fetch HF + import
```

`fetch` pulls safetensors from HF; `import` converts a local dir or file.

## Invariance test

The ultimate validation: for any source model M, import M, then read
back from `.model`, re-dequantize, compare with source:

- Same set of tensor names after normalization
- Same shapes (after canonical transposition)
- Dequantized values within format tolerance (Q4_K: 1.5e-2 per weight)
- Same tokenizer output for test strings

Import that fails this is broken. [test.md](test.md) defines the
specific test.
