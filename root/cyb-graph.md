---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .graph, graph format, cyb graph spec
---

# .graph — knowledge graph snapshot in [[.cyb|format]]

.graph follows the [[.cyb|format]] three rules. a .graph file IS a .cyb file — same parsing, same tools. the extension tells humans and tools: this container holds a cybergraph snapshot at a fixed block height.

one file. ready to compile, ready to query, ready to verify against chain.

## required files

| name | format | what it does |
|------|--------|-------------|
| card | .md | what this snapshot is, which chain, which block |
| config | .toml | chain id, block height, counts, root hashes, lineage |
| program | .tri or .rs | graph operators: focus, semcon discovery, ranking |
| particles | .toml | particle index: id → CID + first-seen block |
| tokens | .toml | token denominations: id → symbol + weight |
| eval | .toml | graph metrics: spectral gap, focus entropy, diameter |
| cyberlinks | .records | the seven-tuple records (binary, fixed 128 B each) |
| proof | .stark | merkle/STARK proof binding snapshot to chain root |

no optional files. everything is required. proof may be the empty proof for unsigned local snapshots; production snapshots from go-cyber always carry a real proof.

program reads all params from config — one program works for any snapshot of the same chain. change the snapshot → different graph state, same operators.

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
name = "program"
format = "tri"

[[files]]
name = "particles"
format = "toml"

[[files]]
name = "tokens"
format = "toml"

[[files]]
name = "eval"
format = "toml"

[[files]]
name = "cyberlinks"
format = "records"
size = 346281344

[[files]]
name = "proof"
format = "stark"
size = 8192
```

## card

first thing you see. markdown.

```markdown
~~~card
# bostrom-23195000

cybergraph snapshot from bostrom chain at block 23,195,000.

2,921,230 particles. 2,705,332 cyberlinks. 1,240 active neurons.
captured 2026-03-23 14:22 UTC.

verifies against bostrom apphash QmXyZ...
license: cyber license
```

## config

everything about the snapshot. integers only — no floats.

```toml
~~~config
chain_id = "bostrom-1"
block = 23195000
captured_at = 1742740920
particles = 2921230
cyberlinks = 2705332
neurons = 1240
axon_particles = 1842110
semcons_registered = 12

[provenance]
source = "go-cyber"
node_version = "0.5.0"
apphash = "0x9f3c..."
state_root = "0x1a2b..."

[lineage]
prev_snapshot = "blake3:...:bostrom-23000000"
delta_blocks = 195000
delta_links = 142331
```

| section | what it holds |
|---------|---------------|
| top-level | chain id, block, capture time, counts |
| [provenance] | which node produced it, app hash, state root |
| [lineage] | parent snapshot CID and delta stats |

## program

graph operators as source code. reads all params from config — not hardcoded. the same program runs on any snapshot from any chain.

```trident
~~~program
module graph.operators

use vm.io.io
use vm.core.convert
use std.graph.csr
use std.graph.iter

// folds raw cyberlinks into stake-weighted CSR adjacency
pub fn build_adjacency(snap: Snapshot, cfg: Config) -> Csr {
    let a = csr.empty(cfg.particles, cfg.particles)
    for link in iter.cyberlinks(snap) bounded 16777216 {
        let w = effective_stake(link.amount, link.valence, link.token)
        if w > 0 {
            csr.add(a, link.from, link.to, w)
        }
    }
    a
}

// computes focus distribution by power iteration
pub fn focus(snap: Snapshot, cfg: Config) -> Vec<u64> {
    let a = build_adjacency(snap, cfg)
    let p = csr.column_normalize(a)
    let mut pi = vec.uniform(cfg.particles)
    for k in 0..256 bounded 256 {
        let next = csr.times(p, pi)
        let blended = vec.blend(next, vec.uniform(cfg.particles), 850, 1000)
        if vec.l1_diff(blended, pi) < 100 { break }
        pi = blended
    }
    pi
}

// discovers registered semcons via axon scan + label scoring
pub fn discover_semcons(snap: Snapshot) -> Vec<Particle> {
    let omega = iter.cyberlinks(snap) | map(|l| hash(l.from, l.to)) | set
    let labels = iter.cyberlinks(snap) | filter(|l| omega.has(l.to))
    let scores = labels | group_by(|l| l.from) | map(score_label)
    scores | top_by_threshold(1000)  // θ = 0.1% of max
}
```

operators compose: `compile_transformer(snap)` runs `build_adjacency` → `focus` → `discover_semcons` → ranks.

| | trident | rs |
|--|---------|-----|
| compiles to | [[nox]] (18 instructions) | native binary |
| proof | [[zheng]] witness every execution | none |
| use | provable graph queries, on-chain verification | fast local compilation, indexer |

## particles

TOML index. one entry per particle. id matches the offset used in cyberlink records.

```toml
~~~particles
[0]
cid = "0x1a2b3c4d..."
first_seen = 1
kind = "content"

[1]
cid = "0x5e6f7a8b..."
first_seen = 1
kind = "content"

[2]
cid = "0x9c0d1e2f..."
first_seen = 47
kind = "axon"
from = 0
to = 1
```

| field | meaning |
|-------|---------|
| cid | 32-byte hemera hash, lowercase hex |
| first_seen | block height of first cyberlink mentioning this particle |
| kind | `content` (real particle) or `axon` (induced via H(p,q)) |
| from, to | only present when kind = "axon"; ids of the endpoints |

axon-particles are derivable but stored explicitly so consumers can mmap the index without rehashing. the in-memory representation is just two `Vec<u8; 32>` columns; toml is the on-disk human-readable form.

## tokens

token denominations and their relative weight. analog of vocab in .model.

```toml
~~~tokens
[0]
symbol = "BOOT"
decimals = 0
weight = 1000

[1]
symbol = "VOLT"
decimals = 0
weight = 50000

[2]
symbol = "AMPERE"
decimals = 0
weight = 1000000

[3]
symbol = "HYDROGEN"
decimals = 0
weight = 100000000
```

`weight` is the multiplier applied to stake amounts when computing effective stake. integer per-mille values: weight 1000 = 1.0×, weight 50000 = 50.0×. matches the integer convention from .model config.

cyberlink records reference token by index (u32 in the binary record), not symbol. rebinding a chain to a different weight schedule is a config-only change.

## eval

graph-level metrics, computable from the snapshot. routing and compilation read these to size architectures.

```toml
~~~eval
[topology]
spectral_gap = 13       # λ₂ × 100, per-cent
diameter = 10           # BFS lower bound
density = 314           # ρ × 10⁹, parts per billion
clustering = 421        # ⟨C⟩ × 1000

[focus]
entropy_bits = 13730    # H(π) × 1000
top_concentration = 1040  // top particle's focus, per-mille of total

[semcons]
registered = 12
default_share = 600     // fraction of edges in default bucket, per-mille

[stake]
gini = 873              // gini × 1000
top_neuron_share = 184  // per-mille
```

per-mille and integer-scaled, same convention as .model `eval`. updatable as the snapshot is re-analyzed.

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

records appear in canonical chain order: ascending block height, then ascending intra-block index. snapshot consumers must preserve this order — it determines particle ids in the index, attention head ordering in compiled transformers, and proof reproducibility.

cids in the record reference the `particles` table by content-address (full 32-byte hash). resolving a cid to its index is O(log n) via binary search over the sorted particle table, or O(1) via a built side-table.

## proof

binary STARK proof attesting:

1. the `cyberlinks` records are exactly the union of all cyberlinks committed to chain at heights `[1, config.block]`
2. the `apphash` in `[provenance]` matches chain consensus at that height
3. the `state_root` is reachable from genesis via verifiable execution

go-cyber emits this proof as part of the snapshot RPC. verifiers replay the STARK against the published chain headers — no need to re-execute the chain. local snapshots (developer tools, replays) write an empty proof; the runtime gates compilation behind a `proof_required` flag in `[provenance]`.

```
proof layout:
  [0..32]      version + circuit id
  [32..96]     public inputs hash (BLAKE3 of frontmatter + counts)
  [96..end]    STARK proof bytes (variable, sized in frontmatter)
```

## runtime load

```
file.graph
  → parse frontmatter
  → read ~~~card (display)
  → read ~~~config → chain id, block, counts
  → verify ~~~proof against ~~~config (if proof_required)
  → compile ~~~program(config) → hardware kernels (cached)
  → read ~~~particles → particle index (zero-copy mmap)
  → read ~~~tokens → denomination weights
  → read ~~~eval → routing data
  → mmap ~~~cyberlinks → typed slice of records
  → graph operations ready
```

## go-cyber integration

go-cyber emits .graph natively from two endpoints:

```
GET /cyber/graph/snapshot?block=H        → bostrom-H.graph
GET /cyber/graph/snapshot?block=latest   → bostrom-latest.graph
```

snapshot endpoints stream the file; clients can pipe directly into a compiler:

```
curl -s https://node.bostrom.cybernode.ai/cyber/graph/snapshot?block=23195000 \
  | mc - -o bostrom-23195000.model
```

snapshot validity is also checkable offline with a single CLI:

```
cyb-graph verify bostrom-23195000.graph
  → frontmatter ✓
  → particles count matches config ✓
  → cyberlinks count matches config ✓
  → proof verifies against apphash 0x9f3c... ✓
  → snapshot is valid
```

## relation to .model

a .graph compiles into a .model via the [[compiled transformers spec]] (CT-1):

```
*.graph                 *.model
─────────               ───────
particles      ───►     embed.weight rows
cyberlinks     ───►     attention W_Q, W_K, W_V
focus(graph)   ───►     embedding ranking
semcons        ───►     attention head count
diameter,κ     ───►     layer count
```

both follow the same .cyb three-rule contract, so the same parser walks both. compiled output references the source snapshot in its `[lineage]` section by CID — every compiled model carries a verifiable pointer to the exact graph state that produced it.

## why .graph

a knowledge graph snapshot used to be a tar of jsonl files plus a config plus a stake dump plus a proof — four formats, ad-hoc layouts, no zero-copy load. .graph collapses all of that into one self-describing container that mmaps directly, parses in milliseconds, and verifies in seconds. `head -50 file.graph` tells you everything; `cyb-graph verify` tells you it is real.

three rules. frozen.

see [[.cyb|format]] for the base container. see [[cyb-model]] for the inference-side counterpart. see [[cyb-registry]] for the ecosystem catalog.
