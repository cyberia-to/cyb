# import crate specification

How external model formats become canonical `.model` files.

## Scope

`import` reads source models (GGUF, safetensors, ONNX, HF PyTorch, MLX),
normalizes naming, shapes, dtypes, and config, and writes the canonical
`.model` file consumed by [run/](../../run/).

Source-format quirks are normalized at import time. The runtime never
deals with them. The normalization contract lives in
[import.md](import.md).

## Files

- [import.md](import.md) — invariants, name/shape/dtype/config
  normalization, validation, multi-shard handling
- [manifest.md](manifest.md) — what makes a model MVP-eligible
- [hub.md](hub.md) — HuggingFace fetch contract: artifact priority, sibling metadata, failure modes, implementation status
- [cli.md](cli.md) — `mi <subcommand>` surface
- [graph.md](graph.md) — when a graph IR section is emitted at import time

## Cross-references (shared with runtime)

The following specs are owned by `run/specs/` because the runtime
consumes them; `import` follows them on the producer side:

- [format.md](../../run/specs/format.md) — `.model` file layout
- [tensor.md](../../run/specs/tensor.md) — canonical tensor names + shapes
- [quant.md](../../run/specs/quant.md) — Q4_0/Q4_K/Q5_K/Q6_K/Q8/ternary bit layouts
- [tokenizer.md](../../run/specs/tokenizer.md) — special tokens, chat templates
- [test.md](../../run/specs/test.md) — four-tier test strategy

## Source of truth

When code and spec disagree: spec is authoritative. If spec is wrong,
update spec first (one commit), then propagate to code (separate commit).
