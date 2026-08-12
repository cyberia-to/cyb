---
tags: cyber, cyb, core, spec
crystal-type: spec
crystal-domain: cyber
alias: .cyb, cyb format, cyb container
---

# .cyb — universal knowledge container

one file. self-describing. human-readable index. editable as text. native [[particle]] format for [[hemera]].

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

## encoding

UTF-8. LF (0x0A) line endings. the delimiter `~~~name` is recognized only when followed by LF. CRLF is not a valid delimiter sequence. tools creating or editing `.cyb` files must write LF and must not convert line endings.

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
name = "weights"
format = "safetensors"
size = 1200000000
```

any fields can be added to `[cyb]` or `[[files]]`. extensible through new fields, not through versions.

## delimiter

`~~~name` for every file inside. text and binary alike.

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

## files

`format` is any string. the container does not interpret contents — stores as-is. see [[cyb-registry]] for the ecosystem catalog.

`type` on `[[files]]` is optional — groups files logically when a container holds multiple types.

## .cyb-compatible extensions

.cyb is a generic container. specific use cases get their own extensions that follow the same three rules:

| extension | spec |
|-----------|------|
| .cyb | this page |
| .model | [[cyb-model]] |
| .graph | [[cyb-graph]] |
| .vocab | [[cyb-vocab]] |

a .model file IS a .cyb file. the extension is a hint — not a different format. formats like .jpg, .gguf, .exe are NOT .cyb-compatible. they can be embedded inside .cyb as binary files. see [[cyb-registry]].

## hemera

[[hemera]] is the only hash format natively supported by .cyb. deliberate decision: the entire cyber ecosystem is optimized around a unified hash function.

any .cyb file is a valid hemera [[particle]].

## parsing

```
1. read lines until first "~~~" → frontmatter (TOML)
2. text files: "~~~name\n" → content until next "~~~" or EOF
3. binary files: "~~~name\n" → read `size` bytes
4. order in container = order in [[files]] array
```

## why .cyb

`head -50 file.cyb` tells you everything. `vim file.cyb` lets you edit text files. binary data sits at the end untouched. no other container does all three.

three rules. frozen.
