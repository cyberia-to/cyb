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
| signals | .signals | the atomic broadcast units — signed bundles of cyberlinks |

no optional files. no other required files. `particles`, `cyberlinks` (as a flat stream), `semcons`, `focus`, `spectral_gap` — all derive from `signals` + `config` in milliseconds.

the snapshot preserves the chain's atom: a [[cyber/signal|signal]] is the unit a [[neuron]] commits in one step, carrying an ordered vector of [[cyberlinks]] under one signature, one block, one optional proof. flattening signals into a link stream throws away structure the chain pays for every block. the base spec keeps signals intact.

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
name = "signals"
format = "signals"
size = 312456720
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
chain_id       = "bostrom-1"
block          = 23195000
captured_at    = 1742740920
format_version = 1

[[tokens]]
cid    = "0x1a2b3c4d..."
symbol = "BOOT"
weight = 1000

[[tokens]]
cid    = "0x5e6f7a8b..."
symbol = "VOLT"
weight = 50000

[[tokens]]
cid    = "0x9c0d1e2f..."
symbol = "AMPERE"
weight = 1000000
```

| field | meaning |
|-------|---------|
| chain_id | the chain this snapshot was taken from |
| block | block height at which the snapshot was cut |
| captured_at | unix seconds when the snapshot was emitted |
| format_version | spec revision (currently `1`) |
| [[tokens]] | per-denomination entries: cid (hemera hash), symbol (human label), weight (multiplier) |

tokens are first-class particles — each denomination is identified by its hemera hash, the same way every other node in the graph is. the link record carries `τ` as the 32-byte CID; the compiler looks up `weight` and `symbol` in this table by content match.

the token table defines how the compiler combines cross-denomination stakes. weight is integer per-mille: `weight = 1000` means 1.0×. the table may come from chain governance (if the chain pins it) or from the snapshot publisher (if it is policy). either way, the snapshot commits to these specific weights — different token tables produce different snapshots.

user-defined tokens are declared the same way. a private graph with `mytoken` just adds an entry; no other change is needed.

nothing else belongs in config. the number of signals, particles, cyberlinks, axons, and semcons — all derivable from the `signals` section. keeping them out of config removes the consistency risk of denormalized counts.

## signals

signals are variable-length: a 44-byte header followed by `n` link records of 96 bytes each. header fields are native-aligned so the whole section mmaps and reads without copying.

```
signal header (44 bytes):
  [0..32]   ν   neuron (hemera hash, 32 B)
  [32..40]  t   unix timestamp, seconds (u64 little-endian)
  [40..44]  n   link count (u32 little-endian, n ≥ 1)

link record (105 bytes), repeated n times:
  [0..32]    p   from (hemera hash, 32 B)
  [32..64]   q   to (hemera hash, 32 B)
  [64..96]   τ   token (hemera hash, 32 B)
  [96..104]  a   stake amount (Goldilocks field element, u64 little-endian, a < 2^64 − 2^32 + 1)
  [104..105] v   valence (i8: -1, 0, +1)
```

total signal size = `44 + 105·n` bytes. no padding — fields pack tight.

signals appear in canonical time order: ascending `t`, then (for chain-sourced snapshots) ascending intra-block index. links within a signal appear in commit order — the sequence the neuron chose. `t ≤ config.captured_at`. `τ` is always a valid index into `config.tokens`.

unix seconds is chain-agnostic. a chain-sourced `.graph` sets each signal's `t` from the chain header timestamp of the block the signal landed in; a user-defined graph (e.g. `mytoken`) sets `t` from wall-clock at commit. either way, consumers compare, sort, and range-query signals without knowing anything about blocks.

### what the header carries once

`ν` and `t` are signal-level facts — one neuron, one moment per atomic broadcast. storing them on the header saves redundancy (a 5-link signal stores `ν` and `t` once instead of five times) and preserves the atom the chain (or user) natively produces.

### iteration

```
for signal in signals:
    ν, t, n = signal.header
    for link in signal.links:
        p, q, τ, a, v = link
        # compile using the full tuple (ν, p, q, τ, a, v, t)
```

variable-length records mean random access by signal index needs a side-table; linear scan is what compilers and queriers use. an optional `signals.idx` extension can provide `(offset, block)` entries for fast seek by block height, specified separately when needed.

## parsing

```
1. parse frontmatter (TOML until first ~~~)
2. read ~~~card for display
3. read ~~~config for chain id, block, token table
4. mmap ~~~signals, iterate as variable-length records
5. derive whatever else is needed (particles, links, axons, focus, semcons)
```

five steps. no proof check in the base spec; see `proof` extension below.

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
| proof | stark | recursive STARK binding each signal to the chain apphash — one proof per signal, as the chain emits |
| impulse | binary | sparse `π_Δ` focus delta per signal (lets compilers skip power iteration for proof-carrying signals) |
| signals.idx | binary | (offset, block) side-table for random access by block height |

each extension is specified on its own page when its design is real. mc ignores all of them — the compiler regenerates anything an extension caches.

## relation to .model

a .graph compiles into a .model via the [[compiled transformers spec]] (CT-1):

```
*.graph                 *.model
─────────               ───────
signals        ───►     embedding + attention + MLP (via SVDs on adjacency)
signal.ν       ───►     neuron-level stats in eval; alignment partitions
signal ordering ───►    signal-respecting walks for the MLP pass
config.tokens  ───►     per-denomination stake weighting
config.block   ───►     [lineage].block in the compiled model
BLAKE3(.graph) ───►     [lineage].source in the compiled model
```

the compiled model's `[lineage]` section carries the exact snapshot CID, so every compile is provable against its input.

## why three sections

a `.graph` used to be a tar of jsonl files, a config, a stake dump, a proof — four formats, ad-hoc layouts, no zero-copy load. the first draft of this spec collapsed that into eight sections in one container, which was still more than necessary.

three sections is the floor. every field is either chain fact or committed snapshot policy; nothing is a convenience duplicate; nothing requires an algorithm spec of its own. `head -50 file.graph` tells you the chain, the block, and the denomination weights; mmap the signals section, iterate, compile.

the signals-first design preserves one chain atom per file atom. proofs, impulses, and signal types all have a natural attachment point. the link stream is recovered by iterating links inside signals — no information lost.

---

see [[.cyb|format]] for the base container. see [[cyb-model]] for the inference-side counterpart. see [[cyb-registry]] for the ecosystem catalog.
