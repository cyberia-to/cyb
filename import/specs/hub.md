# import hub fetch

How `import` retrieves source files from HuggingFace Hub. Implemented in
`import/hub/mod.rs` as a thin layer over `hf-hub`'s sync API.

## Surface

```
hub::download_model(model_id) -> PathBuf  // ONNX-only probe
hub::download_tokenizer(model_id) -> PathBuf
hub::download_file(model_id, filename) -> PathBuf
```

Only `download_model` is wired into the CLI today (via
[`mi download`](cli.md#mi-download-repo--onnx-scope)). The others are
the building blocks for the planned `mi fetch` subcommand that
fetches source files for the import flow.

## Cache

`hf-hub` writes into `~/.cache/huggingface/hub/`. `import` never touches
the cache directly — `hf-hub` owns the layout
(`models--<org>--<repo>/`, `refs/`, `snapshots/`, `blobs/`).

`mi list` enumerates that directory and prints `org/repo` entries.

## Probe order (download_model)

ONNX-quantized variants, first match wins:

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

After the model is fetched, `<model>_data` is attempted (ONNX large
external-data files). Missing `_data` is not an error — small models
ship inline.

## Failure modes

| Failure | Behavior |
|---|---|
| HF API init failure (network, auth) | Returns `Err(String)` with the underlying error |
| All probes 404 | Returns `Err` listing the candidates tried |
| Network drop mid-download | `hf-hub` retries internally per its policy; `import` adds none |
| Disk full | `hf-hub` propagates `io::Error`; `import` forwards as `String` |

## Out of scope today

- Safetensors / GGUF auto-fetch from HF — currently the user must
  download the source directory by hand, then run `mi import <dir>`.
- Authenticated repos (gated models). `hf-hub` honors `HF_TOKEN` from
  the environment, but `import` neither documents nor verifies this.
- Resumable downloads beyond `hf-hub` defaults.
- Pinning to a specific commit/revision. `hf-hub` defaults to the
  repo's main branch HEAD at fetch time.

The above gaps are real: the manifest models in
[manifest.md](manifest.md) are presumed already on disk.
