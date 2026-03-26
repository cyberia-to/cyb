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
# stack

six repos form the spine. five algebras form the foundation. three layers sit on top. together: the complete software architecture of [[cyber]].

```
        nebu ──┐
        kuro ──┤
        trop ──┤ = lens (five PCS backends)
      genies ──┤
        jali ──┘
           ↓
hemera → lens → trident → nox → zheng → bbg
 (hash) (commit) (compile) (run) (prove) (store)
           ↓        ↓        ↓      ↓       ↓
        (identity) rs     15 lang  (proofs)  tru
                   rune              foculus
                                     plumb
                                     mudra
                                      ↓
                              cyb, cybernode, optica
```

## the spine

six repos in a chain. remove any one → nothing above works. each does one thing.

| # | repo | verb | what |
|---|------|------|------|
| 1 | [[hemera]] | hash | [[Poseidon2]] sponge over [[Goldilocks field]]. gives [[particles]] identity |
| 2 | [[lens]] | commit | five polynomial commitment backends — one per algebra |
| 3 | [[Trident]] | compile | .td source → [[nox]] noun. the only way to write programs |
| 4 | [[nox]] | run | 16 patterns + [[hint]] + jets. execution trace = constraint system |
| 5 | [[zheng]] | prove | [[SuperSpartan]] + [[WHIR]] + [[sumcheck]]. [[zheng]] proof, not STARK |
| 6 | [[bbg]] | store | 13 sub-roots. polynomial commitment indexes. completeness proofs |

### hemera — hash

[[Poseidon2]] sponge over [[Goldilocks field]]. parameters: d=7, t=16, Rf=8, Rp=64, r=8, c=8. single function, single mode, 64-byte output. ~300 constraints in a [[zheng]] proof (vs ~50,000 for Blake3).

hemera gives [[particles]] their identity. every CID in the [[cybergraph]] is a hemera output. three implementations: rs, wgsl, cli. see [[hemera]]

### lens — commit

five polynomial commitment schemes — one per [[four algebras|execution regime]]. the layer between identity ([[hemera]]) and execution ([[nox]]). same three operations (commit, open, verify), different algebraic backends.

| lens | construction | algebra | what it commits to |
|------|-------------|---------|-------------------|
| scalar | Brakedown | [[nebu]] (F_p) | field polynomials, execution traces |
| binary | Binius | [[kuro]] (F₂) | binary witnesses, quantized AI |
| ring | Ikat | [[jali]] (R_q) | encrypted computation, TFHE |
| tropical | Assayer | [[trop]] (min,+) | optimization witnesses, dual certificates |
| isogeny | Porphyry | [[genies]] (F_q) | curve polynomials, privacy proofs |

[[nebu]] lives on two levels: raw F_p arithmetic (consumed by hemera) and scalar PCS (inside lens). see [[lens]]

### trident — compile

the provable language. .td source compiles to [[nox]] nouns. every Trident construct maps to exactly one nox pattern — no intermediate representation destroys information. 57K LOC, 24 VM targets, self-hosts in Stage 2 of the [[bootstrap plan]].

without trident, nox is a bare CPU with no assembler. without nox, trident has nowhere to target. they are co-dependent but trident stands BEFORE nox in the user flow: write → compile → run. see [[Trident]]

### nox — run

sixteen deterministic reduction patterns over hemera-authenticated trees. five structural (axis, quote, compose, cons, branch), six field (add, sub, mul, inv, eq, lt), four bitwise (xor, and, not, shl), one hash. plus non-deterministic [[hint]] injection and 30 [[jets]] across five algebras.

the execution trace IS the algebraic constraint system — no translation layer between program and proof. computation IS linking: `ask(ν, subject, formula, τ, a, v, t)` has seven arguments — the seven fields of a [[cyberlink]]. the [[cybergraph]] is a universal memo cache: before executing, nox checks if `axon(formula, subject)` already has a verified result. see [[nox]]

### zheng — prove

[[SuperSpartan]] IOP + [[WHIR]] PCS + [[sumcheck]]. not a STARK — a fundamentally new proof type that covers all five execution regimes through one verification backbone. zero trusted setup, post-quantum, sub-millisecond verification.

every nox computation produces a [[zheng]] proof of correct execution. recursive composition via field tower F_{p³}. see [[zheng]]

### bbg — store

the Big Badass Graph. 13 sub-roots under one state commitment: 9 public [[NMT]] indexes (particles, axons_out, axons_in, neurons, locations, coins, cards, files, time) + 3 private indexes (cyberlinks, spent, balance via mutator set) + 1 finalization index (signals). [[LogUp]] cross-index consistency.

three laws: bounded locality, constant-cost verification, structural security. see [[bbg]]

## five algebras (inside lens)

the arithmetic foundation. five execution regimes, each irreducible by its own criterion.

| algebra | repo | field/structure | what it computes |
|---------|------|----------------|-----------------|
| [[nebu]] | ~/git/nebu | F_p (Goldilocks) | truth — proofs, hashing, commitments |
| [[kuro]] | ~/git/kuro | F₂ tower → F₂¹²⁸ | efficiency — quantized AI, binary proving |
| [[trop]] | ~/git/trop | (min, +) semiring | optimality — shortest paths, scheduling, DP |
| [[genies]] | ~/git/genies | F_q (CSIDH prime) | privacy — stealth, VRF, blind signatures |
| [[jali]] | ~/git/jali | R_q = F_p[x]/(x⁶⁴+1) | encrypted computation — TFHE, lattice KEM |

see [[four algebras]] for why five (not three, not seven) and the convergence from four branches of mathematics.

## protocol layer (nox programs on the spine)

not kernel. not apps. the rules of [[cyber]], deployed as [[Trident]] programs on the spine. consensus-critical, updatable.

| program | repo | what it computes |
|---------|------|-----------------|
| [[tru]] | ~/git/tru | [[relevance]]: [[tri-kernel]] ([[diffusion]] + [[springs]] + [[heat]]) → [[focus]], [[cyberank]], [[karma]], [[syntropy]] |
| [[foculus]] | ~/git/foculus | [[consensus]]: [[collective focus theorem]] → finality from topology, no voting |
| [[plumb]] | ~/git/plumb | [[tokens]]: conservation laws, [[UTXO]], [[will]] locks, conviction accounting |

tru closes the feedback loop: [[neurons]] create [[cyberlinks]] → bbg stores them → tru computes [[focus]] → focus informs memoization, ranking, markets. the intelligence feeds back into every layer.

## infrastructure (agent-facing services)

| service | repo | what it provides |
|---------|------|-----------------|
| [[mudra]] | ~/git/mudra | post-quantum crypto: KEM, dCTIDH, AEAD, TFHE, threshold |
| [[radio]] | ~/git/radio | P2P transport: QUIC, BAO streaming, gossip (hemera instead of Blake3) |

proofs ([[zheng]]) verify and charge. mudra hides and shares. orthogonal concerns.

## languages (compile to nox)

[[Rs]] syntax → [[Trident]] AST → [[nox]] noun. 15 [[cyb/languages]] provide domain-specific abstractions over the same execution target:

| language | domain |
|----------|--------|
| [[Trident]] | constraints, field arithmetic |
| [[Rs]] | systems code (restricted Rust) |
| [[rune]] | nervous system (Rs + hints + host jets) |
| [[Arc]] | graphs, topology |
| [[Ten]] | tensors, linear algebra |
| [[Bt]] | binary logic (F₂) |
| [[Tok]] | resources, conservation |
| [[Seq]] | events, causality |
| [[Wav]] | signals, FHE |
| [[Bel]] | beliefs, probability |
| [[Dif]] | differential geometry |
| [[Sym]] | symplectic geometry |
| [[Ren]] | Clifford geometric algebra |
| [[Qu]] | quantum circuits |
| [[markup]] | addressing (the 15th, non-computational) |

## interface (user-facing)

| interface | what |
|-----------|------|
| [[cyb]] | the browser — renders the [[cybergraph]] |
| [[cybernode]] | the node — runs the spine |
| [[optica]] | the publisher — renders knowledge graphs |

## the three boundaries

| boundary | below | above | criterion |
|----------|-------|-------|-----------|
| trust | spine (hemera → bbg) | protocol + apps | below = proven ([[zheng]] proofs) |
| semantic | spine + algebras | protocol + apps | below = protocol-agnostic |
| freeze | spine (checkpoints 0-4) | protocol + apps | below = immutable after mainnet |

the spine is proven, general-purpose, and frozen. the protocol layer runs ON the spine as nox programs. apps run on the protocol. like a microkernel OS: consensus and tokens are user-space services, not kernel features.

## bootstrap order

three stages. see [[bootstrap plan]] for full detail.

```
Stage 1 (Rust bootstrap):      hemera → lens → trident → nox (Rs)
Stage 2 (classical self-host):  trident.td → arithmetic.td → nox.td
Stage 3 (proven bootstrap):     zheng → proven re-self-host → jets → bbg
Application:                    tru.td ∥ foculus.td ∥ plumb.td ∥ mudra.td
```

see [[cyb/core]] for the applications built on this stack. see [[cyb/os]] for the kernel. see [[cyb/architecture]] for the design
