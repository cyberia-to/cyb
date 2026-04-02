---
tags: cyber, cyb, core, spec
crystal-type: spec
crystal-domain: cyber
alias: .cyb format, cyb container, cyb format
---

# .cyb — universal knowledge container

one file. self-describing. human-readable. heterogeneous parts in optimal formats. content-addressable. editable in vim.

replaces: ZIP (opaque), tar (no index), ONNX (models only), PDF (documents only), Docker layers (overkill).

the missing primitive: a container where you `head -50 file.cyb` and immediately understand what is inside. extract any part. verify integrity. address by content.

## design principles

1. open the file → read the frontmatter → know everything
2. text parts are human-readable and editable. binary parts are at the end
3. one file = one [[particle]] in the [[cybergraph]]. addressable by CID
4. works across OS boundaries. not a folder. a flat file with a readable index
5. each part in its optimal format. no forced serialization
6. single delimiter `~~~name` for everything. no ambiguity

## file structure

```
anything.cyb

┌─ frontmatter (TOML) ────────────────────┐
│ [cyb]                                   │
│ version = 2                             │
│ types = ["model", "dataset"]            │
│ name = "qwen3-0.6b-abliterated"         │
│ cid = "bafy...abc"                      │
│                                         │
│ [[parts]]                               │
│ name = "config"                         │
│ format = "toml"                         │
│                                         │
│ [[parts]]                               │
│ name = "weights"                        │
│ format = "safetensors"                  │
│ size = 1200000000                       │
│ cid = "bafy...ghi"                      │
├─────────────────────────────────────────┤
│ ~~~config                               │
│ architecture = "Qwen3ForCausalLM"       │
│ hidden_size = 1024                      │
│ ...                                     │
├─────────────────────────────────────────┤
│ ~~~program                              │
│ transformer_decoder {                   │
│   layers: 28                            │
│   attn: flash_attention                 │
│ }                                       │
├─────────────────────────────────────────┤
│ ~~~weights                              │
│ <binary safetensors data until EOF>     │
└─────────────────────────────────────────┘
```

## delimiter: `~~~name`

one delimiter for all parts: three tildes + part name.

```
~~~config
content of config part here

~~~program
content of program part here

~~~weights
<binary data>
```

rules:

- `~~~name` must be at the start of a line, followed by newline
- `name` matches a `parts.name` from frontmatter
- text parts: content runs from `~~~name\n` until the next `~~~` or EOF
- binary parts: content runs from `~~~name\n` for exactly `size` bytes (from frontmatter). binary parts must be last in the file
- everything before the first `~~~` is frontmatter (TOML)

why `~~~`: three tildes + name does not appear in any known format (TOML, YAML, markdown, JSON, nox, Python, Rust, C). markdown uses `~~~` for fenced code but always in pairs (open + close). a single `~~~name` at line start is unambiguous.

## frontmatter

always TOML. always UTF-8. always at the start of the file. ends at the first `~~~name`.

```toml
[cyb]
version = 2
types = ["model"]
name = "qwen3-0.6b-abliterated"
cid = "bafy...abc"
created = 2026-03-31T10:00:00Z

[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
base_cid = "bafy...base"
method = "abliteration + q4_0"

[[parts]]
name = "config"
format = "toml"

[[parts]]
name = "program"
format = "nox"

[[parts]]
name = "vocab"
format = "toml"

[[parts]]
name = "chat"
format = "toml"

[[parts]]
name = "weights"
format = "safetensors"
size = 1200000000
cid = "bafy...ghi"
chunks = 4688
```

`size` is optional for text parts (parser finds next `~~~`). `size` is required for binary parts (parser reads exact bytes).

## parts

each part has:
- `name` — identifier (must be unique within the file)
- `type` — optional, which logical type this part belongs to
- `format` — how to interpret the bytes
- `size` — byte count (required for binary, optional for text)
- `cid` — optional, content address for verification and deduplication
- `chunks` — optional, number of BAO chunks for large binary parts

### part formats

| format | readable | use for |
|--------|:-:|---------|
| toml | yes | config, metadata, vocab, sampling, eval results |
| nox | yes | computation programs (forward pass, preprocessing) |
| md | yes | documentation, model card, README |
| json | yes | structured data, HF compat |
| safetensors | no | tensor weights (safe, mmap-ready) |
| cbor | no | compact structured binary |
| raw | no | arbitrary bytes |

text formats are stored as UTF-8. binary formats are stored as raw bytes. no re-encoding.

## types

the `types` array in frontmatter declares what the .cyb contains. a single file can contain multiple types. each `[[parts]]` entry can have a `type` field for filtering.

### model

a neural network ready for inference.

```toml
[[parts]]
name = "config"
type = "model"
format = "toml"

[[parts]]
name = "program"
type = "model"
format = "nox"

[[parts]]
name = "weights"
type = "model"
format = "safetensors"
size = 1200000000
```

required: config, program, weights. optional: vocab, chat, sampling, preprocess.

### dataset

training or evaluation data.

```toml
[[parts]]
name = "schema"
type = "dataset"
format = "toml"

[[parts]]
name = "data"
type = "dataset"
format = "cbor"
size = 50000000
```

### document

general content — text, images, mixed.

```toml
[[parts]]
name = "content"
type = "document"
format = "md"
```

### graph

a knowledge subgraph.

```toml
[[parts]]
name = "pages"
type = "graph"
format = "cbor"
size = 10000000

[[parts]]
name = "links"
type = "graph"
format = "cbor"
size = 500000
```

### checkpoint

training state for resumption.

```toml
[[parts]]
name = "weights"
type = "checkpoint"
format = "safetensors"
size = 5000000000

[[parts]]
name = "optimizer"
type = "checkpoint"
format = "safetensors"
size = 5000000000

[[parts]]
name = "step"
type = "checkpoint"
format = "toml"
```

## multi-type example

one .cyb = model + evaluation results + documentation:

```
[cyb]
version = 2
types = ["model", "dataset", "document"]
name = "qwen3-0.6b-abliterated-full"

[[parts]]
name = "config"
type = "model"
format = "toml"

[[parts]]
name = "program"
type = "model"
format = "nox"

[[parts]]
name = "eval"
type = "dataset"
format = "toml"

[[parts]]
name = "card"
type = "document"
format = "md"

[[parts]]
name = "weights"
type = "model"
format = "safetensors"
size = 1200000000

~~~config
architecture = "Qwen3ForCausalLM"
hidden_size = 1024
num_attention_heads = 16
num_key_value_heads = 8
num_hidden_layers = 28
intermediate_size = 3072
vocab_size = 151936
max_position_embeddings = 40960
rope_theta = 1000000.0
rms_norm_eps = 0.000001

~~~program
transformer_decoder {
  layers: 28
  hidden: 1024
  heads: 16
  kv_heads: 8
  head_dim: 64
  rope_theta: 1e6
  norm: rmsnorm(eps=1e-6)
  attn: flash_attention
  ffn: swiglu(intermediate=3072)
  embed: token_embed(vocab=151936)
  output: linear(vocab=151936)
}

~~~eval
[needle_in_haystack]
context = 104000
score = 0.991

[mmlu_pro]
score = 0.724

[humaneval]
pass_at_1 = 0.652

~~~card
# qwen3-0.6b-abliterated

0.6B parameter model for routing and intent classification.
abliterated: refusal vectors removed from weights.
soma tier 0 — always on, <15ms latency.

source: huihui-ai/Qwen3-0.6B-abliterated
license: Apache 2.0

~~~weights
<1.2GB safetensors binary data>
```

everything above `~~~weights` is readable in any text editor. the model card, benchmarks, architecture, forward pass — all visible at a glance.

## nox program (model type)

the forward pass described as a [[nox]] program. replaces ONNX graph entirely. ~50 lines instead of millions of nodes.

```nox
transformer_decoder {
  layers: 28
  hidden: 1024
  heads: 16
  kv_heads: 8
  head_dim: 64
  rope_theta: 1e6
  norm: rmsnorm(eps=1e-6)
  attn: flash_attention
  ffn: swiglu(intermediate=3072)
  embed: token_embed(vocab=151936)
  output: linear(vocab=151936)
}
```

the runtime reads this and compiles to optimal code for the target hardware:
- Apple Silicon → [[acpu]] AMX matmul + [[rane]] ANE for tier 0 + [[aruminium]] Metal for attention
- NVIDIA → CUDA with tensor cores
- wgpu → cross-platform compute shaders
- CPU → NEON/AVX2 SIMD

### why not ONNX

ONNX stores a static computation graph with frozen shapes. flash attention cannot be expressed as ONNX nodes. every serious runtime rewrites the ONNX graph before execution anyway. ONNX has 4000+ operators, protobuf bloat, vendor-specific extensions.

a 10-parameter nox program replaces millions of ONNX nodes, compiles to optimal hardware-specific code, and supports dynamic shapes natively.

| | ONNX / graph IR | nox program |
|--|-----------------|-------------|
| size | millions of nodes | ~50 lines |
| flash attention | cannot express | `attn: flash_attention` |
| dynamic shapes | limited | native |
| hardware optimization | runtime rewrites graph | compiler generates optimal code |
| human readable | no (protobuf) | yes |

## parsing algorithm

```
1. read file as bytes
2. scan for first line starting with "~~~"
   everything before it = frontmatter (parse as TOML)
3. for each text part (no `size` in frontmatter):
   content = bytes from "~~~name\n" until next "~~~" or EOF
4. for each binary part (`size` specified in frontmatter):
   must be after all text parts
   content = next `size` bytes after "~~~name\n"
5. binary parts are read sequentially by size
   (no delimiter scanning in binary zone)
```

text editing safety: editing text parts does not require updating any offsets or sizes in frontmatter. only binary parts have `size`, and binary parts are not edited by hand.

## content addressing

every .cyb file has a CID = BLAKE3 hash of entire file.

large binary parts are additionally BAO-chunked (256KB blocks). each chunk has its own CID. enables:
- verify any part independently
- deduplicate shared tensors across model family
- parallel download from multiple peers
- partial load (stream only needed layers)
- [[hemera]] network integration

model lineage is a CID DAG:

```
base.cyb (CID: bafy...base)
  ├─ finetune.cyb (lineage.base_cid = bafy...base)
  ├─ quantized.cyb (lineage.base_cid = bafy...base, method = "q4_0")
  └─ abliterated.cyb (lineage.base_cid = bafy...base, method = "abliteration")
```

## CLI

```bash
cyb info model.cyb              # show frontmatter
cyb cat model.cyb program       # print nox program
cyb cat model.cyb card          # print model card
cyb extract model.cyb weights   # extract binary part to file
cyb pack model.cyb              # create .cyb from parts
cyb verify model.cyb            # check CIDs
cyb parts model.cyb             # list parts with sizes
```

## why .cyb

| container | readable index | editable | heterogeneous | multi-type | content-addressable |
|-----------|:-:|:-:|:-:|:-:|:-:|
| ZIP | no | no | yes | no | no |
| tar | no | no | yes | no | no |
| GGUF | no | no | no | no | no |
| ONNX | no | no | no | no | no |
| PDF | no | no | partially | no | no |
| **.cyb** | **yes** | **yes** | **yes** | **yes** | **yes** |

.cyb is the first container where `head -50 file.cyb` tells you everything, `vim file.cyb` lets you edit config and architecture, and the binary weights sit quietly at the end untouched.
