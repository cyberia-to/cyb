---
tags: cyb, core, architecture
crystal-type: entity
crystal-domain: cyber
alias: chroma architecture, the 9
---
# chroma

cyb's UI is a **3×3 grid**: eight fixed overlay elements ("chroma") arranged
around a central renderer slot ("spacetime"). Each chroma owns a fixed
screen position, a single semantic role, and a strictly typed
[[cyberlink]] interface. Chromas never share state directly — every
interaction crosses a typed cyberlink. The architecture is closed: the
nine slots are the entire public surface of cyb.

## layout

```
┌──────────┬──────────┬──────────┐
│  space   │    ad    │   ava    │
│  where   │  answer  │   who    │
├──────────┼──────────┼──────────┤
│  sense   │          │  sigma   │
│  notify  │spacetime │ resource │
├──────────┼──────────┼──────────┤
│  brain   │   com    │   time   │
│   map    │ command  │   when   │
└──────────┴──────────┴──────────┘
```

axes:
- **vertical**: identity (top) ↔ action (bottom)
- **horizontal**: location/perception (left) ↔ value/identity (right)
- **center**: spacetime, the only pluggable slot

## the nine

| slot      | position    | role                                          |
|-----------|-------------|-----------------------------------------------|
| space     | top-left    | location, "where" — current world, breadcrumb |
| ad        | top-center  | advisor — app responses, hints, ambient dialogue without disrupting spacetime |
| ava       | top-right   | avatar — "who" am I, identity, profile        |
| sense     | mid-left    | notifications, sensors, ambient signals       |
| **spacetime** | center  | pluggable renderer (mir / sugarloaf / wry…)   |
| sigma     | mid-right   | resource — wallet, tokens, economic state     |
| brain     | bottom-left | map — knowledge graph navigation, file browse |
| com       | bottom-ctr  | commander — universal command input; keyboard nav is context-sensitive across all chromas |
| time      | bottom-rt   | when — history, log, future planning          |

## spacetime renderers

Spacetime hosts **exactly one** renderer at a time. The current renderer
defines the active "world". Switching renderers = world transition.

| renderer  | crate              | what it shows                  |
|-----------|--------------------|--------------------------------|
| mir       | `mir`              | 3D knowledge graph (wgpu)      |
| sugarloaf | `sugarloaf`        | terminal canvas (wgpu)         |
| wry       | `wry`              | web content (Portal world)     |
| bevy-ui   | `bevy_ui`          | native UI panels               |
| symphony  | (future)           | audio scene                    |

Spacetime owns its own camera (order 0). The eight chromas all live on a
single overlay camera (order 100, no clear). Chromas are agnostic to
which renderer is active — they only see the renderer's identity via the
`space` chroma.

## cyberlink interface

Chromas communicate **only** via cyberlinks — the same primitive the
cybergraph uses on-chain. Direct resource reads across chromas are
forbidden.

### cyberlink shape

A cyberlink is a 5-tuple across three layers:

```
ℓ = (from, to, token, amount, valence)
```

| field     | type      | layer       | meaning                                       |
|-----------|-----------|-------------|-----------------------------------------------|
| `from`    | Particle  | structural  | source — the particle of the sending chroma   |
| `to`      | Particle  | structural  | destination — the particle of the target chroma |
| `token`   | Particle  | economic    | what moves — the intent type as a particle    |
| `amount`  | u64       | economic    | weight of this link (1 for most UI messages)  |
| `valence` | i8 {-1,0,+1} | epistemic | Bayesian judgment: confirm / neutral / refute |

Each chroma has a particle identity. Intents are also particles — not an
enum. Adding a new intent means minting a new particle, not changing
code.

### signal shape

When a single user action must emit multiple cyberlinks atomically, they
are bundled into a **signal**:

```
s = (neuron, links, Δφ*, proof, height)
```

| field    | type            | meaning                                          |
|----------|-----------------|--------------------------------------------------|
| `neuron` | NeuronId        | signing chroma (or the user's neuron)            |
| `links`  | Vec<Cyberlink>  | one or more cyberlinks in this atomic step       |
| `Δφ*`   | sparse vec      | proven focus shift across all links              |
| `proof`  | Option<Proof>   | zheng proof (omitted for local-only signals)     |
| `height` | u64             | block height (0 until finalized on-chain)        |

Local chroma-to-chroma signals carry `proof = None`; they become
provable on-chain when the neuron signs and broadcasts.

### example cyberlinks

| from particle | to particle   | token particle   | meaning                              |
|---------------|---------------|------------------|--------------------------------------|
| com           | spacetime     | submit           | user submitted text                  |
| com           | spacetime     | switch-renderer  | replace active renderer              |
| spacetime     | spacetime     | output           | renderer self-feeds output           |
| ad            | com           | hint             | suggestion / placeholder update      |
| *             | sense         | notify           | any chroma posts a notification      |
| ava           | sigma         | identity         | request identity for resource lookup |
| sigma         | ava           | resource         | reply with current resource state    |
| spacetime     | space         | locate           | tell space which world is active     |
| brain         | brain         | map              | graph navigation self-feed           |
| *             | time          | record           | append event to history              |

A switch-renderer + notify pair (e.g. switching to terminal world) is one
Signal with two cyberlinks — proved and applied atomically.

### invariants

1. A chroma reads its own state and writes cyberlinks. Nothing else.
2. Every chroma must function with zero other chromas present (modulo
   cyberlinks it has subscribed to).
3. Cyberlink delivery is one-way. Replies are separate cyberlinks.
4. Chromas may not block on a cyberlink reply. All flows are async.
5. The eight slot positions are fixed. New "features" compose existing
   chromas via new token particles, not new slots.

## example flows

### `ls` typed in com

1. user types `ls\n` in com
2. com emits cyberlink `(com, spacetime, submit, 1, +1)` carrying the text
3. spacetime sees current renderer is mir — emits `(spacetime, spacetime, switch-renderer, 1, 0)` + `(spacetime, spacetime, output, 1, 0)` as one signal
4. spacetime now sugarloaf, replays the submit into nushell
5. nushell output streams via `(spacetime, spacetime, output, 1, 0)` self-links
6. sugarloaf redraws

### `graph` typed in com

1. user types `graph\n`
2. com emits `(com, spacetime, switch-renderer, 1, +1)`
3. mir takes over; no other chroma touched

### incoming notification

1. external event arrives at sense
2. sense emits `(sense, sense, notify, 1, 0)` self-link to render its slot
3. nothing else fires

### asking the advisor

1. as user types in com, com emits `(com, ad, submit, 1, 0)` partial text
2. ad emits `(ad, com, hint, 1, 0)` with suggestion
3. com renders hint in placeholder slot

## implementation map

**crate layout** — chroma and spacetime live in `prysm` (the visual
crate); `shell` depends on `prysm` and on `cybergraph` for the
Cyberlink / Signal / Particle types:

```
prysm/
├── chroma/
│   ├── mod.rs          # ChromaId, particle identities, signal bus
│   ├── ad.rs           # answer (top-center)
│   ├── ava.rs          # avatar (top-right)
│   ├── brain.rs        # map (bottom-left)
│   ├── com.rs          # commander (bottom-center)
│   ├── sense.rs        # notifications (mid-left)
│   ├── sigma.rs        # resource (mid-right)
│   ├── space.rs        # location (top-left)
│   └── time.rs         # history (bottom-right)
├── spacetime/
│   ├── mod.rs          # renderer slot, swap logic
│   ├── mir.rs          # 3D graph backend
│   ├── sugarloaf.rs    # terminal backend
│   ├── wry.rs          # web view backend
│   └── ui.rs           # bevy-native UI backend
└── molecules.rs        # intra-chroma composition primitives

shell/                  # thin Bevy app — wires prysm + platform I/O
```

**dependency graph**:

```
shell  →  prysm  →  cybergraph   (Cyberlink, Signal, Particle)
                 →  bevy
```

`cybergraph` is the authoritative source for the Cyberlink / Signal
5-tuple. `prysm` owns the visual layer — chromas, spacetime renderers,
molecules. `shell` is the thin Bevy app that boots everything and wires
platform I/O.

Each chroma is a Bevy `Plugin` with:
- its own components & systems
- writes `Signal` onto the bus (one or more cyberlinks, atomically)
- reads `Signal`s where any link targets `to == Self`
- **no** `ResMut` borrow of any other chroma's state

A single `Messages<Signal>` resource is the bus.

## molecules

Within a chroma, **molecules** (`prysm/molecules.rs`) are reusable
composition primitives. Molecules can produce render targets that
spacetime consumes — e.g. a terminal molecule produces a sugarloaf
canvas. Molecules are private to their owning chroma; cross-chroma
composition happens at the cyberlink layer only.

## what this replaces

| old                              | new                                              |
|----------------------------------|--------------------------------------------------|
| `shell/chrome.rs` (ad + com)     | `prysm/chroma/ad.rs` + `prysm/chroma/com.rs`    |
| `worlds/` enum + state           | `prysm/spacetime/` with renderer swap            |
| `PendingShellCmd` resource       | `(com, spacetime, submit, …)` cyberlink in Signal |
| direct `NextState<WorldState>`   | `(com, spacetime, switch-renderer, …)` cyberlink  |
| chrome accessing terminal state  | impossible — different chromas, different crates  |

## why

- bugs like "commander writes into terminal twice" become structurally
  impossible: there is no path for one chroma's keyboard events to
  reach another's input handler.
- adding a new feature is always: emit a new cyberlink, or subscribe
  to an existing one. never: thread a shared resource through five files.
- the architecture is closed and finite. nine slots, one bus, typed
  intents. you can hold the whole picture in your head.

see [[cyb/core]] for the conceptual roles of the nine apps. see
[[cyb/com]] for command intent semantics. see [[cyb/rendering]] for
how spacetime backends share the wgpu device.
