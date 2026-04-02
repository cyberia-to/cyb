---
tags: cyber, cyb, core, spec
crystal-type: registry
crystal-domain: cyber
alias: cyb registry, format registry, type registry
---

# cyb-registry — formats and types for [[cyb-format]]

living catalog of part formats and container types supported in the cyber ecosystem. not exhaustive — any format can be stored in .cyb. this registry tracks what tools understand natively.

## part formats

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

## container types

| type | description | spec | required parts |
|------|-------------|------|----------------|
| model | neural network | [[cyb-model]] | config, program, weights |
| dataset | training/eval data | — | schema, data |
| document | text, media, mixed | — | any |
| graph | knowledge subgraph | — | pages, links |
| checkpoint | training state | — | weights, optimizer, step |
| executable | runnable program | — | manifest, code |
| package | collection of files | — | manifest, contents |

### adding a type

any string is valid as `types` entry in .cyb. tools that encounter an unknown type inspect parts individually.

to register a type: add it to this table, optionally create a spec page (like [[cyb-model]]) defining required and optional parts.

## model type — format details

see [[cyb-model]] for the full specification of the model type, including:
- nox program (replaces ONNX)
- tensor naming convention
- quantization methods
- model lineage (CID chain)
- supported architectures
