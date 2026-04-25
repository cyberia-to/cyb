# .model file format

Single-file self-contained model package. All metadata, vocabulary,
program, and binary weights in one file. Content-addressed (hash of
file = model identity).

## File layout

```
┌────────────────────────┐
│ frontmatter (TOML)     │   file-level metadata
├────────────────────────┤
│ ~~~card                │   markdown: human description
│ ...                    │
├────────────────────────┤
│ ~~~config              │   TOML: architecture + tokenizer + sampling
│ ...                    │
├────────────────────────┤
│ ~~~program             │   Rust-like source (future: nox bytecode)
│ ...                    │
├────────────────────────┤
│ ~~~graph               │   hex-encoded binary IR graph (optional)
│ ...                    │
├────────────────────────┤
│ ~~~tensors             │   TOML: tensor index (name, shape, dtype, offset, size)
│ ...                    │
├────────────────────────┤
│ ~~~vocab               │   TOML: tokenizer vocab + merges
│ ...                    │
├────────────────────────┤
│ ~~~eval                │   TOML: benchmark results (optional)
│ ...                    │
├────────────────────────┤
│ ~~~weights\n           │   delimiter, then raw binary bytes
│ <binary tensor data>   │   aligned to 64 bytes, tensors packed
└────────────────────────┘
```

## Section delimiter

Sections are separated by lines starting with `~~~` followed by the
section name. The `~~~weights` delimiter is the last one before
binary data — everything after `~~~weights\n` is binary.

## Frontmatter

Minimal metadata at the top of the file, before any `~~~` section:

```toml
[cyb]
types = ["model"]
name = "qwen3-0.6b-abl"

[[files]]
name = "card"
format = "md"

[[files]]
name = "config"
format = "toml"

[[files]]
name = "program"
format = "rs"                    # or "tri" for trident

[[files]]
name = "tensors"
format = "toml"

[[files]]
name = "vocab"
format = "toml"

[[files]]
name = "eval"
format = "toml"

[[files]]
name = "weights"
format = "tensors"
size = 335503360                 # bytes of binary weights section
```

## config section

Canonical architecture description. Flat TOML (nested is normalized
at import time to flat).

```toml
model_type = "qwen3"             # dispatch key for arch template
parameters = 600000000
license = "Apache-2.0"

[architecture]
hidden_size = 1024
num_attention_heads = 16
num_key_value_heads = 8
num_hidden_layers = 28
intermediate_size = 3072
vocab_size = 151936
max_position_embeddings = 40960
rope_theta = 1000000
rms_norm_eps = 1e-6
tie_word_embeddings = true

[tokenizer]
type = "bpe"
eos_token_ids = [151645, 151643]
pad_token_id = 151643

[sampling]
temperature = 0.6                # defaults; overridable at runtime
top_p = 0.95
top_k = 20
```

Multimodal models store per-modality sub-sections if needed:
`[architecture.text]`, `[architecture.vision]`, but `hidden_size`
etc at `[architecture]` root refers to the primary modality.

### LlamaStyle+ (Gemma 3/4) extra fields

LlamaStyle+ models add the following optional fields to `[architecture]`:

```toml
hidden_activation = "gelu_pytorch_tanh"   # selects FFN activation
final_logit_softcapping = 30.0            # clamp logits before sample
attention_k_eq_v = true                   # K and V projections share weights
sliding_window = 1024                     # window for sliding layers

# Per-layer: one entry per layer, in order. Values: "sliding" | "full".
layer_types = [
  "sliding", "sliding", "sliding", "sliding", "sliding", "full",
  # ... repeated for num_hidden_layers entries total
]

# Gemma-4 only: full-attention layers use different attention dims.
# Omit for Gemma 3 (single attention shape across all layers).
global_head_dim = 512
num_global_key_value_heads = 4

# Gemma-4 only: per-kind RoPE. rope_theta (above) is the sliding value.
# rope_theta_full is the full-attention value.
# partial_rotary_factor_full is the fraction of head_dim that gets
# rotated for full layers (rest pass through unrotated).
rope_theta_full = 1000000.0
partial_rotary_factor_full = 0.25
```

A model is LlamaStyle (no plus) when none of the above are present and
no `layer_types` array is set. Presence of any of these fields triggers
the LlamaStyle+ codepath. See `arch.md` §LlamaStyle+ for semantics.

### VL config example (qwen2_vl)

```toml
model_type = "qwen2_vl"
parameters = 7100000000

[architecture]
# Primary modality (text tower)
hidden_size = 3584
num_attention_heads = 28
num_key_value_heads = 4
num_hidden_layers = 28
intermediate_size = 18944
vocab_size = 152064
max_position_embeddings = 32768
rope_theta = 1000000
rms_norm_eps = 1e-6
tie_word_embeddings = false

[architecture.vision]
hidden_size = 1280
num_attention_heads = 16
num_hidden_layers = 32
intermediate_size = 5120
patch_size = 14
image_size = 448
temporal_patch_size = 2            # for video input
spatial_merge_size = 2              # patch merge factor
window_size = 112

[architecture.projector]
input_dim = 5120                    # vision_hidden_size * spatial_merge_size^2
output_dim = 3584                   # text hidden_size
mlp_depth = 2
```

### ASR config example (whisper)

```toml
model_type = "whisper"
parameters = 244000000               # small

[architecture]
# Primary modality (text decoder)
hidden_size = 768
num_attention_heads = 12
num_hidden_layers = 12
vocab_size = 51865
max_position_embeddings = 448

[architecture.audio_encoder]
hidden_size = 768
num_attention_heads = 12
num_hidden_layers = 12
n_mels = 80
max_source_positions = 1500
conv_kernel = 3
```

Root `[architecture]` is always the primary generation/output tower
(text decoder for VL and ASR). Other modalities get named subsections.

## program section

Declarative description of the pipeline. One of:

- **`.rs` format**: Rust-like source that references `cyb::nn::*`
  primitives. Human-readable. Current default.
- **`.tri` format**: trident source. Higher-level DSL.
- **`.nox` format** (future): compiled nox bytecode.

The runtime does not execute the program section directly today.
It is read for:
1. Documentation — what the model does
2. Dispatch hints — which curated family applies
3. Future compilation to nox

## graph section (optional)

Serialized IR graph for models that should run via the graph
executor path. See [ir.md](ir.md) for node structure and binary
format.

Presence of `graph` section signals the graph executor can run this
model. A model with neither a matching curated family nor a graph
section is unrunnable (import should have rejected it).

The graph binary is hex-encoded (lowercase) in the text section so it
lives before `~~~weights\n` without breaking the UTF-8 text header
scan. The binary is ~80–120 KB for a 28-layer decoder; hex-encoded
that is ~160–240 KB — well within the 32 MB scan limit.

Frontmatter entry:
```toml
[[files]]
name = "graph"
format = "hex"                 # hex-encoded binary IR, see ir.md
```

## chat section (optional)

Chat template and EOS handling. See [tokenizer.md](tokenizer.md#chat-templates)
for full format.

```
~~~chat
format = "chatml"

template = """..."""            # Jinja template, overrides format preset

system_default = "..."
eos_sequences = ["<|im_end|>"]
```

## tensors section

TOML array of tensor metadata. Each tensor has a name (string),
shape (array of ints), encoding (dtype string), offset (byte offset
into weights section), size (bytes).

```toml
["model.embed_tokens.weight"]
shape    = [151936, 1024]        # [rows, cols] row-major
encoding = "q4"                  # see encoding names below
offset   = 0
size     = 87515136

["model.layers.0.input_layernorm.weight"]
shape    = [1024]
encoding = "u32"                 # f32, stored as raw u32-sized bytes
offset   = 87515136
size     = 4096
```

### Encoding names

Match [quant.md](quant.md) formats:

| Encoding | DType | Notes |
|---|---|---|
| `u32` | F32 | raw f32, little-endian |
| `u16` | F16 | raw f16, little-endian |
| `q4` | Q4_0 | 18 bytes/block of 32 |
| `q8` | Q8_0 | 34 bytes/block of 32 |
| `q2k` | Q2_K | 84 bytes/block of 256 |
| `q3k` | Q3_K | 110 bytes/block of 256 |
| `q4k` | Q4_K | 144 bytes/block of 256 |
| `q5k` | Q5_K | 176 bytes/block of 256 |
| `q6k` | Q6_K | 210 bytes/block of 256 |
| `ternary` | Ternary | 4 values/byte, 2 bits each |

## vocab section

BPE tokenizer. Two subsections:

```toml
[tokens]
0 = "<unk>"
1 = "<s>"
2 = "</s>"
...
151665 = "<|im_end|>"
...

[merges]
0 = ["Ġ", "t"]
1 = ["h", "e"]
...
```

Index is token ID. Merges are ordered by priority (earlier = applied
first). Special tokens are detected by pattern `<|...|>`; the
tokenizer registers them as added tokens.

## eval section (optional)

Benchmarks for this model, canonical prompts with expected outputs
and tolerances:

```toml
[[benchmarks]]
name = "math-basic"
prompt = "What is 2+2? /no_think"
expected_contains = ["4"]
tokens = 20
temperature = 0.0
```

Used by `cyb-llm status` to validate correctness.

## weights section

Raw binary data. Aligned to 64 bytes from the section start (after
`~~~weights\n`). Tensors are packed according to their offsets from
the tensors section.

Offsets are relative to the start of binary data (byte after
`~~~weights\n`).

## Alignment

All tensor offsets must be multiples of the greatest element size
needed by that tensor's dtype (typically 2 bytes for F16, 4 for F32,
block size for K-quants). This enables zero-copy mmap to aligned
buffers.

## File size computation

```
total = len(frontmatter) + sum(len(section_i)) + len("~~~weights\n") + weights_size
```

The `size = N` field in `[[files]].size = N` for weights must equal
the binary weights section size exactly. Loader validates.

## Content addressing

The file's SHA-256 hash is its cyb identity (CID). A `.model` with
the same name but different content is a different model.

Changes to any section (config, vocab, program, weights) change the
hash. Model versions are separate files with separate CIDs.

## Reading

Loader procedure (see [import.md](../../mi/specs/import.md) for writing):

1. Read up to some reasonable limit (default 32 MB — some models
   have large text sections).
2. Scan for `~~~weights\n` marker.
3. Parse text sections before the marker.
4. For large files, mmap from the marker offset for zero-copy
   tensor access during inference.
5. Validate: every tensor's `offset + size` ≤ weights section size.

## Future: binary format

An efficient binary format may replace the TOML sections for
production. Text format stays for readability and git-diff-ability.
Conversion is lossless.
