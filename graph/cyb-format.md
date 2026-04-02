---
tags: cyber, cyb, core, spec
crystal-type: spec
crystal-domain: cyber
alias: .cyb format, cyb container
---

# .cyb — universal knowledge container

one file. self-describing. human-readable index. editable in vim. content-addressable.

## structure

```
file.cyb
├── frontmatter (TOML)     ← what is inside
├── ~~~part1               ← text part (readable)
├── ~~~part2               ← text part (readable)
├── ~~~part3               ← binary part (size in frontmatter)
└── ~~~part4               ← binary part (until EOF)
```

## frontmatter

TOML. UTF-8. at the start. ends at first `~~~`.

```toml
[cyb]
version = 2
types = ["model", "dataset"]
name = "qwen3-0.6b-abliterated"
cid = "bafy...abc"
created = 2026-04-02T00:00:00Z

[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
base_cid = "bafy...base"

[[parts]]
name = "config"
format = "toml"

[[parts]]
name = "data"
format = "cbor"
size = 50000000
```

## delimiter

`~~~name` for every part. text and binary alike.

```
~~~config
architecture = "Qwen3ForCausalLM"
hidden_size = 1024

~~~program
transformer_decoder { layers: 28 }

~~~weights
<binary bytes>
```

## rules

| | text parts | binary parts |
|--|-----------|-------------|
| delimiter | `~~~name` required | `~~~name` required |
| size in frontmatter | optional | required |
| boundary detection | next `~~~` or EOF | `size` bytes after `~~~name\n` |
| editable | yes | no |
| position in file | before binary parts | after text parts |

binary parts must come after all text parts. within binary zone, parser reads by `size` sequentially.

## part formats

| format | readable | examples |
|--------|:-:|---------|
| toml | yes | config, metadata, eval results |
| nox | yes | computation programs |
| md | yes | documentation, model cards |
| json | yes | structured data |
| safetensors | no | tensor weights |
| jpg, png | no | images |
| wav, mp3 | no | audio |
| mp4 | no | video |
| cbor | no | compact binary data |
| raw | no | arbitrary bytes |

## types

`types` array in `[cyb]`. one file can contain multiple types. each `[[parts]]` can have `type` for filtering.

| type | purpose | spec |
|------|---------|------|
| model | neural network | [[cyb-model]] |
| dataset | training/eval data | — |
| document | text, media, mixed | — |
| graph | knowledge subgraph | — |
| checkpoint | training state | — |

## content addressing

every .cyb file has CID = BLAKE3 hash. large binary parts are BAO-chunked (256KB). each chunk has its own CID.

enables: integrity verification, deduplication, parallel download, partial load, [[hemera]] network integration.

## parsing

```
1. read until first "~~~" → frontmatter (TOML)
2. text parts: "~~~name\n" → content until next "~~~"
3. binary parts: "~~~name\n" → read `size` bytes
4. order in file = order in [[parts]] array
```

## CLI

```bash
cyb info file.cyb            # show frontmatter
cyb cat file.cyb config      # print text part
cyb extract file.cyb weights # extract binary part
cyb parts file.cyb           # list parts with sizes
cyb verify file.cyb          # check CIDs
cyb pack file.cyb            # create from parts
```

## why .cyb

`head -50 file.cyb` tells you everything. `vim file.cyb` lets you edit text parts. binary data sits at the end untouched. no other container does all three.
