---
tags: cyb, cyber, core
alias: cyb filesystem, cyber filesystem, cyb/fs
crystal-type: entity
crystal-domain: cyb
---
the [[cybergraph]] as a filesystem — content-addressed, append-only, patch-based

every [[particle]] is a file. every [[cyberlink]] is a reference. every [[neuron]] has a home directory (`~/`). the filesystem is the graph, navigated via [[markup|cybermark]]

## operations

| Operation | What it does | Page |
|---|---|---|
| read | query any [[particle]] by [[Hemera]] hash or path | native — no special mechanism |
| create | hash content → new [[particle]] → first [[cyberlink]] names it | [[cyber/link]] |
| edit | create a new [[particle]] with modified content → link old → new | [[cyb/fs/edit]] |
| patch | commutative morphism over [[particles]] and [[cyberlinks]] | [[cyb/fs/patch]] |
| delete | withdraw conviction + valence -1 — structural record stays, economic weight removed | [[cyber/link]] |

there is no mutation. editing creates a new [[particle]] (new hash). the old version persists permanently (axiom A3: append-only). the diff between versions is itself navigable

## addressing

three ways to reach a [[particle]]:

```
#QmXyz...           by content hash (immutable, permanent)
cyber/truth         by path (mutable, human-navigable)
~market             by name (per-neuron, personal)
```

see [[markup]] for the full sigil grammar. see [[cyberspace]] for navigating the filesystem as a space. see [[cyb/fs/sync]] for how file operations sync across devices with five-layer verification. see [[cyb/fs/patch]] for commutative patch semantics