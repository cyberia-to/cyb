---
tags: cyber, cyb, core, spec
crystal-type: spec
crystal-domain: cyber
alias: .cyb format, cyb container, cyb format
---

# .cyb — universal knowledge container

one file. self-describing. human-readable index. heterogeneous parts in optimal formats. content-addressable.

replaces: ZIP (opaque), tar (no index), ONNX (models only), PDF (documents only), Docker layers (overkill).

the missing primitive: a container where you open it and immediately understand what is inside, extract any part independently, verify integrity, address by content.

## design principles

1. open the file → read the frontmatter → know everything. no binary parsing needed to understand contents
2. each part stored in its optimal format. text stays text. binary stays binary. no forced serialization
3. one file = one [[particle]] in the [[cybergraph]]. addressable by CID. linkable via [[cyberlinks]]
4. works across OS boundaries. not a folder. not a ZIP. a flat file with a readable index

## structure

```
anything.cyb

┌─────────────────────────────────────────┐
│ frontmatter (TOML, UTF-8, readable)     │
│                                         │
│ [cyb]                                   │
│ version = 2                             │
│ type = "model"                          │
│ name = "qwen3-0.6b-abliterated"         │
│ cid = "bafy...abc"                      │
│ created = 2026-03-31T10:00:00Z          │
│                                         │
│ [[parts]]                               │
│ name = "config"                         │
│ format = "toml"                         │
│ size = 512                              │
│ cid = "bafy...def"                      │
│                                         │
│ [[parts]]                               │
│ name = "program"                        │
│ format = "nox"                          │
│ size = 256                              │
│                                         │
│ [[parts]]                               │
│ name = "vocab"                          │
│ format = "toml"                         │
│ size = 48000                            │
│                                         │
│ [[parts]]                               │
│ name = "weights"                        │
│ format = "safetensors"                  │
│ size = 1200000000                       │
│ cid = "bafy...ghi"                      │
│ chunks = 4688                           │
│                                         │
├─────────────────────────────────────────┤
│ \n---\n (separator)                     │
├─────────────────────────────────────────┤
│ part: config (TOML, readable)           │
│   architecture = "Qwen3ForCausalLM"     │
│   hidden_size = 1024                    │
│   ...                                   │
├─────────────────────────────────────────┤
│ part: program (nox, readable)           │
│   transformer_decoder {                 │
│     layers: 28                          │
│     hidden: 1024                        │
│     ...                                 │
│   }                                     │
├─────────────────────────────────────────┤
│ part: vocab (TOML, readable)            │
│   type = "bpe"                          │
│   vocab_size = 151936                   │
│   ...                                   │
├─────────────────────────────────────────┤
│ part: weights (safetensors, binary)     │
│   [raw tensor bytes]                    │
└─────────────────────────────────────────┘
```

## frontmatter

always TOML. always UTF-8. always at the start of the file. separated from parts by `\n---\n`.

```toml
[cyb]
version = 2
type = "model"          # model | dataset | graph | document | checkpoint
name = "qwen3-0.6b-abliterated"
cid = "bafy...abc"      # CID of entire file
created = 2026-03-31T10:00:00Z

# lineage — how this file was produced
[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
base_cid = "bafy...base"        # original unquantized model
method = "abliteration + q4_0"

# parts index — what is inside
[[parts]]
name = "config"
format = "toml"
size = 512

[[parts]]
name = "weights"
format = "safetensors"
size = 1200000000
cid = "bafy...ghi"
chunks = 4688
```

reading the frontmatter = reading TOML until `\n---\n`. any language, any OS, no special parser.

## parts

each part has:
- **name** — identifier within the container
- **format** — how to interpret the bytes
- **size** — exact byte count
- **cid** — optional, content address for verification/deduplication

### supported part formats

| format | human-readable | use for |
|--------|:-:|---------|
| toml | yes | config, metadata, vocab, sampling params |
| nox | yes | computation programs (forward pass, preprocessing) |
| md | yes | documentation, README, model card |
| json | yes | tokenizer.json (HF compat), structured data |
| safetensors | no | tensor weights (safe, mmap-ready) |
| cbor | no | compact structured binary data |
| bao | no | chunked binary with verification tree |
| raw | no | arbitrary bytes |

text parts are stored as-is (UTF-8). binary parts are stored as-is (raw bytes). no re-encoding, no wrapping.

## types

the `type` field in frontmatter determines the expected parts:

### type = "model"

a neural network ready for inference.

```
required parts:
  config    (toml)  — architecture params
  program   (nox)   — forward pass definition
  weights   (safetensors | cbor | bao)  — tensor data

optional parts:
  vocab     (toml)  — tokenizer vocabulary
  chat      (toml)  — chat template + special tokens
  sampling  (toml)  — default inference params
  preprocess (toml) — image/audio/video preprocessing
```

### type = "dataset"

training or evaluation data.

```
required parts:
  schema    (toml)  — column names, types, splits
  data      (cbor | raw)  — samples

optional parts:
  readme    (md)    — dataset card
```

### type = "graph"

a knowledge subgraph (pages + links).

```
required parts:
  pages     (cbor)  — page content + frontmatter
  links     (cbor)  — wiki-links between pages

optional parts:
  config    (toml)  — graph metadata
```

### type = "checkpoint"

training state for resumption.

```
required parts:
  config    (toml)  — training config
  weights   (safetensors) — model weights
  optimizer (safetensors) — optimizer state
  step      (toml)  — current step, loss, lr
```

### type = "document"

general heterogeneous document.

```
parts: any combination of text, images, data, code
```

## part addressing

each part is accessible by name:

```bash
cyb info model.cyb                    # show frontmatter
cyb extract model.cyb config          # extract config part as TOML
cyb extract model.cyb weights > w.st  # extract weights to file
cyb cat model.cyb program             # print nox program to stdout
```

## binary layout

```
[TOML frontmatter bytes (UTF-8)]
[0x0A 0x2D 0x2D 0x2D 0x0A]  ← "\n---\n" separator (5 bytes)
[part 1 bytes]
[part 2 bytes]
...
[part N bytes]
```

parts are concatenated in the order listed in `[[parts]]`. no padding between parts (sizes are exact). to find part N: sum sizes of parts 0..N-1, add frontmatter size + 5 (separator).

for mmap: frontmatter gives offset of each part. runtime mmaps the file, seeks to weight offset, reads directly into GPU. zero copies.

## content addressing

every .cyb file has a CID (content identifier) = BLAKE3 hash of entire file contents.

large parts (weights) are additionally BAO-chunked: split into 256KB blocks, each block has its own CID. the BAO tree root = part CID.

this enables:
- verify integrity of any part independently
- deduplicate shared tensors across model family
- parallel download: fetch chunks from multiple peers
- partial load: stream only needed layers
- [[hemera]] integration: chunks addressable in the network

## why not existing containers

| container | self-describing | random access | human-readable index | heterogeneous | content-addressable |
|-----------|:-:|:-:|:-:|:-:|:-:|
| ZIP | no | yes (central dir) | no | yes | no |
| tar | no | no (sequential) | no | yes | no |
| GGUF | yes (KV metadata) | yes | no (binary) | no (models only) | no |
| ONNX | partially (protobuf) | no | no | no (graphs only) | no |
| PDF | no | yes (xref table) | no | partially | no |
| Docker layer | yes (JSON manifest) | yes | partially | yes | yes (digest) |
| **.cyb** | **yes (TOML)** | **yes (offsets)** | **yes** | **yes** | **yes (CID)** |

.cyb is the first container where you can `head -50 file.cyb` and understand what is inside.

## relationship to cyber stack

.cyb files are [[particles]] in the [[cybergraph]]. every .cyb has a CID. CIDs are linked via [[cyberlinks]].

```
model.cyb (CID: bafy...model)
    ├── cyberlink → dataset.cyb (CID: bafy...data, rel: "trained_on")
    ├── cyberlink → base_model.cyb (CID: bafy...base, rel: "derived_from")
    └── cyberlink → benchmark.cyb (CID: bafy...bench, rel: "evaluated_by")
```

the knowledge graph grows with every model trained, every dataset published, every benchmark run. [[tri-kernel]] computes relevance. high-quality models gain [[gravity]].
