---
tags: cyber, cyb, core, spec
crystal-type: spec
crystal-domain: cyber
alias: .cyb format, cyb container
---

# .cyb — universal knowledge container

one file. self-describing. human-readable index. editable in vim. native [[particle]] format for [[hemera]].

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
created = 2026-04-02T00:00:00Z

[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"

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

## parts

`format` is any string. the container does not interpret part contents — it stores them as-is. text or binary is determined by whether `size` is present in frontmatter.

`type` on `[[parts]]` is optional — groups parts logically when a file contains multiple types.

`types` array in `[cyb]` declares what the file contains. a single file can have multiple types. types and formats are not hardcoded in this spec — see [[cyb-registry]] for the ecosystem catalog of supported formats and types.

## hemera integration

every .cyb file is a [[particle]] — the native content unit of the [[hemera]] network. hemera computes the CID (content identifier), handles chunking, verification, deduplication, and distribution. the .cyb format itself does not define how addressing works — that is hemera's responsibility.

what .cyb provides to hemera: a self-describing container with a readable index. hemera can inspect the frontmatter to understand what the particle contains without parsing binary data.

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
