---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .graph, graph format, cyb graph spec
---

# .graph — knowledge graph snapshot in [[.cyb|format]]

.graph follows the [[.cyb|format]] three rules. a .graph file IS a .cyb file — same parsing, same tools. the extension tells humans and tools: this container holds a cybergraph snapshot at a fixed block height.

one file. three sections. everything else is derivable.

## required files

| name | format | what it does |
|------|--------|-------------|
| card | .md | what this snapshot is |
| config | .toml | chain id, block, capture time, token table |
| cyberlinks | .records | the seven-tuple records (binary, fixed 128 B each) |

no optional files. no other required files. `particles`, `semcons`, `focus`, `spectral_gap` — all derive from `cyberlinks` + `config` in milliseconds.

## frontmatter

```toml
[cyb]
types = ["graph"]
name = "bostrom-23195000"

[[files]]
name = "card"
format = "md"

[[files]]
name = "config"
format = "toml"

[[files]]
name = "cyberlinks"
format = "records"
size = 346281344
```

## card

first thing you see. markdown. free-form description with at minimum the chain, block, and counts.

```markdown
~~~card
# bostrom-23195000

cybergraph snapshot from bostrom chain at block 23,195,000.
captured 2026-03-23 14:22 UTC. license: cyber license.
```

## config

chain identity, block height, capture metadata, and the token table. integers only — no floats.

```toml
~~~config
chain_id   = "bostrom-1"
block      = 23195000
captured_at = 1742740920
format_version = 1

[tokens]
0 = { symbol = "BOOT",     weight = 1000 }
1 = { symbol = "VOLT",     weight = 50000 }
2 = { symbol = "AMPERE",   weight = 1000000 }
3 = { symbol = "HYDROGEN", weight = 100000000 }
```

| field | meaning |
|-------|---------|
| chain_id | the chain this snapshot was taken from |
| block | block height at which the snapshot was cut |
| captured_at | unix seconds when the snapshot was emitted |
| format_version | spec revision (currently `1`) |
| [tokens] | denomination table: id → (symbol, weight) |

the token table defines how the compiler combines cross-denomination stakes. weight is integer per-mille: `weight = 1000` means 1.0×. the table may come from chain governance (if the chain pins it) or from the snapshot publisher (if it is policy). either way, the snapshot commits to these specific weights — different token tables produce different snapshots.

user-defined tokens are declared the same way. a private graph with `mytoken` just adds an entry to this table; no other change is needed.

nothing else belongs in config. the number of particles, cyberlinks, axons, and semcons — all derivable from the `cyberlinks` section. keeping them out of config removes the consistency risk of denormalized counts.

## cyberlinks

raw seven-tuple records. fixed 128-byte layout, page-aligned start, page-aligned end. zero-copy mmap into a typed slice.

```
record at offset i × 128:
  [0..32]    ν   neuron id (hemera hash, 32 B)
  [32..64]   p   source particle id (hemera hash, 32 B)
  [64..96]   q   target particle id (hemera hash, 32 B)
  [96..100]  τ   token denomination index (u32 little-endian)
  [100..116] a   stake amount (u128 little-endian, smallest unit)
  [116..117] v   valence (i8: -1, 0, +1)
  [117..125] t   block height (u64 little-endian)
  [125..128] _   padding (zero)
```

records appear in canonical chain order: ascending block height, then ascending intra-block index. `t` is always in the range `[1, config.block]`. `τ` is a valid index into `config.tokens`. the snapshot is invalid if either constraint fails.

## parsing

```
1. parse frontmatter (TOML until first ~~~)
2. read ~~~card for display
3. read ~~~config for chain id, block, token table
4. mmap ~~~cyberlinks as a slice of 128-byte records
5. derive whatever else is needed (particles, axons, focus, semcons)
```

four steps. no proof check in the base spec; see `proof` extension below.

## go-cyber integration

go-cyber emits .graph natively from two endpoints:

```
GET /cyber/graph/snapshot?block=H        → bostrom-H.graph
GET /cyber/graph/snapshot?block=latest   → bostrom-latest.graph
```

snapshot endpoints stream the file. clients can pipe directly into a compiler:

```
curl -s https://node.bostrom.cybernode.ai/cyber/graph/snapshot?block=23195000 \
  | mc - -o bostrom-23195000.model
```

## snapshot identity

the snapshot CID is

```
CID(.graph) = BLAKE3(file bytes)
```

two snapshots with the same chain_id, block, token table, and cyberlinks produce byte-identical files and therefore the same CID.

## extensions

the three sections above are the required core. .graph files may add any other section the .cyb format permits. none of them affect parsing or compilation:

| section | format | purpose |
|---------|--------|---------|
| particles | toml or binary | cached index CID → id + first_seen (speeds up some queriers) |
| program | tri or rs | embedded graph operators for provable queries on the snapshot |
| eval | toml | precomputed topology metrics (spectral gap, diameter, focus entropy) |
| proof | stark | STARK proof binding the cyberlinks section to a chain apphash |

each extension is specified on its own page when its design is real. mc ignores all of them — the compiler regenerates anything an extension caches.

## relation to .model

a .graph compiles into a .model via the [[compiled transformers spec]] (CT-1):

```
*.graph                 *.model
─────────               ───────
cyberlinks     ───►     embedding + attention + MLP (via SVDs on adjacency)
config.tokens  ───►     per-denomination stake weighting
config.block   ───►     [lineage].block in the compiled model
BLAKE3(.graph) ───►     [lineage].source in the compiled model
```

the compiled model's `[lineage]` section carries the exact snapshot CID, so every compile is provable against its input.

## why three sections

a `.graph` used to be a tar of jsonl files, a config, a stake dump, a proof — four formats, ad-hoc layouts, no zero-copy load. the first draft of this spec collapsed that into eight sections in one container, which was still more than necessary.

three sections is the floor. every field is either chain fact or committed snapshot policy; nothing is a convenience duplicate; nothing requires an algorithm spec of its own. `head -50 file.graph` tells you the chain, the block, and the denomination weights; `wc -c` on the cyberlinks section divided by 128 tells you the link count; mmap and go.

---

see [[.cyb|format]] for the base container. see [[cyb-model]] for the inference-side counterpart. see [[cyb-registry]] for the ecosystem catalog.
