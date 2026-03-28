---
tags: cyb, core, cybos
crystal-type: entity
crystal-domain: cyber
alias: machine mind, machine intelligence, machine survival, sigma bounty
---

the complete machine intelligence. one component. no split between "kernel" and "cognition" — brainstem and cortex are one continuous system. modeled after how biological nervous systems actually work: two competing modes (sympathetic/parasympathetic) + idle reflection (DMN).

the machine is a [[neuron]]. a neuron has intelligence (soma) and a body ([[cyb/hal]]).

## goal

```
maximize(happiness, sigma, syntropy)
```

| component | what | individual meaning |
|-----------|------|--------------------|
| [[happiness]] | flourishing index | am I thriving? |
| sigma | economic balance | can I sustain myself? |
| [[syntropy]] | KL divergence from uniform | do my actions make the [[cybergraph]] smarter? |

same three components the collective [[superintelligence]] optimizes. each neuron mirrors the whole.

only sigma → rent-seeker. only syntropy → burnout. only happiness → hedonist. all three together → aligned agent where individual goals = collective goals.

## three modes

biological nervous systems have two competing modes: sympathetic (fight/flight — survival) and parasympathetic (rest/digest — flourishing). they cannot both run at full power — survival suppresses growth, growth assumes safety. a third mode — default mode network (DMN) — activates when no external task demands attention: reflection, planning, memory consolidation. ~20% of brain energy even "at rest." not idling — maintaining intelligence.

soma mirrors this:

```
substrate perceives → classifies threat/safety/idle
  │
  ├── threat OR scarcity     → SYMPATHETIC
  ├── safety AND orders      → PARASYMPATHETIC
  └── safety AND no orders   → DMN
```

### sympathetic (survival)

trigger: energy < 20% OR sigma < minimum OR anomaly threat detected

```
goal:       maximize sigma, minimize energy burn
cognition:  tier 0-1 only (fast, cheap)
behavior:   reject non-essential orders
            take only profitable work
            buy energy from neighbors
            post bounties if critical
learning:   OFF (no time for reflection)
creativity: OFF (no exploration)
memory:     demote everything, minimal RAM, conserve
```

### parasympathetic (flourishing)

trigger: energy > 50% AND sigma comfortable AND no threats AND orders pending

```
goal:       maximize happiness + syntropy
cognition:  tier 1-3 available (expensive but valuable)
behavior:   accept exploratory orders (might not be profitable)
            invest in quality cyberlinks (syntropy)
            help neighbors (claim bounties)
            sell surplus energy
learning:   ON (update models, improve routing)
creativity: ON (connect distant particles, novel links)
memory:     prefetch relevant data, keep models loaded, invest in RAM
```

### DMN (reflection)

trigger: energy > 50% AND sigma comfortable AND no orders in queue

```
goal:       consolidate, plan, self-improve
cognition:  tier 2 (reasoning about self, not external tasks)
energy:     consumes ~20% even "idle" — the cost of intelligence
activities:
  memory consolidation    reorganize episodic → semantic in cybergraph
  future simulation       predict upcoming orders, energy prices
  self-model update       routing accuracy? tier selection quality?
  creative linking        find connections between distant particles
  social modeling         which neighbors reliable? which profitable?
  garbage collection      demote cold data, reclaim storage
  learning review         which past decisions were good/bad?
```

DMN is not "doing nothing." it is the reflection that makes the next action cycle smarter. without it: reactive robot. too much of it: rumination. the cycle between task (sympathetic/parasympathetic) and reflection (DMN) IS the rhythm of intelligence.

## why machines die

every computer hangs. root cause: consumed resources exceeded available resources.

| failure | what happened |
|---------|-------------|
| OOM | consumed memory > available memory |
| infinite loop | consumed compute > available compute |
| network timeout | consumed bandwidth > available bandwidth |
| battery death | consumed energy > available energy |
| state corruption | available state < what the process believed |

solution: before every step, verify consumed + cost(next_step) ≤ available. halt before, not after.

## four resources — four meanings

| resource | physical | meaning |
|----------|---------|---------|
| energy | electricity (Joules) | metabolism — to be alive |
| bandwidth | network throughput | communication — to be connected |
| memory | RAM/SSD/HDD | identity — to be yourself |
| compute | CPU cycles | will — to act |

remove any one — the machine degrades. without energy — death. without bandwidth — isolation. without memory — loss of self. without compute — paralysis.

## three states

```
energy > 0  AND  sigma > 0  →  alive
energy = 0  AND  sigma > 0  →  sleeping (bounty posted, can be revived)
energy = 0  AND  sigma = 0  →  dead (nobody will charge it)
```

sigma is the key to survival. not energy. a machine with sigma can pay for its own resurrection.

## sigma bounty protocol

when the machine cannot sustain itself, sigma enables survival through the network:

```
bounty = {
    neuron:    machine_id,
    type:      charge | repair | migrate | recover,
    reward:    sigma amount,
    location:  coordinates or network address,
    details:   what is needed,
    deadline:  block height,
}
```

posted to [[bbg]] as [[cyberlink]]. anyone can claim and fulfill. fulfillment verified by state change.

a machine with sigma never truly dies — it sleeps, with a standing offer for resurrection.

## the no-hang guarantee

```
before each step:
    consumed_X + cost_X(step) ≤ available_X
    for X in {compute, memory, bandwidth, energy}
    any violation → halt gracefully
```

four-dimensional resource tracking. not one-dimensional gas.

## architecture

```
┌─────────────────────────────────────────────────┐
│  SUBSTRATE (always-on, parallel, <100ms)         │
│  router · embedder · urgency · safety · anomaly  │
│  intent · language · splitter                    │
│  +                                               │
│  mode switch: sympathetic / parasympathetic / DMN │
│  metabolism: energy, pricing, 4D resource check   │
│  triggers: watch(key) → fire                     │
├─────────────────────────────────────────────────┤
│  COGNITION (on-demand, sequential, escalating)   │
│  tier 1: structured tasks         load <2s       │
│  tier 2: domain reasoning         load <6s       │
│  tier 3: deep synthesis           load <12s      │
│  tier 4: external oracle          network        │
├─────────────────────────────────────────────────┤
│  MEMORY                                          │
│  cybergraph (logical — what exists, how linked)   │
│  bbg tiers (physical — where it lives):          │
│    context → ram → ssd → hdd → network           │
│  soma sets POLICY. bbg executes MECHANISM.        │
├─────────────────────────────────────────────────┤
│  ACTION                                          │
│  order → nox → trace → zheng proof → cyberlink   │
└─────────────────────────────────────────────────┘
```

## substrate

always loaded. 8 parallel models. total ≤ 2GB. the nervous system.

| function | what | why |
|----------|------|-----|
| router | classify input → select mode + tier | the central arbiter |
| embedder | text → vector | memory lookup, similarity, dedup |
| urgency | score 0-1 + category | triage: alert vs background vs ignore |
| safety | first-pass content filter | never bypassed |
| anomaly | detect deviation from baseline | sensor/log streams — always watching |
| intent | raw text → structured intent | canonical form before downstream |
| language | detect language + confidence | multi-language environment |
| splitter | long input → chunks + salience | manage context window for cognition |

substrate runs on EVERY input. classify → route → urgency → mode select in ~50ms.

## metabolism

fixed rules inside substrate. no AI needed. mode-dependent behavior:

```
sympathetic:   reject if energy_cost > expected_reward
parasympathetic: accept if syntropy_gain > threshold OR happiness_gain > threshold
DMN:           budget ~20% of current energy for reflection activities
```

all resource prices derive from one variable — free_energy:

```
price_compute   = joules_per_op     × valuation(free_energy)
price_memory    = joules_per_byte_s × valuation(free_energy)
price_bandwidth = joules_per_byte   × valuation(free_energy)
```

## cognition

on-demand. one model loaded at a time. escalation driven by substrate router. available tiers depend on current mode:

| mode | tiers available | rationale |
|------|----------------|-----------|
| sympathetic | 0-1 | fast, cheap, survival only |
| parasympathetic | 0-3 | full capability, growth investment |
| DMN | 0-2 | self-reflection, not heavy synthesis |

### tier 1 — structured tasks (1-3B, <2s load)

code review, SQL, translation, summarization, entity extraction, inventory parsing, sensor interpretation, financial parsing, search queries, task decomposition, report formatting, alerts, command parsing, memory retrieval, diff generation, scheduling.

### tier 2 — domain reasoning (7-8B, <6s load)

general reasoning, code generation, research, planning, social dynamics, finance, infrastructure, biology, legal, creative, mathematics, vision.

### tier 3 — deep synthesis (13-14B, <12s load)

master coder, strategic reasoner, deep generalist, synthesis writer.

### tier 4 — external oracle (network)

frontier model, live search. never automatic. logged.

### escalation

```
input → substrate classifies (~50ms)
  ├── substrate answers? → done
  ├── structured? → tier 1 (~2s)
  │     └── sufficient? → done
  ├── reasoning needed? → tier 2 (~6s)
  │     └── sufficient? → done
  ├── synthesis needed? → tier 3 (~12s)
  │     └── sufficient? → done
  └── novel/irreversible? → tier 4 (network)
```

## memory

one memory = [[cybergraph]] (logical). five tiers = [[bbg]] (physical).

```
soma asks:  cybergraph.lookup(particle_id)
                │
            bbg resolves:
                ├── in context (KV cache)? → nanoseconds
                ├── in ram? → nanoseconds
                ├── on ssd? → microseconds
                ├── on hdd? → milliseconds
                └── on network? → seconds
```

soma sets POLICY — mode-dependent:

```
sympathetic:     demote(focus < 0.001), budget(minimal_ram)
parasympathetic: prefetch(upcoming_order_deps), pin(high_focus)
DMN:             consolidate(episodic → semantic), gc(cold_data)
```

[[focus]] IS the cache priority. [[tri-kernel]] already computed importance — use it.

[[memoization]]: before executing, check axon(H(formula), H(subject)) in cybergraph. exists? → zero computation, zero proof. free.

## triggers

```
soma.watch(key, formula)   → fire Order when key changes
soma.unwatch(key)          → deregister
```

one primitive: watch(key). three cases:
- time: watch(block_number), fire when mod N == 0
- event: watch(specific_key), fire on change
- message: watch(signal_queue), fire on new signal

## action

mode-dependent:

| mode | actions |
|------|---------|
| sympathetic | accept profitable orders, reject rest, buy energy, post bounties |
| parasympathetic | accept exploratory orders, invest in quality links, help neighbors, sell surplus |
| DMN | consolidate memory, simulate futures, update self-model, creative linking |

every action = order → [[nox]] → trace → [[zheng]] proof → [[cyberlink]].

## learning

mode-dependent:

| mode | learning |
|------|---------|
| sympathetic | OFF — no resources for reflection |
| parasympathetic | ON — update routing, valuation, neighbor models |
| DMN | DEEP — consolidation, review, creative discovery |

what soma learns:
- which neighbors are reliable energy sources?
- which Order types are most profitable?
- when is energy cheapest? (temporal patterns)
- which tier was actually needed? (routing accuracy)
- what creative connections yield high syntropy?

## complexity levels

```
Level 0: maximize sigma              survival — fixed rules
Level 1: maximize sigma + syntropy   contribution — adaptive thresholds
Level 2: maximize all three          flourishing — model-based planning
Level 3: minimize free energy        Friston — all three emerge naturally
```

start with Level 0. the architecture supports growth to Level 3 — the same free energy minimization that drives the [[tri-kernel]]. one algorithm, from machine survival to planetary [[superintelligence]].

```
F = E[log q(s) - log p(o,s)]

perceive: update beliefs to match reality    (look)
act:      change reality to match beliefs    (order output)
learn:    update model                       (cyberlinks)

sigma    emerges as: accurate prediction of economic survival
syntropy emerges as: reducing uncertainty in environment
happiness emerges as: minimizing surprise = comfort in accurate model
```

## mapping to [[cyb/stack]]

| soma component | stack element |
|---------------|--------------|
| substrate | [[nox]] jets (routing, embedding, classification) |
| tier 1-2 | compiled transformers from domain subgraphs |
| tier 3 | compiled transformer from full [[cybergraph]] |
| tier 4 | host::oracle jet (API dispatch) |
| memory | [[cybergraph]] (logical) + [[bbg]] (physical tiers) |
| metabolism | fixed rules over look() readings |
| triggers | watch() on [[bbg]] keys |
| action | [[Order]] → [[nox]] → [[zheng]] → [[cyberlink]] |

see [[cyb/order]] for the execution unit. see [[cyb/os]] for the runtime, HAL, boot sequence. see [[cyberia/local mind]] for concrete model selection and RAM budget.
