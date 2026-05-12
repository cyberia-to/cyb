---
tags: cyb, core
crystal-type: entity
crystal-domain: cyber
alias: cyb stack, software stack, proof pipeline
---
# stack

fourteen repos form the core. [[cybergraph]] is the vertebra — everything attaches to it. [[strata]] is the floor — every proof reduces to operations in its five algebras. the boundary is sharp: below it, Rust bootstrap required. above it, everything is pure [[trident]].


## the core

fourteen repos. fourteen verbs. remove any one → nothing above works.

| # | repo | verb | one sentence |
|---|------|------|-------------|
| 0 | [[strata]] | algebra | four trait tiers × five algebras. the arithmetic every proof reduces to |
| 1 | [[hemera]] | hash | [[Poseidon2]] sponge. gives [[particles]] identity |
| 2 | [[lens]] | commit | five polynomial commitment backends — one per algebra |
| 3 | [[trident]] | compile code | .tri → .nox. the only way to write programs |
| 4 | [[nox]] | run code | 16 patterns + [[hint]] + jets. trace = constraint system |
| 5 | [[zheng]] | prove & verify | [[SuperSpartan]] + [[WHIR]] + [[sumcheck]]. [[zheng]] proof |
| 6 | [[cybergraph]] | link | connects everything to everything. jets, memos, types, knowledge |
| 7 | [[bbg]] | store | one polynomial, 10 dimensions. ~200 byte proofs, 10-50 μs |
| 8 | [[tru]] | compile model | .graph → .model + graph field: φ*, eigenvectors, cyberank |
| 9 | [[glia]] | run model | universal .model runtime. graph-agnostic |
| 10 | [[mir]] | render | tru positions + glia features → [[R-1.0]] world. makes it physical |
| 11 | [[mudra]] | encrypt | post-quantum: KEM, dCTIDH, AEAD, TFHE, threshold |
| 12 | [[radio]] | transmit | P2P transport: QUIC, BAO streaming, gossip |
| 13 | [[foculus]] | consensus | [[collective focus theorem]] → finality from topology |

## compiler / runtime duality

the stack has two compiler/runtime pairs — the same pattern at two levels:

| | compiler | runtime |
|--|---------|---------|
| programs | trident (.tri → .nox) | nox (runs any .nox) |
| models | tru (.graph → .model) | glia (runs any .model) |

trident knows .tri. nox knows nothing about .tri — it just runs .nox. tru knows .graph. glia knows nothing about graphs — it just runs .model. mir reads both.

## foundation — math, identity, commitment

---

## strata — algebra

the floor. every proof, every hash, every commitment reduces to operations in one of five algebras. four trait tiers — each consumed by a different set of core components:

| tier | crate | traits | consumed by |
|------|-------|--------|------------|
| 1: universal | strata-core | Codec, Semiring, Ring, Field | hemera, lens, nox, zheng, bbg, mudra |
| 2: proofs | strata-proof | Reduce (bytes→F), Dot (Σaᵢbᵢ) | lens, zheng |
| 3: compute | strata-compute | Spectral (NTT roots), Bits | nox, jali |
| 4: structure | strata-ext | Extension (tower), Batch (Montgomery inv), Blind (ct-ops) | lens, mudra, genies |

five algebras — each maps to one lens construction:

| algebra | structure | lens | construction | regime |
|---------|-----------|------|-------------|--------|
| [[nebu]] | F_p (Goldilocks) | scalar | Brakedown | truth — field polynomials, execution traces |
| [[kuro]] | F₂ tower → F₂¹²⁸ | binary | Binius | efficiency — binary witnesses, quantized AI |
| [[jali]] | R_q = F_p[x]/(xⁿ+1) | ring | Ikat | encrypted computation — TFHE, FHE bootstrapping |
| [[trop]] | (min, +) semiring | tropical | Assayer | optimality — optimization witnesses, dual certificates |
| [[genies]] | F_q (CSIDH-512) | isogeny | Porphyry | privacy — curve polynomials, stealth addresses |

zheng is decomposed by strata: SuperSpartan constraint evaluation = Dot products over Field; Fiat-Shamir challenges = Reduce over hemera output bytes; WHIR folding = Spectral NTT over nebu extensions. see [[strata]]

## hemera — hash

[[Poseidon2]] sponge over [[Goldilocks field]]. t=16, Rf=8 (x⁷), Rp=16 (x⁻¹), r=8, c=8. 24 rounds. 32-byte output. ~736 constraints in a [[zheng]] proof (vs ~50,000 for Blake3).

hemera gives [[particles]] their identity. every CID in the [[cybergraph]] is a hemera output. see [[hemera]]

## lens — commit

five polynomial commitment backends — one per strata algebra. same three operations (commit, open, verify), different algebraic structures. see [[strata]] for the algebra definitions.

see [[lens]]

## programs — compile, run, prove

---

## trident — compile code

the provable language. .tri source compiles to .nox. every trident construct maps to exactly one nox pattern. 57K LOC, 24 VM targets, self-hosts in Stage 2 of the [[bootstrap plan]].

trident's compiler backend includes a neural optimizer: a GNN+Transformer (~13M params, GATv2 encoder + 6-layer decoder) that optimizes TIR→TASM at compile time. classical lowering always runs; neural output accepted only when stack-verified equivalent and strictly cheaper. speculative, not required.

without trident, nox is a bare CPU with no assembler. trident already targets 28 VMs — including inefficient legacy ones. nox is the only efficient destination. see [[trident]]

## nox — run code

sixteen deterministic reduction patterns over hemera-authenticated trees. five structural (axis, quote, compose, cons, branch), six field (add, sub, mul, inv, eq, lt), four bitwise (xor, and, not, shl), one hash. plus non-deterministic [[hint]] injection.

nox core is frozen (16 patterns, [[checkpoint]] 0). jets are external — looked up in the [[cybergraph]] by formula hash during reduction. adding a jet does not change nox. removing all jets does not break nox (just slower).

computation IS linking: `ask(ν, subject, formula, τ, a, v, t)` — seven arguments = seven fields of a [[cyberlink]]. the [[cybergraph]] is a universal memo cache. see [[nox]]

## zheng — prove & verify

[[SuperSpartan]] IOP + [[WHIR]] PCS + [[sumcheck]]. a fundamentally new proof type covering all five execution regimes through one verification backbone. zero trusted setup, post-quantum, sub-millisecond verification.

every nox computation produces a [[zheng]] proof. recursive composition via field tower F_{p³}. see [[zheng]]

## knowledge — link, store

---

## cybergraph — link

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

### signal pipeline

every [[signal]] (cyberlink with `ask(ν, p, q, τ, a, v, t)`) fans out to direct readers, then flows downstream:

| component | reads from | produces |
|-----------|-----------|---------|
| [[nox]] | signal: p as formula, q as subject | result particle + [[zheng]] proof |
| [[tru]] | signal: p, q as graph edges; a × v per link | .model artifacts + field state (φ*, eigenvectors) |
| [[bbg]] | signal: all fields | persistent storage across all 10 dimensions |
| [[glia]] | tru: .model artifacts | inference outputs (neural features) |
| [[mir]] | tru: positions + φ* · glia: neural features | [[R-1.0]] world |

`signal.a` is raw stake amount — NOT focus. [[tru]] runs the [[tri-kernel]] to convert stake-weighted cyberlinks into φ*. focus is always computed, never stored.

see [[cybergraph]]

## bbg — store

the Big Badass Graph. one polynomial, all state. BBG_poly(index, key, t) = value. 10 dimensions (particles, axons_out, axons_in, neurons, locations, coins, cards, files, time, signals). cross-index consistency is structural — same polynomial, different dimensions. no NMT, no [[LogUp]]. ~200 bytes per proof, 10-50 μs verification.

bbg is to [[cybergraph]] what a database engine is to a schema. cybergraph defines WHAT. bbg implements HOW. see [[bbg]]

## intelligence — compile model, run model, render

---

## tru — compile model

two jobs, one engine: compile and field.

**compile**: reads the [[cybergraph]] as a weighted graph and compiles it to a `.model` artifact — the CT-1.1 model that [[glia]] will run. `.graph` is one compiler target; tru is the compiler that understands graphs.

**field**: runs graph field computation over every signal. reads signal.a (raw stake) and signal.v (valence) → [[tri-kernel]] → φ* (focus distribution). runs the eigensolver (LOBPCG on the screened Laplacian) → particle positions in spectral space. computes [[cyberank]], [[karma]], [[syntropy]].

two outputs:
- **runtime state** — φ*, eigenvectors, focus → consumed by [[mir]] every epoch
- **compiled model** — .model artifacts → handed to [[glia]] for inference

tru closes the feedback loop: [[neurons]] create [[cyberlinks]] → bbg stores → tru reads signal.a × signal.v → tri-kernel → φ* → feeds back into memoization, ranking, markets. see [[tru]]

## glia — run model

universal `.model` runtime. graph-agnostic: no knowledge of [[cybergraph]], [[particles]], or [[cyberlinks]]. runs any `.model` → outputs (tensors, features, neural activations).

`.graph` is one compiler target — tru compiles it. glia does not know this. glia receives a `.model` and runs it. the same runtime that executes a graph-compiled model executes any other model.

hardware: [[rane]] (ANE) for NRF head inference; [[acpu]] (AMX) for heavy matrix ops. outputs neural features → consumed by [[mir]]. see [[glia]]

## mir — render

Russian мир: world, peace, community. the thing that makes it physical.

reads two inputs: tru's field state (particle positions, φ*, focus) and glia's inference outputs (neural features). produces the [[R-1.0]] deterministic 3D world — every neuron running mir on the same inputs sees the same world.

mir knows nothing about graphs or models. it receives coordinates and features and makes them visible. rendering tiers T0–T3 (content entry, labels, analytic impostors, Gaussian splats) + T∞ (neural radiance field, Phase 2+). heat-kernel BVH for LOD. epoch/frame split: heavy geometry frozen per epoch, luminosity and flow animate per frame.

hardware: [[aruminium]] (Metal GPU) for all draw calls; [[unimem]] IOSurface for zero-copy frame handoff. see [[mir]]

## network — encrypt, transmit, consensus

---

## mudra — encrypt

post-quantum cryptographic primitives. KEM (key encapsulation), dCTIDH (CSIDH-based key exchange, constant-time isogeny), AEAD (authenticated encryption), TFHE (fully homomorphic encryption over booleans), threshold protocols. consumed by [[plumb]] (private state), [[identity]] (key proofs), [[glia]] (encrypted model weights). see [[mudra]]

## radio — transmit

P2P transport layer. QUIC for reliable encrypted streams, BAO for content-addressed streaming with incremental verification, gossip for signal propagation across the [[cybergraph]]. the nervous system that carries signals between [[neurons]]. see [[radio]]

## foculus — consensus

[[collective focus theorem]]: focus topology determines finality. when the φ* distribution converges to a stable attractor, the network has reached consensus. no leader election, no voting rounds — consensus emerges from the same field equations that drive [[tru]]. see [[foculus]]

---

## the boundary

the core is the MINIMAL set of components that cannot implement themselves. once the core exists, EVERYTHING above it is pure .tri — written, compiled, run, proven, and stored using only core tools.

| component | needs Rust? | nature |
|-----------|-------------|--------|
| strata (nebu, kuro, trop, genies, jali) | yes (bootstrap) | algebra floor |
| hemera, lens, trident, nox, zheng, bbg | yes (bootstrap) | spine |
| tru | yes (eigensolver, AMX) | graph field + model compiler |
| glia | yes (ANE, AMX) | model runtime |
| mir | yes (Metal, honeycrisp) | render engine |
| mudra | yes (constant-time crypto) | encryption |
| radio | yes (QUIC, networking) | transport |
| foculus | no | consensus engine |
| plumb, identity, social, geo | no | core semcons |
| rune, prysm, cybernode | no | interface |

the spine has dual existence: Rust (bootstrap + jet implementation) and trident (proven canonical). everything above the boundary has single existence: trident only.

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

the first inhabitants of the spine. consensus-critical [[trident]] programs that define what [[cyber]] IS. not kernel, not apps — protocol.

| semcon | what it defines | stack depth |
|--------|----------------|-------------|
| [[plumb]] | tokens, staking, delegation, conservation, UTXO | zheng (proofs) + bbg (private state) + nox (metering) |
| identity | neuron registration, key proof, ownership | zheng (proofs) + bbg (neuron index) |
| social | following, reputation edges | bbg (social index) |
| geo | location proofs, physical attestation | zheng (geo proofs) |

these are not simple typed edges. they are "heavy" semcons that reach deep into the spine — conservation laws in zheng, private state in bbg, metering in nox. [[plumb]] alone requires support from every spine element.

## languages (15, compile to nox)

[[Rs]], [[rune]], [[Arc]], [[Ten]], [[Bt]], [[Tok]], [[Seq]], [[Wav]], [[Bel]], [[Dif]], [[Sym]], [[Ren]], [[Qu]], [[trident]], [[markup]]. see [[cyb/languages]]

## interface

[[prysm]] (browser), [[cybernode]] (node), [[optica]] (publisher)

## bootstrap order

see [[bootstrap plan]] for full detail.

```
Stage 0 (Rust):       strata (nebu, kuro, trop, genies, jali)
Stage 1 (Rust):       hemera → lens → trident → nox (Rs)
Stage 2 (self-host):  trident.tri → arithmetic.tri → nox.tri
Stage 3 (proven):     zheng → proven re-self-host → jets → bbg
Genesis:              genesis.tri (crystal, unlimited focus, one-time)
Protocol:             plumb.tri ∥ identity.tri ∥ social.tri ∥ geo.tri
Computed:             tru ∥ foculus.tri
Infrastructure:       glia ∥ mir ∥ mudra ∥ radio
```

discover all [[concepts]]
