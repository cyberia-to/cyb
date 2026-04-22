---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .graph, graph format, cyb graph spec
---

# .graph — knowledge graph snapshot in [[.cyb|format]]

.graph follows the [[.cyb|format]] three rules. a .graph file IS a .cyb file — same parsing, same tools. the extension tells humans and tools: this container holds a cybergraph snapshot at a fixed block height.

three required sections. four documented extensions. one external format ([[cyb-vocab|.vocab]]) for shared particle vocabulary.

## required sections

| name | format | what it does |
|------|--------|-------------|
| card | .md | what this snapshot is |
| config | .toml | chain id, block, capture time, token table, vocab refs |
| signals | .signals | the atomic broadcast units — signed bundles of cyberlinks |

three sections is the floor. `particles` (as a CID list), `cyberlinks` (as a flat stream), `semcons`, `focus`, `spectral_gap` — all derive from `signals` + `config` in milliseconds.

the snapshot preserves the chain's atom: a [[cyber/signal|signal]] is the unit a [[neuron]] commits in one step, carrying an ordered vector of [[cyberlinks]] under one signature, one timestamp, one optional proof. flattening signals into a link stream throws away structure the chain pays for every block. the base spec keeps signals intact.

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

first thing you see. markdown. free-form description with at minimum the chain, snapshot moment, and counts.

```markdown
~~~card
# bostrom-23195000

cybergraph snapshot from bostrom chain at block 23,195,000.
captured 2026-03-23 14:22 UTC. 2,921,230 particles, 540,166 signals,
2,705,332 cyberlinks. license: cyber license.
```

## config

chain identity, snapshot moment, token table, and (optionally) the vocab files the snapshot defers to.

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

[[vocab]]
cid  = "0xaabbccdd..."
name = "bostrom-23000000"

[[vocab]]
cid  = "0xeeff0011..."
name = "mytoken-private"
```

| field | meaning |
|-------|---------|
| chain_id | the chain (or user-defined namespace) the snapshot belongs to |
| block | source-native sequence number; chain block height (omit for non-chain graphs) |
| captured_at | unix seconds when the snapshot was emitted |
| format_version | spec revision (currently `1`) |
| [[tokens]] | per-denomination entries: cid (hemera hash), symbol (human label), weight (multiplier) |
| [[vocab]] | optional list of [[cyb-vocab|.vocab]] files this snapshot defers to, in resolution order |

### tokens

tokens are first-class particles — each denomination is identified by its hemera hash, the same way every other node in the graph is. the link record carries `τ` as the 32-byte CID; the compiler looks up `weight` and `symbol` in this table by content match. weight is integer per-mille: `weight = 1000` means 1.0×.

the table may come from chain governance (if the chain pins it) or from the snapshot publisher (if it is policy). either way, the snapshot commits to these specific weights — different token tables produce different snapshots.

user-defined tokens are declared the same way. a private graph with `mytoken` just adds an entry; no other change is needed.

### vocab

each `[[vocab]]` entry references an external [[cyb-vocab|.vocab]] file by its hemera CID. order matters: when resolving a particle CID to a vocab id, files are searched in declared order — first hit wins. particles found in `signals` but absent from all referenced vocab files are appended to the end at compile time.

a snapshot without any `[[vocab]]` entries derives its vocab entirely from its own signals. snapshots that share vocab files produce models with stable, comparable token id assignments — a particle has the same id across compiles that pull the same vocab.

## signals

signals are variable-length: a 44-byte header followed by `n` link records of 105 bytes each.

```
signal header (44 bytes):
  [0..32]   ν   neuron (hemera hash, 32 B)
  [32..40]  t   unix timestamp, seconds (u64 little-endian)
  [40..44]  n   link count (u32 little-endian, n ≥ 1)

link record (105 bytes), repeated n times:
  [0..32]    p   from (hemera hash, 32 B)
  [32..64]   q   to (hemera hash, 32 B)
  [64..96]   τ   token (hemera hash, 32 B)
  [96..104]  a   stake amount (Goldilocks field element, u64 LE, a < 2^64 − 2^32 + 1)
  [104..105] v   valence (i8: -1, 0, +1)
```

total signal size = `44 + 105·n` bytes. no padding — fields pack tight.

example: a 2-link signal raw bytes (truncated CIDs for readability):

```
header  = AA…AA  ‖  20180101_12:34:56  ‖  02
link 0  = P0…P0  ‖  Q0…Q0  ‖  TBOOT…  ‖  1000  ‖  +1
link 1  = P0…P0  ‖  Q1…Q1  ‖  TBOOT…  ‖  500   ‖  +1
total   = 44 + 2·105 = 254 bytes
```

signals appear in canonical time order: ascending `t`, then (for chain-sourced snapshots) ascending intra-batch index. links within a signal appear in commit order — the sequence the neuron chose. `t ≤ config.captured_at`. `τ` must match a `cid` in `config.tokens`; conforming consumers reject snapshots with unknown token CIDs.

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

variable-length records mean random access by signal index needs a side-table; linear scan is what compilers and queriers use.

## parsing

```
1. parse frontmatter (TOML until first ~~~)
2. read ~~~card for display
3. read ~~~config for chain id, snapshot moment, token table, vocab refs
4. (optional) load referenced .vocab files for stable id assignment
5. mmap ~~~signals, iterate as variable-length records
6. derive whatever else is needed (particles, links, axons, focus, semcons)
```

six steps. extensions (program, proof, impulse, particle) plug into this flow when present.

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
CID(.graph) = hemera(file bytes)
```

two snapshots with the same frontmatter, card, config (including identical token table and vocab refs), and signals produce byte-identical files and the same CID. adding or removing any extension changes the file bytes and therefore the CID.

## extensions

four documented extensions — none required, each with a defined layout. mc ignores them by default; a compiler may opt in to `program`, `impulse`, or `particle` when producing a model with stronger lineage guarantees.

### program

embedded graph operators (focus, semcon discovery, ranking) as trident or rust source. ships with the snapshot for two reasons: provable queries (a verifier re-runs and checks the result against a STARK proof) and algorithm pinning (future versions of focus need not break old snapshots).

frontmatter entry:

```toml
[[files]]
name = "program"
format = "tri"
```

example contents:

```trident
~~~program
module graph.operators

use std.graph.csr

pub fn focus(snap: Snapshot, cfg: Config) -> Vec<u64> {
    let a = build_adjacency(snap, cfg)
    let p = csr.column_normalize(a)
    let mut pi = vec.uniform(cfg.particles)
    for k in 0..256 bounded 256 {
        pi = vec.blend(csr.times(p, pi), vec.uniform(cfg.particles), 850, 1000)
    }
    pi
}
```

every chain ships the same canonical program; user-defined graphs may substitute their own. the program section's hemera CID identifies the operator set the snapshot was produced with.

### proof

per-signal recursive STARK that the signal's links and focus impulse are valid against the chain apphash at the signal's `t`.

frontmatter entry:

```toml
[[files]]
name = "proof"
format = "stark"
size = 41984
```

layout:

```
[0..4]              n   signal count (must equal signals count, u32 LE)
repeated n times:
  [0..4]            len  proof length L (u32 LE)
  [4..4+len]        STARK bytes
```

ordered identically to the signals section. consumers verify against the chain header at `signal.t` (or the published apphash for that block).

### impulse

per-signal sparse focus delta `π_Δ` that the chain proved when the signal was accepted. compilers may sum these across signals to skip the power-iteration step entirely when computing focus.

frontmatter entry:

```toml
[[files]]
name = "impulse"
format = "impulses"
size = 12345678
```

layout:

```
[0..4]                 n  signal count (u32 LE)
repeated n times:
  [0..4]               m  number of nonzero entries for this signal (u32 LE)
  repeated m times:
    [0..32]            cid    (hemera hash of the affected particle)
    [32..40]           delta  (Goldilocks field element, i64 LE — signed)
```

ordered identically to signals.

### particle

inline content for some or all CIDs the snapshot references. the canonical mechanism for bundling particle bytes inside the file. combined with `[[vocab]]` references in config, this covers the spectrum from "topology only, fetch content elsewhere" to "fully self-contained".

frontmatter entry:

```toml
[[files]]
name = "particle"
format = "particles"
size = 8478912
```

layout:

```
[0..4]                 n  particle count (u32 LE)
repeated n times:
  [0..32]              cid    (hemera hash, 32 B)
  [32..40]             len    (u64 LE)
  [40..40+len]         bytes  (raw content; consumer interprets)
```

`n = 0` is a valid empty section. entries appear in any order; a CID present in `~~~particle` overrides any same-CID lookup against vocab files.

a snapshot that wants full self-containment puts every CID referenced by signals into `~~~particle`. a snapshot that wants compact + topology only omits the section entirely and relies on vocab refs in config.

## relation to .model

a .graph compiles into a .model via the [[compiled transformers spec]] (CT-1):

```
*.graph                 *.model
─────────               ───────
signals        ───►     embedding + attention + MLP (via SVDs on adjacency)
signal.ν       ───►     neuron-level stats; alignment partitions
signal ordering ───►    signal-respecting walks for the MLP pass
config.tokens  ───►     per-denomination stake weighting
config.vocab   ───►     stable token id assignment across compiles
config.block   ───►     [lineage].block in the compiled model
hemera(.graph) ───►     [lineage].source in the compiled model
```

the compiled model's `[lineage]` section carries the exact snapshot CID, so every compile is provable against its input.

## why three required sections

a `.graph` used to be a tar of jsonl files, a config, a stake dump, a proof — four formats, ad-hoc layouts, no zero-copy load. the first draft of this spec collapsed that into eight sections in one container, which was still more than necessary.

three required is the floor. every required field is either chain fact or committed snapshot policy; nothing is a convenience duplicate; nothing requires an algorithm spec of its own. the four extensions exist for cases where a publisher wants to ship more (provable program, per-signal proof, per-signal impulse, inline particle bytes); each extension is opt-in and has a defined layout.

`head -50 file.graph` tells you the chain, the moment, the denomination weights, the vocab the snapshot defers to. mmap the signals section, iterate, compile.

---

see [[.cyb|format]] for the base container. see [[cyb-vocab]] for the standalone vocab format. see [[cyb-model]] for the inference-side counterpart. see [[cyb-registry]] for the ecosystem catalog.
