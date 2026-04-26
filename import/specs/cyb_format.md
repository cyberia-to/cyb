# import .model writer

The producer side of the `.model` file format. The reader contract
lives in [run/specs/format.md](../../run/specs/format.md); this spec
covers the writer's discipline (section ordering, frontmatter, optional
sections, encoding choices) and what the writer *guarantees* on disk.

Implementation: `import/cyb_format.rs`.

## File layout

A `.model` file is one TOML frontmatter block, an ordered list of
named text sections, and a trailing binary weights blob.

```
<TOML frontmatter>
~~~card
<markdown>
~~~config
<TOML>
~~~program
<source code or empty>
~~~graph              ← optional, see below
<lowercase hex>
~~~tensors
<TOML index>
~~~vocab
<TOML>
~~~eval
<TOML>
~~~weights
<raw bytes>           ← length declared in frontmatter
```

The order is fixed. Readers locate `~~~weights\n` by scanning the
prefix as text up to the marker, then mmap the binary tail.

## Frontmatter

```toml
[cyb]
types = ["model"]
name = "<NAME>"

[[files]]
name = "card"
format = "md"

[[files]]
name = "config"
format = "toml"

[[files]]
name = "program"
format = "<rs|empty|...>"

# optional [[files]] entry for "graph" inserted here when graph is present

[[files]]
name = "tensors"
format = "toml"

[[files]]
name = "vocab"
format = "toml"

[[files]]
name = "eval"
format = "toml"

[[files]]
name = "weights"
format = "tensors"
size = <weight blob byte count>
```

Order of `[[files]]` entries matches the order of `~~~`-marked
sections in the body.

## Optional `~~~graph` section

The graph IR section is included iff the writer is given a non-`None`
hex string for it. Position: between `~~~config` and `~~~tensors`,
matching its `[[files]]` entry. Encoding: lowercase hex of the
canonical binary serialization defined in
[run/specs/ir.md](../../run/specs/ir.md). Hex keeps the prefix
text-safe so readers can still scan for `~~~weights\n` without
binary detection.

When emitted, see [graph.md](graph.md) for the import-time decision.

## Newline policy

Every named text section ends with exactly one `\n` before the next
section marker. The writer ensures this by appending `\n` only when
the body doesn't already end with one. Empty bodies (e.g. a blank
`program` section) emit `~~~marker\n\n`.

## Weights blob

`~~~weights\n` is followed by exactly `frontmatter.weights.size`
bytes. The byte count is declared in the frontmatter so a reader can
allocate or mmap the right slice without parsing tensor offsets.

The blob is the concatenation of every tensor's bytes in the order
declared by the `~~~tensors` index. Each tensor's start offset is
the sum of sizes of all preceding tensors.

## What the writer guarantees

| Property | Guarantee |
|---|---|
| Section order | fixed (card, config, program, [graph], tensors, vocab, eval, weights) |
| Frontmatter ↔ body | every `[[files]]` entry has a matching `~~~name` section in the same order |
| Weights size | matches `frontmatter.weights.size` exactly |
| Graph placement | if present, between `~~~config` and `~~~tensors` only |
| Trailing newlines | every text section ends with exactly one `\n` |
| Atomicity | not yet — see Implementation status |

## Implementation status

| Property | Status |
|---|---|
| Atomic write (temp file + rename) | not implemented; partial files possible on crash |
| Checksum of weights blob | not implemented |
| Re-validation after write (read back, compare) | not implemented |

The reader (`run::format::read_model_file`) detects truncation via
size mismatch but won't catch byte-level corruption.

## Related

- [run/specs/format.md](../../run/specs/format.md) — reader contract
- [run/specs/ir.md](../../run/specs/ir.md) — graph binary encoding
- [graph.md](graph.md) — when the optional graph section is emitted
