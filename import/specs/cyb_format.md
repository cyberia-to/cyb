# import .model writer — divergence from canonical spec

The canonical `.model` format spec is the graph page
[cyb/cyb-model](https://cyber.page/cyb/cyb-model)
(source: `cyb/root/cyb-model.md`). That page is `crystal-type: spec`
and is authoritative.

This note tracks where the current writer (`import/cyb_format.rs`)
produces files that **do not** satisfy the canonical spec. Closing
each divergence is real engineering work, not a docs edit.

## Divergence table

| Dimension | Canonical (cyb/cyb-model) | Today (`import/cyb_format.rs`) |
|---|---|---|
| Encodings | five fixed: `u32`, `u16`, `q8`, `q4`, `ternary` | open: F16/F32 + GGUF K-quants (Q4_K / Q5_K / Q6_K / Q8_0 / IQ2/3/4) |
| Floats in weights | banned (integers only) | F16 is canonical for non-quantized weights |
| Sections | seven, all required: `card`, `config`, `program`, `tensors`, `vocab`, `eval`, `weights` | same seven plus optional `graph` between `config` and `tensors` |
| Eps storage | inverted integer (`rms_norm_eps = 1000000`) | direct float (`rms_norm_eps = 1e-6`) |
| Sampling | per-mille integers (`temperature = 700`) | not addressed by writer; copied from source |
| Program | `.tri` preferred, `.rs` fallback | `.rs` only; `.tri` emission unimplemented |
| Conversion | `GGUF Q4_K → q4` (dequant + requant down to 5-encoding set) | `GGUF Q4_K` kept as-is |

## What this means

A `.model` file produced today is **not a canonical .model file** by
the cyb/cyb-model definition. It happens to be readable by the
current `run/format::read_model_file` because the reader is also
implemented against today's code, not the canonical spec.

Closing the gap requires: an integer quantization pipeline at import
time, a `.tri` program emitter, the eps/sampling integer encoding,
and the fixed five-encoding set. Each is its own work item.

## What lives where now

- Canonical format definition: `cyb/cyb-model` (graph page)
- Today's writer code: `import/cyb_format.rs`
- Today's reader code: `run/format.rs`
- Optional graph IR section (an extension layered onto today's
  format): [graph.md](graph.md)
