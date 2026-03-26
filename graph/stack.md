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

seven repos form the spine. [[cybergraph]] is the vertebra — everything attaches to it. five algebras form the arithmetic foundation. the boundary is sharp: below it, Rust bootstrap required. above it, everything is pure [[Trident]].

```
                        [[nebu]] ──┐
                        [[kuro]] ──┤
                        [[trop]] ──┤── [[lens]]
                      [[genies]] ──┤
                        [[jali]] ──┘
                           │
     ┌─────────────────────┼─────────────────────────────────────────┐
     │                     │                                         │
     │    [[hemera]]   [[lens]]   [[Trident]]    ·    [[nox]]   [[zheng]]   [[bbg]]    │
     │       hash  →  commit  →  compile  →  ·  →  run  →  prove  →  store   │
     │                                       ·                               │
     │                              ╔═══════════════════╗                     │
     │                              ║   [[cybergraph]]  ║                     │
     │                              ║    the vertebra   ║                     │
     │                              ╚═══════════════════╝                     │
     │                                       ·                               │
     │    jets · memos · types · deps · knowledge · semcons · programs       │
     │                                                                       │
     └───────────────────────────────────────────────────────────────────────┘
```

## the spine

seven repos. seven verbs. remove any one → nothing above works.

| # | repo | verb | one sentence |
|---|------|------|-------------|
| 1 | [[hemera]] | hash | [[Poseidon2]] sponge. gives [[particles]] identity |
| 2 | [[lens]] | commit | five polynomial commitment backends — one per algebra |
| 3 | [[Trident]] | compile | .tri source → [[nox]] noun. the only way to write programs |
| 4 | [[cybergraph]] | link | connects everything to everything. jets, memos, types, knowledge |
| 5 | [[nox]] | run | 16 patterns + [[hint]] + jets. trace = constraint system |
| 6 | [[zheng]] | prove | [[SuperSpartan]] + [[WHIR]] + [[sumcheck]]. [[zheng]] proof |
| 7 | [[bbg]] | store | 13 sub-roots. polynomial commitment indexes. completeness proofs |

### hemera — hash

[[Poseidon2]] sponge over [[Goldilocks field]]. t=16, Rf=8 (x⁷), Rp=16 (x⁻¹), r=8, c=8. 24 rounds. 32-byte output. ~736 constraints in a [[zheng]] proof (vs ~50,000 for Blake3).

hemera gives [[particles]] their identity. every CID in the [[cybergraph]] is a hemera output. see [[hemera]]

### lens — commit

five polynomial commitment schemes — one per execution regime. same three operations (commit, open, verify), different algebraic backends.

| lens | construction | algebra | what it commits to |
|------|-------------|---------|-------------------|
| scalar | Brakedown | [[nebu]] (F_p) | field polynomials, execution traces |
| binary | Binius | [[kuro]] (F₂) | binary witnesses, quantized AI |
| ring | Ikat | [[jali]] (R_q) | encrypted computation, TFHE |
| tropical | Assayer | [[trop]] (min,+) | optimization witnesses, dual certificates |
| isogeny | Porphyry | [[genies]] (F_q) | curve polynomials, privacy proofs |

[[nebu]] lives on two levels: raw F_p arithmetic (consumed by hemera) and scalar PCS (inside lens). see [[lens]]

### trident — compile

the provable language. .tri source compiles to [[nox]] nouns. every Trident construct maps to exactly one nox pattern. 57K LOC, 24 VM targets, self-hosts in Stage 2 of the [[bootstrap plan]].

without trident, nox is a bare CPU with no assembler. without nox, trident has nowhere to target. see [[Trident]]

### cybergraph — link

the vertebra. the universal linker. everything in [[cyber]] is [[particles]] connected by [[cyberlinks]] — and the [[cybergraph]] is the totality of these connections.

the [[cybergraph]] is not storage (that is [[bbg]]). the cybergraph is STRUCTURE — the schema, the type system, the memo cache, the jet registry, the knowledge graph. all in one graph.

| what | how in cybergraph |
|------|------------------|
| jet | particle(formula) → cyberlink → particle(implementation) |
| memo | particle(formula, subject) → cyberlink → particle(result) |
| type | particle(program) → cyberlink → particle(type_signature) |
| dependency | particle(program) → cyberlink → particle(library) |
| knowledge | particle(concept) → cyberlink → particle(concept) |

jets and memos are the SAME pattern: formula → answer. a jet maps formula to fast implementation. a memo maps (formula, subject) to cached result. structurally identical. both are cyberlinks.

see [[cybergraph]]

### nox — run

sixteen deterministic reduction patterns over hemera-authenticated trees. five structural (axis, quote, compose, cons, branch), six field (add, sub, mul, inv, eq, lt), four bitwise (xor, and, not, shl), one hash. plus non-deterministic [[hint]] injection.

nox core is frozen (16 patterns, [[checkpoint]] 0). jets are external — looked up in the [[cybergraph]] by formula hash during reduction. adding a jet does not change nox. removing all jets does not break nox (just slower).

computation IS linking: `ask(ν, subject, formula, τ, a, v, t)` — seven arguments = seven fields of a [[cyberlink]]. the [[cybergraph]] is a universal memo cache. see [[nox]]

### zheng — prove

[[SuperSpartan]] IOP + [[WHIR]] PCS + [[sumcheck]]. a fundamentally new proof type covering all five execution regimes through one verification backbone. zero trusted setup, post-quantum, sub-millisecond verification.

every nox computation produces a [[zheng]] proof. recursive composition via field tower F_{p³}. see [[zheng]]

### bbg — store

the Big Badass Graph. 13 sub-roots under one state commitment: 9 public [[NMT]] indexes + 3 private indexes (mutator set) + 1 finalization index. [[LogUp]] cross-index consistency.

bbg is to [[cybergraph]] what a database engine is to a schema. cybergraph defines WHAT. bbg implements HOW. see [[bbg]]

## five algebras (inside lens)

five execution regimes, each irreducible by its own criterion. see [[four algebras]].

| algebra | repo | structure | regime |
|---------|------|-----------|--------|
| [[nebu]] | ~/git/nebu | F_p (Goldilocks) | truth |
| [[kuro]] | ~/git/kuro | F₂ tower → F₂¹²⁸ | efficiency |
| [[trop]] | ~/git/trop | (min, +) semiring | optimality |
| [[genies]] | ~/git/genies | F_q (CSIDH prime) | privacy |
| [[jali]] | ~/git/jali | R_q = F_p[x]/(x⁶⁴+1) | encrypted computation |

## the boundary

the stack is the MINIMAL set of components that cannot implement themselves. once the spine exists, EVERYTHING above it is pure .tri — written, compiled, run, proven, and stored using only spine tools.

| component | needs Rust? | nature |
|-----------|-------------|--------|
| hemera, lens, trident, nox, zheng, bbg | yes (bootstrap) | spine |
| plumb, identity, social, geo | no | core semcons |
| tru, foculus | no | computed over semcons |
| mudra | no (jets for speed only) | infrastructure |
| rune, cyb, cybernode | no | interface |

the spine has dual existence: Rust (bootstrap + jet implementation) and Trident (proven canonical). everything above the boundary has single existence: Trident only.

## genesis crystal

the [[cybergraph]] starts empty. core semcons cannot deploy without tokens. tokens cannot exist without the plumb semcon. the [[bootstrap plan]] resolves this with a genesis crystal — a .tri program that runs once with unlimited focus:

```
genesis.tri:
  create_token(CYB, HYDROGEN, VOLT, AMPERE)
  register_semcon(plumb, identity, social, geo)
  distribute(initial_balances)
  // genesis focus expires. normal rules apply.
```

the crystal is the seed structure that determines the growth pattern. without it — empty graph, no rules. with it — economics, types, constraints. even genesis is a proven .tri program.

## core semcons (protocol layer)

the first inhabitants of the spine. consensus-critical [[Trident]] programs that define what [[cyber]] IS. not kernel, not apps — protocol.

| semcon | what it defines | stack depth |
|--------|----------------|-------------|
| [[plumb]] | tokens, staking, delegation, conservation, UTXO | zheng (proofs) + bbg (private state) + nox (metering) |
| identity | neuron registration, key proof, ownership | zheng (proofs) + bbg (neuron index) |
| social | following, reputation edges | bbg (social index) |
| geo | location proofs, physical attestation | zheng (geo proofs) |

these are not simple typed edges. they are "heavy" semcons that reach deep into the spine — conservation laws in zheng, private state in bbg, metering in nox. [[plumb]] alone requires support from every spine element.

## computed layer (nox programs over core semcons)

| program | repo | what it computes |
|---------|------|-----------------|
| [[tru]] | ~/git/tru | [[relevance]]: [[tri-kernel]] → [[focus]], [[cyberank]], [[karma]], [[syntropy]] |
| [[foculus]] | ~/git/foculus | [[consensus]]: [[collective focus theorem]] → finality from topology |

tru closes the feedback loop: [[neurons]] create [[cyberlinks]] → bbg stores → tru computes [[focus]] → focus feeds back into memoization, ranking, markets.

## infrastructure

| service | repo | what it provides |
|---------|------|-----------------|
| [[mudra]] | ~/git/mudra | post-quantum crypto: KEM, dCTIDH, AEAD, TFHE, threshold |
| [[radio]] | ~/git/radio | P2P transport: QUIC, BAO streaming, gossip |

## languages (15, compile to nox)

[[Rs]], [[rune]], [[Arc]], [[Ten]], [[Bt]], [[Tok]], [[Seq]], [[Wav]], [[Bel]], [[Dif]], [[Sym]], [[Ren]], [[Qu]], [[Trident]], [[markup]]. see [[cyb/languages]]

## interface

[[cyb]] (browser), [[cybernode]] (node), [[optica]] (publisher)

## bootstrap order

see [[bootstrap plan]] for full detail.

```
Stage 1 (Rust):       hemera → lens → trident → nox (Rs)
Stage 2 (self-host):  trident.tri → arithmetic.tri → nox.tri
Stage 3 (proven):     zheng → proven re-self-host → jets → bbg
Genesis:              genesis.tri (crystal, unlimited focus, one-time)
Protocol:             plumb.tri ∥ identity.tri ∥ social.tri ∥ geo.tri
Computed:             tru.tri ∥ foculus.tri
```

discover all [[concepts]]
