---
tags: cyber, core, research
crystal-type: pattern
crystal-domain: cyber
crystal-size: deep
alias:: cyberpatch spec, cyberpatch specification, patch spec
---

# CyberPatch: Specification v0.1
## A Content-Addressed, Identity-Sovereign Patch System for Planetary-Scale Knowledge Networks

The mathematical foundations of [[patch theory]] (Pierre-Etienne Meunier et al.) inspired this design, built from first principles for the [[cyber]] ecosystem.

---

## 1. Motivation and First Principles

### 1.1 The Problem with Snapshot-Based Version Control

Traditional version control systems (Git, SVN, Mercurial) model repository state as a sequence of snapshots. A commit records the complete state of the working tree at a point in time. Merging is an operation over two snapshots relative to a common ancestor — a fundamentally 3-way comparison that is:

- Order-dependent: the result of merging A into B differs from B into A in edge cases
- History-dependent: rebasing rewrites identity, creating phantom conflicts
- Human-centric: designed for sequential human workflow, not parallel agent execution
- Conflict-opaque: conflicts are byproducts of snapshot comparison, not first-class objects

For planetary-scale agent networks where thousands of agents modify a shared [[knowledge]] graph simultaneously, snapshot-based VCS is a fundamental architectural mismatch.

### 1.2 The Patch Theory Insight

The mathematical theory of patches (rooted in the work of Meunier on the categorical semantics of version control) models repository state as a set of changes rather than a sequence of snapshots. This shift has profound consequences:

Key insight: If changes are represented as morphisms in an appropriate category, and independent changes commute, then:

```
apply(P₁, apply(P₂, S)) = apply(P₂, apply(P₁, S))
```

for any two independent patches P₁, P₂ applied to state S. Merging becomes set union. Conflicts become first-class mathematical objects with well-defined structure, not algorithmic failures.

### 1.3 Why This Maps to the Cybergraph

The [[cyber]] [[cybergraph]] already models [[knowledge]] as:

- [[Particles]]: content-addressed knowledge particles
- [[Cyberlinks]]: signed, weighted, timestamped directed edges between particles
- [[Neurons]]: agents with identity, stake, and [[focus]]

A version control system for this ecosystem should be native to these primitives, not a foreign layer bolted on. CyberPatch achieves this by:

- Treating patches as [[cyberlinks]] between repository states
- Treating repository snapshots as [[particles]] (content-addressed)
- Using [[neuron]] identity as author identity
- Integrating focus vector π as patch prioritization signal
- Using Δπ (focus shift) as the economic signal for patch reward

### 1.4 Design Axioms

```
A1. Content addressing is the only stable identity.
A2. All changes are cryptographically attributed.
A3. Independent changes must commute — no exception.
A4. Conflicts are data, not errors.
A5. No global recompute for local change.
A6. Agent and human workflows are equivalent primitives.
A7. Post-quantum cryptography from genesis.
A8. The system must scale to 10¹⁵ tracked objects.
```

---

## 2. Mathematical Foundations

### 2.1 Categories and Patches

Let Repo be a category where:

- Objects are repository states `S` (sets of tracked content [[particles]])
- Morphisms are patches `P: S₁ → S₂`
- Composition is sequential patch application: `P₂ ∘ P₁`
- Identity morphism is the null patch `ε` (no change)

A patch `P` is well-defined independently of the path through history that produced the source state `S₁`. This is the key departure from git, where commits encode both change and position in a linear history.

### 2.2 Patch Dependency

For two patches `P` and `Q` acting on state `S`:

Independent (`P ⊥ Q`): `P` and `Q` operate on disjoint regions of `S`.
Then: `apply(Q, apply(P, S)) = apply(P, apply(Q, S))` — they commute.

Dependent (`P → Q`): `Q` operates on content created or modified by `P`.
Then: `Q` cannot be applied without first applying `P`. `P` is in the dependency closure of `Q`.

Conflicting (`P ⊗ Q`): `P` and `Q` make incompatible changes to the same region.
Then: conflict is a first-class object, not a failure. It can be:
- Resolved (a new patch `R` is the resolver)
- Left in the state (the state holds both versions simultaneously)
- Arbitrated by [[consensus]] ([[focus]] vector π selects the winner)

### 2.3 The Dependency Graph

Define `D = (P, E)` where `P` is the set of all patches and `E ⊆ P × P` where `(P₁, P₂) ∈ E` iff `P₁ → P₂` (P₁ is a dependency of P₂). `D` must be a DAG — no circular dependencies.

The dependency closure of a patch `Q` is:
```
closure(Q) = {P ∈ P | P →* Q}
```
where `→*` is the transitive closure of `→`.

Applying `Q` to any state `S` requires first applying all patches in `closure(Q)` in any topological ordering of `D`. The result is the same for all valid orderings (confluence theorem — to be proved in formal verification).

### 2.4 Conflicts as Algebraic Objects

A conflict `C(P, Q)` between patches P and Q over state S is itself a typed object with structure:

```
Conflict {
    lhs: Patch,           // P's version
    rhs: Patch,           // Q's version
    region: ContentRange, // affected region in S
    resolution: Option<Patch> // R such that apply(R, conflict_state) = resolved_state
}
```

A conflict resolution patch `R` has both `P` and `Q` in its dependency closure. Once applied, the conflict is permanently resolved across all channels — a fundamental improvement over git where conflict resolutions must be repeated per-branch.

### 2.5 Patch Identity and Hashing

The identity of a patch is its content [[hash]] — a deterministic function of:

1. The set of primitive operations in the patch
2. The content hashes of all dependency patches
3. The author's [[public key]]
4. The author's [[signature]] over (1) and (2)
5. A timestamp (monotonic, not wall clock)

```
patch_id = H(ops || dep_hashes || pubkey || signature || timestamp)
```

where `H` is a collision-resistant hash function (see §4 for post-quantum hash selection).

This means the same logical change by the same author at the same time always produces the same patch ID, making patches content-addressed and globally unique without a central registry.

---

## 3. Core Ontology

### 3.1 Primitive Types

```
/// A content-addressed particle of tracked data
Particle {
    cid:     CID,          // content identifier (hash of content)
    size:    u64,          // byte size
    mime:    Option<str>,  // content type hint
    // payload stored off-graph via CID-verified blob store
}

/// A primitive change operation — the irreducible unit of mutation
Operation {
    kind: OperationKind,
    target: CID,           // CID of particle being affected
    payload: Option<CID>,  // CID of new content (for additions/replacements)
}

OperationKind =
    | AddParticle       // introduce new particle to tracked set
    | RemoveParticle    // remove particle from tracked set
    | AddEdge(from: CID, to: CID, kind: EdgeKind)   // link two particles
    | RemoveEdge(from: CID, to: CID, kind: EdgeKind)
    | ReplaceParticle(old: CID, new: CID)             // atomic content swap

/// A signed, dependency-linked set of operations
Patch {
    id:           PatchID,          // H(content) — see §2.5
    ops:          Vec<Operation>,   // ordered list of primitive ops
    deps:         Set<PatchID>,     // explicit dependency set
    author:       NeuronID,         // author identity (see §4)
    signature:    Signature,        // post-quantum signature
    timestamp:    u64,              // monotonic counter (chain height or logical clock)
    metadata:     Option<CID>,      // CID of off-graph metadata blob
}

/// A named, mutable pointer to a set of patches
Channel {
    name:         ChannelName,
    patches:      Set<PatchID>,     // the channel state IS this set
    head:         Option<PatchID>,  // latest applied patch (for UI convenience)
    owner:        NeuronID,
}

/// A repository — collection of channels over a shared particle space
Repository {
    id:           CID,              // hash of genesis state
    channels:     Map<ChannelName, Channel>,
    particles:    Set<CID>,         // union of all tracked particles
    focus:        FocusVector,      // π — computed by tri-kernel ranking
}
```

### 3.2 State Derivation

Given a channel `C` with patch set `P_C`, the derived state `state(C)` is the repository as it would appear after applying all patches in `P_C` in any valid topological order. The confluence property guarantees this is unique.

```
state(C) = fold(topological_sort(closure_of(P_C)), empty_state, apply)
```

State is never stored directly — it is always derived from the patch set. This is the fundamental storage inversion that enables the commutativity properties.

### 3.3 The Cybergraph Embedding

Every CyberPatch repository is simultaneously a [[cybergraph]] subgraph:

```
Patch       ↔  Cyberlink (signed, timestamped, weighted by Δπ)
Particle    ↔  Particle (content-addressed node)
Channel     ↔  Focus subgraph (named view over the global graph)
Repository  ↔  Named neuron-owned subgraph
Author      ↔  Neuron (identity + stake + focus vector)
```

This embedding is not metaphorical — CyberPatch repositories ARE [[cybergraph]] structures and can be queried, ranked, and rewarded by the [[cyber]] [[consensus]] layer directly.

---

## 4. Identity and Cryptography

CyberPatch inherits the [[cyber]] cryptographic stack — it adds no primitives of its own

### 4.1 Cryptographic Primitives

all primitives come from the protocol layer:

- [[hash]]: [[Poseidon2]]-Goldilocks (see [[hemera/spec]]). 64-byte digests, [[stark]]-native, single canonical function for all content addressing
- [[signature]]: post-quantum from [[genesis]]. the specific scheme is a protocol-level decision (see [[cyber/security]])
- proofs: [[starks]] over [[Goldilocks field]] ($p = 2^{64} - 2^{32} + 1$), verified by [[nox]] programs
- identity: [[neuron]] = [[public key]], derived from [[spell]]. see [[cyber/particle]] for CID structure

### 4.2 Neuron Identity in CyberPatch

a [[neuron]] authors patches using the same keypair that signs [[cyberlinks]]:

```
NeuronID = H(public_key)  // Hemera hash, 64-byte identifier

PatchAuthor {
    public_key:  NeuronPublicKey,
    neuron_id:   NeuronID,          // derived via Hemera
}
```

the neuron_id is the stable external identifier. keypair rotation is handled at the protocol level (on-chain rotation proof) — CyberPatch trusts the current binding

### 4.3 Patch Signing

```
patch_content = ops || deps || author_id || timestamp
patch_id      = H(patch_content || signature)
signature     = sign(secret_key, H(patch_content))
```

verification:
```
valid = verify(author.public_key, H(patch_content), signature)
     && patch_id == H(patch_content || signature)
     && all dep_ids are known and valid
```

where H is the protocol [[hash]] function and sign/verify use the protocol [[signature]] scheme

### 4.4 Identity Resolution

neuron IDs are resolved to [[public key]]s through the [[cyber]] [[name]] system:

- direct resolution: `neuron_id → public_key` via on-chain registry
- name resolution: `cyber-name.cyber → neuron_id → public_key`
- CID resolution: `cid → blob content` via distributed blob store
- no URL dependency: no HTTP endpoints required for core operations

this enables cloning repositories by CID or neuron_id without DNS or centralized infrastructure

---

## 5. Patch Algebra

### 5.1 Primitive Operations

Primitive operations are the irreducible atoms of change. All higher-level operations are composed from these:

```
Op = AddParticle(cid: CID)
   | RemoveParticle(cid: CID)
   | AddEdge(from: CID, to: CID, label: CID)
   | RemoveEdge(from: CID, to: CID, label: CID)
   | ReplaceParticle(old: CID, new: CID)
```

Invariants:
- `RemoveParticle(x)` requires `AddParticle(x)` in dependency closure
- `RemoveEdge(a,b,l)` requires `AddEdge(a,b,l)` in dependency closure
- `ReplaceParticle(old, new)` requires `AddParticle(old)` in dependency closure
- Edges can only reference [[particles]] present in the current state

### 5.2 Commutativity Rules

Two operations `op₁` and `op₂` commute (`op₁ ⊥ op₂`) iff:

```
apply(op₂, apply(op₁, S)) = apply(op₁, apply(op₂, S))  ∀S
```

Commutativity table:

| op₁ \ op₂      | AddParticle(y) | RemoveParticle(y) | AddEdge(a,b,l) | RemoveEdge(a,b,l) | ReplaceParticle(old,new) |
|----------------|-----------|--------------|---------------|------------------|-------------------|
| AddParticle(x)     | x≠y: ✓   | x≠y: ✓      | x∉{a,b}: ✓   | x∉{a,b}: ✓      | x∉{old,new}: ✓   |
| RemoveParticle(x)  | x≠y: ✓   | x≠y: ✓      | x∉{a,b}: ✓   | x∉{a,b}: ✓      | x≠old: ✓          |
| AddEdge        | —         | —            | (a,b,l)≠(a',b',l'): ✓ | different edge: ✓ | edge unchanged: ✓ |

When `x = y` or operations touch the same edge: conflict (see §5.3).

### 5.3 Conflict Types

```
ConflictKind =
    | DoubleAdd(cid: CID)          // two patches add same particle with different content
    | DeleteDelete(cid: CID)       // two patches delete same particle (benign — same result)
    | EditDelete(cid: CID)         // one patch edits, another deletes
    | DoubleEdit(cid: CID)         // two patches replace same particle differently
    | EdgeConflict(from,to,label)  // conflicting edge operations
```

`DeleteDelete` is a zombie conflict — both patches achieve the desired result. It is auto-resolved without user/agent intervention.

All other conflicts are stored as first-class state. A conflicted repository is valid — it can be read, cloned, and further patched. Resolution patches are ordinary patches with the conflicting patches in their dependency set.

### 5.4 Patch Application Algorithm

```
fn apply(patch: Patch, state: State) -> Result<State, ApplyError> {
    // 1. Verify signature
    verify_signature(patch)?;

    // 2. Verify all dependencies are present in state
    for dep_id in patch.deps {
        state.contains_patch(dep_id)?;
    }

    // 3. Apply each operation, collecting conflicts
    let mut new_state = state.clone();
    let mut conflicts = Vec::new();

    for op in patch.ops {
        match apply_op(op, &new_state) {
            Ok(updated) => new_state = updated,
            Err(Conflict(c)) => conflicts.push(c),
        }
    }

    // 4. Add patch to state's patch set (even with conflicts)
    new_state.add_patch(patch.id);
    new_state.add_conflicts(conflicts);

    Ok(new_state)
}
```

Key property: application never fails due to conflicts. Conflicts are accumulated as state data.

### 5.5 Dependency Resolution

When applying a patch whose dependencies are not yet locally present:

```
fn apply_with_resolution(patch: Patch, state: State, store: PatchStore) -> Result<State, Error> {
    let missing = patch.deps - state.patch_set();

    if missing.is_empty() {
        return apply(patch, state);
    }

    // Recursively fetch and apply missing dependencies
    let mut current = state;
    for dep_id in topological_sort(missing, store)? {
        let dep_patch = store.fetch(dep_id)?;
        current = apply_with_resolution(dep_patch, current, store)?;
    }

    apply(patch, current)
}
```

---

## 6. Graph Model

### 6.1 Repository as Directed Hypergraph

A repository state is formally a directed labeled hypergraph `G = (V, E, L)` where:

- `V ⊆ CID`: set of [[particle]] content identifiers (vertices)
- `E ⊆ V × V × CID`: directed labeled edges (from, to, label)
- `L: CID → Blob`: off-graph content store

Labels are themselves CIDs — edge semantics are content-addressed, not hardcoded. This means the relationship ontology is extensible without schema changes.

### 6.2 Graph Operations as Patch Operations

All graph mutations reduce to the primitive patch operations defined in §5.1:

```
// Create a new node with content
create_node(content: Bytes) → AddParticle(H(content))

// Delete a node and all its edges
delete_node(cid: CID) →
    [RemoveEdge(cid, _, _) for all edges from cid] ++
    [RemoveEdge(_, cid, _) for all edges to cid] ++
    [RemoveParticle(cid)]

// Create a typed relationship
add_relation(from: CID, to: CID, relation_type: CID) →
    AddEdge(from, to, relation_type)

// Rename / update content
update_content(old: CID, new_content: Bytes) →
    let new = H(new_content) in
    ReplaceParticle(old, new)
```

### 6.3 Filesystem View (Optional Projection)

For human-readable interaction, a filesystem namespace can be projected over the graph:

```
FilesystemView {
    // Maps filesystem paths to particle CIDs
    tree: Map<Path, CID>
}
```

A filepath change is a patch to the tree map, not to content. Content changes are patches to particle content. The two are independent and can be composed:

```
// Rename without changing content = patch to tree only
rename("/old/path", "/new/path") → ReplaceParticle(path_particle_old, path_particle_new)

// Change content without renaming = patch to particle content only
edit("/path", new_content) → ReplaceParticle(content_cid_old, new_content_cid)
```

This eliminates the git confusion between "[[file]] rename" and "file modification" detection.

---

## 7. Channel Theory

### 7.1 Channels as Named Views

A channel is a named, mutable pointer to a subset of the global patch DAG. Formally:

```
Channel = (name: ChannelName, patches: Set<PatchID>)
```

The state of a channel is the graph derived from applying its patch set. Two channels sharing patches share that history — there is no copying.

### 7.2 Channel Operations

Create channel from current state:
```
fork(source: Channel, new_name: ChannelName) → Channel {
    name: new_name,
    patches: source.patches.clone()  // O(1) — set copy
}
```

Merge channels — add patches from one channel to another:
```
merge(source: Channel, target: Channel) → Channel {
    name: target.name,
    patches: target.patches ∪ source.patches  // set union
}
```

Note: merge is exactly set union. There is no merge commit. There is no common ancestor computation. Conflicts that arise are already encoded in the patch DAG.

Cherry-pick — apply specific patches:
```
cherry_pick(patches: Set<PatchID>, target: Channel) → Channel {
    let to_apply = patches ∪ closure_of(patches);  // include deps
    name: target.name,
    patches: target.patches ∪ to_apply
}
```

Revert — remove patches:
```
revert(patches: Set<PatchID>, target: Channel) → Channel {
    // Can only remove patches with no dependents still in channel
    let removable = patches - has_dependents_in(patches, target.patches);
    name: target.name,
    patches: target.patches - removable
}
```

### 7.3 Channel Identity

Channels are named within a repository namespace:

```
channel_ref = "neuron_id/repo_name:channel_name"
// e.g. "abc123.../cyber:main"
//      "cyber-name.cyber/whitepaper:draft-v2"
```

Resolution order:
1. Local neuron_id lookup
2. Blockchain [[name]] → neuron_id → repository
3. CID direct resolution (for immutable historical snapshots)

### 7.4 Convergence Properties

Given two replicas `R₁` and `R₂` of the same repository that diverge (receive different patches independently) and then sync:

Theorem (Eventual Consistency): If `R₁.patches ∪ R₂.patches` form a valid patch DAG (no missing dependencies), then `state(sync(R₁, R₂)) = state(sync(R₂, R₁))`.

Proof sketch: State derivation is a function of the patch set alone (order-independent by commutativity). Sync produces the same set union regardless of direction. QED (formal proof to be included in verification annex).

This result aligns with the [[collective focus theorem]] — convergence of distributed state under commutative operations.

---

## 8. Content Addressing and Transport

### 8.1 CID Format

Content identifiers follow a self-describing format:

```
CID = H(content)  // raw 64-byte Hemera digest

No multicodec prefix, no multihash header, no version byte.
See [[hemera/spec]] for rationale and format.
Inside the protocol, the 64-byte digest is the complete identifier.
```

### 8.2 Blob Store (Off-Graph Payloads)

All content payloads are stored off the live graph in a content-addressed blob store. The graph holds only CIDs. Blob store backends are pluggable:

```
BlobStore trait {
    fn get(cid: CID) -> Result<Bytes, Error>;
    fn put(content: Bytes) -> CID;
    fn has(cid: CID) -> bool;
}

// Implementations:
LocalBlobStore    // filesystem, for local repos
IPFSBlobStore     // IPFS / Kubo compatible
ArweaveBlobStore  // permanent archival
CyberBlobStore    // native nox DA layer
MemoryBlobStore   // for testing
```

### 8.3 Patch Wire Format

Patches are serialized as CBOR (RFC 7049) for network transport:

```cbor
{
    "v": 1,                        // protocol version
    "id": bytes(32),               // patch_id
    "ops": [                       // operations array
        {"k": "AddParticle", "cid": bytes(36)},
        {"k": "AddEdge", "from": bytes(36), "to": bytes(36), "label": bytes(36)},
        ...
    ],
    "deps": [bytes(32), ...],      // dependency patch ids
    "author": bytes(32),           // neuron_id
    "pubkey": bytes(1952),         // dilithium public key
    "sig": bytes(3293),            // dilithium signature
    "ts": uint,                    // logical timestamp
    "meta": option<bytes(36)>,     // optional metadata CID
}
```

### 8.4 Transport Protocols

primary: direct peer-to-peer over QUIC with post-quantum encrypted sessions

Repository cloning by CID:
```
cyber clone cid:bafk2bzaced...
// resolves CID → genesis patch set → full repo
```

Cloning by [[neuron]] identity:
```
cyber clone neuron:abc123.../repo-name
// resolves neuron_id → public endpoint → repo
```

Cloning by blockchain [[name]]:
```
cyber clone cyber-name.cyber/repo-name
// resolves name on nox chain → neuron_id → repo
```

No URL required: the entire resolution chain is on-graph/on-chain. HTTP transport is an optional compatibility layer, not a requirement.

---

## 9. Consensus Integration

### 9.1 On-Chain Patch Registration

Patches may be optionally registered on-chain to:
- Establish temporal ordering for dispute resolution
- Earn rewards proportional to Δπ contribution
- Become immutable epistemic NFT assets
- Enable cross-repository dependency verification

On-chain registration records only: `(patch_id, author_neuron_id, timestamp, deps_root)` — not the patch content (too large). Content is verified via CID.

### 9.2 Focus-Weighted Patch Ranking

The [[cyber]] [[tri-kernel]] probability engine assigns a [[focus]] weight to each patch based on its impact on the knowledge graph:

```
focus_weight(P) = w_d · diffusion_score(P)
                + w_s · spring_score(P)
                + w_h · heat_score(P)
```

Where:
- `diffusion_score`: measures how widely this patch's [[particles]] are referenced (exploration)
- `spring_score`: measures structural balance contribution (coherence)
- `heat_score`: measures contextual relevance decay (recency/locality)

Patches with high focus weights:
- Are prioritized in conflict resolution ([[consensus]] prefers higher-ranked resolution)
- Earn proportionally higher Δπ rewards
- Gain faster propagation priority in the network

### 9.3 Conflict Resolution via Consensus

When a conflict cannot be resolved locally (no resolution patch exists), the network can arbitrate:

```
ConsensusResolution {
    conflict_id: ConflictID,
    candidates:  Vec<PatchID>,    // competing resolution patches
    vote_window: u64,             // blocks to accept votes
    result:      Option<PatchID>, // winning resolution
}
```

Voting weight = [[neuron]]'s stake × focus_weight. This ties version control conflict resolution directly to [[cyber]]'s economic and epistemic consensus layer.

### 9.4 Reward Mechanics

```
reward(P) = base_fee + Δπ(P) × reward_coefficient

Δπ(P) = π_after(P) - π_before(P)   // change in network focus from adding patch P
```

Patches that increase network [[knowledge]] coherence (positive Δπ) earn rewards. Patches that fragment or duplicate existing knowledge earn less or nothing. This creates an economic pressure toward high-quality, well-connected knowledge contributions — directly aligned with [[collective focus theorem]] predictions.

---

## 10. Agent Interface

### 10.1 Agent Capabilities

Autonomous agents interact with CyberPatch through the same primitives as humans, with additional affordances:

```
NeuronSession {
    neuron_id:     NeuronID,       // agent's identity
    signing_key:   NeuronKey,      // held in secure enclave
    repo:          Repository,
    pending:       Vec<Operation>, // buffered ops before patch creation
}

// Core agent operations:
session.add_particle(content: Bytes) → CID
session.remove_particle(cid: CID)
session.add_edge(from, to, label: CID)
session.remove_edge(from, to, label: CID)
session.commit(message_cid: Option<CID>) → PatchID   // create and sign patch
session.propose(patch: PatchID) → ProposalID          // submit for consensus
session.apply(patch: PatchID)                         // apply locally
session.sync(remote: ChannelRef)                      // sync with remote
```

### 10.2 Parallel Agent Workflow

Multiple agents operating on the same repository simultaneously:

```
Agent₁: ops on particles {a, b, c}  →  Patch P₁(deps: ∅)
Agent₂: ops on particles {d, e, f}  →  Patch P₂(deps: ∅)
Agent₃: ops on particles {a, g}     →  Patch P₃(deps: ∅)

// P₁ and P₂ are independent: they can be applied in any order
// P₁ and P₃ may conflict (both touch particle 'a')
// Conflict C(P₁, P₃) is recorded, does not block P₂

// Agent₄ resolves the conflict:
Agent₄: resolution_patch R(deps: {P₁, P₃})
```

No agent needs to coordinate with any other agent to produce patches. Coordination happens only at resolution time, and even then can be done asynchronously by a third party or through [[consensus]].

### 10.3 GFlowNet Integration

[[GFlowNet]]s can propose patches weighted by expected Δπ:

```
GFlowNetAgent {
    fn propose_patch(state: State, target_π: FocusVector) → Patch {
        // Sample a trajectory through patch space
        // weighted by P(trajectory) ∝ exp(Δπ(trajectory))
        // Returns patch most likely to improve network focus
    }
}
```

This directly implements the [[cyber]] design directive of GFlowNet-weighted patch proposals.

### 10.4 Active Inference Integration

Agents implementing [[active inference]] minimize [[free energy]] by adaptively staking on patches:

```
ActiveInferenceAgent {
    beliefs: BeliefState,       // P(world_state | observations)

    fn update_beliefs(new_patches: Vec<Patch>) {
        // Update posterior over repository state
        // Minimize variational free energy: F = E_q[log q - log p]
    }

    fn stake_on_patch(patch: PatchID) → StakeAmount {
        // Stake proportional to expected surprise reduction
        // = expected reduction in free energy from applying this patch
    }
}
```

---

## 11. Cyber License Compatibility

### 11.1 Independence from GPL Codebases

CyberPatch is specified from first principles, drawing on:

- Academic literature on [[patch theory]] (publicly available, not copyrightable)
- [[Category theory]] (mathematical framework, not copyrightable)
- Independent derivation of algorithms from mathematical definitions

No GPL-licensed code is incorporated. No GPL-licensed code was used as implementation reference. This specification is the clean-room design document from which implementation proceeds.

References acknowledged (inspiration, not derivation):
- P-E. Meunier — mathematical theory of patches, categorical VCS foundations
- The Pijul project — proof that patch-theory VCS is practically achievable
- Darcs — early exploration of patch commutation in VCS

### 11.2 Licensing

This specification and all derivative implementations are released under the [[Cyber License]].

Key properties of Cyber License (as intended by the [[cyber]] project):
- All derivative works must remain open
- Commercial use permitted with attribution
- Network use triggers copyleft (stronger than GPL's binary distribution trigger)
- Patent retaliation clause
- Quantum-safe attribution requirements ([[signatures]], not just text)

[Cyber License full text to be incorporated by reference upon publication]

---

## 12. Appendix: Formal Definitions

### 12.1 Glossary

| Term | Definition |
|------|-----------|
| Patch | A signed, dependency-linked set of primitive operations |
| PatchID | [[hash]] of patch content including [[signature]] (Hemera digest) |
| Channel | Named mutable pointer to a set of patches |
| [[Particle]] | Content-addressed unit of tracked data (vertex) |
| CID | Content Identifier — self-describing hash of content |
| Conflict | First-class object representing incompatible concurrent changes |
| [[Neuron]] | Agent identity with signing keypair and on-chain stake |
| [[Focus]] (π) | Emergent attention vector over the [[knowledge]] graph |
| Δπ | Change in focus induced by applying a patch |
| Dependency Closure | All patches that must precede a given patch |
| Commutativity | Property that independent patches produce same result in any order |

### 12.2 Theorems to Prove (Formal Verification Backlog)

```
T1. Confluence: ∀ patch sets P, topological_sort(P) produces same state
T2. Monotonicity: ∀ valid patch P, apply(P, S) extends S
T3. Termination: dependency resolution terminates for finite acyclic dep graphs
T4. Conflict completeness: all conflicts are detected and recorded
T5. Resolution soundness: resolved states are conflict-free
T6. Sync commutativity: sync(R₁, R₂) = sync(R₂, R₁)
T7. Scale bound: focus computation has O(log n) update cost per local change
T8. Adversarial soundness: no patch can forge authorship under ML-DSA
```

### 12.3 Open Questions

1. Efficient state materialization: Resolved via dynamic names.

    A checkpoint is a user-defined [[cyberlink]] in the [[neuron]]'s own namespace:

    ```
    cyberlink(
        from:  patch_set_root_CID,     // H() of canonical patch set = channel state ID
        to:    materialized_state_CID, // CID of fully derived state blob
        label: "checkpoint",           // semantic label, in neuron's namespace
    )
    ```

    This is not a special protocol primitive — it is an ordinary cyberlink assertion.
    Any neuron may publish checkpoints for any patch set. Consumers choose which
    checkpoint to trust based on the author's focus weight π.

    Properties:
    - O(1) state access: `blob_store.get(materialized_state_CID)` — no replay needed
    - Mathematical purity preserved: the patch DAG remains ground truth; checkpoints are assertions over it, not replacements; any client may verify by replaying all patches and comparing result CIDs
    - No central authority: any neuron can checkpoint; the market of checkpoints is ranked by π — high-π [[neurons]] trusted without re-verification, unknown neurons require local verification
    - Incremental chains: `checkpoint_N → Δpatches → checkpoint_N+1` — consumers start from nearest trusted checkpoint, not genesis
    - Namespace sovereignty: only the neuron holding the signing key can write into its namespace; checkpoint authorship is cryptographically verified by ML-DSA [[signature]]
    - Mutable by design: neuron may update its checkpoint (new cyberlink for same `from`) — old link persists in history, new link wins in resolution; update cost is O(1)

2. Garbage collection: Can patches ever be pruned from the DAG? Under what conditions is a patch no longer needed for state derivation?

3. Privacy: Can patches be applied to an encrypted state (FHE) without revealing content? How does this interact with conflict detection?

4. Cross-repository dependencies: Can a patch in repository A depend on a patch in repository B? What are the consistency implications?

5. [[Focus]] computation over patches: The [[tri-kernel]] ranking currently operates over the content graph. How does it extend to the patch DAG itself (ranking contributions, not just content)?

---

CyberPatch Specification v0.1 — draft for internal review
[[cyber]] Ecosystem — nox
Status: Pre-implementation design