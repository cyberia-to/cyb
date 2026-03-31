# cyber-sync: local device data availability

structural sync at local scale — erasure-coded virtual filesystem across a neuron's device set.

## architecture

```
nebu (field)  ✓ ready
  ↓
hemera (hash + NMT)  ✓ ready
  ↓
erasure (RS over Goldilocks)        ← this crate
  ↓
das (2D grid + sampling)            ← this crate
  ↓
store (chunk store + CRDT)          ← this crate
  ↓
device (sync over radio/iroh)       ← this crate
  ↓
vdisk (virtual disk manager)        ← this crate
  ↓
cli (create-disk, attach, status)   ← this crate
```

## modules

### erasure — Reed-Solomon codec over Goldilocks field

encode: polynomial evaluation at n points via NTT
decode: polynomial interpolation from k points via inverse NTT
2D grid: √n × √n data → 2√n × 2√n with parity rows/columns

~1,500 LOC

### das — data availability sampling

2D grid commitment (Hemera hash per row/column)
random cell sampling with inclusion proof
sample verification against committed roots
fraud proof: k+1 cells from a row → proof of bad encoding

~2,500 LOC

### store — content-addressed chunk storage + CRDT

chunk store: Hemera hash → bytes on disk
chunk index: which chunks are local, capacity tracking
LRU cache: hot/cold promotion, eviction
G-Set CRDT: append-only file metadata, commutative merge

~2,000 LOC

### device — sync daemon over radio

device discovery via mDNS (leverage iroh)
chunk request/response protocol (want chunk X → chunk X + proof)
background rechunking on device join/leave
health monitoring, capacity alerts

~3,000 LOC

### vdisk — virtual disk manager

create-disk: name, redundancy (f), tier, cache policy
attach: device → disk allocation, capacity accounting
placement: capacity-weighted chunk distribution (bin-packing)
transparent fetch: local hit → serve, miss → peer → verify → serve
rebalance: priority queue (tier 0 → 1 → 2)

~2,500 LOC

### cli — command-line interface

```
cyber-sync create-disk keys --redundancy max --tier 0
cyber-sync create-disk work --redundancy 1 --tier 1
cyber-sync attach phone work --capacity 50GB
cyber-sync status
cyber-sync health
```

~1,000 LOC

## structural sync mapping

```
layer   mechanism           local (devices)
─────   ─────────           ───────────────
1       validity            chunk encoding proven (RS commitment)
2       ordering            hash chain per device (causal order)
3       completeness        NMT per namespace (nothing withheld)
4       availability        erasure coding + DAS (data survives f losses)
5       merge               CRDT (cooperative, same neuron)
```

## dependencies

- nebu (Goldilocks field arithmetic, NTT)
- cyber-hemera (Poseidon2 hash, Merkle trees)
- radio/iroh (P2P transport, mDNS discovery) — future phase

## MVP target

~17,000 LOC total. testable end-to-end:
encode file → erasure code → distribute chunks → lose one → reconstruct from remaining
