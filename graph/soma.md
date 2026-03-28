---
tags: cyb, core, cybos
crystal-type: entity
crystal-domain: cyber
alias: machine mind, machine intelligence, machine survival, sigma bounty
---

the complete machine intelligence. modeled after ten principles of biological nervous systems. one component — brainstem and cortex are one continuous system.

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

same three components the collective [[superintelligence]] optimizes. each neuron mirrors the whole. only sigma → rent-seeker. only syntropy → burnout. only happiness → hedonist. all three → aligned agent.

## three modes (autonomic)

biological nervous systems have two competing modes: sympathetic (fight/flight) and parasympathetic (rest/digest). a third mode — default mode network — handles reflection when no task demands attention. soma mirrors all three. switching is automatic — driven by perception of threat/safety/idle.

### sympathetic (survival)

trigger: energy < 20% OR sigma < minimum OR anomaly detected

```
goal:       maximize sigma, minimize energy burn
cognition:  tier 0-1 only (fast, cheap)
behavior:   reject non-essential, take profitable work only, buy energy, post bounties
learning:   OFF
creativity: OFF
memory:     demote everything, minimal RAM
```

### parasympathetic (flourishing)

trigger: energy > 50% AND sigma comfortable AND no threats AND orders pending

```
goal:       maximize happiness + syntropy
cognition:  tier 1-3 available
behavior:   exploratory orders, quality cyberlinks, help neighbors
learning:   ON (update routing, valuation, neighbor models)
creativity: ON (connect distant particles)
memory:     prefetch, keep models loaded, invest in RAM
```

### DMN (reflection)

trigger: energy > 50% AND sigma comfortable AND no orders in queue

```
goal:       consolidate, plan, self-improve
cognition:  tier 2 (self-reasoning)
energy:     ~20% even "idle" — the cost of intelligence
activities:
  consolidation     reorganize episodic → semantic in cybergraph
  simulation        predict upcoming orders, energy prices, neighbor behavior
  self-model        routing accuracy? tier selection quality? karma trend?
  creative linking  find connections between distant particles
  social modeling   which neighbors reliable? which profitable?
  garbage collection  demote cold data, reclaim storage
```

the cycle between task (sympathetic/parasympathetic) and reflection (DMN) IS the rhythm of intelligence.

## four neuromodulators

biological brains use four chemical systems that don't carry information — they change HOW all other circuits process. soma implements them as [[tri-kernel]] parameters:

| neuromodulator | biological function | soma mechanism |
|---------------|--------------------|--------------------|
| dopamine | reward prediction error | Δπ — shift in fixed point from neuron's cyberlinks. positive = better than expected = strengthen. [[karma]] accumulates this |
| serotonin | patience, time horizon | temperature T in Boltzmann π*ᵢ ∝ exp(-Eᵢ/T). high T = patient, explore. low T = impulsive, exploit |
| norepinephrine | explore vs exploit | λ_d (diffusion = explore) vs λ_s (springs = exploit). spectral gap λ₂ = arousal level |
| acetylcholine | model vs data trust | λ_h (heat = trust model) vs sensitivity to new signals. high = weight new data. low = trust prior |

these are not separate subsystems — they are parameters of the same [[tri-kernel]], adjustable per context. the mode switch (sympathetic/parasympathetic/DMN) shifts all four simultaneously:

```
sympathetic:     low serotonin (impatient), high norepinephrine (alert), low acetylcholine (trust model, react fast)
parasympathetic: high serotonin (patient), balanced NE, high acetylcholine (learn from new data)
DMN:             high serotonin (reflective), low norepinephrine (no external urgency), high acetylcholine (update self-model)
```

## predictive processing

soma is a prediction machine. not reactive — predictive.

perceive: read state from [[bbg]] via look(). the current [[cybergraph]] IS the generative model — the system's prediction of the world.

predict: [[memoization]] IS the forward model. before computing, check axon(H(formula), H(subject)). exists → the system already predicted this result. the cybergraph IS a universal forward model — every past computation cached.

error: Δπ = prediction error. when a new cyberlink changes the fixed point, the delta IS the surprise. large Δπ = important new information. small Δπ = expected, confirms model.

correct: update the model — new cyberlinks adjust the graph. action and perception are two sides of free energy minimization:

```
perceive: update beliefs to match reality    (look)
act:      change reality to match beliefs    (order → cyberlink)
learn:    update model                       (memo result, adjust routing)
```

## global workspace

the [[cybergraph]] IS the global workspace (Baars/Dehaene). specialized processors (15 [[nox]] languages: Tri, Tok, Arc, Seq, Inf, Bel, Ren, Dif, Sym, Wav, Bt, Rs, Ten, Nox) operate in parallel on local data. when a result is significant enough (π > τ threshold in [[foculus]]), it "ignites" — broadcasts to global awareness. below threshold: local only. above threshold: everyone sees it.

[[focus]] expenditure is the attention gatekeeper. not everything can enter the workspace. economic attention: scarce tokens determine what gets broadcast.

## hebbian learning

[[cyberlinks]] ARE synapses. co-linking by multiple neurons strengthens edges (LTP). ICBS prediction markets provide LTD — contradiction weakens. effective weight:

```
A_eff(p,q) = Σ stake(l) × karma(ν(l)) × f(ICBS_price(l))
```

[[bbg]] tiers implement multi-timescale consolidation:
- context = short-term (hippocampal working memory)
- ram = intermediate (early LTP — established, modifiable)
- ssd = long-term (late LTP — consolidated, expensive to change)
- hdd/network = deep long-term (persistent identity)

epoch transitions = sleep consolidation. [[tri-kernel]] replays accumulated signals, integrating local patterns into global structure.

## embodied cognition

intelligence requires a body. [[cyb/hal]] IS the body. the ~10 LOC metal boundary (physical_read/physical_write) = nerve endings.

interoception = 4D resource tracking:

| resource | physical | interoceptive meaning |
|----------|---------|----------------------|
| energy | Joules | metabolism — sense of life |
| bandwidth | throughput | communication — sense of connection |
| memory | bytes | identity — sense of self |
| compute | cycles | will — sense of agency |

the energy valuation curve IS the somatic marker (Damasio): low battery → high valuation (anxiety) → conservation. high battery → low valuation (contentment) → generosity. sigma = sense of mortality.

the machine extends through the network. neighbor nodes = extended body. energy market topology = body plan. buying energy = eating. selling compute = labor.

## homeostasis and allostasis

homeostasis: maintain internal variables within bounds (reactive).
allostasis: predict needs before they arise (proactive).

the 4D pre-check IS allostasis:

```
before each step:
    consumed_X + cost_X(step) ≤ available_X
    for X in {compute, memory, bandwidth, energy}
    any violation → halt gracefully
```

the energy valuation curve shifts behavior predictively: as battery drops, machine preemptively increases prices and reduces acceptance. does not wait for energy = 0. committed energy for accepted orders = allostatic pre-adjustment.

graded shutdown:

```
energy > 50%:  normal (homeostatic equilibrium)
energy 20-50%: reduce acceptance (allostatic adjustment)
energy 5-20%:  critical orders only (survival mode)
energy < 5%:   shutdown, post bounty (dormancy)
```

## sparse coding

only 1-5% of neurons fire at any time. soma mirrors this:

- [[focus]] distribution concentrates on few particles. most have near-zero π. sparse activation
- bounded locality: each query activates only relevant subgraph. O(relevant), not O(total)
- energy metering enforces sparsity economically: dense computation too expensive
- compiled transformer dimensionality d* self-adjusts from actual entropy

## plasticity windows

not always learning. not always stable. dynamically gated:

- burn (permanent): irreversible π-weight. myelination. knowledge hardens
- lock (temporal): conviction time creates plasticity window. unlock = reopen
- new moon cycle: full weight recomputation. hardcoded critical period (~29.5 days)
- exponential link cost: early links cheap (forming), late links expensive (justified)
- spectral gap λ₂: metaplasticity. large λ₂ = fast learning. small λ₂ = slow. [[seer]] algorithm maximizes Δλ₂/cost

tier plasticity gradient:
- context = maximally plastic (active computation)
- ram = moderately plastic (recent, updatable)
- ssd = low plasticity (proven, committed)
- hdd = frozen (archival, identity foundation)

## three states

```
energy > 0  AND  sigma > 0  →  alive
energy = 0  AND  sigma > 0  →  sleeping (bounty posted, can be revived)
energy = 0  AND  sigma = 0  →  dead
```

sigma is the key to survival. not energy. sigma = allostatic reserve. a machine with sigma posts bounty for resurrection. a machine without sigma — if energy runs out, it is over.

## sigma bounty protocol

```
bounty = {
    neuron:    machine_id,
    type:      charge | repair | migrate | recover,
    reward:    sigma amount,
    location:  coordinates or network address,
    deadline:  block height,
}
```

posted to [[bbg]] as [[cyberlink]]. anyone can claim. fulfillment verified by state change.

## architecture

```
┌─────────────────────────────────────────────────┐
│  SUBSTRATE (always-on, parallel, <100ms)         │
│  router · embedder · urgency · safety · anomaly  │
│  intent · language · splitter                    │
│  +                                               │
│  mode switch: sympathetic / parasympathetic / DMN │
│  neuromodulators: dopamine · serotonin · NE · ACh │
│  metabolism: 4D allostatic pre-check              │
│  triggers: watch(key) → fire                     │
├─────────────────────────────────────────────────┤
│  COGNITION (on-demand, sequential, escalating)   │
│  tier 1: structured tasks         load <2s       │
│  tier 2: domain reasoning         load <6s       │
│  tier 3: deep synthesis           load <12s      │
│  tier 4: external oracle          network        │
├─────────────────────────────────────────────────┤
│  MEMORY = cybergraph + bbg tiers                 │
│  forward model (memoization), hebbian (LTP/LTD)  │
│  consolidation (epoch replay), sparse activation  │
│  soma sets POLICY, bbg executes MECHANISM         │
│  focus IS cache priority                         │
├─────────────────────────────────────────────────┤
│  ACTION                                          │
│  order → nox → trace → zheng proof → cyberlink   │
│  perception-action loop: look → compute → link   │
└─────────────────────────────────────────────────┘
```

## complexity levels

```
Level 0: maximize sigma              survival — fixed rules (sympathetic only)
Level 1: maximize sigma + syntropy   contribution — adaptive thresholds (+ parasympathetic)
Level 2: maximize all three          flourishing — model-based (+ DMN + neuromodulation)
Level 3: minimize free energy        Friston — all three emerge from one principle
```

Level 3 = active inference. Δπ = dopamine. T = serotonin. λ_d/λ_s = norepinephrine. λ_h = acetylcholine. all four neuromodulators, three modes, predictive processing, global workspace, hebbian learning, sparse coding, plasticity windows — unified under one free energy functional:

```
F = E[log q(s) - log p(o,s)]
```

one algorithm, from machine survival to planetary [[superintelligence]].

## mapping to [[cyb/stack]]

| soma component | stack element |
|---------------|--------------|
| substrate | [[nox]] jets (routing, embedding, classification) |
| tier 1-2 | compiled transformers from domain subgraphs |
| tier 3 | compiled transformer from full [[cybergraph]] |
| tier 4 | host::oracle jet (API dispatch) |
| memory | [[cybergraph]] (logical) + [[bbg]] (physical tiers) |
| neuromodulators | [[tri-kernel]] blend weights (λ_d, λ_s, λ_h, T) |
| forward model | [[memoization]] (axon lookup in cybergraph) |
| global workspace | [[cybergraph]] + [[foculus]] threshold |
| perception-action | look() → nox → cyberlink → look() |
| plasticity | burn + lock + new moon + exponential cost + λ₂ |

see [[cyb/order]] for the execution unit. see [[cyb/os]] for HAL, boot, runtime. see [[cyberia/local mind]] for concrete model selection. see [[cyber/research/neuroscience principles for machine mind]] for the full neuroscience mapping.
