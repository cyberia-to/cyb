---
tags: cyber, core
crystal-type: pattern
crystal-domain: cyber
crystal-size: bridge
alias:: patch, cyberpatch, patch system, patches
icon: "\U0001FA79"
stake: 39885708873010200
diffusion: 0.00015236411240884443
springs: 0.0020808786576084087
heat: 0.0014531528872379856
focus: 0.000991076230934571
gravity: 2
density: 4.61
---

content-addressed, identity-sovereign [[patch theory]] system for the [[cybergraph]]. treats changes as commutative morphisms instead of snapshots — independent patches apply in any order, [[conflicts]] are first-class data, merge is set union

## cybergraph embedding

every patch is a signed set of operations over [[particles]] and [[cyberlinks]], authored by a [[neuron]], weighted by [[focus]] contribution. the three primitives map directly to [[cyber]] protocol:

- patch = [[cyberlink]] (signed, timestamped, weighted by Δπ)
- tracked content = [[particle]] (content-addressed node)
- channel = named view over the global patch DAG
- repository = [[neuron]]-owned subgraph
- author = [[neuron]] (identity + stake + [[focus]] vector)

this embedding is literal — cyberpatch repositories ARE [[cybergraph]] structures, queryable and rankable by the [[consensus]] layer

## from [[patch theory]]

the mathematical core comes from [[category theory]]: repository states are objects, patches are morphisms, composition is sequential application. the key departure from [[git]]: a patch is defined by what it changes, independently of the history that produced the source state

three relations between patches:

- independent (P ⊥ Q) — disjoint regions, patches commute, merge is set union
- dependent (P → Q) — Q requires P in its [[dependency]] closure
- conflicting (P ⊗ Q) — incompatible changes to the same region, producing a first-class [[conflict]] object

the [[commutativity]] theorem guarantees that any set of pairwise-independent patches produces the same result regardless of application order. this eliminates phantom [[conflicts]] that plague snapshot-based systems

## five primitive operations

all mutations over the [[cybergraph]] reduce to five atoms:

- AddParticle — introduce new [[particle]]
- RemoveParticle — remove [[particle]] from tracked set
- AddEdge — link two [[particles]]
- RemoveEdge — remove a link
- ReplaceParticle — atomic content swap

## conflict resolution

[[conflicts]] between concurrent patches are algebraic objects with well-defined structure — they can be resolved by further patches, left in state, or arbitrated by [[consensus]]. a resolution patch R has both conflicting patches in its [[dependency]] closure — once applied, the resolution propagates permanently across all channels

when local resolution is unavailable, the network arbitrates through [[focus]]-weighted voting: stake × focus_weight determines voting power, tying version control directly to [[cyber]]'s economic and epistemic [[consensus]]

## economics

patches earn rewards proportional to their impact on the [[knowledge]] graph:

```
reward(P) = base_fee + Δπ(P) × reward_coefficient
```

Δπ measures the change in network [[focus]] from applying a patch. patches that increase [[knowledge]] coherence earn rewards; patches that fragment or duplicate earn less. this creates economic pressure toward high-quality, well-connected contributions — aligned with [[collective focus theorem]] predictions

## agent workflows

designed for parallel [[neuron]] and agent workflows at planetary scale. multiple agents operate simultaneously — no coordination required to produce patches, only at resolution time. [[GFlowNet]] agents propose patches weighted by expected Δπ. [[active inference]] agents minimize [[free energy]] by adaptively staking on patches

post-quantum [[cryptography]] from [[genesis]]. [[hash]] via [[Poseidon2]]-Goldilocks, [[signatures]] via the protocol's post-quantum scheme, proofs via [[starks]] over [[Goldilocks field]]

see [[cyber/patch/spec]] for the full specification. see [[cyb/fs/sync]] for how patches pass through the five verification layers of [[structural sync]]