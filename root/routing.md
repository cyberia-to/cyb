---
tags: cyb, core, architecture
crystal-type: pattern
crystal-domain: cyber
---

# cyb routing

one address space across all worlds. uri as the navigation primitive. the address bar is the universal interface.

---

## the insight

cyb is heterogeneous — text streams, 3d spatial graphs, terminal sessions, media playback, external web. these run in different rendering models ([[rendering]]) but the [[neuron]] should navigate across them with one consistent mental model. that model is the uri.

browsers normalized urls for http content. cyb normalizes them for content-addressed, multi-world computing. typing `cyb://graph/ai/safety` in the address bar dispatches the graph world focused on a specific location. typing `cyb://particle/bafyrei...` dispatches the particle viewer. typing `cyb://app/oracle` launches the oracle cell. one input, one mental model, every destination.

---

## the uri scheme

the `cyb://` scheme is the navigation contract. its authority/path components select a world and tell that world what to do.

```
cyb://<world>/<path>?<query>
```

| world | resolves to | example |
|---|---|---|
| `terminal` | sugarloaf running an optional shell | `cyb://terminal/?cmd=nu` |
| `graph` | mir focused at a graph location | `cyb://graph/ai/safety/alignment` |
| `app` | a named sugarloaf-hosted cyb app | `cyb://app/oracle` |
| `particle` | particle viewer (renderer dispatched by content type) | `cyb://particle/bafyrei...` |
| `neuron` | conversation with a peer | `cyb://neuron/bostrom...` |
| `session` | replay a recorded byte stream | `cyb://session/bafyrei...` |
| `web` | wry loading an external url | `cyb://web/https://wikipedia.org/...` |
| `media` | media player for a particle | `cyb://media/bafyrei...` |

the scheme is the dispatcher. typing or pasting any `cyb://` uri into the address bar resolves it to a world transition. the rest is the world's argument: a path inside the graph, a particle hash, a shell command, an external url.

addresses are first-class. they are shareable — paste a `cyb://` uri into a chat, an email, another particle, an external site that links into cyb. they are bookmarkable — save to a particle, recall later. they are recordable — every navigation enters history. they are programmatic — agents compose uris and request transitions.

---

## the address bar

the address bar is the top row of [[rendering|chrome]]. it is rendered by bevy as a cell-grid overlay above whichever world is active.

```
┌─ cyb://graph/ai/safety/alignment ─────────────  master · 12 peers · 482 ξ ─┐
```

four roles, all served by one widget:

| role | mechanism |
|---|---|
| show where you are | render the current uri on the left |
| go elsewhere | click or hotkey to focus → edit → enter to navigate |
| share location | copy the uri string from the bar |
| identity context | render neuron, stake, peer count on the right |

the address bar follows the [[neuron]] across every world. switching from terminal to graph does not lose the bar. it is always editable, always reflects the current location.

---

## the commander

the commander is the bottom row of chrome. it accepts commands, not addresses. addresses go in the address bar; commands go here. they look similar but serve different purposes.

```
│  > _                                                          ⌘k commander │
```

commands operate on the current world or invoke cross-world actions

| command | effect |
|---|---|
| `search <query>` | run a query in oracle, regardless of current world |
| `go <address>` | shortcut for typing into address bar |
| `chat <neuron>` | open a conversation; equivalent to `cyb://neuron/<neuron>` |
| `record start` | begin recording the current session as a particle |
| `pipe <cmd>` | pipe the current world's output through a transform |
| nushell command | execute in the terminal world if active |

commands are the action layer; addresses are the navigation layer. some commands navigate (`go`, `chat`, `play`); others act in place (`search`, `record`). all are typeable, all are scriptable, all are recordable.

---

## the navigation flow

```
[neuron] types `cyb://graph/ai/safety` in address bar
  → address bar fires Navigate event with parsed uri
  → bevy reads the world component of the uri → Graph
  → bevy state transition: NextState<World>::set(Graph)
  → previous world's render pass deactivated (with optional 150ms crossfade)
  → mir activates, reads the path component → focuses camera on /ai/safety
  → chrome continues rendering above; address bar shows the new uri
  → history pushes the previous uri onto the back stack
```

three concerns, three clean ownerships

| concern | owner | mechanism |
|---|---|---|
| parsing the uri | address bar | `cyb://world/path` → `(World, String)` tuple |
| world dispatch | bevy | `NextState<World>::set(...)` based on uri scheme world component |
| within-world routing | the active world | uses the path component to focus / select / launch |

each world implements its own path semantics. mir parses `/ai/safety/alignment` as a knowledge graph path. sugarloaf with `app/oracle` parses `oracle` as the cell name to launch. wry with `web/https://...` treats the rest as the external url. world-specific routing is opaque to bevy and the address bar — they only need to dispatch.

---

## navigation triggers

multiple ways to navigate, one mechanism

| trigger | mechanism |
|---|---|
| typing in address bar | bevy reads uri, dispatches world transition |
| clicking a link in cell-grid content | sugarloaf detects osc 8 hyperlink, fires the same Navigate event |
| pressing a hotkey | bevy fires Navigate event with the hotkey's bound uri |
| commander `go <uri>` | commander parses, fires Navigate event |
| agent emitting a uri in its stream | osc-tagged hyperlink → click → Navigate event |
| voice input → uri | speech-to-text result piped through commander |
| external app deep link | os opens cyb with a `cyb://` uri argument → Navigate event |
| programmatic navigation from code | direct call to bevy's NavigationSystem |

all of these funnel into the same Navigate event. the navigation contract is the event; the trigger is incidental. no special cases per trigger.

---

## history is the cybergraph

navigation history is not a sidecar data structure. it is the [[cybergraph]] itself. every state transition is a [[cyberlink]] — the same primitive that records every other meaningful event in cyber. the [[neuron]]'s path through cyb is a trace of cyberlinks, permanent, content-addressed, and queryable like any other knowledge in the graph.

this is the fundamental shift from browser-style history. a browser tracks a per-session stack that dies when the window closes. cyber tracks navigation as part of the graph that survives across sessions, devices, and time. closing cyb and reopening it next year, you can still ask "what was i looking at on 2026-03-12?" and traverse the cyberlinks. you can share a slice of your trace with another neuron. you can join your trace with someone else's at a common ancestor. you can compute karma over navigation patterns.

### the navigation cyberlink

every Navigate event emits a [[cyberlink]] in the canonical form

```
ask(ν, p, q, τ, a, v, t):
  ν = neuron       — who navigated
  p = from-uri     — source location (hashed to a particle)
  q = to-uri       — destination location (hashed to a particle)
  τ = navigation   — the dialect for state transition
  a = 0            — no stake on routine navigation (configurable per neuron)
  v = +1           — positive valence by default
  t = timestamp    — exact transition time
```

the from-uri and to-uri are themselves [[particle|particles]] — content-addressed by their hash. a uri like `cyb://graph/ai/safety` becomes a particle whose content is its uri string. particle references (`cyb://particle/<hash>`) are already particles. so navigation cyberlinks compose with the rest of the graph naturally — the destinations exist as graph nodes, and the transitions are edges between them.

navigation cyberlinks land in the `locations` dimension of [[bbg|BBG_poly]] (per [[evy]]'s namespace schema, dimension 4 — spatial proofs). they are committed at tick boundary like any other cybergraph state. a navigation today is provable forever.

### privacy by default

navigation is private by default. cyberlinks emitted by Navigate events go to the `A(x)` private commitment polynomial, not the public dimensions. only the neuron and parties they explicitly grant access to can read their trace.

three privacy modes

| mode | where | who can read |
|---|---|---|
| private | A(x) commitment polynomial | only the neuron |
| shared | encrypted, addressed to specific neurons | recipients + the neuron |
| public | locations dimension (public BBG_poly) | anyone |

the default is private. publishing navigation is opt-in per-uri or per-session. a neuron can mark certain destinations as always-public (their published bibliography of work) and others as never-public (private exploration).

### local cache as a view

bevy maintains a local cache of recent cyberlinks for fast back/forward — but it is a cache, not the source of truth. the cybergraph remains authoritative.

```rust
struct NavigationCache {
    cursor:    usize,
    cyberlinks: Vec<Cyberlink>,   // recent local view; full history is in cybergraph
}
```

three navigation actions

| action | local effect | cybergraph effect |
|---|---|---|
| navigate to new uri | append cyberlink, advance cursor, truncate forward | emit cyberlink to A(x), commit at next tick |
| back | move cursor back in cache, reactivate world for previous uri | no new cyberlink (back is a cursor move, not a transition) |
| forward | move cursor forward in cache, reactivate world | no new cyberlink (cursor move) |

back/forward are local cursor moves on the cache and do not create new cyberlinks. only forward-progress navigation (new destinations) emits to the graph. this prevents back/forward thrashing from polluting history with noise.

if the cache misses (older than the cache window), the back operation fetches from the cybergraph. local cache is the fast path; cybergraph is the durable record.

### sessions as composite particles

a session is a contiguous window of navigation. starting cyb opens a session; closing it ends one. all cyberlinks emitted during the session reference the session particle by parent link. the session particle itself is a particle whose content is its metadata (start time, end time, neuron, device, optional title).

querying "show me my session from yesterday morning" returns the session particle and its child cyberlinks. replaying the session is traversing those cyberlinks in order. sharing the session is publishing the session particle with appropriate privacy on its cyberlinks.

sessions can also be branched. forking from an arbitrary point in a past session creates a new session particle that links to the parent session at the fork point. this is git for navigation history.

### implications

| capability | how it falls out |
|---|---|
| time travel | query cybergraph for cyberlinks where ν = me, t in range |
| cross-device continuity | open cyb on a phone after using desktop — recent cyberlinks gossip via radio, the cache reconstitutes |
| shared exploration | publish a slice of cyberlinks; a peer follows your trace through their own cyb |
| discovery | "where did other neurons go after this particle?" → query incoming cyberlinks on the destination, see who linked from where |
| collaborative paths | two neurons exploring the same topic produce parallel traces that can be merged or compared |
| audit | every action in cyb is provably attributable to a neuron at a time, signed and committed |
| karma over navigation | foculus computes attention distribution from real navigation, not synthetic clicks |
| agent provenance | when an agent navigates on behalf of a neuron, the cyberlink records both — neuron-as-actor, agent-as-tool |

the cybergraph being the history is not a feature added on top. it is what cyber is. cyb's navigation simply joins the graph as a first-class participant rather than maintaining a parallel disconnected log.

---

## address bar editing

the address bar is also an input. focus it (mouse click or hotkey `cmd+l`), edit the uri, press enter to navigate. while editing

- autocomplete from history and bookmarks
- particle hash recognition (paste `bafyrei...` → suggest `cyb://particle/bafyrei...`)
- world hint as you type (`cyb://gra...` → highlight `graph` from known worlds)
- syntax error feedback (red underline on malformed uris)

the same uri parsing logic serves the address bar input, the commander `go` command, and external deep links. one parser, many call sites.

---

## addresses as particles

uris are content-addressable too. an address like `cyb://graph/ai/safety` is a string; when stored as a [[particle]], it is just bytes with a hash. addresses can be linked to, hashed, gossiped, indexed in the cybergraph. a [[neuron]] can author a particle that is "a curated list of cyb uris on ai safety" — the particle is bytes containing several `cyb://` uris and prose, addressable by its own hash.

this means the cybergraph itself can guide navigation. an oracle search for "ai safety" returns particles, many of which contain `cyb://` uris. clicking those uris navigates to the destinations. cyb is internally hyperlinked at the cybergraph layer.

---

## external deep linking

os-level uri handlers register `cyb://` as a protocol scheme. clicking a `cyb://...` link in mail, in a browser, in a chat app — opens cyb (or focuses it if running) and dispatches the uri.

```
[external app] user clicks cyb://graph/ai/safety
  → os resolves cyb:// scheme → /Applications/cyb.app
  → if cyb is not running: launch it, pass uri as argument
  → if cyb is running: focus window, dispatch uri via Navigate event
  → bevy state transition happens identically to in-app navigation
```

the navigation contract is the same. external links are just another trigger funneling into the Navigate event.

---

## addressing the terminal stream

inside a sugarloaf world (terminal session, agent chat, app), individual outputs can also be addressable. ansi+osc 8 hyperlinks already standardize this — terminals can wrap text spans with hyperlink targets

```
\x1b]8;;cyb://particle/bafyrei...\x1b\\look at this image\x1b]8;;\x1b\\
```

the substring "look at this image" becomes clickable. clicking it fires a Navigate event with the embedded uri. this works in any modern terminal — third-party terminals that understand osc 8 also navigate when the target is a `cyb://` uri (assuming cyb is the registered handler).

agents that emit ansi can include hyperlinks naturally. claude's stream might say "this commit looks good — see `\x1b]8;;cyb://particle/bafy...\x1b\\diff\x1b]8;;\x1b\\`". the user clicks "diff" and navigates to a viewer for that particle.

---

## browser fallback

cyb runs in a browser tab too. when `cyb://` uris are not available as an os protocol, the equivalent is `https://cyb.ai/<world>/<path>` — same scheme structure, http transport. the browser address bar acts as the cyb address bar; the page renders the appropriate world. history is the browser's history.

in this fallback mode

- mir falls back to webgl
- sugarloaf falls back to a wasm cell renderer
- wry is the host browser itself (cyb is inside it)
- hotkeys are not available
- chrome is rendered by leptos in the browser, not bevy

this fallback is not the primary deploy. it exists for shareability (anyone with a browser can follow a `https://cyb.ai/...` link), for seo (particle pages render as proper html), and for low-friction discovery. the desktop and android targets are the primary.

---

## why this is the architecture

the previous routing model treated bevy and leptos as two subsystems sharing url ownership via pushState/popState plumbing. that worked but coupled the chrome to wry. it required the always-on transparent webview, which forced the compositing fight we eventually resolved by separating concerns.

this model separates them cleanly

- bevy owns the address bar (it is a cell-grid overlay rendered by bevy)
- bevy owns world dispatch (state transitions based on uri scheme world)
- each world owns its own routing (path semantics interpreted locally)
- history is one stack in bevy, shared across all worlds
- external links and in-app links go through the same event

no two layers race for who owns the url. no pushState/popState contract spans layers. no webview to keep alive for chrome to function. just events, parsing, and state transitions. simple, single-owner, single-direction.

---

see [[rendering]] for the surface architecture and rendering models, [[prysm]] for the address bar and commander composition, [[mir]] for graph-world path semantics, [[sugarloaf]] for cell-grid worlds and osc hyperlinks, [[radio]] for particle resolution behind addresses
