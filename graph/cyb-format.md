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
2. `~~~name` separates every file inside
3. binary files have `size` in frontmatter

everything else follows from these three.

## structure

```
anything.cyb
├── frontmatter (TOML)     ← what is inside
├── ~~~config              ← text file (readable)
├── ~~~program             ← text file (readable)
├── ~~~weights             ← binary file (size in frontmatter)
└── ~~~image               ← binary file (until EOF)
```

## frontmatter

TOML. UTF-8. at the start of the file. ends at first `~~~`.

```toml
[cyb]
types = ["model"]
name = "qwen3-0.6b-abliterated"

[[files]]
name = "config"
format = "toml"

[[files]]
name = "program"
format = "nox"

[[files]]
name = "weights"
format = "safetensors"
size = 1200000000
```

any fields can be added to `[cyb]` or `[[files]]`. the format is extensible through new fields, not through versions.

## delimiter

`~~~name` for every file inside the container. text and binary alike.

```
~~~config
architecture = "Qwen3ForCausalLM"
hidden_size = 1024

~~~program
transformer_decoder { layers: 28 }

~~~weights
<binary bytes>
```

`~~~name` at the start of a line. `name` matches `files.name` from frontmatter.

## text files vs binary files

| | text | binary |
|--|------|--------|
| `size` in frontmatter | not needed | required |
| boundary | next `~~~` or EOF | `size` bytes after `~~~name\n` |
| editable | yes | no |
| position | before binary files | after all text files |

text files: parser finds next `~~~` to determine boundary. editing does not require updating frontmatter.

binary files: parser reads exactly `size` bytes. binary files must come after all text files. within the binary zone, files are read sequentially by `size`.

## files

`format` is any string. the container does not interpret contents — stores as-is. see [[cyb-registry]] for the ecosystem catalog of formats.

`type` on `[[files]]` is optional — groups files logically when a container holds multiple types.

## .cyb-compatible extensions

.cyb is a generic container. specific use cases get their own extensions that follow the same three rules:

| extension | type | spec | what is inside |
|-----------|------|------|----------------|
| .cyb | any | this page | generic container |
| .model | model | [[cyb-model]] | neural network (config + nox + weights) |

a .model file IS a .cyb file. `cyb info file.model` works. `head -50 file.model` is readable. the extension is a hint for tools and humans — not a different format.

formats like .jpg, .gguf, .exe are NOT .cyb-compatible — they do not follow the three rules. they can be embedded inside .cyb as binary files, but they are not .cyb containers themselves. see [[cyb-registry]] for the distinction.

## hemera

[[hemera]] is the only hash format natively supported by .cyb. deliberate decision: the entire cyber ecosystem is optimized around a unified hash function. one hash, no fragmentation, no compatibility matrices.

any .cyb file is a valid hemera [[particle]]. hemera handles hashing, chunking, verification, and distribution. .cyb handles packaging and human readability.

## parsing

```
1. read lines until first "~~~" → frontmatter (TOML)
2. text files: "~~~name\n" → content until next "~~~" or EOF
3. binary files: "~~~name\n" → read `size` bytes
4. order in container = order in [[files]] array
```

## CLI

```bash
cyb info file.cyb            # show frontmatter
cyb cat file.cyb config      # print text file
cyb extract file.cyb weights # extract binary file
cyb files file.cyb           # list files with sizes
cyb verify file.cyb          # verify hemera hash
cyb pack file.cyb            # create from files
```

## why .cyb

`head -50 file.cyb` tells you everything. `vim file.cyb` lets you edit text files. binary data sits at the end untouched. no other container does all three.

no versions. no breaking changes. three rules, frozen.
