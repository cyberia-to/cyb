---
tags: cyber, cyb, core, spec
crystal-type: registry
crystal-domain: cyber
alias: cyb registry, format registry, type registry
---

# cyb-registry — formats and types for [[cyb-format]]

living catalog of file formats and .cyb-compatible extensions supported in the cyber ecosystem. any format can be embedded in .cyb — this registry tracks what tools understand natively.

## .cyb-compatible extensions

extensions that follow the [[cyb-format]] three rules (TOML frontmatter + `~~~name` delimiters + size for binary). a .model file IS a .cyb file.

| extension | type | spec | description |
|-----------|------|------|-------------|
| .cyb | any | [[cyb-format]] | generic container |
| .model | model | [[cyb-model]] | neural network (config + nox + weights) |

adding an extension: follow the three rules, create a spec page, add to this table.

## non-.cyb formats (embedded inside .cyb as files)

formats that do NOT follow the three rules. stored inside .cyb containers as binary or text files.

## file formats

### text (human-readable, editable)

| format | description | tools |
|--------|-------------|-------|
| toml | config, metadata, structured key-value | cyb cat, vim |
| nox | [[nox]] computation programs | cyb-llm compile |
| md | markdown documentation | optica, any renderer |
| json | structured data (HF compat) | any JSON parser |
| nu | [[nushell]] scripts | nu |
| rs | Rust source (via codematter) | rustc, cargo |
| sh | shell scripts | bash, zsh |
| yml | YAML config | any YAML parser |
| csv | tabular data | any CSV parser |
| txt | plain text | cat |

### binary (machine-readable)

| format | description | tools |
|--------|-------------|-------|
| safetensors | tensor weights (mmap-safe) | cyb-llm load |
| cbor | compact structured binary (RFC 8949) | any CBOR parser |
| jpg | JPEG image | any image viewer |
| png | PNG image | any image viewer |
| webp | WebP image | any image viewer |
| wav | audio waveform | any audio player |
| mp3 | compressed audio | any audio player |
| ogg | Ogg Vorbis audio | any audio player |
| mp4 | video container | any video player |
| webm | WebM video | any video player |
| onnx | ONNX model (legacy import) | cyb-llm import |
| gguf | GGUF model (legacy import) | cyb-llm import |
| pt | PyTorch checkpoint (legacy import) | cyb-llm import |
| wasm | WebAssembly module | wasmtime, browser |
| mach-o | macOS executable | macOS loader |
| elf | Linux executable | Linux loader |
| metallib | compiled Metal shaders | Metal GPU |
| raw | arbitrary bytes | — |

### adding a format

any string is valid as `format` in .cyb parts. tools that encounter an unknown format treat it as raw bytes (binary) or UTF-8 text (based on `size` presence).

to register a format for ecosystem-wide support: add it to this page and implement handling in the relevant tool.

### adding a format

any string is valid as `format` in .cyb `[[files]]`. tools that encounter an unknown format treat it as raw bytes (binary) or UTF-8 text (based on `size` presence).

to register a format for ecosystem-wide support: add it to this page and implement handling in the relevant tool.
