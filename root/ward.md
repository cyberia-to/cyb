---
tags: cyb, core, security, permissions
alias: ward, the ward, permissions, capabilities, caps, cyb permissions, cyb-ward
crystal-type: pattern
crystal-domain: cyber
---

# ward

the permission system of the robot. the authority boundary between code that *wants* to act and the [[robot]] that *decides* whether it may.

[[cyb]] runs many runtimes — [[Nox]] and the languages that compile to it (including [[Inf]] queries and [[Trident]]), [[rune]], [[glia]] (model inference), [[wysm]] (WASM). they execute different things, but they share one rule for touching the world: every **act** — drawing to a surface, reading the graph, writing a [[cyberlink]], invoking another runtime, calling out — passes through the ward. the ward holds the [[neuron]]'s policy, checks each request against the **caps** granted to that code, and performs it or refuses. it is the one place permission lives, and the one place it is enforced, **for every runtime**.

---

## the principle

> **a runtime evaluates. it cannot touch the world directly — it can only *ask*. when it needs an *act*, it requests one through the ward, carrying the *caps* it was granted. cyb — through the ward — *defines* the acts, *checks* permission, *performs* them, and *prompts* the [[neuron]]. no runtime acts on its own.**

this split is the whole design, and it holds for *every* runtime, not one. a runtime is a way to evaluate — it should not know what "write to the graph" means, nor what your policy allows. it evaluates, and when it needs the world it *asks*. cyb owns the verbs, the policy, and the answer. the ward is where cyb's half lives.

*(an **act** is an `effect` in programming-language terms, and the ward is an effect handler — but it is named for what it is to the [[neuron]]: a consequential thing done in the world that needs permission, not "non-pure computation".)*

the payoff: each runtime stays a clean, reusable substrate, and cyb owns security end to end — one place to add acts, change policy, revoke grants, and evolve the prompt UX, governing [[rune]], [[wysm]], [[glia]], and [[Nox]] alike.

> the ward is to *acts* what [[prysm]] is to *rendering*: a single runtime-blind boundary that many runtimes feed. prysm renders any runtime's chunks; the ward authorizes any runtime's acts.

---

## the runtimes

every runtime cyb mounts is handed an **authority context** (the caps it was granted) and routes its world-touching through the ward. what differs is only *how each carries its caps* — the ward interface is identical.

| runtime | executes | carries caps as | typical acts |
|---------|----------|-----------------|--------------|
| [[Nox]] | the structural tree-reduction substrate (all langs compile here) | `~caps` subject slot (axis 30) | `look` / `call` → any act |
| [[rune]] | [[Rs]]-on-Nox, dynamic, the authoring layer | `~caps` subject slot | `emit`, `query`, `link`, `host` |
| [[Inf]] | [[Datalog]] queries over the graph ([[inf]] engine) | the readable graph scope | `query`, `subscribe` |
| [[glia]] | model / LLM inference — the avatar's cognition | the session's tool allowlist | `emit`, `query`, `link`, `host` (tools) |
| [[wysm]] | sandboxed WASM modules | the provided host-import set | whatever imports the ward supplies |

[[wysm]] and [[glia]] deserve note. WASM is *already* capability-shaped: a module has no ambient authority and can only call the host functions it was given. the ward is precisely the supplier of those gated imports — capability security falls out of the import boundary. and [[glia]] is where it matters most: an inference's acts are the tools and actions a model may invoke, so the ward bounds what the avatar's mind is permitted to *do* — capability-gated cognition, the backbone of [[AI alignment|alignment]] in cyb.

invoking one runtime from another (rune calling a WASM module, [[soma]] running an inference) is itself a warded act — see `host` / `run` below. the ward mediates both world-acts and cross-runtime calls.

---

## why a permission system at all

most runtimes here are **pure by default**: their core evaluation (arithmetic, structure, reduction) can affect nothing — there is no way to reach outside the evaluation. the *only* ways to touch the world are a small, fixed set of **acts**:

| act | what it does | backed by |
|-----|--------------|-----------|
| `emit`      | render [[prysm]] chunks to a surface     | the runtime→[[prysm]] binding |
| `query`     | read the [[cybergraph]] (one-shot)        | [[Inf]] / scry |
| `subscribe` | stream graph events                       | graph subscription |
| `link`      | **write** a cyberlink to the graph        | [[cybergraph]] `link` |
| `seal`      | finalize an intent with a proof           | [[cybergraph]] `seal` |
| `host` / `run` | invoke another runtime — WASM, GPU, inference, nox formula | [[wysm]] / [[glia]] / [[Nox]] |

because pure evaluation cannot reach the world, gating *just these acts* gates everything, in any runtime. there is no ambient authority to leak. that is the backbone of the model, not a policy choice.

---

## caps

a **cap** (capability) is the unforgeable right to perform one act, optionally narrowed by a constraint. it is just a value:

```
[emit  surface-id]     may draw, only to this surface
[link  namespace]      may write, only under this namespace
[run   module-hash]    may invoke, only this runtime/module
[query scope]          may read, only this graph slice
```

how a runtime *holds* its caps varies — [[Nox]] and [[rune]] keep them in the **`~caps`** subject slot (axis 30); [[wysm]] holds them as its import set; [[glia]] as a session tool allowlist; [[Inf]] as a readable scope. but in all cases: the runtime can *read* its authority, it cannot *fabricate* a cap it was not given, and caps **only attenuate** — a runtime can hand a *subset* to work it delegates (a sub-gate, a sub-module, a sub-agent), never a superset. you can narrow authority but never widen it. this is the object-capability guarantee, uniform across runtimes.

---

## the ward — one chokepoint

a runtime does not perform acts; it **yields** them — packaging a request and handing control to the host. the ward is the host side, and it is runtime-blind:

```
runtime (any)                             ward (cyb)
  needs to touch the world
  reads its granted caps
  yield { act, args, caps } ──────────►   ward.perform(act, args, caps)
                                             ├ cap for `act` present in caps?
                                             ├ constraint satisfied?
                                             ├ yes → do it  +  record
                                             └ no  → deny   (→ maybe prompt → grant → retry)
  resume(k, result) ◄──────────────────    result  |  denied
```

`ward.perform(act, args, caps) -> Result<Value, Denied>` is the single enforcement point for the whole robot. every world-touching act from every runtime routes through it, and returns its **result** or a **denied**. pure evaluation never reaches it. this is what makes the trust surface auditable: it is exactly *{acts} × {granted caps}* and nothing else, regardless of which runtime asked.

---

## grants — where caps come from

the **[[soul]]** grants caps. when cyb mounts an [[entrypoint|scripting]] — a [[rune]] gate, a [[wysm]] module, a [[glia]] session — the ward fills its authority context from the soul's **policy**: cyberlinks declaring "code of kind X from source Y gets caps Z". the default is **deny**:

- **your own code** (terminal, your cells, [[soma]]'s loops) — broad caps, but still expressed *as* caps, so a compromised loop or model is bounded and every act is recorded for audit.
- **foreign code** (a stranger's resolver, an imported module, a remote model) — `{emit}` only by default. it can draw; it cannot read your graph, write a link, or invoke another runtime.

anything beyond a piece of code's default triggers a **prompt** — like mobile app permissions:

> `master.moon` wants to **write to your graph** (`link @master/*`). allow · deny · always

a grant is itself a cyberlink on your soul: auditable, revocable, and it syncs across your [[bodies|body]]. *(the prompt buttons are a [[prysm]] surface — specified later; the policy and enforcement below do not depend on them.)*

---

## static guarantees

because acts appear **only** at the runtime's act boundary, the ward can inventory what a piece of code *could* do before running it — scan a lowered [[Nox]] formula for act tags, or a WASM module for its imports, and you know its full reach. code may also declare a **manifest** of the caps it needs; the ward diffs requested against granted and prompts for the gap. this is a real bound, not a promise: the code cannot reach an act that is not in its formula / import list.

---

## provable confinement

for runtimes that produce a [[Nox]] trace ([[Nox]], [[rune]], [[Inf]], [[Trident]]), the granted caps are *in* the trace and the ward's check is deterministic over it — so "this computation ran using only the caps it was granted" is a **[[zheng]] proof**, not a trust assumption. capability-safe *and* verifiable. for opaque runtimes ([[wysm]], [[glia]]) confinement is enforced at the import / tool boundary rather than proven in-trace — the genuine trust frontier, and exactly where the ward concentrates its prompts and audit.

---

## the boundary — runtime vs cyb

| concern | lives in | why |
|---------|----------|-----|
| evaluate, yield acts, carry & attenuate caps | **each runtime** ([[Nox]]/[[rune]]/[[wysm]]/[[glia]]/[[Inf]]) | generic ocap plumbing — no policy, reusable |
| act vocabulary + implementations | **ward** (cyb) | cyb owns the verbs (`emit`→[[prysm]], `link`→[[cybergraph]], …) |
| capability type, encoding, policy, grants | **ward** (cyb) | the [[soul]]'s authority, not the runtime's |
| enforcement (`perform`), audit, prompt, manifest, scan | **ward** (cyb) | one chokepoint, one owner, all runtimes |

a runtime's *entire* contribution to security is: carry an opaque token, include it when asking, allow narrowing. for [[rune]]/[[Nox]] the only change the ward requires is generalizing the act yield to carry `(act, args, caps)`; for [[wysm]] it is providing gated imports; for [[glia]] a tool allowlist. everything else is cyb.

---

## status

- **principle + design** — settled (this document), runtime-agnostic.
- **`emit` act** — the [[rune]]↔[[prysm]] binding ships and renders in the [[terminal|scripting]] world; today it runs **ungated** (your terminal = full trust).
- **the ward** — not yet built. staging:
  1. introduce `ward.perform` in cyb with a single `emit` act; route the terminal's [[rune]] chunks through it (grant-all). establishes the seam.
  2. runtimes yield acts with caps; the ward checks `emit`; seed caps from a stub policy.
  3. add `query` / `link` / `seal` / `host`; soul-derived policy; the prompt surface. bring [[wysm]] (gated imports) and [[glia]] (tool allowlists) under the same `perform`.
  4. manifest + static scan + [[zheng]] proof hook.

see [[scripting]] for how [[rune]] code is mounted and what the subject holds, [[languages]] for the full runtime/language landscape, [[rune]] / [[wysm]] / [[glia]] / [[Inf]] for the individual runtimes, [[prysm]] for the `emit` target, [[soul]] for the grant authority, [[zheng]] for the confinement proof.
