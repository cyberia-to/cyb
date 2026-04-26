# import contract

How external model formats become canonical `.model` files.

## Boundary

`import` is the boundary where source-format variance ends and `.model`
invariance begins. Every transformation that a source needs in order to
satisfy the invariants below happens here, once, at import time. After
that, the runtime in `run/` operates on a single canonical shape; it
never sees BF16, GGUF block ordering, fused KV tensors, or nested VL
configs.

When import and runtime disagree on what a model "is", import is the
side that changes. Workarounds in runtime are bugs.

## Inputs

| Format | Source | CLI status |
|---|---|---|
| GGUF (single-file or sharded `*.gguf.NNNNN-of-NNNNN`) | llama.cpp, ollama | wired |
| safetensors (single-file or sharded `*.safetensors.index.json`) | HuggingFace | loader present, not wired into `mi import` |
| ONNX | generic | loader present, not wired into `mi import` |
| HF PyTorch (`.bin`) | legacy HF | not implemented; prefer safetensors |
| MLX | Apple MLX | not implemented |

"Wired" means `mi import <DIR>` reads it today. The unwired loaders
exist as Rust modules but the CLI's `run_import` only locates `*.gguf`
files in the source directory. See [cli.md](cli.md) §Out-of-scope.

## Output invariants

After import, the `.model` file MUST satisfy:

1. **Canonical tensor names** — HF naming scheme defined in
   [run/specs/tensor.md](../../run/specs/tensor.md#tensor-identity).
2. **Canonical shapes** — row-major, `[out, in]` for weight matrices,
   `[vocab, hidden]` for embeddings.
3. **Canonical dtypes** — no BF16 (converted to F16); no Q4_0 / Q4_1
   (converted to Q4_K).
4. **Canonical config** — flat schema matching
   [run/specs/format.md](../../run/specs/format.md).
5. **Special tokens registered** — every `<|...|>` pattern in vocab
   appears as an added token with a model-valid ID.
6. **All tensors required by declared `model_type` present** — import
   fails with explicit error otherwise.
7. **Non-weight metadata inline** — sampling, chat template, license
   live in the `.model`, not external files.

Violation of any of the above is an import bug. Fix import; do not
work around in runtime.

## Transformations

Each transformation is derived from the gap between source variance
and the invariants above.

### Tensor names

The canonical target is the HF decoder-only naming scheme
([run/specs/tensor.md](../../run/specs/tensor.md#tensor-identity)).
Every source tensor maps to exactly one canonical name; mappings live
in code so the spec doesn't drift:

- GGUF → canonical: `naming::gguf_to_hf` in `import/naming.rs`.
- HF safetensors → canonical: identity for HF-named decoder weights;
  encoder models need a position-swap mapping
  (`embeddings.word_embeddings.weight` → `model.embed_tokens.weight`,
  `encoder.layer.{i}.*` → `model.layers.{i}.*`).
- HF PyTorch / MLX / ONNX → canonical: not yet specified.

**Implementation status** (safetensors): two distinct gaps.

| Gap | Where |
|---|---|
| No `safetensors_to_hf` function in `naming.rs` | even the trivial decoder identity case is unimplemented |
| `main.rs::run_import` only locates `*.gguf` | the safetensors loader (`loader/safetensors.rs`) is reachable via `loader::load_model` for `.safetensors` paths, but the CLI never feeds it any |

A source tensor that no mapping recognizes is a fatal import error
with an actionable message:

```
Unknown tensor 'foo.bar.weight' in source;
add mapping to import/naming.rs.
```

### Shapes

The canonical layout is row-major `[out_features, in_features]` for
weight matrices and `[vocab_size, hidden_size]` for embeddings. GGUF
sources may store dimensions in either order; the loader transposes
to canonical. Source-side detail lives in `import/loader/gguf.rs`.

If a tensor's declared shape × declared dtype byte size does not
equal the source byte count, import refuses (the source is
malformed).

### Dtypes

Every source tensor is dequantized to f32 then re-encoded into one of
the canonical five encodings (`u32`, `u16`, `q8`, `q4`, `ternary`)
defined by [`cyb/cyb-model`](https://cyber.page/cyb/cyb-model). The
canonical encoding for each tensor is chosen by role
(`import/naming::canonical_encoding_for`):

| Tensor role | Canonical encoding |
|---|---|
| layer norms (`*norm.weight`, `model.norm.weight`) | `u32` |
| biases (`*.bias`) | `u32` |
| matmul weights (q/k/v/o_proj, mlp gate/up/down, embed_tokens, lm_head) | `q8` (default; q4 for compact-storage targets) |

Block-size constraint: q4 / q8 / ternary require element count
divisible by 32. Tensors below that threshold (e.g. gemma-4's
per-layer scalar `layer_output_scale`) fall back to u32 with a
warning.

Source dtype is irrelevant to the output layout — only the canonical
policy decides what bytes land on disk. Source dtypes the loader
recognizes today: F32 / F16 / BF16 (safetensors), GGUF Q4_0 / Q4_1 /
Q4_K / Q5_K / Q6_K / Q8_0.

Bit layouts for the canonical five live in
[run/specs/quant.md](../../run/specs/quant.md).

### K=V shared projection (Gemma 3/4)

When the source declares K and V projections share weights
(`attention_k_eq_v: true` in HF config, or a fused `kv_proj` tensor in
GGUF), import emits **two canonical tensors with identical bytes**:

```
model.layers.{i}.self_attn.k_proj.weight
model.layers.{i}.self_attn.v_proj.weight   # same bytes as k_proj
```

The runtime sees the standard split layout and stays one codepath.
The duplication cost (`kv_dim × hidden × bytes_per_elem` per K=V
layer) is the price of avoiding a runtime fork in every backend's
weight loader.

`attention_k_eq_v = true` is preserved in the config so storage
deduplication (load once, alias both names) is a possible later
optimization.

### Config

The `.model` config is flat TOML keyed by the schema in
[run/specs/format.md](../../run/specs/format.md). Source variance
collapses into that schema:

- VL models with nested `text_config` / `vision_config` flatten into
  `[architecture]` (text tower) and `[architecture.vision]` (vision
  tower). **Implementation status**: today `mi import` only emits
  `[architecture]`; the vision sub-table is unimplemented.
- Numeric types narrow to f32 (RoPE θ, eps, etc.); f64 source values
  round-trip through f32 on write.
- Boolean flags use TOML `true`/`false`, not 0/1.

### Special tokens

Extracted from the source's tokenizer metadata:

- GGUF: `tokenizer.ggml.tokens` array + `tokenizer.ggml.eos_token_id`,
  `bos_token_id`, etc.
- HF: `tokenizer.json` (incl. `added_tokens` for chat-template
  specials like `<|im_start|>`) + `special_tokens_map.json`.

Every token matching `<|.+|>` plus a few common patterns (`[CLS]`,
`[MASK]`) is registered as an added/special token in the vocab
section with a model-valid ID.

## Validation

Before writing the `.model`, import verifies the invariants hold.
A failed check aborts the write — no partial `.model` ever lands on
disk.

| Check | Status | What it asserts |
|---|---|---|
| Tensor completeness | enforced | required tensors for the declared `model_type` are all present |
| Shape consistency | not implemented | embed is `[vocab_size, hidden_size]`; Q proj is `[num_heads × head_dim, hidden_size]`; etc. |
| Dtype uniformity | not implemented | a single tensor uses one dtype (mixed across tensors is fine) |
| Weight count | not implemented | sum of tensor element counts matches a per-arch budget derived from config |
| Round-trip token | not implemented | encode → decode of `"hello world"` reproduces the input within tokenizer lossiness |
| EOS-id coherence | not implemented | tokenizer's EOS id matches the value in config |

As checks ship, their `Status` column flips to `enforced`.

## Failure modes

| Failure | Behavior |
|---|---|
| Source tensor with no name mapping | abort with the actionable message above |
| Declared shape × dtype ≠ source byte count | abort |
| Required tensor missing for `model_type` | abort |
| Tokenizer file missing | abort |
| Source format not in [Inputs](#inputs) | abort with a note pointing to this section |

Partial output is never written.
