---
tags: cyb, core, networking, sync
alias: the wire, cyb wire, wire protocol, cyb/sync, graph routing, following
crystal-type: pattern
crystal-domain: cyber
---

# wire

how [[cell]]s converge over [[radio]], with no server between them — and why there is almost nothing to configure: **following is a [[cyberlink]], a node's address is a cyberlink, and the wire obeys the graph.**

the code is one module, `crates/cyb/src/wire.rs`; the command is one verb:

```
cy wire up [cell.log]                     # a cell on the wire
cy wire up <peer-id> <ip:port> [cell.log] # first contact; after that the graph dials
```

the bootstrap contact is the only address a human ever types.

## the vocabulary

three well-known [[particle]]s — just the hemera hashes of the words, mintable by anyone:

| particle | link | meaning |
|---|---|---|
| `FOLLOW` | `FOLLOW → neuron` on **my** chain | i want that neuron's chain, live |
| `ANTENNA` | `ANTENNA → endpoint-id` on a node's **own** chain | how to call me: my radio identity |
| `SOCKET` | `SOCKET → packed ip:port` on a node's **own** chain | where my endpoint listens |

a node casts its ANTENNA and SOCKET links as **one atomic signal at startup** — its address record. `follow X` casts one FOLLOW link. that is the entire control surface: subscription, discovery and routing are graph content, replicated by the very mechanism they route. an address learned through *anyone* is enough to call the owner, because the record travels inside the owner's chain like any other signal.

## the session

ALPN `cyb/sync/1`, one QUIC bi-stream per peer pair, over [[radio]]. every blob wears a one-byte tag:

- **HELLO** — the 32-byte [[neuron]] ids the sender wants: its follows, plus itself (another device of mine may hold signals this one lost). answered with a snapshot of exactly those chains. HELLO may repeat at any time — casting a new follow re-hellos every live session, so a subscription made mid-conversation takes effect at once.
- **FRAMES** — [[signal]]s, as the same tape frames the cell's own log holds ([[foculus]] encoding). the opening snapshot and every later live push are the same shape.

everything received feeds the one idempotent commit ([[cell]]'s signal chain dedups equivocation), so **replay, anti-entropy and push are one mechanism**. a frame that applies is forwarded to every other connected peer whose follows want that neuron; a frame that dedups is not re-forwarded, so echoes die at one hop. any topology converges to the union of what its follow edges ask for.

## the graph dials

a background loop watches my FOLLOW links. for any followed neuron whose ANTENNA and SOCKET links my cell holds — and to whose endpoint i am not already connected — it places a call. no address book, no peer list, no config: the routing table *is* the graph, and it updates the way everything updates, by signals arriving.

this is what makes following transitive in practice: follow a stranger knowing only their neuron id, and the moment their chain reaches you through any friend who carries it, your cell reads the address out of the graph and calls them directly.

## proven

2026-09-01, three nodes on one machine:

1. **B** bootstrapped to **A**, cast `follow A` mid-session — the re-hello brought A's address record over at once.
2. **C** joined knowing *only B*. it cast `follow A` — A's chain reached C through B's overlap, C unpacked ANTENNA and SOCKET from graph content, and logged `° dialed … via the graph`: a direct QUIC session to a node it was never told about.
3. A cast a link; C received it on the direct session, B on its own. exactly one session per pair; duplicates died at the dedup.

the earlier pair proof also holds: push both directions, durability across restart (the log replays), anti-entropy across a gap (signals cast while a peer was down arrive on redial), and weight merge when two neurons cast the same axon.

## honest limits

- **neurons are endpoint-derived**, not [[mudra]] keys, and signals are not yet signature-verified on receipt — the wire trusts its peers. mudra identity is the named next step.
- **SOCKET packs IPv4 only.** relay urls and richer reachability are content, and content belongs to blobs — later.
- **echo suppression is pair-grade.** dense topologies want real gossip dedup and reconciliation, which is [[foculus]]'s seat, not the wire's.
- the [[cyb]] shell app does not yet speak the protocol its own `cy` speaks; wiring the graph world to the wire is next.

the legacy JS-era sync (indexer polling) is documented in [[sync]]; the wire replaces it for the soft3 path.
