# mi importer specification

How external model formats become canonical `.model` files.

## Scope

mi reads source models (GGUF, safetensors, ONNX, HF PyTorch, MLX),
normalizes naming/shapes/dtypes/config, and writes a `.model` file that
the runtime ([run/](../../run/)) can mmap.

mi never reads source-format quirks at runtime; runtime never sees them
at all. The boundary is the `.model` invariants in [import.md](import.md).

## Files

- [import.md](import.md) — invariants, name/shape/dtype/config
  normalization, validation, multi-shard handling
- *(planned)* `manifest.md` — what makes a model MVP-eligible
- *(planned)* `hub.md` — HuggingFace fetch policy: caching, retry, recovery
- *(planned)* `cli.md` — `mi <subcommand>` surface
- *(planned)* `graph.md` — when a graph IR section is emitted at import time

## Cross-references (shared with runtime)

The following specs are owned by `run/specs/` because the runtime
consumes them; mi follows them on the producer side:

- [format.md](../../run/specs/format.md) — `.model` file layout
- [tensor.md](../../run/specs/tensor.md) — canonical tensor names + shapes
- [quant.md](../../run/specs/quant.md) — Q4_0/Q4_K/Q5_K/Q6_K/Q8/ternary bit layouts
- [tokenizer.md](../../run/specs/tokenizer.md) — special tokens, chat templates
- [test.md](../../run/specs/test.md) — four-tier test strategy

## Source of truth

When code and spec disagree: spec is authoritative. If spec is wrong,
update spec first (one commit), then propagate to code (separate commit).
