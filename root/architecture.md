---
tags: cyb, article, research, core
alias: cyb architecture, cyb-system-architecture
icon: "\U0001F310"
crystal-type: entity
crystal-domain: cyber
crystal-size: deep
---
# Architecture

[[cyb]] is a sovereign browser that becomes an operating system. identity is a keypair, state lives on-chain, [[smart contracts]] run locally, and the entire render stack compiles to GPU. one binary, all platforms, 130K lines of [[Rust]], no WebView, no V8, no Google.

cyb/os is a stack of typed universes — fourteen computation [[languages]] compiled through one structural IR, rendered through nine perception primitives, driven by ten decision primitives — all sharing one toolchain, one tree substrate, and one [[proof]] system. see [[languages]] for the algebraic completeness argument and [[cyb/multiproof]] for the proving design.

core stack: [[radio]] for data publishing, [[cyber]] for search and [[learning]], [[rune]] for orchestration ([[Rs]] on [[Nox]] with host jets — ms-start, async, dynamic, with native access to WASM, GPU, and ONNX), [[CozoDB]] graph storage, [[cosmos-sdk]] chains via [[IBC]]. builds for [[web]], [[desktop]], [[mobile]].

---

## Part I: The Three Grids

the operating system is the membrane between three grids

```
COMPUTATION (what the machine thinks)     PERCEPTION (what the human sees)
─────────────────────────────────         ────────────────────────────────
Nox    → trees                            struct    → collapsible tree
Bt     → bits                             pixels    → raster image
Rs     → words                            text      → prose, code
Trident→ fields                           formula   → math notation
Arc    → graphs                           vector    → SVG, paths, curves
Seq    → events                           video     → moving pixels
Inf    → relations                        table     → 2D grid
Wav    → signals                          sound     → audio waveform
Ten    → tensors                          component → nested composition

DECISION (what the human does)
──────────────────────────────
observe   → gather without choosing
filter    → narrow by criteria
select    → choose one from many
rank      → order by preference
compose   → build a new value
split     → one becomes many
merge     → many become one
delegate  → route to another agent
reject    → explicitly not-choose
confirm   → irreversible commit
```

every computation type has a canonical rendering. a tree computed in [[Nox]] naturally displays as a collapsible `struct`. a graph traversed in [[Arc]] naturally draws as `vector` paths. a relation queried in [[Inf]] naturally fills a `table`. a signal processed in [[Wav]] naturally plays as `sound`. the mapping is many-to-many, but the canonical pairing is the path of least impedance — where the shape of the data matches the shape of the display.

every rendering invites a decision. the human responds with typed decision primitives — select, rank, compose, confirm — each with its own algebra, its own temporal mode, and its own relationship to the computation and perception grids.

### 1. The Computation Languages

every language has a short name (2-3 letters, used in code) and a long name (used in prose). the canonical naming convention — short, long, algebra, lens, domain — is the [[languages]] spec, embedded in §3 below.

a data type deserves its own language when its algebraic laws are so different from other types that forcing it into a foreign language creates constant impedance mismatch. fourteen fundamental types pass this test. each inhabits a universe defined by its characteristic algebraic structure. some universes share a proof system. some share a compiler. none share semantics. see [[languages]] for the full completeness argument and irreducibility proof.

### 2. The Value Tower — Three Modes of Reference

Byte and Field share the same mathematical substrate — the [[Goldilocks field]] 𝔽ₚ where p = 2⁶⁴ − 2³² + 1. this substrate provides three atom types sufficient for twelve of the fourteen universes.

| Tag  | Name    | Representation      | Valid Range   | Use        |
|------|---------|---------------------|---------------|------------|
| 0x00 | `field` | Single 𝔽ₚ element   | [0, p)        | Arithmetic |
| 0x01 | `word`  | Single 𝔽ₚ element   | [0, 2⁶⁴)     | Bitwise    |
| 0x02 | `hash`  | 4 × 𝔽ₚ elements     | 256-bit digest| Identity   |

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
```

three atoms are complete — for one characteristic. the single exception is Bt (Bitwise): a bit is genuinely not an element of 𝔽ₚ. it lives in 𝔽₂ — different characteristic, different algebra. that is exactly why Bt has a separate proof system, not just a new type tag.

```
Nox value tower (3 atoms: field, word, hash)
  sufficient for: Rs, Tri, Arc, Ren, Dif, Sym, Bel, Seq, Inf, Wav, Ten, Tok
  NOT sufficient for: Bt

Bt value tower (separate, 𝔽₂)
  sufficient for: Bt only
```

### 3. The Languages

each language has its own page with ops tables, use cases, and proof paths. the canonical list lives in the [[languages]] spec:

{{embed [[languages#the languages]]}}

see [[languages]] for the completeness argument, algebra coverage, and perception mapping. see [[cyb/multiproof]] for how all sixteen settle under one proving umbrella

### 4. Compilation Architecture

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

\* Dif, Sym, Bel are research horizon — proof paths are open mathematical problems.

| Source  | When proof needed                | When proof absent             |
|---------|----------------------------------|-------------------------------|
| Bt      | Binius FRI circuit               | always proving                |
| Rs      | TASM → zheng (word→field lift)   | native binary (Nox)           |
| Tri     | TASM → zheng (field native)      | WASM/EVM (Layer 0)            |
| Arc     | decomposes into Tri              | optimized graph engine        |
| Ren    | geometric product → Tri          | native Clifford engine        |
| Dif     | research                         | native manifold solver        |
| Sym     | research                         | native Hamiltonian integrator |
| Bel     | research                         | native statistical engine     |
| Seq     | temporal constraints → zheng     | scheduler / runtime           |
| Inf     | derivation trace → zheng         | Datalog engine                |
| Wav     | decomposes into Tri              | native DSP pipeline           |
| Ten     | decomposes into Tri              | native BLAS / GPU             |
| Tok     | conservation constraints → zheng | native ledger engine          |

see [[cyb/multiproof]] for how all fourteen languages settle under one proving umbrella via [[Hemera]] and Tri.

### 5. Nine Perception Primitives

the irreducible visual types — the atoms of everything a human can perceive through a screen and speakers. any UI, any document, any application is a composition of these nine. the four new computation languages (Ren, Dif, Sym, Bel) render through existing perception primitives: Ren → vector, Dif → vector, Sym → formula, Bel → formula.

| Primitive   | What it is                    | GPU mapping                          |
|-------------|-------------------------------|--------------------------------------|
| `text`      | Markdown, prose, code         | Glyphs via compute shader            |
| `struct`    | JSON, TOML — trees & configs  | Collapsible tree of text glyphs      |
| `table`     | 2D data, CSV                  | Grid of text cells, virtualized rows |
| `vector`    | SVG, paths, Bezier curves     | Path rasterization via Vello         |
| `pixels`    | Raster image                  | Texture upload, GPU sampler          |
| `video`     | Moving pixels                 | Hardware decode, texture per frame   |
| `sound`     | Waveform, audio stream        | Audio pipeline (visual: waveform shader) |
| `formula`   | LaTeX / MathML                | Glyph layout + vector curves via Vello |
| `component` | Composition of primitives     | Nested render pass                   |

`component` is to perception what `Nox` is to computation. Nox composes computations (cons, axis, branch). component composes renderings (nest, layout, pass).

### 6. Ten Decision Primitives

every human interaction with a computer is a decision. strip the physics away — what remains is pure decision structure.

| #  | Primitive | Action                | Reversible? | Time Mode | Comp Language | Perception |
|----|-----------|-----------------------|-------------|-----------|---------------|------------|
| 1  | observe   | Gather without choosing | Always    | Stream    | Wav           | any        |
| 2  | filter    | Narrow by criteria    | Yes         | Stack     | Inf           | struct     |
| 3  | select    | Choose one            | Yes         | Stream    | Inf           | table      |
| 4  | rank      | Order by preference   | Yes         | Stream    | Ten           | table      |
| 5  | compose   | Build new value       | Yes         | Stack     | Rs            | text/vector|
| 6  | split     | One becomes many      | Depends     | Heap      | Arc           | vector     |
| 7  | merge     | Many become one       | Depends     | Heap      | Arc + Inf     | vector     |
| 8  | delegate  | Route to agent        | Sometimes   | Heap      | Arc           | vector     |
| 9  | reject    | Explicitly not-choose | Mostly      | Stream    | Seq           | video      |
| 10 | confirm   | Irreversible commit   | Never       | Stack     | Trident       | formula    |

the machine computes, the human decides. computation produces options. perception displays them. decision collapses them to action. the action commits to new state, and the cycle continues.

confirm is the only primitive that is always irreversible. it is structurally unique — the moment where possibility collapses into fact. every other primitive can be undone, revised, or abandoned.

### 7. Cross-Grid Connections

the three grids interlock in a continuous decision loop — the cyb/os event loop:

```
loop {
  state   = nox_tree(current)           // authenticated tree
  options = compute(state)              // some universe produces alternatives
  display = render(options)             // canonical primitive shows them
  choice  = decide(human_input)         // decision primitive applied
  proof   = commit(choice, state)       // irreversible, potentially zheng-proven
  state   = update(state, choice, proof)// new tree root
}
```

all three grids share one universal structural pair — fork and join:

```
            fork (one → many)          join (many → one)
            ─────────────────          ─────────────────
Computation  axis (decompose tree)      cons (build pair)
Perception   expand (drill into view)   nest (compose views)
Decision     split (divide choice)      merge (combine choices)
```

fork is how structure grows. join is how consensus forms. the same skeleton wearing three costumes.

### 8. The Comparison Matrix

the per-language comparison matrix — universe, primitive, reference, free/costly ops, proof path, rendering — is maintained in the [[languages]] spec.

---

see [[cyb/features]] for PureRender, smart contracts, legacy web compatibility, and numbers. see [[cyb/os]] for kernel architecture, cells, transport, bounded liveness runtime, and hardware abstraction. see [[cyb/stack]] for the seven crates. see [[cyb/core]] for the proof pipeline.

---

## Build Order

### Phase 1 — Foundation (Now)

1. [[Nox]] — Define the 16-pattern structural IR with abstract Merkle authentication
2. [[Trident]] — Refine compiler and TIR
3. [[Rs]] — Strict [[Rust]] subset, same compiler backend as Trident, target Nox runtime

### Phase 2 — Expansion (Next)

4. [[Arc]] — Graph DSL for [[cybergraph]] programming. Compiles to Trident for proofs, native engine for queries.
5. [[Seq]] — Temporal logic for consensus rules and scheduling. Three temporal modes built in.
6. [[Inf]] — Datalog over the cybergraph. Rule-based inference turns explicit links into implicit knowledge.
7. [[Tok]] — Token conservation language. UTXO constraints compile to [[zheng]], native ledger engine for execution.

### Phase 3 — Specialization (When needed)

8. [[Bt]] — Binary circuits for legacy hash verification and cross-chain bridges.
9. [[Wav]] — Signal processing. Start as Rs library, promote to language if sensor workloads justify it.
10. [[Ten]] — Tensor operations. Start as Rs/Tri library, promote if ML inference verification becomes core.

### Phase 4 — Geometry (Research horizon)

11. Ren — Clifford geometric algebra. Engineering-ready, closest to Tri. Completes the Arc → SVG rendering pipeline.
12. Dif — Differential geometry. Riemannian manifolds over finite fields. Needed for [[tri-kernel]] formalization.
13. Sym — Symplectic geometry. Hamiltonian mechanics, [[conservation]] laws. Physics simulation.
14. Bel — Information geometry. Fisher metric on [[probability]] simplices. Self-model for [[superintelligence]].

see [[languages]] for the algebraic completeness argument. see [[cyb/multiproof]] for how all fourteen settle under one proving umbrella.

---

## Helical navigation

cyberspace — the navigable semantic space of the [[cybergraph]] — has a natural helical geometry derived from the tri-kernel's three operators.

the three axes of cyberspace:

| axis | operator | navigation direction | what you find |
|------|----------|---------------------|---------------|
| D (helix axis) | diffusion | follow links outward from a particle | concepts downstream in the same attention flow |
| S (helix radius) | springs | rotate to structurally equivalent particles | concepts at the same focus level, different domain |
| H (helix pitch) | heat | zoom scale: atom → enzyme → bridge → article → deep | concepts at the same location but different resolution |

the three axes are orthogonal — movement along one does not change position on the others. this gives cyberspace the structure of a helical coordinate system: (D-distance, S-angle, H-scale).

navigation primitives derived from the helix:

```
step_along(p, n)     — follow D-axis n hops; returns particles in the diffusion stream from p
rotate_to(p, domain) — move to S-angle of target domain; returns particles at same focus level
zoom_to(p, scale)    — adjust H-pitch to target size class; returns same concept at new resolution
unwind(cluster)      — compute the winding number of a cluster; returns its topological charge (φ*)
```

the Arc universe (topology language, #4 in the fourteen) is the natural computation substrate for helical navigation. Arc graphs have adjacency algebra — the right structure for winding numbers and helical traversal. Arc's canonical rendering is vector paths — the natural visualization of helical trajectories in the graph.

### Graph world helical visualization

the Graph world (Cmd+2) renders the [[cybergraph]] as a force-directed layout. the helix geometry suggests an alternative: project the tri-kernel's three axes onto a 3D helical coordinate system. particles at the same focus level sit at the same helix radius. high-focus particles (φ* > 0.01) sit near the helix axis. the Graph world can toggle between flat (current) and helical projection — giving an intuitive view of where conceptual density concentrates.

## The Thesis

cyb/os rests on three observations and one boundary.

one. every computational universe has a native type whose algebraic laws define how programs think. forcing computations across universe boundaries creates encoding overhead that scales with complexity. fourteen algebras → fourteen [[languages]].

two. every perceptual channel has a native format whose rendering laws define how humans see. forcing display across format boundaries creates visual noise. nine senses → nine primitives.

three. every human action is a decision with its own algebra: options, preferences, beliefs, commitments. ten decision types → ten interaction primitives.

the boundary. the machine computes, the human decides. computation produces options. perception displays them. decision collapses them to action. the action commits to new state, and the cycle continues.

all values in all universes (except Binary) decompose into three atoms — three modes of reference that are exhaustive:

```
field = the value IS the reference     (by content)
word  = position IS the reference      (by location)
hash  = name IS the reference          (by commitment)
```

these atoms compose through one structural substrate ([[Nox]], authenticated trees). they persist through three temporal modes (stack, heap, stream). they are present through one register — the singular now, the atom of attention where computation happens.

all three grids share one universal structural pair — fork and join — wearing three costumes:

```
Computation:  axis / cons      (decompose / build)
Perception:   expand / nest    (drill in / compose)
Decision:     split / merge    (diverge / converge)
```

fourteen languages. nine primitives. ten decisions. three atoms. three times. one fork. one join. one tree. one [[proof]]. one operating system.

see [[cyb]], [[cyb/whitepaper]], [[languages]], [[cyb/multiproof]], [[Rust]], [[cyber]]