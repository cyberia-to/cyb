# import hf fetch

How `import` retrieves source files from HuggingFace Hub. Implemented
in `import/hf.rs` as a thin layer over the `hf-hub` crate's sync API.

## Contract

Fetch any model from HuggingFace and the sibling files needed to
import it. The artifact selection is by priority — first present
wins:

1. `*.safetensors` (single-file or sharded via `*.safetensors.index.json`)
2. `*.gguf` (single-file or sharded `*.gguf.NNNNN-of-NNNNN`)
3. `*.onnx` (plus any external-data sidecars `<name>_data`)

Sibling files always fetched when present:

- `config.json`
- `tokenizer.json` *or* `tokenizer.model` + `tokenizer_config.json`
- `special_tokens_map.json`
- `generation_config.json`

## Surface

```
hf::download_model(model_id) -> PathBuf  // fetches the artifact, returns its path
```

Wired into the CLI as [`mi download`](cli.md#mi-download-repo-contract).

## Cache

Downloads land in `~/.cache/huggingface/hub/`, owned by the `hf-hub`
crate. `import` never touches that cache directly. `mi list`
enumerates the directory and prints `org/repo` entries.

## Failure modes

| Failure | Behavior |
|---|---|
| HF API init failure (network, auth) | Returns `Err(String)` with the underlying error |
| No artifact matching priority list | Returns `Err` listing the candidates tried |
| Network drop mid-download | `hf-hub` retries internally per its policy; `import` adds none |
| Disk full | `hf-hub` propagates `io::Error`; `import` forwards as `String` |

## Implementation status

`download_model` today only probes a fixed list of ONNX-quantized
filenames:

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

After the artifact, `<artifact>_data` is attempted (ONNX large
external-data files). Missing `_data` is not an error — small models
ship inline.

The contract above (safetensors > GGUF > ONNX, plus sibling metadata)
is the target. Closing the gap is two changes:

1. List repo files via `hf-hub`'s repo info API rather than probing
   fixed names.
2. Walk the priority list against the listing; pull the artifact +
   its sibling metadata files.

## Out of scope today

- Authenticated repos (gated models). `hf-hub` honors `HF_TOKEN` from
  the environment, but `import` neither documents nor verifies this.
- Resumable downloads beyond `hf-hub` defaults.
- Pinning to a specific commit/revision. `hf-hub` defaults to the
  repo's main branch HEAD at fetch time.

The above gaps are real: the manifest models in
[manifest.md](manifest.md) are presumed already on disk.
