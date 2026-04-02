---
tags: cyber, cyb, core, spec
crystal-type: spec
crystal-domain: cyber
alias: .cyb format, cyb container
---

# .cyb — universal knowledge container

one file. self-describing. human-readable index. editable in vim. native [[particle]] format for [[hemera]].

this spec is frozen. three rules, no versions, no breaking changes.

## three rules

1. TOML frontmatter until first `~~~`
2. `~~~name` separates every part
3. binary parts have `size` in frontmatter

everything else follows from these three.

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

TOML. UTF-8. at the start of the file. ends at first `~~~`.

```toml
[cyb]
types = ["model", "dataset"]
name = "qwen3-0.6b-abliterated"

[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"

[[parts]]
name = "config"
format = "toml"

[[parts]]
name = "program"
format = "nox"

[[parts]]
name = "weights"
format = "safetensors"
size = 1200000000
```

any fields can be added to `[cyb]` or `[[parts]]`. the format is extensible through new fields, not through versions.

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

`~~~name` must be at the start of a line. `name` matches `parts.name` from frontmatter.

## text parts vs binary parts

| | text | binary |
|--|------|--------|
| `size` in frontmatter | not needed | required |
| boundary | next `~~~` or EOF | `size` bytes after `~~~name\n` |
| editable | yes | no |
| position | before binary parts | after all text parts |

text parts: parser finds next `~~~` to determine boundary. editing text parts does not require updating frontmatter.

binary parts: parser reads exactly `size` bytes. binary parts must come after all text parts. within the binary zone, parts are read sequentially by `size`.

## parts

`format` is any string. the container does not interpret contents — stores as-is. see [[cyb-registry]] for the ecosystem catalog of formats and types.

`type` on `[[parts]]` is optional — groups parts logically when a file contains multiple types.

`types` array in `[cyb]` declares what the file contains. a single file can have multiple types.

## hemera

[[hemera]] is the only hash format natively supported by .cyb. this is a deliberate decision: the entire cyber ecosystem is optimized around a unified hash function. one hash, no fragmentation, no compatibility matrices.

any .cyb file is a valid hemera [[particle]]. hemera handles hashing, chunking, verification, and distribution. .cyb handles packaging and human readability.

## parsing

```
1. read lines until first "~~~" → frontmatter (TOML)
2. text parts: "~~~name\n" → content until next "~~~" or EOF
3. binary parts: "~~~name\n" → read `size` bytes
4. order in file = order in [[parts]] array
```

## CLI

```bash
cyb info file.cyb            # show frontmatter
cyb cat file.cyb config      # print text part
cyb extract file.cyb weights # extract binary part
cyb parts file.cyb           # list parts with sizes
cyb verify file.cyb          # verify hemera hash
cyb pack file.cyb            # create from parts
```

## why .cyb

`head -50 file.cyb` tells you everything. `vim file.cyb` lets you edit text parts. binary data sits at the end untouched. no other container does all three.

no versions. no breaking changes. three rules, frozen.
