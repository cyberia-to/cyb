---
tags: cyber, cyb, core
crystal-type: process
crystal-domain: cyber
alias: cyb sync, sovereign sync, unified sync
---

# sovereign sync — patches verified at every layer

the unification of [[cyb/fs/patch|patch theory]] and [[structural sync]]. patches are the language of change. structural sync is the infrastructure of trust. together they produce a system where every file operation is a provably correct, provably complete, provably available morphism over the [[cybergraph]].

a [[patch]] IS a [[signal]]. a [[signal]] IS a patch. the same object described by two theories:

| [[patch theory]] | [[structural sync]] | meaning |
|---|---|---|
| morphism P: S₁ → S₂ | signal s = (ν, l⃗, π_Δ, σ) | a change to the graph |
| commutativity (P ⊥ Q) | [[CRDT]] merge (a ⊔ b = b ⊔ a) | order does not matter |
| dependency DAG | [[hash chain]] (prev field) | causal history |
| confluence theorem | [[Verified Eventual Consistency]] | convergence to unique state |
| conflict object Conflict(P,Q) | competing [[particles]] in [[foculus]] π | disagreement as data |

## five layers applied to patches

each patch passes through five verification layers before it becomes part of the canonical [[cybergraph]]. no layer depends on another for its guarantee. a verifier can check any subset.

### layer 1: validity — the patch is correct

every patch carries a [[zheng]] proof σ covering all operations in the batch:

- AddParticle — [[Hemera]] hash matches content
- AddEdge — source and target particles exist (NMT membership proof)
- RemoveEdge — edge exists and author has authority (conviction check)
- ReplaceParticle — old particle referenced correctly, new hash valid
- [[focus]] sufficient for conviction weight

an invalid patch cannot be constructed. the constraint system is the security boundary — not the network, not validators, not honest majority. a single device offline for a year can verify a patch in 50 μs.

### layer 2: ordering — the patch carries its own history

every patch embeds its causal position:

- `prev = H(author's previous patch)` — per-[[neuron]] [[hash chain]]
- `merkle_clock = H(all patches the device has seen)` — compact causal state
- `vdf_proof = VDF(prev, T_min)` — physical time since last patch. [[VDF]] prevents flood: each patch costs minimum T_min wall-clock seconds. equivocation (two patches with same prev) costs 2×T_min — detectable by any peer
- `step` — monotonic counter, gap = missing patch = detectable
- dependency closure — explicit set of patches this patch requires

the [[patch theory]] dependency DAG and the structural sync [[hash chain]] unify here. dependent patches (P → Q) embed P's hash in Q's closure. independent patches (P ⊥ Q) have disjoint closures — application order irrelevant.

deterministic total order from the set of patches, without coordination:
1. causal order (A in B's deps → A before B)
2. [[VDF]] time (lower elapsed → earlier)
3. hash tiebreak (H(A) < H(B) → A before B)

### layer 3: completeness — nothing was omitted

each [[neuron]] commits its patch chain to a per-neuron [[NMT]] (namespaced by step):

```
NMT[ step → patch_hash ]
```

a peer requesting "all patches from neuron N in steps [100, 200]" receives a structural completeness proof. the tree's sorting invariant physically prevents omission — the neuron cannot hide a patch in the requested range.

this closes the critical gap in pure [[patch theory]]: confluence guarantees correct state IF you have all patches in the dependency closure. [[NMT]] proves you DO have all patches. together: provably correct state from a provably complete set.

### layer 4: availability — the data survives

patch content (new [[particles]], modified files) is erasure-coded via 2D Reed-Solomon over [[Goldilocks field]]:

- chunks distributed across device set (local) or neuron set (global)
- any k-of-n chunks reconstruct the original
- [[DAS]]: O(√n) random samples verify availability without downloading

a device dies → patches survive in parity chunks across remaining devices. a neuron goes offline → other neurons holding erasure chunks reconstruct. [[patch theory]] says "if you have the patches, the state is correct." [[DAS]] says "the patches physically exist, even if some holders are gone."

### layer 5: merge — convergence is algebraic

two scales, one principle:

local (devices of one [[neuron]]): [[CRDT]] merge. independent patches commute by the [[commutativity]] theorem. dependent patches apply in topological order from the dependency DAG. the [[G-Set]] of patches grows monotonically — union is the merge function.

global ([[neurons]] in the network): [[foculus]] merge. when two neurons create conflicting patches (P ⊗ Q), both enter the [[cybergraph]]. π convergence assigns each a [[focus]] score weighted by stake. the patch crossing threshold τ becomes canonical. the other persists as history (axiom A3: append-only) but carries zero weight.

the [[patch theory]] conflict resolution maps directly: a resolution patch R has both P and Q in its dependency closure. once R crosses τ, the conflict is resolved network-wide. no voting round. no leader. convergence from topology.

## the lifecycle

```
neuron creates patch on device
  ↓
device: zheng proof generated (layer 1, ~100ms)
  ↓
device: hash chain extended, VDF computed (layer 2, ~1s)
  ↓
device: NMT updated with new step (layer 3, O(log n))
  ↓
device: content erasure-coded across device set (layer 4)
  ↓
local sync: CRDT merge with other devices (layer 5)
  ↓
network: signal submitted to validators
  ↓
global: foculus π-weighted merge (layer 5)
  ↓
canonical: patch enters BBG_root, indexed in NMT
  ↓
queryable: any light client verifies with one proof
```

## what this enables

### sovereign personal computing

a [[neuron]] owns devices. devices run [[CybOS]]. every file operation is a patch. patches sync between devices via layers 1-5. no cloud account. no server. no permission from anyone. the five layers guarantee: if your devices have the patches, your state is correct (layer 1), ordered (layer 2), complete (layer 3), available even if a device dies (layer 4), and convergent (layer 5).

### offline-first without compromise

devices work offline for weeks. patches accumulate locally. when they reconnect:
- [[merkle clock]] comparison: O(1) — one hash, equal means in sync
- exchange missing patches with [[NMT]] completeness proofs — nothing hidden
- [[CRDT]] merge — deterministic, no conflicts for independent changes
- conflicts materialized as first-class objects — not "sync failed"

### agent collaboration at scale

multiple [[neurons]] and agents create patches simultaneously. no coordination required during creation — only the five layers at merge time. a [[GFlowNet]] agent proposes patches weighted by expected Δπ. an [[active inference]] agent stakes on patches that minimize [[free energy]]. [[foculus]] resolves disputes through π convergence — stake-weighted, mathematically determined, no politics.

### verifiable version control

every patch is provably correct (zheng), causally ordered (hash chain + VDF), complete in context (NMT), physically available (DAS), and deterministically merged (CRDT/foculus). this is [[git]] with mathematical guarantees instead of social trust. no force-push. no rebase that rewrites history. no "trust the server." the [[hash chain]] is append-only, the proofs are unforgeable, and the merge is algebraic.

### light client repositories

a new device joins a neuron's repository:
1. download checkpoint (BBG_root + folding accumulator)
2. verify ONE [[zheng]] proof — proves all history valid (~50 μs)
3. sync namespaces of interest with [[NMT]] completeness proofs
4. sample availability with [[DAS]] (~20 samples)

no replay of all patches. no download of full repository. no trust in the peer serving data. the proof speaks for itself.

## relationship to existing systems

| system | patches | validity | ordering | completeness | availability | merge |
|---|---|---|---|---|---|---|
| [[git]] | snapshot diffs | hash integrity | DAG (no proof) | clone all | full replication | manual / 3-way |
| [[IPFS]] | none | CID hash | none | none | DHT (best effort) | none |
| [[Automerge]] | [[CRDT]] ops | type safety | Lamport clock | none | none | [[CRDT]] auto |
| Pijul | categorical | hash integrity | dependency DAG | clone all | full replication | commutative |
| [[Celestia]] | blobs | [[NMT]] + DA | Tendermint | [[NMT]] proofs | [[DAS]] | N/A (DA only) |
| cyb/fs/sync | categorical | [[zheng]] proof | chain + [[VDF]] | [[NMT]] proof | [[DAS]] + erasure | [[CRDT]] / [[foculus]] |

cyb/fs/sync is the first system combining categorical patch semantics with all five verification layers. Pijul has the algebra. Celestia has layers 3-4. Automerge has layer 5. git has social trust. cyb/fs/sync has all five, unified.

see [[cyb/fs/patch]] for patch algebra and five primitive operations. see [[structural sync]] for the five-layer theory and consequences. see [[sync]] for the full protocol specification. see [[foculus-vs-crdt]] for why layer 5 splits at the local/global boundary. see [[cyb/fs]] for the filesystem model. see [[CybOS]] for the operating system
