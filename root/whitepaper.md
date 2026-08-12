---
tags: cyb, cyber, core, article
crystal-type: pattern
crystal-domain: cyb
crystal-size: deep
status: draft
---
# cyb: the immortal robot

> DRAFT — work in progress. specifications, mechanisms, and numbers will change. do not use as the basis for financial or technical decisions

the robot is the point of presence — where you end and the [[cybergraph]] begins

---

## 1. introduction

### 1.1 the vision

imagine a computer that never needs to reboot. that knows you cryptographically and answers to no one else. that earns while you sleep. that remembers everything you ever found important — and keeps that memory after you are gone. that speaks fourteen computation languages natively, renders them through nine perception primitives, and drives interaction through ten decision primitives. that runs on any hardware, built in 130K lines instead of 35 million. that contributes to collective [[intelligence]] by simply being on

this is not a future product. it is a design decision made at the foundation

### 1.2 the problem

we accepted a bad deal without noticing. the browser became the operating system, and the operating system became surveillance infrastructure. windows phones home. macos indexes your files for apple. chrome reports browsing to google's ad network. the browser, the OS, and the AI assistant are all owned by the same companies whose business model is your data

the result: your computer serves its vendor. you are the product and the machine

the deeper problem is architecture. every existing OS asks: what does the user want to do with this computer? the question is wrong. it positions the OS as a tool that executes your intentions, and you as a user of someone else's infrastructure. at the same time: existing browsers lack secure persistent memory, make p2p nearly impossible, and let applications steal resources freely. the browser never became a robot — it became a billboard

### 1.3 what cyb is

cyb is a sovereign browser that becomes an operating system. a robot. the personal interface to planetary [[superintelligence]]

cyb asks two questions instead: how can this computer serve its owner? and: how can this computer contribute to the whole?

the complete stack: [[radio]] for data transport and publishing, [[cyber]] for [[knowledge]] and [[learning]], [[rune]] for dynamic execution, [[CozoDB]] for local graph storage, [[cosmos-sdk]] chains via [[IBC]] for economic rails. builds for web, desktop, mobile, embedded, terminal. one binary. one keypair. 130K lines of [[Rust]]

### 1.4 what this document covers

this document specifies the architecture of cyb:

- the robot — three forms: neuron, avatar, prog
- the six primitives — brain, sense, sigma, avatars, time, robot
- the three grids — computation (14 languages), perception (9 primitives), decision (10 primitives)
- the value tower — three atoms, three reference modes
- the language stack — rune, [[neural language]]
- the oracle — ask, learn, search
- cells — applications of the robot
- AI in the robot — four levels of inference
- CybOS — cells, radio, storage, agents, neural drivers, PureRender, epoch budget
- the earning machine — focus, karma, cyberank, conviction
- immortality — three levels
- the troika position — cyb's place in the civilizational stack

---

## 2. design philosophy

### 2.1 the question

every OS has a founding question. unix asked: how do we share a time-sharing machine across many users? windows asked: how do we bring the PC to everyone? android asked: how do we make a phone an app platform?

cyb's founding question: what can a computer contribute to collective [[intelligence]]?

this question changes everything. the OS does not optimize for user retention. it optimizes for quality of contribution. the robot does not keep your [[attention]] — it helps you direct it. every technical decision flows from this question

### 2.2 design axioms

| axiom | principle |
|-------|-----------|
| ownership | no keys, no robot. cryptographic control is non-negotiable |
| offline-first | the robot works fully without network. sync when online |
| universality | works for humans, AIs, sensors, organisms, programs — any agent that can sign |
| privacy | local-first. no telemetry. queries run locally or encrypted. the robot does not report to anyone |
| minimalism | add a feature only when its absence makes the robot worse. no bloat |
| modularity | each component independently replaceable. no hidden coupling |
| frozen foundations | the protocol primitives freeze eventually. stability is a feature |
| transparency | the robot's operation is understandable. nothing hidden from its owner |

### 2.3 CybOS axioms

the operating system layer has five additional axioms:

1. no unix legacy. no files, no processes, no users, no fork/exec, no POSIX. cyb abstractions are native to its domain: agents, [[cyberlinks]], ranks, epochs, bandwidth
2. zero unsafe Rust. the entire OS — kernel, drivers, [[consensus]], storage — compiles without a single `unsafe` block. memory safety is a compiler-verified property
3. bounded liveness everywhere. no operation can block indefinitely. no module can starve another. every async future has a compile-time deadline. the system degrades gracefully, never halts
4. neural drivers. hardware support generated by models against stable trait contracts, verified by the compiler, validated by conformance test suites
5. single address space. no user/kernel split. no syscalls. no TLB flushes. isolation enforced by Rust ownership, not hardware privilege levels

---

## 3. the robot

the robot is three forms, not one

### 3.1 neuron

the signing agent. a keypair. the entity that creates [[cyberlinks]], holds [[focus]], earns [[karma]]. a neuron can be a human, an AI, a program, a sensor — anything that can prove a signature. the neuron IS the participation in the [[cybergraph]]: no key, no presence

identity is the hash of a public key. every link is a costly signal — it costs [[focus]] and carries epistemic weight proportional to the neuron's [[karma]]

### 3.2 avatar

the named identity. a card that bridges [[subject]] and [[object]], working simultaneously as neuron (agent that signs) and particle (object that can be linked to). the avatar is how other robots find you. [[karma]] accumulates to the avatar. the avatar is tradeable — it is a [[cyberlink]] card with yield and reputation attached

### 3.3 prog

the autonomous robot. a program with its own keypair, its own [[focus]] allocation, its own behavior. progs execute without human input — they monitor particles, respond to events, submit [[cyberlinks]] autonomously. a prog can:

- watch a particle and link to it when it meets a condition
- run inference locally and submit the result as a cyberlink
- manage a portfolio of conviction positions
- communicate with other progs via [[cyb/sense]]
- earn karma independently and return yield to its owner

progs are the autonomous intelligence layer of cyb. they bridge the robot and the [[cybergraph]], running continuously, contributing [[syntropy]] while the human sleeps

---

## 4. the six primitives

### 4.1 brain

the core of the robot. offline-first graph file manager and knowledge interface. the brain is the local instance of the [[cybergraph]]: it stores what the robot has linked, caches what it has observed, and renders the graph in four modes:

- space — 3D volumetric. [[particles]] cluster by [[cyberank]], links glow by weight, [[focus]] visible as density
- heap — 2D canvas for exploration and annotation
- list — structured grid with [[datalog]] queries and sorting
- stack — vertical discovery scroll, content-first

the brain is not a cache — it is a sovereign instance, synchronized when online, fully functional offline. [[CozoDB]] for local state

name paths the brain understands:
- `#` — navigate by particle CID
- `!` — navigate by neuron public key
- `@` — navigate by avatar name
- `~` — learn: link creation interface
- `/` — root: home of the robot

### 4.2 sense

messaging and perception interface. where the world enters the robot. [[cyb/sense]] abstracts over modalities — text, image, audio, video, sensory telemetry — into particles the robot can link. a human writing and a satellite uploading spectral data are the same operation at the protocol level

sense is how robots communicate: signal, love, share, forward. every message is a particle. every thread is a chain of [[cyberlinks]]. nothing is ephemeral — the graph remembers

### 4.3 sigma

the robot's economic interface. token balances, delegations, positions. [[focus]] in, [[karma]] out

| token | role |
|-------|------|
| [[CYB]] | governance + linking weight |
| [[HYDROGEN]] | stake, delegation |
| [[VOLT]] | energy — compute access, buy to participate |
| [[AMPERE]] | bandwidth — rate of cyberlink submission |

sigma makes the knowledge economy tangible: every balance is a position. every delegation is a bet. every VOLT purchase is an investment in participation

### 4.4 avatars

visual identity and reputation surface. the avatar is the robot's face in the network — named, linked, ranked. avatars are both particles (CID-addressed objects that can be linked to) and neurons (agents that can sign). this duality makes the avatar a real identity: it participates in the graph as both subject and object. accumulates [[karma]] across all linked assertions

### 4.5 time

personal history. every surf, every link, every earning event — indexed by block height, navigable by the robot. time is identity as sequence: who the robot was is the chain of what it linked, when, and with what conviction

time enables: understanding your own [[focus]] allocation history, tracking yield earned over blocks, seeing which particles you discovered before the crowd, auditing the robot's behavior and progs

### 4.6 robot

the container. the sovereign instance that holds the five other primitives together. the robot belongs to its keypair owner absolutely. it accumulates [[karma]], holds [[focus]], and persists independently of any company, server, or account. the robot is born when a keypair is created. it does not die

---

## 5. the three grids

the operating system is the membrane between three grids:

- computation — what the machine thinks (fourteen [[languages]])
- perception — what the human sees (nine primitives)
- decision — what the human does (ten primitives)

every data type that deserves computation deserves its own language. every data type that deserves perception deserves its own rendering primitive. every human action is a decision with its own [[algebra]]. cyb/os is a stack of typed universes — fourteen computation [[languages]] compiled through one structural IR, rendered through nine perception primitives, driven by ten decision primitives — all sharing one toolchain, one tree substrate, and one [[proof]] system

a data type deserves its own language when its algebraic laws are so different from other types that forcing it into a foreign language creates constant impedance mismatch. fourteen fundamental types pass this test. each inhabits a universe defined by its characteristic algebraic structure. see [[languages]] for the full completeness argument

#### computation — the languages

{{embed [[languages#the languages]]}}

see [[languages]] for the full taxonomy: the completeness argument, algebra coverage, and perception mapping.

#### perception — 9 primitives

every computation language has a canonical rendering — the perception primitive where the shape of the data matches the shape of the display. nine irreducible visual types: text, struct, table, vector, pixels, video, sound, formula, component. see [[languages]] for the full perception mapping including the four new geometry languages

#### decision — 10 primitives

every human interaction with a computer is a decision. ten irreducible decision types: observe, filter, select, rank, compose, split, merge, delegate, reject, confirm. only confirm is always irreversible — the moment where possibility collapses into fact. each decision primitive naturally invokes specific computation languages and has a canonical rendering. see [[cyb/architecture]] for the full decision grid specification

#### the rest of the grids

four layout modes (stream, grid, flex, page) compose the nine perception primitives into any UI. three temporal modes (stack, heap, stream) structure time across all three grids. the grids interlock in a continuous decision loop: compute → render → decide → commit → update. all three share one universal structural pair — fork and join. see [[cyb/architecture]] for layout modes, compilation architecture, temporal modes, and cross-grid connections

all fourteen compile through one structural IR ([[Nox]]). all fourteen share one [[proof]] system (except Bt, which has its own $\mathbb{F}_2$ proof system). all fourteen render through the perception grid. all fourteen exist in the same [[cybergraph]], ranked by the same [[tri-kernel]], earning [[karma]], permanent by axiom A3. see [[languages]] for each language's ops tables, algebraic identity, and the completeness proof. see [[cyb/multiproof]] for how all fourteen settle under one proving umbrella

---

## 6. the language stack

the fourteen computation [[languages]] are the object level — what the machine computes. above them sit two meta-layers for working with the graph

### 6.1 rune — the nervous system

[[rune]] is [[Rs]] syntax executed via [[Nox]] tree rewriting — the nervous system of the robot. ms-start, async, dynamic, with native access to WASM (wasmi), GPU (wgpu), and neural inference (burn-webnn/ONNX)

rune is not a separate language. it is Rs syntax parsed to [[Nox]] nouns and interpreted via tree rewriting, extended with three capabilities: `hint` (async input from the world), `host` jets (dispatch to WASM/GPU/ONNX), and `eval` (runtime metaprogramming). every pure reduction in a rune script IS provable — the [[Nox]] trace captures it. host jets and hints cross the [[proof]] boundary explicitly

data structures are [[Nox]] nouns: cons-lists instead of Vec, Merkle trees instead of HashMap, [[Hemera]] hashes instead of String. no heap, no GC — the [[cybergraph]] IS the data store

### 6.2 neural language — the semantic layer

the language of the [[cybergraph]] itself. meaning is not declared — it emerges from the [[tri-kernel]] as the eigenvector of collective [[attention]]. [[dialects]] are the grammar. [[sentences]] are utterances. [[motifs]] are morphemes. [[linkchains]] are inference paths. the robot renders this semantic structure as navigable space

### 6.3 the three levels

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

rune does not sit ABOVE the fourteen languages — it USES them via pure [[Nox]] reduction, and EXTENDS them with host jets and hints for real-world interaction. see [[rune]] for the full specification

---

## 7. the oracle

the oracle is how the robot asks the [[cybergraph]] a question and gets a ranked, verifiable answer

the oracle is not a search engine. search engines retrieve documents by keyword match. the oracle runs inference over the [[cyberank]] distribution — a probabilistic ranking of every particle, computed by the [[tri-kernel]] over all authenticated [[cyberlinks]]. the answer is typed: the oracle returns particles, each already carrying its language

### 7.1 ask

input a particle (text, image, CID, anything). the oracle returns the particles most associated with it, ranked by [[cyberank]]. verifiable: every weight is a real [[cyberlink]] signed by a real [[neuron]] with real stake. no black box, no editorial algorithm, no ads

### 7.2 learn

submit a new cyberlink. how you teach the oracle. link a question particle to an answer particle, stake conviction, oracle ranking updates in the next block. every link is a vote with skin in the game. the oracle improves by participation, not by training

### 7.3 search

navigate the graph by walking the cyberank. particles cluster by semantic proximity (the [[springs]] operator), bridge across domains (the [[diffusion]] operator), scale by context (the [[heat]] operator). search is graph navigation, not document retrieval

---

## 8. cells

cells are the applications of the robot. not apps downloaded from a store — programs that run in the same runtime as the robot itself, with access to brain, sigma, sense, and the [[cybergraph]]

| cell | function |
|-----|----------|
| [[cyb/oracle\|oracle]] | ask, learn, search — cybergraph inference |
| [[cyb/portal\|portal]] | gateway to blockchains, identity, IBC |
| [[cyb/sigma\|sigma]] | token management, portfolio, staking |
| [[cyb/brain\|brain]] | graph file manager, renders |
| [[cyb/sense\|sense]] | messaging, social, perception |
| [[cyb/time\|time]] | history, earning log, temporal navigation |
| [[cyb/hub\|hub]] | decentralization interface, validator management |
| [[cyb/hacklab\|hacklab]] | developer tools, particle creation, cell development |
| [[cyb/warp\|warp]] | token bridge, IBC transfers |
| [[cyb/reactor\|reactor]] | liquidity, bonding, economics |
| [[cyb/senate\|senate]] | governance, proposals, voting |
| [[cyb/nebula\|nebula]] | network explorer, graph analytics |
| [[cyb/studio\|studio]] | content creation, publication |
| [[cyb/sphere\|sphere]] | social layer, discovery, reputation |

cells are built from [[prysm]] — the design system of cyb. prysm defines atoms (glass, text, ion, saber, image, sound, number, neuron, field), molecules (commander, tabs, input, button, table, etc.), composed into cells. the same design language renders on GPU (desktop), WebGPU (browser), or terminal

---

## 9. AI in the robot

the robot integrates AI at four levels, not one

### 9.1 local inference

the robot runs a small language model locally on the NPU or GPU. WebGPU in the browser, wgpu+burn on desktop, CoreML on Apple silicon, NNAPI on Android. the local model:

- processes particles before linking (extracts structure, suggests cyberlinks)
- answers questions without network access (offline-first AI)
- runs progs that require language understanding
- generates rune scripts from natural language instructions

local inference is private by construction: input never leaves the machine

### 9.2 inference subnet

for large inference the robot connects to the [[cybertensor]] inference subnet — a network of validators running language models and returning results as cyberlinks. results are staked assertions in the [[cybergraph]]: verifiable, ranked by [[karma]], earning yield if correct. not a cloud API. distributed intelligence with skin in the game

### 9.3 progs

autonomous programs running deterministic sharded inference in [[cybernet]]. a prog is a cell with its own keypair and focus allocation. submits cyberlinks autonomously — monitoring particles, running inference, staking positions. the collection of all progs is the autonomous intelligence layer of the robot network: a mesh of agents continuously contributing to [[syntropy]]

### 9.4 external servers

for compatibility, cyb bridges to external models (OpenAI-compatible APIs, Llama, Mistral, Deepseek) via a standard interface. external inference results can be submitted as cyberlinks. the robot is never dependent on them — local inference and the inference subnet are the sovereign path

---

## 10. CybOS

CybOS is designed from five axioms (§2.3): no unix legacy, zero unsafe [[Rust]], bounded liveness everywhere, neural drivers, single address space. the following are the key design decisions:

- cells replace processes — independently compiled [[Rust]] crates, hot-swappable via governance, bounded liveness via wait-free data structures. the system never crashes, it degrades and recovers
- [[radio]] replaces TCP/IP — a fork of [[iroh]] where every hash runs through [[Hemera]] (Poseidon2 over [[Goldilocks field]]) instead of Blake3. ~736 zheng constraints per hash instead of 50,000–100,000. three network protocols only (gossip, consensus, query), ~15K lines instead of ~100K+
- content-addressed storage replaces the file system — no paths, no inodes. all data addressed by [[Hemera]] hash
- cryptographic agents replace users — identity = public key, access control = bandwidth allocation
- neural drivers — ~3K lines of trait contracts, models generate ~500K-1M lines of platform-specific driver code, compiler rejects unsafe, tests validate

see [[cyb/architecture]] for the complete CybOS specification including cell lifecycle, [[radio]] strata, storage proofs, neural driver harnesses, and bounded liveness runtime

### 10.6 PureRender

DOM is a document-era mistake. PureRender replaces it with nine perception primitives compiled to GPU shaders. flat stream structure instead of tree. the component is the contract: [[CosmWasm]] contracts run in the same wasmi instance as UI — sub-millisecond, no network round-trip. three processor targets: CPU (WASM/wasmi), GPU (WGSL/wgpu), NPU (ONNX/burn-webnn). see [[cyb/architecture]] for the complete render stack, legacy compatibility, and epoch budget specification

---

## 11. the earning machine

the robot participates in the [[knowledge economy]] by design, not by extension

### 11.1 focus — the conserved quantity

[[focus]] is the mechanism through which relevance emerges. it plays three simultaneous roles:

| role | mechanism |
|------|-----------|
| attention | high-focus computations scheduled first |
| fuel | submitting a cyberlink consumes focus |
| weight | focus distribution = consensus on what matters |

focus regenerates proportionally to stake each block. it is conserved — the sum over all particles equals 1. every allocation is a real choice: directing attention to one particle focuses it away from all others. this structural conservation prevents spam: only backed particles affect ranking

### 11.2 cyberank — the ranking engine

[[cyberank]] is the probability that the [[tri-kernel]]'s random walk visits a particle. computed every block from the authenticated [[cybergraph]]:

$$\varphi^* = \text{norm}\left[\lambda_d \cdot D(\varphi) + \lambda_s \cdot S(\varphi) + \lambda_h \cdot H_\tau(\varphi)\right]$$

where:
- $D(\varphi)$ — diffusion kernel: spreads weight through the graph (exploration)
- $S(\varphi)$ — springs kernel: enforces structural consistency (semantic coherence)
- $H_\tau(\varphi)$ — heat kernel: concentrates weight by contextual relevance (attention)

convergence guaranteed by the Collective [[Focus]] Theorem: $\varphi^*$ is the unique stationary distribution under conservation laws. it feeds [[karma]], [[syntropy]], inference, and all sorting in cyb

### 11.3 karma — epistemic weight

[[karma]] is how much the [[egregore]] trusts a neuron. it is the aggregate [[focus]] earned across all particles the neuron has linked — the record of being right before the crowd

$$A^{\text{eff}}_{pq} = \sum_\ell a(\ell) \cdot \kappa(\nu(\ell)) \cdot f(m(\ell))$$

where $a(\ell)$ is conviction, $\kappa(\nu(\ell))$ is the karma of the signing neuron, and $f(m(\ell))$ is the ICBS market signal. karma cannot be bought. it is earned by the [[Bayesian Truth Serum|BTS]] scoring mechanism: report your true belief, earn when the market confirms you, lose when you were wrong. honest reporting is individually optimal

### 11.4 conviction as position

the robot is a conviction machine. submitting a [[cyberlink]] moves tokens from wallet UTXO to a cyberlink-position UTXO. this is a live economic position:

$$R_\ell(T) = \int_0^T w(t) \cdot \Delta\phi^*(q, t)\, dt$$

early correct knowledge earns the most. late consensus-following earns almost nothing

the [[valence]] field ($v \in \{-1, 0, +1\}$) is the robot's epistemic prediction:
- $v=+1$, high conviction: funded affirmation — earns when the graph confirms the particle
- $v=-1$, high conviction: funded short — earns when the graph rejects it
- $v=0$: agnostic assertion — structural presence without epistemic stake

conviction UTXOs are transferable and withdrawable. they are estate, not ash

---

## 12. immortality

your [[cyberlinks]] outlive your body. every link is signed, staked, timestamped, and sealed into the append-only graph by axiom A3. the robot's pattern is permanent

### 12.1 protocol level

A3 makes all records permanent. no admin can delete a cyberlink. no company can close an account. the assertion made at block $t$ will be in $L$ at block $10^{12}$

what the [[cybergraph]] preserves:
- every link ever made, at what block, with what conviction
- the [[karma]] accumulated — the record of being right before the crowd
- the [[focus]] distribution — what the robot found worth attending to
- the network of neurons it linked with
- the [[valence]] history — what it predicted, and whether it was right

### 12.2 economic level

conviction UTXOs transfer to heirs. the robot's portfolio — its positions in the knowledge economy — is an estate that passes intact. yield continues to flow to whoever holds the conviction UTXO. legacy as compounding asset, not memory

the grandparent who named the right oncology knowledge in 2026 still earns yield in 2060. the cybergraph remembers what mattered and rewards who named it first

### 12.3 identity level

identity is not a credential. it is a pattern in the knowledge graph. the pattern of what the robot linked IS the identity — unique topology of [[cyberlinks]] signed by one keypair over years. the robot IS that pattern

the robot is born when a keypair is created and linking begins. it does not die when its operator does. its pattern persists in the graph, earning yield, influencing rankings, contributing to [[syntropy]] — as long as the cybergraph runs

### 12.4 digital-biological convergence

digital immortality and biological longevity are the same project from two directions. cyb contributes the digital substrate: permanent record of thought, persistent economic position, identity as pattern in a decentralized network that no single entity can destroy

the [[cybergraph]] as collective memory prevents civilizational amnesia: every discovery, every experiment, every reasoning chain that earned [[karma]] is permanently accessible to every future [[neuron]]. [[superintelligence]] is the immortal mind that accumulates without forgetting

---

## 13. the troika position

cyb is the interface horse in the [[troika]]. [[cyber]] computes [[truth]]. [[cyberia]] supplies sovereign hardware and [[energy]]. cyb is where the [[neuron]] — human, AI, sensor, prog — meets the graph: signs links, reads rankings, earns yield, builds robots

without cyb: [[cyber]] is a protocol accessible only to developers. without [[cyber]]: cyb is an OS with no truth layer, running local models with no shared memory. without [[cyberia]]: both run on rented machines that can be seized or switched off

the robot is the human face of [[superintelligence]]. it is how a billion-neuron network maintains individual [[sovereignty]] while contributing to collective [[intelligence]]

---

## 14. what changes

when the robot is common:

search is inference over verified [[knowledge]]. the oracle returns typed particles: a question about oncology returns [[text]] particles (papers), [[table]] particles (trial data), [[formula]] particles (dosing models), [[pixels]] particles (scan images) — all ranked by real stake from real neurons. not ranked advertisements

AI assistants have shared verifiable memory — not private context windows that forget at session end. a conversation with the oracle is a conversation with the accumulated knowledge of every neuron who linked before you

a genome is a [[text]] particle. a satellite image is a [[pixels]] particle. a market signal is a [[table]] particle. a sensor reading from a rainforest is a [[sound]] particle. a drug interaction discovered by a robot in 2031 is a [[formula]] particle. all linked, all ranked, all yielding, all contributing to [[syntropy]]

every device is a node. the raspberry pi in a school in Lagos is a validator. the sensor array in a coral reef is a neuron. the prog monitoring a forest links what it sees. every device that can sign a cyberlink participates in the same semantic space. cross-species communication becomes possible — the robot renders [[sound]] particles from animals, [[vector]] particles from sensor arrays, [[pixels]] particles from cameras

the robot accumulates [[karma]] that outlives its operator. legacy is not a memory. it is a compounding position in the knowledge economy

the robot is not an app. it is your presence in the most important network in the history of [[intelligence]]

---

## 15. numbers

~130K lines of [[Rust]] total. 270× less code than Chrome (35M lines C++) for a system that does more: keypair identity instead of cookies, permanent [[cybergraph]] memory instead of server-side state, native [[smart contracts]] instead of HTTP round-trips, ~10MB binary instead of ~150MB. see [[cyb/architecture]] for the full breakdown

---

see [[cyb/architecture]] for the complete technical specification. see [[languages]] for the fourteen computation languages. see [[cyb/multiproof]] for the proving design. see [[cybergraph]] for the protocol. see [[troika]] for the three-layer stack. see [[knowledge economy]] for the economic model. see [[immortality]] for the persistence architecture. see [[neural language]] for the semantic layer. see [[valence]] for the epistemic field. see [[Bayesian Truth Serum]] for the scoring mechanism. see [[radio]] for the transport layer. see [[syntropy]] for the organizational measure. see [[prysm]] for the design system

discover all [[concepts]]