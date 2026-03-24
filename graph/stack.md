---
tags: cyb, core
crystal-type: entity
crystal-domain: cyber
alias: cyb stack, software stack, proof pipeline
diffusion: 0.0001875203252339389
springs: 0.0010881103028447855
heat: 0.0008189020023478881
focus: 0.0005839736539399998
gravity: 5
density: 5.36
---
# Stack

seven [[Rust]] crates that implement [[cyb]]. five form the [[cyb/core]] proof pipeline; two extend it with agent crypto and P2P transport. together they are the complete software foundation — everything else ([[cyb/os]], [[cyb/features]], [[cyb/apps]]) is built from these.

```
                    ┌→ mudra (crypto for agents)
nebu → hemera ──────┤               ┌→ tru (intelligence)
                    ├→ nox → zheng → bbg ─┤
                    │                └→ plumb (tokens)
                    └→ radio (transport for data)
```

## the nine crates

| # | crate | repo | role | depends on |
|---|-------|------|------|-----------|
| 1 | [[nebu]] | ~/git/nebu | [[Goldilocks field]] arithmetic + [[NTT]] | — |
| 2 | [[hemera]] | ~/git/hemera | Poseidon2 hash, Merkle trees, CIDs | nebu |
| 3 | [[nox]] | ~/git/nox | VM: 16 patterns + hint + 5 jets + memoization | hemera |
| 4 | [[zheng]] | ~/git/zheng | [[stark]] proofs: WHIR + SuperSpartan | nox |
| 5 | [[bbg]] | ~/git/bbg | authenticated state: indexes + commitments | zheng |
| 6 | [[tru]] | ~/git/tru | [[tri-kernel]] + [[consensus]]: computes [[focus]], [[cyberank]], [[karma]] | bbg |
| 7 | [[plumb]] | ~/git/plumb | [[token]] accounting: [[basic token operations]], conservation, [[UTXO]] | bbg |
| 8 | [[mudra]] | ~/git/mudra | post-quantum crypto: KEM, CSIDH, TFHE, threshold | hemera |
| 9 | [[radio]] | ~/git/radio | P2P transport: QUIC, BAO streaming, gossip | hemera |

## proof pipeline (crates 1-7)

seven crates in a chain that transform field arithmetic into collective [[intelligence]] with a [[token]] economy. remove any one and the system has no foundation

```
nebu (field) → hemera (hash) → nox (VM) → zheng (proofs) → bbg (state) → tru (intelligence)
                                                                        → plumb (tokens)
```

### nebu — field arithmetic

the [[Goldilocks field]] $\mathbb{F}_p$ where $p = 2^{64} - 2^{32} + 1$. six operations: add, sub, mul, inv, eq, lt. plus [[NTT]] over $2^{32}$ roots of unity. every number in [[cyb]] is a nebu field element. every computation reduces to nebu operations. the field is the atom.

nebu is shared across 12 of 14 [[cyb/languages]] — only [[Bt]] (characteristic 2) needs its own field. see [[nebu]]

### hemera — hashing and trees

Poseidon2 sponge over [[nebu]]. takes field elements in, produces 4-element digests out. ~300 constraints in a [[stark]] proof (vs ~50,000 for Blake3). one hash function for the entire system: content addressing, Merkle trees, commitments, key derivation, verified streaming.

hemera gives [[particles]] their identity. every CID in the [[cybergraph]] is a hemera output. see [[hemera]]

### nox — virtual machine

sixteen deterministic reduction patterns over hemera-authenticated trees. five structural (axis, quote, compose, cons, branch), six field (add, sub, mul, inv, eq, lt), four bitwise (xor, and, not, shl), one hash. plus non-deterministic hint injection and five jets for verifier acceleration.

the execution trace IS the algebraic constraint system — no translation layer between program and proof. nox is simultaneously the structural IR that all [[cyb/languages]] compile through, the node runtime, and the composition tier for proof aggregation.

computation IS linking. `ask(ν, subject, formula, τ, a, v, t)` has seven arguments — the seven fields of a [[cyberlink]]. ordering a computation and asserting [[knowledge]] are the same act. the [[cybergraph]] is a universal memo cache: before executing, nox checks if `axon(formula, subject)` already has a verified result. if cached → zero computation. the more the graph grows, the fewer computations actually execute. see [[nox]]

### zheng — proof system

[[stark]] proofs over nox execution traces. WHIR polynomial commitments, SuperSpartan constraint satisfaction. every nox computation produces a proof of correct execution as a byproduct. recursive composition via field tower $\mathbb{F}_{p^3}$.

zheng verifies that a nox program ran correctly without re-executing it. this is what makes the [[cybergraph]] trustless — you don't trust the node, you verify the proof. see [[zheng]]

### bbg — authenticated state

the Big Badass Graph. stores the [[cybergraph]] with polynomial commitment indexes: edges by neuron, edges by particle, focus values, balances, token supply, cards. each index provides cryptographic completeness proofs — when you sync a namespace, you get mathematical proof nothing was withheld.

five layers: edge store (content-addressed, immutable) → neuron index → particle index → focus & balance → UTXO state (mutator set for privacy). see [[bbg]]

### tru — intelligence

the [[relevance machine]]. reads the [[cybergraph]] from [[bbg]] and computes what matters: [[focus]] per [[particle]], [[cyberank]] per [[particle]], [[karma]] per [[neuron]], [[syntropy]] of the whole. the [[tri-kernel]] ([[diffusion]], [[springs]], [[heat]]) runs in [[consensus]] — deterministic, verifiable, on-chain

tru closes the loop: [[neurons]] create [[cyberlinks]] → bbg stores them → tru computes [[focus]] → [[focus]] informs nox memoization, [[cyber/hierarchy]] folding, [[cyber/truth]] markets, and [[cyber/self/linking|self-linking]]. the intelligence feeds back into every layer of the stack. see [[tru]]

### plumb — token accounting

the [[token]] layer. five [[basic token operations]] (pay, lock, uber, mint, burn) over [[bbg]] state. enforces conservation laws: every transfer preserves total supply, every mint is backed by proven Δπ, every burn is irreversible. [[UTXO]] management, [[will]] lock mechanics, conviction accounting on [[cyberlinks]]

plumb and tru branch off bbg in parallel: tru computes what matters ([[focus]]). plumb moves what matters ([[tokens]]). together they close the economic loop — [[focus]] determines [[value]], [[tokens]] fund [[attention]], [[attention]] shapes [[focus]]. see [[plumb]]

### the chain

each crate consumes only the one before it:

| crate | consumes | provides | enables |
|-------|----------|----------|---------|
| nebu | — | field arithmetic | every number |
| hemera | nebu | hashing, trees | every identity |
| nox | hemera + [[cybergraph]] | computation, memoization, proofs | every program (and its cached result) |
| zheng | nox | verification | every trust claim |
| bbg | zheng | authenticated state | every graph query |
| tru | bbg | [[focus]], [[cyberank]], [[karma]], [[syntropy]] | every meaning |

the pipeline is not linear — it loops. nox reads from bbg (memo lookup) and writes to bbg (store results). tru reads from bbg (graph state) and writes [[focus]] back — which feeds [[cyber/hierarchy]] folding, [[cyber/truth]] markets, and nox memoization keys. the [[cybergraph]] is simultaneously the knowledge base, the memo cache, and the state store. every computation enriches the graph. every enrichment accelerates future computation. this compounding is the source of the system's growth.

## agent crypto (crate 6)

[[mudra]] branches off hemera. it handles what proofs cannot: confidentiality, key exchange, private computation.

| module | primitive | what neurons do |
|--------|----------|-----------------|
| kem | ML-KEM (lattice) | interactive encrypted channels |
| ctidh | dCTIDH (isogeny) | non-interactive key exchange via graph |
| aead | Poseidon2 PRF + MAC | encrypt channel traffic |
| tfhe | LWE | compute on encrypted data |
| threshold | Shamir SSS, DKG | distributed key management |

proofs ([[zheng]]) verify and charge. mudra hides and shares. orthogonal concerns.

## transport (crate 7)

[[radio]] branches off hemera. a fork of [[iroh]] where every hash runs through hemera instead of Blake3. 20× cheaper in [[stark]] proofs, one hash function end to end.

| stratum | what | crate |
|---------|------|-------|
| protocols | [[radio/blob]], [[radio/docs]], [[radio/gossip]], [[radio/willow]] | iroh-* |
| verified streaming | [[radio/bao]] (hemera Merkle trees) | cyber-bao |
| content identity | Poseidon2 sponge, compression, KDF | cyber-poseidon2 |
| networking | [[radio/endpoint]], [[radio/relay]], [[radio/hole-punching]] | iroh |

## what each crate enables

| crate | what becomes possible |
|-------|----------------------|
| nebu | all arithmetic. the [[Goldilocks field processor]] accelerates it in hardware |
| hemera | content addressing. [[particles]] get identity. trees get authentication |
| nox | all [[cyb/languages]]. programs compile to nox pattern trees. the [[cybergraph]] memoizes results |
| zheng | trustless verification. the [[cybergraph]] does not require trusting nodes |
| bbg | completeness proofs. syncing a namespace proves nothing was withheld |
| tru | [[intelligence]]. the [[tri-kernel]] computes what matters. [[focus]], [[cyberank]], [[karma]], [[syntropy]] |
| plumb | [[token]] economy. conservation-proven transfers, minting, burning, [[will]] locks, conviction |
| mudra | agent privacy. [[neurons]] communicate confidentially and compute on encrypted data |
| radio | P2P connectivity. data moves between devices without centralized infrastructure |

## build order

the dependency chain determines the build order. nebu first, always. hemera next. then three independent branches (nox pipeline, mudra, radio) can proceed in parallel.

```
Phase 1:  nebu → hemera                    (foundation)
Phase 2:  nox ──────→ zheng → bbg → tru      (proof pipeline → intelligence)
                                   → plumb   (token accounting)
          mudra                              (agent crypto)
          radio                              (transport)
Phase 3:  cyb/os                            (kernel + runtime)
Phase 4:  cyb/features                      (render, contracts)
Phase 5:  cyb/apps                          (portal, oracle, sigma...)
```

see [[cyb/core]] for the applications built on this stack. see [[cyb/os]] for the kernel. see [[cyb/architecture]] for the design