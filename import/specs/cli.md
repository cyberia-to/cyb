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
| `mi download <REPO>` | Download a model from HuggingFace into the HF cache | `main.rs::run_download` |

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

## `mi download <REPO>` contract

Fetch a model and its metadata from HuggingFace into the local `hf-hub`
cache. The selection picks the canonical model artifact in this
priority order — first match wins:

1. `*.safetensors` (single-file or sharded `*.safetensors.index.json`)
2. `*.gguf` (single-file or sharded)
3. `*.onnx` (with sibling `*_data` external-data files when present)

Always alongside the artifact:
- `config.json`
- `tokenizer.json` (or `tokenizer.model` + `tokenizer_config.json` when
  the repo is sentencepiece-only)
- `special_tokens_map.json` when present
- `generation_config.json` when present

Output: paths under `~/.cache/huggingface/hub/`. The downloaded files
are sufficient input for `mi import <DIR>` against that cache directory.

### Implementation status

Today the implementation only probes a fixed list of ONNX-quantized
filenames in `hub/mod.rs::download_model`. Safetensors / GGUF auto-fetch
is unimplemented — see [hub.md](hub.md) §"Implementation status" for
the gap detail.

Until the gap closes, the user fetches non-ONNX source files by hand
(e.g. via `huggingface-cli download …`) and runs `mi import` against
the resulting directory.

## Out-of-scope today

- Single-command HF → `.model`. `mi download` puts files in the HF
  cache; `mi import` then converts them. Composing both into one is
  a planned `mi fetch` subcommand, not yet shipped.
- Multi-shard GGUF input (`*.gguf.00001-of-N`). [import.md](import.md)
  declares the contract but the CLI only reads a single GGUF.
- Non-GGUF source formats at import time. The loader module supports
  ONNX and safetensors, but `run_import` only locates `*.gguf` files
  in the source directory.

These gaps are tracked; the CLI surface above is what works today.
