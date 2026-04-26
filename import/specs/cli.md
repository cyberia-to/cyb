# import CLI surface

What the `mi` binary (the CLI entry point of the `import` crate)
exposes today.

## Subcommands

```
mi <SUBCOMMAND>
```

| Subcommand | Purpose | Source |
|---|---|---|
| `mi import <DIR>` | Convert source directory → `~/llm/<name>.model` | `main.rs::run_import` |
| `mi list` | List HF cache entries under `~/.cache/huggingface/hub` | `main.rs::run_list` |
| `mi download <REPO>` | Download ONNX-quantized model from HuggingFace into HF cache | `main.rs::run_download` |

## `mi import <DIR>` contract

Input: a directory containing
- exactly one `*.gguf` file
- a `tokenizer.json`
- a `config.json`

Output: `~/llm/<NAME>.model`, where `NAME` is derived by stripping
`-import` suffix from the source directory name.

Side effects:
- Reads `~/llm/` to find target path; creates if missing.
- Writes a single `.model` file. No staging, no atomic rename.
- Embeds the binary IR graph as a `~~~graph` section when the
  parsed config produces a `LlamaStyle` family that
  `run::ir::family_graph` recognizes — see [graph.md](graph.md).

The `.model` invariants are enforced by [import.md](import.md).

## `mi download <REPO>` — ONNX scope

`download` is **not** the fetcher for the `import` flow. It probes the
repo for ONNX variants in this fixed order:

```
onnx/model_q4.onnx
onnx/model_q4f16.onnx
onnx/model_bnb4.onnx
onnx/model_quantized.onnx
onnx/model_int8.onnx
model_q4.onnx
model_q4f16.onnx
model_quantized.onnx
onnx/model.onnx
model.onnx
onnx/decoder_model.onnx
decoder_model.onnx
```

First match wins. Downloaded into the `hf-hub` cache, not into `~/llm/`.

To bring an HF model into a `.model` file, the user fetches the source
files manually (e.g. via `huggingface-cli download …`) and runs
`mi import` against the resulting directory.

## Out-of-scope today

- Direct HF → `.model` (single command). The `Download` flow targets
  ONNX only; safetensors / GGUF auto-fetch is unimplemented.
- Multi-shard GGUF input (`*.gguf.00001-of-N`). [import.md](import.md)
  declares the contract but the CLI only reads a single GGUF.
- ONNX / safetensors source format. The loader module supports both,
  but `run_import` only locates `*.gguf` files in the source directory.

These gaps are tracked; the CLI surface above is what works today.
