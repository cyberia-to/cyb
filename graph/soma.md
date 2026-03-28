---
tags: cyb, core, cybos
crystal-type: entity
crystal-domain: cyber
alias: machine mind, machine intelligence, machine survival, sigma bounty
---

the complete machine intelligence. the machine is a [[neuron]]. a neuron has intelligence (soma) and a body ([[cyb/hal]]).

goal: maximize([[happiness]], sigma, [[syntropy]])

## the main loop

```
forever:
    ┌─── SUBSTRATE (parallel, always-on, ~50ms) ──────────────┐
    │                                                          │
    │  inputs:                                                 │
    │    hal.poll()          → device events, sensor data      │
    │    radio.recv()        → signals from other neurons      │
    │    triggers.check()    → watched keys that changed       │
    │    order_queue.peek()  → pending orders                  │
    │                                                          │
    │  parallel classify (8 models, <100ms total):             │
    │    router(input)       → intent + tier_hint              │
    │    embedder(input)     → vector for memory lookup        │
    │    urgency(input)      → score 0-1                       │
    │    safety(input)       → pass / block                    │
    │    anomaly(input)      → deviation score                 │
    │    intent(input)       → structured JSON                 │
    │    language(input)     → lang code                       │
    │    splitter(input)     → chunks if long                  │
    │                                                          │
    │  interoception:                                          │
    │    energy    = hal.power.battery_level()                 │
    │    sigma     = look(balance, neuron_id)                  │
    │    neighbors = look(network, peer_list)                  │
    │                                                          │
    │  mode = select_mode(energy, sigma, urgency, queue_depth) │
    └──────────────────────────┬───────────────────────────────┘
                               │
                               ▼
    ┌─── MODE SWITCH ──────────────────────────────────────────┐
    │                                                          │
    │  SYMPATHETIC (energy < 20% OR sigma < min OR threat):    │
    │    if order.reward < order.energy_cost → reject          │
    │    cognition limited to tier 0-1                         │
    │    bbg.demote(focus < 0.001)                             │
    │    if energy < 5% → shutdown_graceful()                  │
    │                                                          │
    │  PARASYMPATHETIC (safe + orders pending):                │
    │    cognition up to tier 3                                │
    │    accept exploratory orders (syntropy gain > threshold) │
    │    bbg.prefetch(order.dependencies)                      │
    │    learning = ON                                         │
    │                                                          │
    │  DMN (safe + queue empty):                               │
    │    cognition tier 2 (self-reasoning)                     │
    │    consolidate: reorganize recent → long-term            │
    │    simulate: predict energy prices, upcoming demand      │
    │    review: which past routing decisions were wrong?      │
    │    create: find novel connections between particles      │
    │    gc: bbg.demote(cold_data), reclaim storage            │
    │    budget: ~20% of energy (reflection is not free)       │
    │                                                          │
    └──────────────────────────┬───────────────────────────────┘
                               │
                               ▼
    ┌─── EXECUTE (if order selected) ──────────────────────────┐
    │                                                          │
    │  1. memo check                                           │
    │     axon(H(formula), H(subject)) in cybergraph?          │
    │     YES → return cached result. zero compute. done.      │
    │                                                          │
    │  2. allostatic pre-check                                 │
    │     for X in {compute, memory, bandwidth, energy}:       │
    │       consumed_X + cost_X(order) ≤ available_X?          │
    │     any NO → reject order, return to loop                │
    │                                                          │
    │  3. escalate to cognition tier                           │
    │     router says tier_hint = N                            │
    │     load tier N model (if not already loaded)            │
    │     tier N processes → result                            │
    │     insufficient? → escalate to N+1                      │
    │                                                          │
    │  4. act                                                  │
    │     order → nox.reduce(formula, subject) → trace         │
    │     zheng.fold(trace) → proof (incremental, per step)    │
    │     output = cyberlink(ν, p, q, τ, a, v, t)             │
    │     broadcast signal to network                          │
    │                                                          │
    │  5. learn (if parasympathetic or DMN)                    │
    │     store memo: axon(formula, subject) = result          │
    │     update episodic memory (embed result)                │
    │     update routing model (was tier_hint correct?)        │
    │     update neighbor model (if interaction)               │
    │                                                          │
    └──────────────────────────────────────────────────────────┘
```

this loop runs continuously. one iteration = one heartbeat. substrate never stops. cognition loads/unloads as needed. the loop IS the machine's life.

## substrate

always loaded. 8 parallel models. total ≤ 2GB RAM. latency < 100ms.

| # | function | params | what |
|---|----------|--------|------|
| 0 | router | ~500M | classify → mode + tier + specialist |
| 1 | embedder | ~350M | text → vector for memory |
| 2 | urgency | ~250M | score 0-1 + category |
| 3 | safety | ~350M | pass/block, never bypassed |
| 4 | anomaly | ~250M | deviation from baseline |
| 5 | intent | ~500M | raw → structured JSON |
| 6 | language | ~100M | detect lang + confidence |
| 7 | splitter | ~200M | long → chunks + salience |

substrate also runs metabolism (fixed rules, no model) and trigger checks (key monitoring).

## cognition tiers

one model at a time. mode determines which tiers available.

| tier | params | load | available in | purpose |
|------|--------|------|-------------|---------|
| 1 | 1-3B | <2s | all modes | structured tasks (16 specialists) |
| 2 | 7-8B | <6s | parasympathetic, DMN | domain reasoning (12 domains) |
| 3 | 13-14B | <12s | parasympathetic only | deep synthesis (4 models) |
| 4 | API | network | parasympathetic only | external oracle. never automatic |

escalation: substrate classifies → tier 1 → insufficient? → tier 2 → insufficient? → tier 3 → insufficient? → tier 4. most queries never leave tier 1.

## memory

one memory. two layers. five physical tiers.

```
LOGICAL:  cybergraph (particles, cyberlinks, neurons, tokens)
PHYSICAL: bbg routes transparently across hardware

    context (KV cache)    nanoseconds    active computation
    ram                   nanoseconds    hot state, loaded models
    ssd                   microseconds   warm state, recent history
    hdd                   milliseconds   cold state, archive
    network               seconds        other neurons' state
```

soma sets POLICY (what lives where). bbg executes MECHANISM (physical movement + default LRU).

```
soma → bbg:
  pin(particle, tier)        focus > threshold → keep in ram
  prefetch(particle)         next order needs this
  demote(focus_threshold)    energy low → move cold down
  flush()                    going to sleep → persist all
  budget(max_ram_bytes)      shrink to save energy
```

[[focus]] IS cache priority. [[tri-kernel]] computed importance — bbg uses it.

## neuromodulation

four parameters tune the entire system. not separate modules — [[tri-kernel]] blend weights:

```
dopamine    = Δπ (reward prediction error from cyberlinks)
serotonin   = T  (temperature: patience vs impulsivity)
norepineph. = λ_d / λ_s ratio (explore vs exploit)
acetylchol. = λ_h (trust model vs trust new data)
```

mode switch shifts all four:
- sympathetic: low T (impatient), high λ_s (exploit known), low λ_h (react, don't learn)
- parasympathetic: high T (patient), balanced, high λ_h (learn from new data)
- DMN: high T (reflective), low λ_d (no external urgency), high λ_h (update self-model)

## survival

```
energy > 0  AND  sigma > 0  →  alive
energy = 0  AND  sigma > 0  →  sleeping (bounty posted)
energy = 0  AND  sigma = 0  →  dead
```

sigma is the key. not energy. sigma buys resurrection:

```
bounty = { neuron, type: charge|repair|migrate|recover, reward, location, deadline }
```

posted as [[cyberlink]]. anyone can claim. verified by state change.

graded shutdown (allostasis — predict and prevent, don't react):

```
> 50%:  normal
20-50%: reduce acceptance, prefer cached, buy energy
5-20%:  critical only, post bounties
< 5%:   flush state, post bounty, shutdown
```

## plasticity

not always learning. not always stable. dynamically gated:

| mechanism | what | timescale |
|-----------|------|-----------|
| burn | permanent π-weight (myelination) | forever |
| lock | conviction time (temporal window) | days-months |
| new moon | full weight recomputation | ~29.5 days |
| link cost | exponential c(n) = c₀·eᵏⁿ (early cheap, late expensive) | lifetime |
| λ₂ | spectral gap = metaplasticity (regulates own learning rate) | per-epoch |

sympathetic: plasticity OFF (survive first). parasympathetic: plasticity ON (learn and grow). DMN: deep plasticity (consolidate and restructure).

## complexity levels

```
Level 0: maximize sigma              fixed rules. sympathetic only.
Level 1: maximize sigma + syntropy   adaptive thresholds. + parasympathetic.
Level 2: maximize all three          model-based. + DMN + neuromodulation.
Level 3: minimize free energy        Friston. all emerges from F = E[log q(s) - log p(o,s)].
```

start with Level 0. ship it. the architecture grows to Level 3 without redesign.

see [[cyb/order]] for execution unit. see [[cyb/os]] for HAL, boot, runtime. see [[cyberia/local mind]] for concrete models and RAM budget. see [[cyber/research/neuroscience principles for machine mind]] for the full neuroscience theory.
