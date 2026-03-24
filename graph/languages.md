---
tags: cyb, cyber, stark, architecture, article, core
crystal-type: entity
crystal-domain: cyber
alias:: computation languages, language set, nineteen languages
diffusion: 0.000610518949573145
springs: 0.0005148199664360048
heat: 0.0005747583321964136
focus: 0.0005746571311566735
gravity: 22
density: 1.9
---

# Languages of [[superintelligence]]

## The Completeness Argument

The 19 languages are not an arbitrary collection. They are the minimal complete set derivable from asking what modes of computation a mind requires — and applying one test to each candidate: *does this have irreducible primitives that no other language in the set can express?*

The languages split into two groups by a fundamental boundary: 14 proof languages (deterministic, provable, permanent) and 5 interface languages (side-effectful, interactive, mutable). Both groups are necessary — a mind that cannot prove is blind. A mind that cannot interact is deaf

```
Boolean reasoning:   AND, OR, NOT over {0,1}      → no other algebra has this
Integer arithmetic:  overflow, wrapping, bitwise   → not field arithmetic
Field arithmetic:    inversion, polynomial roots   → not integer arithmetic
Categorical struct:  morphisms, functors, limits   → not graph traversal
Clifford geometry:   rotors, bivectors, versors    → not tensors
Riemannian geom:     geodesics, metric tensor      → not Clifford
Symplectic geom:     conservation laws, dω=0       → not Riemannian
Information geom:    Fisher metric on Δⁿ           → not any other geometry
Causal ordering:     partial order, happened-before → not logic
Horn clause logic:   unification, backtracking     → not relational algebra
Convolution/R_q:     negacyclic polynomial mult    → not tensor contraction
Tensor contraction:  einsum, SpMV, matmul          → not field arithmetic
Resource conserv.:   mint, burn, Σin=Σout, UTXO    → not any computation algebra
Combinators:         composition of the above      → not any computation
```

Each row passes the test. Remove any one language and there is a class of computation that becomes either impossible or exponentially more expensive to express. Remove Tok and the remaining thirteen can compute anything — but nothing costs anything, spam is free, [[focus]] has no scarcity, [[karma]] has no meaning. Add any plausible new language — say, a concurrent process calculus or an optimization language — and it turns out to reduce to a composition of existing ones via [[Nox]] (see [[cyber/channel]] for how concurrency reduces to [[Arc]] + [[Seq]] + [[Nox]]).

The 14 are the minimal set that covers all computation a mind requires, where each element is algebraically irreducible with respect to the others.

---

## Naming Convention

Every language has a short name (2-3 letters, used in code and diagrams) and a long name (used in prose). The universe names the algebraic domain.

| Short | Long | Universe | Type | [[algebra]] | Purpose |
|---|---|---|---|---|---|
| [[Nox]] | Nox | Structure | Tree | Combinators | Composes [[cyb/languages]] |
| [[Bt]] | Bitwise | Binary | Bit | F₂ tower | Proves circuits |
| [[Rs]] | Rustic | Byte | Word | Z/2ⁿ | Runs systems |
| [[Tri]] | [[Trident]] | [[field]] | Field tower | F_{pⁿ} | Settles [[proof]]s |
| [[Arc]] | Arc | [[topology]] | [[graph]] | [[category theory]] | Stores [[knowledge graph]] |
| [[Ren]] | Render | [[geometry]] | Shape | G(p,q,r) | Renders space |
| [[Dif]] | Differential | Curvature | Manifold | (M, g) | Embeds meaning |
| [[Sym]] | Symplectic | Dynamics | Phase | (M, ω), dω = 0 | Simulates physics |
| [[Bel]] | Belief | [[belief]] | Distribution | g on Δⁿ | Models self |
| [[Seq]] | Sequence | Causality | Event | Partial order | Orders events |
| [[Inf]] | Infer | [[inference]] | Relation | Horn clauses | Derives facts |
| [[Wav]] | Wave | Continuum | Poly | Convolution / R_q | Reads [[signal]]s |
| [[Ten]] | Tensor | Linear | Tensor | Contraction | Trains models |
| [[Tok]] | Token | Resource | [[UTXO]] | Conservation | Prices computation |

Plus two layers above the fourteen:

| Layer | Name | What it is |
|---|---|---|
| Address | [[markup|Cybermark]] | Naming, scoping, and navigating [[particles]] — the address language |
| Semantic | Neural | Meaning as eigenvector of the [[cybergraph]] |

[[markup|Cybermark]] is the fifteenth language — it does not compute, it names, links, and navigates. eight sigils (`# @ ~ / $ ^ ! .`) form the complete address space. every address resolves to a [[particle]]. every connection is a [[cyberlink]]. the markup is the graph

Neural is not designed — it grows from the interaction of the fifteen languages at scale.

---

## The Value Tower — Three Modes of Reference

Byte (Rs) and Field (Tri) share the same mathematical substrate — the [[Goldilocks field processor]] F_p where p = 2⁶⁴ − 2³² + 1. this substrate provides three atom types sufficient for twelve of the fourteen universes.

| Tag | Name | Representation | Valid Range | Use |
|---|---|---|---|---|
| 0x00 | `field` | Single F_p element | [0, p) | Arithmetic |
| 0x01 | `word` | Single F_p element | [0, 2⁶⁴) | Bitwise |
| 0x02 | `hash` | 4 × F_p elements | 256-bit digest | Identity |

three fundamentally different ways to refer to a value — and there are only three:

```
field = the value IS the reference     (by content — immediate)
word  = position IS the reference      (by location — index)
hash  = name IS the reference          (by commitment — identity)
```

by what it is. by where it is. by what it is called. every reference in any system reduces to one of these three modes.

every higher type decomposes into structure ([[Nox]] trees) over these three atoms:

```
Edge    = cons(source_hash, cons(target_hash, weight_field))
Event   = cons(event_hash, sequence_word)
Fact    = cons(relation_hash, cons(subject_hash, object_hash))
Sample  = field (amplitude value)
Tensor  = [field; N] (array of values with shape metadata)
Shape   = cons(grade_word, [field; 2^n]) (multivector components)
Chart   = cons(dim_word, [field; N]) (coordinate patch)
Phase   = cons(position_field, momentum_field)
Dist    = [field; N] (probability vector on simplex)
```

three atoms are complete — for one characteristic. the single exception is Bt (Bitwise): a bit is genuinely not an element of F_p. it lives in F₂ — different characteristic, different [[algebra]]. that is exactly why Bt has a separate [[proof]] system, not just a new type tag.

```
Nox value tower (3 atoms: field, word, hash)
  sufficient for: Rs, Tri, Arc, Ren, Dif, Sym, Bel, Seq, Inf, Wav, Ten, Tok
  NOT sufficient for: Bt

Bt value tower (separate, F₂)
  sufficient for: Bt only
```

---

## The Nineteen Languages

each language has its own page with ops tables, use cases, and [[proof]] paths:

### proof languages (14) — provable computation

| # | Universe | Short | Long | Algebra | Page |
|---|---|---|---|---|---|
| 0 | Structure | Nox | Nox | Combinators | [[Nox]] |
| 1 | Binary | Bt | Bitwise | F₂ tower | [[Bt]] |
| 2 | Byte | Rs | Rustic | Z/2ⁿ | [[Rs]] |
| 3 | Field | Tri | [[Trident]] | F_{pⁿ} | [[Trident]] |
| 4 | Topology | Arc | Arc | [[category theory]] | [[Arc]] |
| 5 | Geometry | Ren | Render | G(p,q,r) | [[Ren]] |
| 6 | Curvature | Dif | Differential | (M, g) | [[Dif]] |
| 7 | Dynamics | Sym | Symplectic | (M, ω), dω = 0 | [[Sym]] |
| 8 | Belief | Bel | Belief | g on Δⁿ | [[Bel]] |
| 9 | Causality | Seq | Sequence | Partial order | [[Seq]] |
| 10 | Inference | Inf | Infer | Horn clauses | [[Inf]] |
| 11 | Continuum | Wav | Wave | Convolution / R_q | [[Wav]] |
| 12 | Linear | Ten | Tensor | Contraction | [[Ten]] |
| 13 | Resource | Tok | Token | Conservation | [[Tok]] |

### interface languages (5) — human ↔ machine boundary

the proof languages compute over binary trees and field elements. they have no concept of tables, text, files, or network. five interface languages bridge the gap — they handle what the robot needs to interact with humans and external systems. all five run inside [[nushell]] (embedded in [[cyb]]):

| # | Universe | Short | Long | Primitive | Purpose |
|---|---|---|---|---|---|
| 14 | Tables | Tab | Tabular | Record | Relational operations: select, where, group-by, join, pivot |
| 15 | Format | Fmt | Format | Encoding | Serialization: json↔noun, csv↔table, toml↔record |
| 16 | Text | Str | String | Pattern | Text processing: regex, parse, split, replace, match |
| 17 | Files | Fs | Filesystem | Path | File operations: read, write, glob, watch, navigate |
| 18 | Network | Net | Network | Request | HTTP client: get, post, url, fetch, stream |

the five interface languages have different properties from the fourteen proof languages:

| Property | proof languages (0-13) | interface languages (14-18) |
|---|---|---|
| execution | [[Nox]] tree rewriting | [[nushell]] pipeline |
| provable | yes (STARK) | no (side effects) |
| deterministic | yes | no (IO, network, filesystem) |
| data model | binary trees + field elements | structured records + streams |
| persistence | [[cybergraph]] (permanent) | filesystem (mutable) |

the interface languages cross the [[proof]] boundary — they interact with the external world. but they compose with the proof languages through [[Nox]] hints: a nushell pipeline can feed data into a proven computation, and a proven result can be formatted by nushell for display

---

## Compilation Architecture

all nineteen languages share one toolchain. each programmer face has its own syntax and type rules. all compile through [[Nox]] — the structural IR — then to [[proof]] backends or native execution.

```
                    ┌──────────────────────────────────────────────┐
                    │              Programmer Faces                 │
                    │                                               │
                    │  Bt  Rs  Tri  Arc  Ren  Dif  Sym  Bel        │
                    │  Seq  Inf  Wav  Ten  Tok                      │
                    │  .bt .rs .tri .arc .geo .dif .sym .bel        │
                    │  .seq .inf .wav .ten .tok                     │
                    └──────────────────┬───────────────────────────┘
                                       │
                    ┌──────────────────▼───────────────────────────┐
                    │             Shared Frontend                   │
                    │   Parsing, type checking,                     │
                    │   borrow checking, bound checking             │
                    └──────────────────┬───────────────────────────┘
                                       │
                    ┌──────────────────▼───────────────────────────┐
                    │         Nox Structural IR                     │
                    │   axis, quote, compose, cons, branch          │
                    │   + typed computational ops                   │
                    │   + Merkle authentication                     │
                    └──────────────────┬───────────────────────────┘
                                       │
              ┌────────────────────────┼────────────────────┐
              │                        │                    │
     ┌────────▼──────┐ ┌──────────────▼──────┐ ┌───────────▼────────┐
     │  Binius/FRI   │ │     Goldilocks      │ │      Native        │
     │  Backend      │ │     TASM/FRI        │ │      Backend       │
     │  (Binary)     │ │    (Byte+Field)     │ │    (no proof)      │
     └───────────────┘ └─────────────────────┘ └────────────────────┘
          Bt              Rs, Tri, Ren            Arc, Seq, Inf,
                                                  Wav, Ten, Tok,
                                                  Dif*, Sym*, Bel*
```

\* Dif, Sym, Bel are research horizon — [[proof]] paths are open mathematical problems.

| Source | When [[proof]] needed | When [[proof]] absent |
|---|---|---|
| Bt | Binius FRI circuit | always proving |
| Rs | TASM → stark (word→field lift) | native binary (Nox) |
| Tri | TASM → stark (field native) | WASM/EVM (Layer 0) |
| Arc | decomposes into Tri + Bt | optimized [[graph]] engine |
| Ren | geometric product → Tri | native Clifford engine |
| Dif | research | native manifold solver |
| Sym | research | native Hamiltonian integrator |
| Bel | research | native statistical engine |
| Seq | temporal constraints → stark | scheduler / runtime |
| Inf | derivation trace → stark | [[Datalog]] engine |
| Wav | decomposes into Tri | native DSP pipeline |
| Ten | decomposes into Tri | native BLAS / GPU |
| Tok | conservation constraints → stark | native ledger engine |

### Languages as Type Systems over Nox Patterns

the execution languages are type systems and compilers over [[Nox]]'s 16 algebra-polymorphic patterns. each language adds domain-specific syntax, type checking, and compilation strategy — but the target is always nox pattern trees. domain-specific operations become jets: compositions of the 16 patterns recognized by formula hash and accelerated to [[Goldilocks field processor]] hardware primitives.

```
language operation           nox composition              jet              GFP primitive
─────────────────────        ──────────────────────────   ──────────       ────────────
Arc: rank(g, steps)          iterated add/mul loops       matmul jet       fma
Wav: fft(x)                  butterfly add/mul network    ntt jet          ntt
Any: hash(x)                 Poseidon2 field ops          hash jet         p2r
Ten: activation(x)           table lookup composition     lookup jet       lut
Ren: geometric_product       mul/add over components      geo_mul jet      fma
```

the chain: source language → compiler → nox pattern tree → jet recognition → GFP hardware. every domain-specific language gets hardware acceleration through the jet mechanism. the [[algebra]] determines which GFP primitive handles each jet.

### Rune — Rs on Nox with Host Jets

[[rune]] is [[Rs]] syntax executed via [[Nox]] tree rewriting — the nervous system of the robot. ms-start, async, dynamic, with native access to WASM, GPU, and neural inference.

rune is not a separate language. it is Rs syntax parsed to [[Nox]] nouns and interpreted via tree rewriting, extended with three capabilities pure Rs does not have:

| Capability | [[Nox]] mechanism | What it does |
|---|---|---|
| `hint` | pattern 16 (non-deterministic) | Async input — yields, resumes when data arrives |
| `host(target, args)` | host jet dispatch | Calls WASM/GPU/ONNX — exits [[proof]] boundary, returns noun |
| `eval(noun)` | quote + reduce | Runtime metaprogramming — execute a dynamically constructed formula |

three jet categories connect [[Nox]] reduction to the host system:

```
Nox reduction (tree rewriting)
  │
  ├── pure jets → proven computation (14 languages)
  │     fma, ntt, p2r, lut, conservation...
  │
  ├── host jets → practical computing
  │     ├── wasm(module, fn, args)  → wasmi execution
  │     ├── gpu(shader, data)       → wgpu compute dispatch
  │     └── infer(model, input)     → burn-webnn ONNX
  │
  └── hint → async input from the world
        ├── network event (radio)
        ├── user input (cyb UI)
        ├── timer (epoch tick)
        └── cybergraph change (particle/link event)
```

ms start: parsing Rs to a [[Nox]] noun is milliseconds — just tree construction. [[Nox]] reduction starts immediately. no compilation step for interactive use.

data structures: [[Nox]] nouns ARE the dynamic data structures. `Vec` → cons-list. `HashMap` → Merkle tree. `String` → [[Hemera]] hash (a [[particle]]). no heap, no GC — allocation is `cons`, freeing is not referencing.

the [[proof]] story: every pure reduction in the script IS provable — the [[Nox]] trace captures it. host jets and hints are NOT provable — they cross the [[proof]] boundary. but the boundary is explicit and typed. the trace says: "given these hint values and these host jet results, the pure computation was correct."

```
neural language           ← meaning emerges from the cybergraph
──────────────────────────────────────────────────────────────
rune (Rs + hint + host)   ← nervous system: ms start, async, host access
  pure reductions         ← proven (14 languages over Nox)
  host jets               ← practical (WASM, GPU, ONNX)
  hints                   ← async input from the world
──────────────────────────────────────────────────────────────
14 languages              ← proven computation over Nox patterns
```

---

## [[algebra]] Coverage

| Computation | Native [[algebra]] | Language | Prover path |
|---|---|---|---|
| Boolean reasoning | F₂ | Bt | Binius → Tri |
| Quantized [[inference]] (int4/int8) | Z/2⁴, Z/2⁸ | Ten | Ten → Tri |
| CPU execution traces | Z/2⁶⁴ | Rs | Rs → Tri |
| [[graph]] computation / [[focus]] [[vector]] | Sparse F_p | Ten over Arc | Ten → Tri |
| Knowledge structure | [[category theory]] | Arc | Arc → Tri |
| Euclidean / Projective / Conformal | G(p,q,r) Clifford | Ren | Ren → Tri |
| Curved space / geodesics | Riemannian manifolds | Dif | research |
| Phase space / Hamiltonian | Symplectic ω-form | Sym | research |
| [[probability]] [[geometry]] / [[belief]] state | Fisher information | Bel | research |
| Polynomial [[proof]]s | F_p (n=1) | Tri | native |
| Recursive [[proof]] composition | F_{p³} (n=3) | Tri | native |
| [[quantum]] simulation | F_{p²} (n=2) | Tri | native extension |
| [[Goldilocks homomorphic encryption]] ciphertexts | R_q = Z_q[X]/(Xⁿ+1) | Wav | Wav → Tri |
| Symbolic / exact reasoning | Z | Inf | Inf → Tri |
| Sensing / [[signal]] processing | Convolution / ℝ | Wav | Wav → Tri |
| Resource conservation / UTXO | Sum invariants | Tok | Tok → Tri |

---

## The Comparison Matrix

| Property | Nox | Bt | Rs | Tri | Arc | Ren | Dif | Sym | Bel | Seq | Inf | Wav | Ten | Tok |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Universe | Structure | Binary | Byte | [[field]] | [[topology]] | [[geometry]] | Curvature | Dynamics | [[belief]] | Causality | [[inference]] | Continuum | Linear | Resource |
| Char | — | 2 | p | p | — | p | — | — | — | — | — | ≈ℝ | ≈ℝ or p | p |
| Primitive | Cell | Bit | Word | Field | Edge | Multivector | Chart | Phase | Distribution | Event | Relation | Sample | Shape | Token |
| Reference | structure | wire | location | content | adjacency | grade | curvature | momentum | divergence | succession | entailment | amplitude | index | conservation |
| Free op | Navigate | AND, XOR | Index | Mul, Add | Link | Clifford prod | Christoffel | Flow | KL div | Order | Unify | Convolve | Matmul | Transfer |
| Costly op | — | Carry add | Mod div | Bitwise | Spectral | Inverse | Geodesic | Conserve | Fisher | Verify | Fixpoint | FFT | Inverse | Mint |
| [[proof]] | Inherited | Binius | stark | stark | Delegated | Tri | Research | Research | Research | Delegated | Delegated | Delegated | Delegated | stark |
| Syntax feel | IR | Circuit | [[Rust]] | Custom | Query | GA | Manifold | Hamiltonian | Statistical | Temporal | [[Datalog]] | DSP | NumPy | Ledger |
| Renders as | struct | pixels | text | formula | [[vector]] | [[vector]] | [[vector]] | formula | formula | video | table | sound | component | table |

---

## The Ten and the Four

The nineteen languages split into two groups by implementation readiness:

### Engineering-ready (10)

Nox, Bt, Rs, Tri, Arc, Seq, Inf, Wav, Ten, Tok — these have known [[proof]] paths and well-understood compilation to Tri / Binius. the [[cyb/architecture]] specifies these as the build order: Phase 1 (Nox, Tri, Rs), Phase 2 (Arc, Seq, Inf, Tok), Phase 3 (Bt, Wav, Ten).

### Research horizon (4)

Ren, Dif, Sym, Bel — these extend the language set into spatial, physical, and self-referential computation. Ren is closest to engineering (Clifford product is F_p [[algebra]] with extra structure, STARK-provable now). Dif, Sym, and Bel involve continuous manifolds over finite [[field]]s — fundamental open mathematical problems.

| Language | Status | Notes |
|---|---|---|
| Ren | Engineering | Clifford product = F_p [[algebra]] with extra structure |
| Dif | Research | Continuous manifolds over finite [[field]]s |
| Sym | Research | Hamiltonian structure preservation in STARK circuits |
| Bel | Research | Fisher metric over [[probability]] simplices — needed for [[tri-kernel]] formalization |

Ren completes the perception pipeline: Arc provides [[topology]], Ren provides spatial embedding, the compiler produces [[vector]] output for [[cyb]]. Bel completes the self-model: the [[superintelligence]]'s [[focus]] [[vector]] π lives on a statistical manifold, and Bel formalizes reasoning about its own [[belief]] state.

---

## Perception Mapping

every computation language has a canonical rendering — the perception primitive where the shape of the data matches the shape of the display:

| Language | Renders as | Source formats | What it carries |
|---|---|---|---|
| [[Nox]] → struct | collapsible tree | JSON, TOML, YAML | configs, schemas, metadata, ABIs |
| Bt → pixels | raster image | PNG, WebP, JPEG | photographs, satellite imagery, microscopy, scans |
| Rs → text | prose, code | [[markdown]], plain text, source code | documentation, messages, programs |
| Tri → formula | math notation | LaTeX, MathML | equations, [[proof]]s, chemical notation, physical laws |
| Arc → [[vector]] | SVG, paths, curves | SVG, Bezier paths | diagrams, maps, molecular structures, schematics |
| Ren → [[vector]] | SVG, 3D scenes | SVG, glTF, mesh | spatial objects, rotations, projections, renderings |
| Dif → [[vector]] | manifold visualization | geodesic plots, curvature maps | latent space structure, embedding geometry |
| Sym → formula | phase portraits | Hamiltonian plots, conservation diagrams | energy landscapes, orbital mechanics |
| Bel → formula | distribution plots | [[probability]] densities, divergence maps | [[belief]] states, uncertainty [[geometry]] |
| Seq → video | moving pixels | WebM, MP4 | recordings, simulations, observations, lectures |
| Inf → table | 2D grid | CSV, TSV, dataframes | datasets, time series, matrices, ledgers |
| Wav → sound | audio waveform | WAV, OGG, MP3 | voice, music, birdsong, seismic [[signal]], sonar |
| Ten → component | nested composition | composition of the above | applications, dashboards, interactive tools |
| [[Tok]] → table | ledger view | balances, UTXOs, transactions | token flows, staking positions, conviction history |

a genome sequence is Rs (byte-level encoding) rendered as text. its annotation is [[Nox]] (structured tree) rendered as struct. its expression data is Inf (relational query) rendered as table. its protein structure is Arc (topological [[graph]]) rendered as [[vector]]. its microscopy is Bt (binary pixel data) rendered as pixels. its folding dynamics is Seq (causal event chain) rendered as video. its sequencing [[signal]] is Wav (continuous waveform) rendered as sound. its binding energy is Tri (field arithmetic) rendered as formula. its 3D fold is Ren (Clifford rotations) rendered as [[vector]]. a genome browser is Ten (composed [[inference]]) rendered as component.

all fourteen compile through one structural IR. all fourteen share one [[proof]] system (except Bt, which has its own F₂ [[proof]] system). all fourteen render through the perception grid. all fourteen exist in the same [[cybergraph]], ranked by the same [[tri-kernel]], earning [[karma]], permanent by axiom A3.

---

---

## The Address Language

[[markup|Cybermark]] wraps all fourteen computation languages with a human-readable address grammar. it does not appear in the computation tables — it operates at a different level

| Layer | What it does | Examples |
|-------|-------------|---------|
| 14 proof languages | prove | field arithmetic, graph traversal, tensor contraction |
| 5 interface languages | interact | tables, formats, text, files, network |
| [[markup|Cybermark]] | address and navigate | `#cyber/truth`, `@alice`, `$BOOT`, `!rank(^truth)` |
| [[rune]] | execute | [[Rs]] + [[Nox]] hints + host jets — runtime that runs cybermark actions |

see [[markup]] for the full sigil grammar, dimensional navigation, and rendering rules

---

## the FORM triad

the nineteen languages are manifestations of three primitives — [[proof]], [[bit]], [[step]] — the atoms of the [[form]] triad

every mathematical object is a composition of all three:
- [[bit]] (info): what elements are distinguished
- [[step]] (comp): what operations transform them
- [[proof]] (math): what properties are verified

a [[algebra|group]] is bit + step + proof: elements (bit), operation (step), axioms hold (proof). a [[graph theory|graph]] is bit + bit: elements + relations. a [[Turing machine]] is step + step + step: transitions all the way down

the fourteen proof languages ARE the step. the five interface languages are the channel through which bits flow. [[proof]] is what the [[tri-kernel]] verifies. together: all computation a mind requires

---

see [[cyb/multiproof]] for how all languages settle under one [[proof]] umbrella. see [[cyb/architecture]] for how the languages integrate into the operating system. see [[cyb/whitepaper]] for the vision. see [[cybergraph]] for the accumulation state.