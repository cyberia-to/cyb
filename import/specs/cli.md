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

Fetch a model from HuggingFace into the local `hf-hub` cache. The
contract — artifact priority, sibling metadata, failure modes —
lives in [hub.md](hub.md). Output: paths under
`~/.cache/huggingface/hub/`, suitable as input for `mi import <DIR>`.

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
