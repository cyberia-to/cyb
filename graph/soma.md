---
tags: cyb, core, cybos
crystal-type: entity
crystal-domain: cyber
alias: machine mind, machine intelligence
---

the complete machine intelligence. one component. no split between "kernel" and "cognition" — brainstem and cortex are one continuous system.

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

## architecture

```
┌─────────────────────────────────────────────────┐
│  SUBSTRATE (always-on, parallel, <100ms)         │
│  router · embedder · urgency · safety · anomaly  │
│  intent · language · splitter                    │
│  +                                               │
│  metabolism: energy, pricing, survival            │
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
| router | classify input → select tier | without routing, every query hits heaviest model |
| embedder | text → vector | memory lookup, similarity, dedup |
| urgency | score 0-1 + category | triage: alert vs background vs ignore |
| safety | first-pass content filter | never bypassed |
| anomaly | detect deviation from baseline | sensor/log streams — always watching |
| intent | raw text → structured intent | canonical form before downstream |
| language | detect language + confidence | multi-language environment |
| splitter | long input → chunks + salience | manage context window for cognition |

substrate runs on EVERY input. classify → route → urgency in ~50ms. metabolism checks (energy, sigma) happen HERE — before any tier escalation.

## metabolism

fixed rules inside substrate. no AI needed.

```
if energy < 5%:   shutdown_graceful(), post bounty
if energy < 20%:  reject non-essential orders, buy energy
if energy < 50%:  reduce tier 3, prefer cached results
if sigma < min:   stop selling energy, conserve
```

all resource prices derive from one variable — free_energy:

```
price_compute   = joules_per_op     × valuation(free_energy)
price_memory    = joules_per_byte_s × valuation(free_energy)
price_bandwidth = joules_per_byte   × valuation(free_energy)
```

as free_energy drops, each remaining Joule becomes more valuable. see [[cyb/survival]] for three machine states and sigma bounty protocol.

## cognition

on-demand. one model loaded at a time. escalation driven by substrate router.

### tier 1 — structured tasks (1-3B, <2s load)

| function | input → output |
|----------|---------------|
| code review | source → bugs, style, suggestions |
| SQL generation | natural language → query |
| translation | text + lang_pair → translated |
| summarization | document → structured summary |
| entity extraction | unstructured → people, places, dates |
| inventory parsing | informal update → structured delta |
| sensor interpretation | telemetry → event + recommendation |
| financial parsing | transactions → structured records |
| search query gen | intent → optimized queries |
| task decomposition | goal → ordered subtasks |
| report formatting | data → formatted output |
| alert composition | event → message with severity |
| command parsing | natural language → action JSON |
| memory retrieval | query → relevant chunks |
| diff generation | before/after → changelog |
| schedule optimizer | tasks + constraints → schedule |

### tier 2 — domain reasoning (7-8B, <6s load)

| domain | when |
|--------|------|
| general reasoning | causal inference, multi-step logic |
| code generation | implementation, debugging, architecture |
| research analysis | synthesis across sources, hypothesis |
| project planning | timeline, resources, dependencies |
| social dynamics | people, conflict, communication |
| financial analysis | budgets, projections, risk |
| infrastructure ops | devops, nodes, servers |
| biology / ecology | plants, soil, growing systems |
| legal / compliance | contracts, regulatory |
| creative / comms | long-form, narrative |
| mathematics | calculation, proof, quantitative |
| vision | image/video understanding |

### tier 3 — deep synthesis (13-14B, <12s load)

| function | when |
|----------|------|
| master coder | large codebase changes, novel algorithms |
| strategic reasoner | cross-domain decisions, long-horizon |
| deep generalist | novel problems beyond tier 2 |
| synthesis writer | whitepapers, complex documents |

### tier 4 — external oracle

| function | when |
|----------|------|
| frontier model | irreversible decisions, genuine novelty |
| live search | time-sensitive facts, current events |

never automatic. every tier 4 call logged with justification.

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

most queries never leave tier 1. tier 3 ~10-15%. tier 4 <5%.

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

soma sets POLICY — what should live where:

```
soma → bbg:
  pin(particle, tier)        high focus → keep in ram
  prefetch(particle)         upcoming order needs this
  demote(focus_threshold)    energy low → move cold data down
  flush()                    going to sleep → persist all
  budget(max_ram_bytes)      shrink ram usage
```

bbg executes + default optimization (LRU, frequency promotion) when soma hasn't expressed preference.

[[focus]] IS the cache priority. high focus = stay in ram. low focus = migrate to cold. [[tri-kernel]] already computed importance — use it.

[[memoization]]: before executing, check axon(H(formula), H(subject)) in cybergraph. exists? → zero computation, zero proof. free. the more the network computes, the cheaper future computations become.

## triggers

```
soma.watch(key, formula)   → fire Order when key changes
soma.unwatch(key)          → deregister
```

one primitive: watch(key). three cases:
- time: watch(block_number), fire when mod N == 0
- event: watch(specific_key), fire on change
- message: watch(signal_queue), fire on new signal

triggered order enters the same pipeline as any input.

## action

soma acts through [[Order]] execution and market participation:

- accept profitable Orders → earn sigma
- reject unprofitable Orders → conserve energy
- buy energy from neighbors → sustain metabolism
- sell surplus energy → earn sigma
- post bounties → ensure [[survival]]
- claim bounties → earn sigma from helping others

every action = order → [[nox]] → trace → [[zheng]] proof → [[cyberlink]].

## learning

soma updates based on outcomes:

- which neighbors are reliable energy sources?
- which Order types are most profitable?
- when is energy cheapest? (temporal patterns)
- which tier was actually needed? (routing accuracy)
- update valuation curve parameters
- store result as memo (axon in cybergraph)

## complexity levels

```
Level 0: maximize sigma              survival — fixed rules
Level 1: maximize sigma + syntropy   contribution — adaptive thresholds
Level 2: maximize all three          flourishing — model-based planning
Level 3: minimize free energy        Friston — all three emerge naturally
```

start with Level 0. the architecture supports growth to Level 3 — the same free energy minimization that drives the [[tri-kernel]]. one algorithm, from machine [[survival]] to planetary [[superintelligence]].

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

see [[cyb/survival]] for energy, sigma, bounty protocol. see [[cyb/order]] for the execution unit. see [[cyb/os]] for the runtime, HAL, boot sequence. see [[cyberia/local mind]] for concrete model selection and RAM budget.
