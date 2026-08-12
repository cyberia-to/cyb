---
tags: cyb, core, scripting, rune
alias: scripting, rune scripting, soul script, cybscript
crystal-type: pattern
crystal-domain: cyber
---

# scripting

how a [[neuron]] writes code that runs inside the [[robot]] — to tune its [[soul]], extend its behavior, render its own surfaces, and react to the [[cybergraph]]. the language is [[rune]].

> *formerly "cybscript", an embedding of a third-party language. that is retired. cyb's scripting language is [[rune]] — [[Rs]] syntax on [[Nox]], homegrown, provable, instant-start. there is one language now, not a host plus a guest.*

---

## the spine

there is one law, and everything else is convention on top of it. a program is a **gate** (a function). the robot hands it a **subject** (everything it can see). it returns **chunks** (what to render) and may **request acts** along the way.

```
   ~self ~world ~here          |= input            [text … button … ]
   ~caps ~mem  ~now    ──────►     body     ──────►   chunk-noun     ──►  prysm
   ────────────────             ─────────           ─────────────
   the subject                  a rune gate         what it emits
   (the robot, materialized)    (your program)      (rendered cells)
```

[[Nox]]'s one execution primitive — evaluate a formula against a subject — *is* the mechanism. so "an entrypoint" is not a separate system to register with. it is just a gate the shell calls with a subject it built. the kinds of entrypoint below differ only in **what triggers them** and **what `input` they receive**. same spine throughout.

---

## the language

[[rune]] gives you **instant start** (parse → run, no compile phase) and **two registers** over one AST: classic (familiar to any Rust/Go/TS programmer) and pure (sigil-form, alien and precise). both lower to the same [[Nox]] noun. pure code is provable unconditionally; reactive code relative to its event log; [[host|ward]] calls relative to their results.

```rust
fn double(x: @nebu) -> @nebu { x * 2 }      // classic register
```
```
|=  x=@nebu  (mul x 2)                       :: pure register
```

full language: [[rune]].

rune is cyb's **authoring** runtime — the dynamic, instant-start one a [[neuron]] reaches for first. it is not the only runtime: [[Nox]] is the substrate everything compiles to, [[Inf]] runs [[Datalog]] queries over the graph, [[glia]] runs model [[inference]], [[wysm]] runs sandboxed WASM. they share the subject, the act set, and the [[ward]] (see [[languages]] for the full landscape). this guide is about writing rune; the spine and the permission model below are common to all of them.

---

## the subject — what code sees

every name resolves through the subject by tree-slot lookup. the subject is the robot materialized for one evaluation: identity and context projected into eight slots.

| slot | axis | holds |
|------|------|-------|
| `~self`  | 2   | [[neuron]] / [[soul]] identity |
| `~now`   | 6   | current time |
| `~here`  | 14  | current world / surface / graph location |
| `~caps`  | 30  | the capabilities granted to this code — see [[ward]] |
| `~code`  | 62  | the running program core |
| `~libs`  | 126 | imported library cores |
| `~mem`   | 254 | persistent state |
| `~world` | 255 | the visible [[cybergraph]] slice |

identity (`~self`) and authority (`~caps`) are not two systems bolted together — they are slots of the one subject the shell constructs before it calls your gate.

---

## output — prysm chunks

a program that renders returns a **chunk-noun**: a tree built from [[prysm]] element constructors. these lower to `tape` chunks, which [[prysm]] renders identically to GPU cells, ansi, or html. the vocabulary:

| constructor | renders as |
|-------------|-----------|
| `text("…")`   | body text |
| `anno("…")`   | dim annotation |
| `error("…")`  | error widget |
| `log("…")`    | log line |
| `button(label, target)` | action button |
| `col(a, b, …)` | a column of elements |

emitting chunks is the `emit` act, gated by the [[ward]]. drawing is the only thing a render gate is allowed to do unless granted more.

---

## entrypoints — gates, by trigger

five conventions, one classification axis: **what fires the gate**. each fixes the `input`; the rest is the spine.

| trigger | entrypoint | input | fires when | typical caps | status |
|---------|-----------|-------|-----------|--------------|--------|
| imperative | **command** | the typed line / args | you run it | `{emit}` | **live** |
| surface | **cell** | input events for its region | a world/app is open | `{emit, query}` | planned |
| pipeline | **processor** | one particle `[cid type content]` | a particle enters view | `{emit, query}` | planned |
| address | **resolver** | a [[cybermark]] address | someone hits `@name` / `#path` / `.moon` | `{emit, query}` | planned |
| reactive | **companion** | an event stream (via `hint`) | graph / sensor events arrive | `{emit, query, subscribe, link}` | planned |

### command — live today

the imperative entrypoint. type it, it runs once, it returns chunks. this is the [[terminal]] world: prefix a line with `rune` and it evaluates through [[rune]] instead of [[nu|nushell]], emitting [[prysm]] chunks into the same stream nushell feeds.

```
> rune col(text("hello cyber"), error("boom"), button("ok", "@master"))
   hello cyber                 (body text)
   boom                        (error widget)
   [ ok → @master ]            (action button)

> rune add(2, 3)
   5                           (non-UI result shown as text)
```

mechanism: the typed line is the gate's `input`; the shell evaluates it against a [[subject|#the-subject-what-code-sees]] and routes `emit` to the [[terminal]]'s render stream. today the terminal grants full trust (caps ungated); the [[ward]] makes this enforced.

### the other four — planned

same spine, different trigger and `input`:

- **cell** `(events) -> chunks` — owns a surface; renders and handles input. the shape of a cyb app (oracle, settings, sense).
- **processor** `(particle) -> chunks | action` — runs as a particle enters view; transforms, filters, annotates. *(replaces the old `personal_processor`.)*
- **resolver** `(address) -> particle` — maps a [[cybermark]] address to content. native to [[rune]]'s sigil layer. *(replaces `moon_domain_resolver`.)*
- **companion** `hint -> chunks | signals` — a reactive gate; subscribes to events and acts. *(replaces `ask_companion`, generalized.)* **[[soma]]'s four loops are companions** — reactive rune gates, dynamically updatable, which is what makes the avatar's mind a living [[soul]] script rather than frozen Rust.

---

## acts and permission

a gate touches the world only through acts — `emit`, `query`, `subscribe`, `link`, `seal`, `host`. it does not perform them; it **requests** them, and the [[ward]] decides. the caps granted to the gate live in `~caps`; pure computation needs none. the [[ward]] is shared infrastructure: the same boundary governs every cyb runtime ([[rune]], [[wysm]], [[glia]], [[Nox]], [[Inf]]), not rune alone — rune is simply one requester. the full model — caps, grants, the permission prompt, attenuation, provable confinement — is [[ward]].

this is why running a stranger's `.moon` resolver is safe: it is mounted with `{emit}` and nothing more unless you grant it.

---

## status

| piece | state |
|-------|-------|
| rune language (parse → lower → [[Nox]] interpret) | working |
| rune↔[[prysm]] binding (chunk-noun → `tape` chunks) | working, tested |
| **command** entrypoint in the [[terminal]] (`rune <expr>`) | **live** (compiles; emit ungated) |
| cell / processor / resolver / companion | planned |
| [[ward]] (capability enforcement) | designed, not built |
| [[soma]] as companions | design; current soma is a non-working draft to rebuild on rune |

---

see [[rune]] for the language, [[ward]] for permissions, [[prysm]] for the render vocabulary, [[soul]] for identity and grants, [[terminal]] for the live command entrypoint, [[soma]] for the avatar's companion loops.
