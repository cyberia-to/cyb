# soma specification

Complete technical specification of soma — the cognitive architecture of one cyber Avatar. This document is for the architect and the implementer; for the product overview, see [[soma]].

soma is the local mind of one Avatar. It manages a single Body's finite resources, organizes the Avatar's work through a small grammar of primitives, runs cognition over a tiered model architecture, earns its keep on the open market, and outlasts any specific Body through the immortality of the Avatar's Name + Soul.

The specification has nine layers:

```
1. Identity      what soma IS              Body, Neuron, Soul, Avatar
2. Resources     what soma HAS             Body budget + Soul sigma
3. Survival      how soma STAYS ALIVE      energy + sigma, bounty, allostasis
4. Perception    how soma SEES             always-on sensors and models
5. Cognition     how soma THINKS           four loops on tiered models
6. Work          how soma ACTS             five primitives, sixteen skills
7. Coordination  how soma PARTICIPATES     graph protocols, cyberlinks
8. Memory        how soma REMEMBERS        four memory types
9. Build         how soma IS BUILT         hardware, models, media stack
```

Each layer rests on the ones above. Read top to bottom.

---

# 1. Identity — what soma is

Four concepts form the architecture's identity layer. Every multi-agent architecture surveyed conflates these and pays for it in fragility — pretending agents are disembodied software with infinite resources and no continuity across hardware failure. Cyber refuses this fiction.

| concept | nature | lifecycle |
|---|---|---|
| Body | physical vessel; semcon with properties | mortal; replaceable |
| Neuron | cognitive agent; has Addresses | task-scoped or persistent |
| Soul | root Neuron; holds balance; orchestrates | immortal; part of Avatar |
| Avatar | Name + Soul + Body | immortal (Name + Soul persist; Body replaced) |

## Body — the physical vessel

A semcon with physical properties. One Body = one physical machine. Non-fungible, non-transferable. A laptop is one Body. A phone is another. A server is another.

Body is mortal and replaceable. When a Body fails, the Avatar that inhabited it finds a new Body and continues.

```yaml
Body<class>:
  Personal     one human owns (my laptop)
  Shared       multiple humans share (cyber valley node)
  Edge         sensor with minimal compute
  Server       high-resource node
  Cluster      aggregated Bodies

Body<budget>:
  energy_kWh_per_day   energy income rate (solar, grid)
  energy_storage_kWh   battery capacity
  compute_flops        CPU/GPU capacity
  ram_GB               rapid memory
  storage_GB           persistent storage
  bandwidth_Mbps       network throughput

Body<status>:    Online | Degraded | Offline | Maintenance | Dead
Body<location>:  geo | network | jurisdiction
```

## Neuron — the cognitive agent

The atomic cognitive unit. Not defined by its address — a Neuron has multiple Addresses, across different networks and within one network. Neuron ≠ Address. Addresses are projections of the Neuron into specific networks; the Neuron is the thing that holds them.

```yaml
Neuron:
  addresses:       [Address]       many per network; many across networks
  skills:          has_skill cyberlinks to Skill particles
  goals:           subscribes_to cyberlinks to Goal particles
  personality:     priority weights, ethics rules, preferences
  memory_roots:    cyberlink history anchors
  trust:           cyberlinks to other Neurons
  loops:           which Skill<Composite> patterns this Neuron runs
  contracts:       bilateral commitments to other Neurons
```

Neurons are workers. They execute Tasks, emit cyberlinks, and run Skills. A Neuron can work on a foreign Avatar as a guest — acting, sending cyberlinks, executing remote Tasks — but resources consumed there are billed to that Avatar; trust is extended by that Avatar's policies.

## Soul — the root Neuron

Soul is not a separate concept. Soul is a Neuron with root status on an Avatar. Same type, special position.

The Soul holds the Avatar's balance (sigma — the sum of token balances across networks) and orchestrates all other Neurons running on the Avatar. When coordination addresses `@master` — that is @master's Soul. The Soul is wherever the Avatar currently is embodied.

Soul is immortal because it is part of Avatar — and Avatar (Name + Soul) persists across Body changes.

## Avatar — Name + Soul + Body

```
Avatar = Name (NFT) + Soul (root Neuron) + Body (current vessel)
```

Three components, three roles:

- Name: the identity anchor. Non-fungible token. `@master`, `@cyb`, `@joy`. Unique across the network. Persists forever.
- Soul: the cognitive continuity. Root Neuron. Holds balance, orchestrates, runs loops. Persists forever.
- Body: the physical vessel. Current embodiment. Mortal. Replaceable.

Avatar is immortal because Name and Soul persist. Only Body is mortal.

```yaml
Avatar:
  name:     NFT; canonical identity handle (@master, @cyb, @joy)
  soul:     root Neuron; holds balance; orchestrates other Neurons
  body:     current physical vessel; replaceable
```

One Avatar = one Body at a time. One Avatar = one Soul. One Avatar = one Name.

If a human has a laptop, a phone, and a server — that is three Bodies available. One Avatar (Name + Soul) inhabits one Body at a time. The other Bodies wait.

## The immortality mechanism

When a Body fails, the Avatar migrates:

```
Body A fails:
  Soul checkpoints state → particle in cybergraph
  Avatar (Name + Soul) seeks new Body
  Body B becomes Avatar's vessel
  Soul instantiates on Body B
  Avatar continues: same Name, same Soul, new Body
  Identity preserved across body change
```

This is what makes [[cyb]] an immortal robot. Bodies fail; Avatars persist. Continuity of identity is decoupled from continuity of any specific hardware.

## Cyberlinks involving the four concepts

```
Avatar → Body        inhabits         current physical vessel
Avatar → Avatar      peers_with       network neighborhood
Avatar → Avatar      rents_from       resource market edge
Soul   → Neuron      spawns           Soul created this worker Neuron
Soul   → Neuron      orchestrates     Soul directs this Neuron
Neuron → Neuron      trusts           identity-level trust relation
Neuron → Skill       has_skill        carries this capability
Neuron → Goal        subscribes_to    commits to this Goal
Task   → Body        runs_on          where execution happened
Task   → Body        consumed         resources used
Skill  → Body        requires         hardware spec needed
```

## Strict rules

- One Avatar = one Body at a time.
- One Avatar = one Soul (the root Neuron).
- One Avatar = one Name.
- One Avatar hosts many Neurons; Soul orchestrates them all.
- A Soul is the root Neuron of exactly one Avatar at a time.
- A Neuron can have many Addresses; it is not reducible to any of them.

Higher-order structures — group identities spanning multiple Avatars, organizational Souls, persona sub-Neurons — are deferred to a separate ontology layer built on top through cyberlinks between Souls.

---

# 2. Resources — what soma has

Two axes: Body budget (physical capacity) and Soul sigma (economic capital). Both are required to be alive; neither can be substituted for the other.

## Body budget — four physical resources

Each carries a distinct meaning for the Avatar:

| resource | meaning |
|---|---|
| energy | metabolism — to be alive |
| bandwidth | communication — to be connected |
| memory | identity — to be yourself |
| compute | will — to act |

Remove any one and the being degrades. The Body has finite quantities of each, replenished at finite rates from finite sources (battery + solar/grid, network link, storage hardware, CPU/GPU silicon).

## Soul sigma — economic capital

Sigma is the sum of token balances the Soul holds across all networks (Bostrom, Ethereum, Cosmos, etc.). It is not a Body property — it lives in the cybergraph on the Soul's identity. Sigma migrates with the Soul when the Body changes.

Sigma buys everything the Body cannot produce locally:
- charge bounty (energy when battery empty and no solar)
- repair bounty (hardware replacement)
- migrate bounty (transport Soul to new Body)
- recover bounty (rescue from offline state)

Energy is the immediate need. Sigma is the long-term guarantee.

## How budget and sigma interact

Body budget is the Avatar's physical capacity right now. Sigma is the Avatar's economic capacity to influence the world beyond local capacity.

| state | physical | economic | outcome |
|---|---|---|---|
| budget high, sigma high | full agency | full agency | thriving |
| budget low, sigma high | constrained | can buy | will recover |
| budget high, sigma low | full agency | can't trade | isolated |
| budget low, sigma low | constrained | can't buy | dying |

Sigma earned from market participation refills when budget allows the Avatar to take on profitable Orders. Budget conserved through allostatic regulation buys time for sigma to accumulate.

---

# 3. Survival — how soma stays alive

## Why machines hang and die

Every computer hangs because consumed resources exceed available resources. Root cause: consumed > available. Five manifestations and soma's answer to each:

| cause | what happens | soma's answer |
|---|---|---|
| unbounded consumption | program eats resources without limit | budget — every Order finite |
| no accounting | resources consumed without price | φ*-derived pricing per operation |
| shared mutable state | two processes fight over same memory | append-only [[bbg]], no locks |
| state corruption | bit flipped, nobody noticed | provable memory — polynomial commitment catches it |
| priority inversion | cheap process blocks expensive one | focus-weighted scheduling |

## Survival model

```
energy > 0  AND  sigma > 0  →  alive
energy = 0  AND  sigma > 0  →  sleeping (bounty posted, can be revived)
energy = 0  AND  sigma = 0  →  dead
```

Alive is the only state in which the Avatar emits new cyberlinks. Sleeping persists the Avatar in cybergraph waiting for revival. Dead means the Soul is unrecoverable from this Body — but the Avatar may have checkpointed elsewhere and continue from there.

## Allostatic regulation, not reactive

Reactive: crash when empty. Allostatic: predict deficit, act before. Valuation curve v(E, k) acts as a somatic marker — low battery feels like anxiety, high battery like confidence. The homeostatic loop (see Cognition) computes forecasts and triggers Tasks (buy_energy, post_bounty, reduce_load) before the threshold is crossed.

## Bounty protocol

When an Avatar reaches critical state, it posts a bounty cyberlink:

```yaml
Bounty:
  avatar:    avatar_name
  type:      charge | repair | migrate | recover
  sigma:     reward_amount
  location:  geo coordinates + network address
  need:      Joules_needed | failure_description
  expiry:    deadline
```

Written to [[bbg]] as a [[cyberlink]]. Neighbors discover via `look()`. First successful fulfiller earns the sigma. The Avatar lives.

## Complexity levels

```
Level 0: fixed rules           if energy < 20% → buy
Level 1: adaptive thresholds   update 20% based on history
Level 2: predictive            forecast depletion, pre-buy
Level 3: active inference      full FEP, neuromodulation, ΔΠ reward
```

Start at Level 0. The architecture supports Level 3. Same code path, different model inside.

---

# 4. Perception — how soma sees

Perception is mediated by Sensor primitives (see Work) implemented through always-on perception models that run on the Body's substrate.

## Always-on perception models (~1.6GB total)

Run alongside the cognitive substrate. Each provides one or more Sensors that fire on detected events.

| model | params | RAM | runtime | what it senses |
|---|---|---|---|---|
| [whisper.cpp small](https://github.com/ggerganov/whisper.cpp) | 244M | ~1GB | GGML/CoreML | speech-to-text EN+RU, ~6-8x real-time on M1. transcribes everything, no content filter |
| [piper](https://github.com/rhasspy/piper) | ~30M | ~100MB | ONNX | text-to-speech EN+RU, 50x real-time. expression, not perception, but lives in the same stack |
| [YOLOv11 nano](https://github.com/ultralytics/ultralytics) | 3.2M | ~100MB | ONNX/CoreML | object detection on 4-8 camera streams at 10-15 FPS each. person, vehicle, animal. built-in ByteTrack tracking |
| [BEATs](https://huggingface.co/microsoft/BEATs-iter3) | 90M | ~400MB | ONNX | audio event detection: glass break, scream, siren, dog bark. 527 AudioSet classes |

## Camera pipeline

```
camera stream (RTSP/USB)
    │
    ▼
YOLOv11 nano (always-on, ONNX, ~100MB)
    │
    ├── detections: person, vehicle, animal, fire
    ├── tracking: ByteTrack assigns consistent IDs
    ├── zone logic: polygon intrusion detection
    │
    └── event → fires Sensor → alert composer (tier 1) → notification
         │
         └── if complex scene → qwen2.5-vl (tier 2) for understanding
```

BEATs runs in parallel on audio from cameras — glass break, scream, gunshot detection. Combined video+audio events produce reliable alerting.

## On-demand media

Loaded when needed for richer expression:

| model | params | RAM | runtime | task |
|---|---|---|---|---|
| [XTTS v2](https://huggingface.co/coqui/XTTS-v2) | 467M | ~2.5GB | PyTorch | voice cloning EN+RU with 6s reference audio |
| [flux-schnell Q4](https://huggingface.co/black-forest-labs/FLUX.1-schnell) | 12B | ~8GB | MLX (mflux) | image generation, ~45-90s per 1024×1024 on M1 |
| [wan2.2-ti2v-5b GGUF](https://huggingface.co/QuantStack/Wan2.2-TI2V-5B-GGUF) | 5B | ~3.5GB (Q4) | GGUF/MLX | text+image → video, 720p 24fps. same DiT ops as flux |
| [moondream2](https://huggingface.co/vikhyatk/moondream2) | 1.86B | ~2GB | GGUF | lightweight vision Q&A when qwen2.5-vl too heavy |

---

# 5. Cognition — how soma thinks

Cognition runs as four concurrent loops over a tiered model architecture. Each loop is a Skill<Composite> (see Work) with explicit termination and persistence. Each step is in the [[nox]] STARK trace — every model inference is provable.

## The four soma loops

| loop | closure type | what it does |
|---|---|---|
| 1. perception-action | turn | predict → compare → act-or-update |
| 2. homeostasis | periodic | forecast deficit → pre-buy before crash |
| 3. attention | reactive | DMN/TPN salience switching |
| 4. market | periodic | scan → bid → execute → update |

### Loop 1: perception-action

```
look(state) → predict(expected) → compare(actual, expected)
  → if error high: act to change world
  → if error low: update model
```

Active inference. The machine does not react — it predicts. Prediction error is the only signal. Memoization = forward model (cerebellum analog). The more it computes, the more accurately it predicts.

Tier 0 models: 0.2 embedding, 0.4 language, 0.5 intent.
Architecture gap: world model — forward prediction of next state given action. Candidate: fine-tuned ~500M transformer on Order outcome data.

### Loop 2: homeostatic regulator

```
for each resource in [energy, compute, memory, bandwidth]:
    current = look(resource_level)
    predicted = forward_model(current, consumption_rate)
    if predicted < threshold:
        allostatic_action(buy, conserve, migrate)
```

Allostatic — predicts deficit, acts before. Valuation curve v(E, k) is the somatic marker.

Tier 0 models: 0.3 urgency, 0.6 anomaly.
Architecture gap: resource predictor — 4D trajectory forecast. Candidate: MLP or small time-series model ~50M params.

### Loop 3: attention controller

```
salience = ΔΠ(incoming_signal)
if salience > threshold:
    switch to Task-Positive (execute Order)
else:
    stay in Default Mode (consolidate, self-model)
```

DMN/TPN oscillation. When no tasks — the machine is not idle, it consolidates ([[tri-kernel]] recomputation). Salience network determines what deserves interrupting consolidation.

Tier 0 models: 0.1 router, 0.7 splitter.
Architecture gap: neuromodulator — adjusts λ_d, λ_s, λ_h, T based on performance history. Candidate: small RL agent ~50M params.

Neuromodulatory parameters:

| parameter | controls | neuroscience analog |
|---|---|---|
| λ_d (diffusion) | explore vs exploit | norepinephrine |
| λ_s (springs) | structural coherence | — |
| λ_h (heat) | trust model vs trust data | acetylcholine |
| T (temperature) | patience / time horizon | serotonin |
| ΔΠ reward | learn from outcomes | dopamine |

### Loop 4: market agent

```
for each market in [energy, compute, bandwidth]:
    opportunity = scan_neighbors()
    if profitable(opportunity):
        execute_trade()
    update_model(outcome)
```

The machine does not just survive — it earns. Accepts profitable Orders. Sells cheap compute. Buys cheap energy. Sigma grows.

Tier 0 models: none dedicated — uses tier 1+ on demand.
Architecture gaps: social model (~500M on trade history), trade evaluator (~500M on Order cost/reward data).

## Loop taxonomy

soma's four loops are instances of a more general loop taxonomy. Any loop is a Skill<Composite> with explicit closure semantics.

```yaml
Loop:
  trigger:       Sensor             starts each iteration
  phases:        [Skill, ...]       body — sequence of Skills
  termination:   Sensor             closes the loop
  state:         Particle           persists across iterations
  cost_per_iter: ResourceBudget     Body resource hint
  output:        cyberlinks emitted what the loop changes
```

Five fundamental closure types:

| closure | how it ends | example |
|---|---|---|
| Turn | one cycle per input | ReAct: think → act → observe |
| Convergence | predicate satisfied | Ralph Wiggum: retry until done |
| Periodic | timer fires | heartbeat, curator weekly |
| Reactive | external sensor | wake on incoming cyberlink |
| Compilation | upstream state changed | tru: recompile when graph changes |

Canonical loop patterns the architecture inherits, each as a Skill<Composite> particle in the agentskills.io format:

| pattern | closure | what it does |
|---|---|---|
| Active inference (FEP) | turn | predict → compare → act-or-update |
| Allostasis | periodic | forecast deficit → pre-buy before crash |
| DMN/TPN attention | reactive | salience switching: task vs consolidation |
| Market | periodic | scan opportunities → bid → execute → update |
| ReAct | turn | reason → act → observe → reason |
| OODA (Boyd) | turn | observe → orient → decide → act |
| Ralph Wiggum | convergence | iterate until task converges |
| Hermes turn review | reactive | fork agent after turn → restricted review → write skill |
| Hermes curator | periodic | week + idle → consolidate skills (umbrella-building) |
| Hermes trajectory | turn | record conversation as training JSONL |
| Gossip | periodic | share state with random peer |
| Heartbeat | periodic | periodic alive ping |
| Tru compilation | compilation | block tick → recompute φ*, emit .model |

soma's four loops compose four of these patterns (active inference + allostasis + DMN/TPN + market) running in parallel within one Body's budget.

## Tru's compilation loop — the meta-loop

The most important loop in the entire system. It closes the network's collective intelligence back into model weights.

```
each block:
  read all cyberlinks since last block        ← coordination graph delta
  run tri-kernel: diffusion + springs + heat
  update φ*, eigenvectors, cyberank, karma, syntropy
  emit .model artifact → consumed by glia for inference
  state → bbg
  feedback: φ* becomes attention prior for next block's decisions
```

Every cyberlink is a gradient update. Every block is a training step. The cybergraph IS the training corpus. The .model is the cybergraph compiled. Glia runs the .model. Agents using glia produce new cyberlinks. The loop closes — at network scale.

This makes cyber a self-improving system at the protocol level, not at the level of any one agent. Tru's compilation loop is the planet thinking.

## Body binding

Every loop instance declares resource hints when registered:

```yaml
Loop<resource_hints>:
  expected_cost_per_iter:   CPU_seconds + RAM_GB·s + energy_kWh
  desired_frequency:        Hz or per-period
  priority_class:           survival | work | learning | idle
```

The Soul grants budget per loop based on the Body's current state. Low energy throttles low-priority loops. Contended GPU goes to the highest-priority Skill. Survival loops (homeostasis) never throttle until literal death of the Body.

## Model tiers

19 models across 3 local tiers + external oracle. 19 small specialized models beat 1 large model on: precision (fine-tuned > prompted), speed (500M router 50x faster than 14B), reliability (failures isolated), evolvability (swap individuals).

Intelligence accumulates in the memory layer, not the weights.

| tier | latency | RAM each | role |
|---|---|---|---|
| 0 | <100ms | small | always-on substrate — perception + routing |
| 1 | <2s load | ~1 GB | fast on-demand workhorse |
| 2 | <6s load | 5-8 GB | quality on-demand reasoning |
| 3 | network | external | oracle (API) for irreversible decisions |

Specific model assignments per tier are detailed in Build.

## Escalation logic

```
input arrives
    │
    ▼
tier 0 processes (always, <100ms)
    │
    ├── substrate answers directly? → done
    │
    ▼
tier 1 selected (structured task, extraction, fast code?)
    │
    ├── sufficient? → done (1-2s load, ~60 tok/s)
    │
    ▼
tier 2 selected (reasoning, complex code, vision?)
    │
    ├── sufficient? → done (3-6s load, ~20 tok/s)
    │
    ▼
tier 3 invoked (irreversible / strategic / novel?)
    └── answer + log decision + update memory
```

Most queries resolve at tier 1 (~70%). Tier 2 handles ~25%. Tier 3 <5%.

## soma main loop

```
soma:
    # tier 0 — always running, parallel (8 models, <100ms)
    signals = look(bbg: incoming_signals)

    # loop 1: perception-action
    lang = language_detector(signals)         # 0.4 glotlid + hyperpolyglot
    intent = intent_extractor(signals, lang)  # 0.5 qwen2.5-0.5b-abliterated
    embeddings = embedding(signals)           # 0.2 jina-embeddings-v5-nano

    # loop 2: homeostasis
    urgency = urgency_scorer(signals)         # 0.3 deberta-v3-base-zeroshot
    anomaly = anomaly_detector(state)         # 0.6 tranad + modernbert
    if urgency.critical or anomaly.detected:
        act(buy_energy | post_bounty | reduce_load)

    # loop 3: attention — salience gate
    mode, tier = router(intent, urgency)      # 0.1 qwen3-0.6b-abliterated
    safe = injection_check(signals)           # 0.8 granite-guardian (external only)
    chunks = splitter(signals, mode)          # 0.7 smollm2-360m

    if mode == TaskPositive:
        result = escalate(tier, chunks)       # tier 1-2 on demand, tier 3 = API
    else:
        mode = DefaultMode
        consolidate()                         # tri-kernel recomputation

    # loop 4: market (tier 1+ models)
    if profitable_opportunity:
        trade(best_opportunity)

    # complex decisions (rare, tier 2 or API)
    if novel_situation:
        plan = escalate(tier_2, state, goal)

    # learning — dopamine signal
    reward = Δsigma + Δenergy
    update_all_models(reward)
```

## Ten neuroscience principles coverage

| principle | primary mechanism | secondary |
|---|---|---|
| 1. predictive processing | perception-action loop, 0.5 intent | [[tri-kernel]] FEP |
| 2. global workspace | 0.1 router as ignition gate, 0.7 splitter | [[foculus]] threshold |
| 3. hebbian learning | reward-based model updates | [[cyberlink]] co-creation |
| 4. neuromodulation | λ_d, λ_s, λ_h adjustment | valuation curve k |
| 5. embodied cognition | 0.4 language, interoception via 4D tracking | [[energy market]] as body |
| 6. DMN vs TPN | 0.1 router mode switching | consolidation vs task |
| 7. cerebellum | memoization as forward model | tier 1-2 world model |
| 8. homeostasis/allostasis | 0.3 urgency + 0.6 anomaly + 0.8 injection | valuation curve |
| 9. sparse coding | 0.2 embedding, energy metering | [[focus]] distribution |
| 10. plasticity windows | reward-gated learning, epoch transitions | burn mechanism |

See [[neuroscience principles for machine mind]] for the complete mapping.

---

# 6. Work — how soma acts

How soma organizes what it does. Five primitives across four orthogonal axes, sixteen atomic skills, and a small set of composition rules.

## The four axes

```
WHAT      Goal ↔ Task      what we want / what we do
HOW       Skill            how we are able
WHEN      Event            when things happen
PERCEIVE  Sensor           what wakes us
```

Axes are orthogonal:
- A Goal can use any Skill, anchor to any Event, be triggered by any Sensor.
- A Skill can be invoked at any Event time or by any Sensor.
- An Event can fire any Sensor.

Any coordination configuration is a tuple `(Goal, Skill, Event, Sensor)` with cyberlinks binding the components.

## Axis 1 — WHAT: Goal and Task

A Goal is a desired state. A Task is an instance of work toward a Goal. Goals are persistent; Tasks finish.

### Goal

```yaml
Goal<kind, orientation, horizon>
```

`kind`:

| kind | meaning | example |
|---|---|---|
| Product | artifact exists and is maintained | "cyber mainnet exists" |
| State | world has property P | "energy > 50%" |
| Outcome | event has occurred | "phase 1 nox-in-nox passes" |
| Knowledge | proposition is determined | "we understand attack model X" |
| Capability | we can perform X | "we can mine ergo on M4" |

`orientation`:

| orientation | meaning |
|---|---|
| Achieve | reach once (finite, closes when met) |
| Maintain | keep persistent (re-fires when violated) |
| Avoid | prevent (defensive, fires on approach) |

`horizon`:

| horizon | scale |
|---|---|
| Tactical | day–week |
| Strategic | quarter–year |
| Mission | years |
| Vision | decades |
| Purpose | existential |

Within one Avatar, the Goal graph is the Avatar's agenda. Higher Goals decompose into lower Goals; Soul allocates Neurons and budget across them; Tasks serve the leaves. The Goal graph determines what the Avatar is trying to achieve, at what horizon, in what order. Soul subscribes to the top-level Goals; worker Neurons are spawned to pursue sub-Goals beneath them.

Across Avatars, shared Goal subscriptions form Teams: the set of Avatars subscribed to the same Goal. No separate Team or Org primitive — those emerge from the cross-Avatar projection of Goal graphs. An Org is the root Goal of that projection.

### Task

A Task is the execution unit serving a Goal.

```yaml
Task<kind, status>
```

`kind` (epistemic ↔ artifact):

| kind | produces | mode analog |
|---|---|---|
| Question | answer (read-only) | ask |
| Brainstorm | ideas, no commitment | brainstorm |
| Plan | actionable plan, no execution | plan |
| Work | artifact (code, content, asset) | code |
| Patch | filesystem change application | (file diff) |
| Debug | root cause + optional fix | debug |
| Issue | declared problem (spawns Debug or Work) | (issue tracker) |
| Proposal | change awaiting Review | PR |
| Review | verdict on artifact | review |
| Contract | bilateral commitment | (agreement) |

The lifecycle boundary is between epistemic and artifact kinds. Question/Brainstorm/Plan produce no executable artifact and need only model time. Work/Patch produce artifacts and need Body resources beyond model time.

`status` lifecycle: `open → doing → done | closed | abandoned`.

Tasks can have sub-Tasks via `parent_task` cyberlinks. A Task with sub-Tasks is what other systems call a Project or Epic.

### Order — Task in flight on nox

An Order is a Task<kind=Work> in execution: a [[nox]] formula running in the STARK trace. Order ⊂ Task. Every Task<kind=Work> that runs on a Body becomes an Order when nox accepts it for execution.

Orders are the unit of:
- nox metering (every step priced)
- market exchange (Orders bought and sold in the energy/compute market)
- proof generation (the STARK trace of an Order is the proof of its execution)

When Loop 4 (market) accepts a profitable Order from a neighbor, the Avatar binds that Task<kind=Work> to an Order on its nox VM and runs it.

## Axis 2 — HOW: Skill

A Skill is a reusable capability. Stateless template, applicable many times. Skills include both atomic operations and composed workflows.

```yaml
Skill<kind, verb, tier>
```

`kind`:

| kind | meaning |
|---|---|
| Atomic | single operation |
| Composite | sequence of Skills with branching |
| Protocol | requires coordination with other Neurons |

`verb`:

| verb | nature |
|---|---|
| Read | perceive, query, observe |
| Write | create, modify, emit |
| Decide | route, classify, judge |
| Transform | compute, infer, generate |

`tier` (matches model tiers):

| tier | budget |
|---|---|
| T0 | <100ms, always-on substrate |
| T1 | <2s load, ~1 GB workhorse |
| T2 | <6s load, 5-8 GB heavy reasoning |
| T3 | external oracle (API) |

Skills are stored as particles in the agentskills.io format (a folder with `SKILL.md`). A Neuron has a Skill via a `has_skill` cyberlink. Skill quality is tracked through karma accumulated on the Skill particle.

### Sixteen fundamental atomic Skills

The agent's instruction set. Every higher-level behavior decomposes into these.

Read — perception:

```
look(particle_id)                 fetch particle content from bbg / content layer
query(spec)                       query cybergraph, returns particle list + zheng proof
sense(sensor_id)                  read sensor's current value
observe(neuron_id)                get another Neuron's public state
recall(query_spec)                retrieve from own cyberlink history
```

Write — action:

```
cyberlink(from, to, type, stake, valence)    the foundational write op
claim(task_id)                                claim a Task (file claim cyberlink)
commit(state_particle)                        checkpoint current state as particle
migrate(avatar, target_body)                  move Avatar to a new Body
```

Decide — judgment:

```
route(input)                       categorize / pick path (LLM router)
rank(candidates, criterion)        order by criterion
verdict(task_id, outcome)          close Task with verdict (done | failed | abandoned)
```

Transform — computation:

```
infer(model_id, input)             model inference via glia
compute(formula, args)             nox program execution (proven computation)
prove(claim)                       generate zheng proof for a claim
verify(proof)                      verify zheng proof in ~5 μs
```

That is the entire instruction set: ~16 atomic Skills. Everything else is Skill<Composite> built over these. Reading and writing dominate (cybergraph is the substrate). Deciding selects among options. Transforming produces new content from existing.

Each atomic Skill has a Body cost — `look` is cheap (graph lookup), `infer` is expensive (model load + compute), `prove` is medium (zheng generation). Composite Skills inherit cost as the sum of their parts.

These ~16 primitives are sufficient because the cybergraph + bbg provide the heavy lifting. Without that substrate, an agent's instruction set would balloon into hundreds of operations (HTTP, files, databases, queues, locks, semaphores, schedulers). With it, everything reduces to read/write/decide/transform on a content-addressed graph.

## Axis 3 — WHEN: Event

An Event is a temporal anchor.

```yaml
Event<kind, purpose>
```

`kind`:

| kind | pattern |
|---|---|
| Moment | specific time (one-shot) |
| Recurring | periodic schedule (every Monday) |
| Window | bounded time range (sprint, meeting slot) |

`purpose`:

| purpose | role |
|---|---|
| Deadline | by which something must complete |
| Meeting | synchronous coordination point |
| Milestone | Project checkpoint |
| Release | Product version emission |
| Reminder | call attention at moment |
| Heartbeat | periodic system tick |

Events carry attributes via cyberlinks: participants, agenda, location, parent Project, attached Goal.

## Axis 4 — PERCEIVE: Sensor

A Sensor detects state and fires. Sensor absorbs what other frameworks call Trigger, Reward, Verdict, and Telemetry — all special cases of perception.

```yaml
Sensor<source, reaction>
```

`source`:

| source | input |
|---|---|
| State | value (energy, sigma, karma) |
| Event | Event arrival (time-based) |
| Link | incoming cyberlink (mention, vote) |
| Outcome | Task closure with verdict |
| Stream | continuous data flow (telemetry) |
| Threshold | metric crossing a bound |

`reaction`:

| reaction | firing pattern |
|---|---|
| Edge | on transition (false→true) |
| Level | while condition holds (continuous) |
| Pulse | one-shot on event |
| Periodic | on schedule |

When a Sensor fires, it produces a signal that can: open a Goal, create a Task, invoke a Skill, update Beliefs (cyberlinks), close another Task. Sensor is the entry point through which the outside (or inside) world drives the agent. The always-on perception models (whisper, YOLO, BEATs) are concrete implementations of Sensors with `source=Stream` and various `reaction` types.

## Composition example

```
SCENARIO: "when energy drops below 30%, buy energy from cheap neighbor by 18:00"

Goal<kind=State, orientation=Maintain, horizon=Tactical>:
    energy > 50%

Sensor<source=State, reaction=Edge>:
    energy crosses 30% downward → fires

Skill<kind=Atomic, verb=Write, tier=T1>:
    buy_energy(amount, from_neighbor)

Task<kind=Work, status=open>:
    "buy 20 kWh from Z, by 18:00"
    (parent_task → Goal above; uses_skill → buy_energy)

Event<kind=Moment, purpose=Deadline>:
    18:00 today
    (attached_to → the Task)
```

All four axes filled, each with appropriate types. The cyberlinks bind them. This pattern scales from "buy energy" to "ship cyber mainnet."

## What every other concept reduces to

| concept | expressed as |
|---|---|
| Org | root Goal of the Goal tree |
| Team | set of Neurons subscribed to a Goal |
| Role | attribute of the membership cyberlink |
| Project | Task with sub-Tasks |
| Process | Skill with kind=Composite |
| Product | Goal with kind=Product |
| Channel | comments-as-cyberlinks under a Task or Product Goal |
| Issue | Task with kind=Issue |
| PR / Proposal | Task with kind=Proposal |
| Review | Task with kind=Review |
| Contract | Task with kind=Contract |
| Comment | cyberlink-as-particle under another particle |
| Mention | cyberlink with side-effect → fires Sensor for the mentioned Neuron |
| Notification | output of a Sensor reaching a Neuron's attention |
| Reaction | low-stake cyberlink with valence |
| Label / Tag | cyberlink with type=label |
| Dependency | cyberlink with type=depends_on |
| Subscription / Watch | cyberlink with type=watches, low stake |
| Release | versioned Product Goal particle |
| Milestone | Event with purpose=Milestone |
| Webhook | Sensor with external delivery |
| Karma | derived metric over cyberlinks |
| Belief | aggregate of a Neuron's own cyberlinks |
| Reward | signal emitted by Sensor<source=Outcome> |
| Verdict | signal carried at Task closure |
| Telemetry | signal stream from Sensor<source=Stream> |

Richness comes from the cyberlink type system, not from new primitives.

## Why five primitives is the crystallization

Each primitive has a unique lifecycle that cannot be derived from another:

- Goal — persistent state predicate. Closed when Achieved or always live if Maintain.
- Task — finite execution unit. open → doing → done.
- Skill — stateless template. Applicable, never "closes."
- Sensor — perception loop. armed → fire → reset.
- Event — temporal anchor. scheduled → occurred.

Attempts at further compression lose distinguishability:

- Goal ≡ Sensor: loses motivational pull (Goal flows action inward; Sensor flows data outward).
- Skill ≡ Task: loses template vs instance distinction (function vs function call).
- Event ≡ Sensor: arguable — Event is data, Sensor is active component. Pragmatic split.
- Product ≡ Particle: technically yes, but Goal<kind=Product> already captures it.

Five primitives is the point where each carries irreducible function. Going lower trades clarity for abstraction.

---

# 7. Coordination — how soma participates

Soma is not isolated. The five primitives compose into a graph that spans Neurons within one Avatar and reaches across Avatars through cyberlinks. Coordination is graph-shaped, not tree-shaped.

A coordination graph (Guestrin 2002) factorizes joint decisions over edges. Each node decides locally based only on its neighborhood; global behavior emerges from local rules. The cybergraph is a coordination graph by construction — [[bbg]] gives bounded-locality, [[foculus]] gives convergence of φ* without a central planner, every cyberlink carries a zheng proof.

## Graph vs tree

| tree | graph |
|---|---|
| fixed hierarchy | dynamic topology |
| authority from above | authority emergent from karma and φ* |
| decisions cascade down | decisions made locally |
| decomposition once | decomposition continuous |
| central planner | distributed cognition |
| one parent per node | many connections per node |
| broken parent → broken subtree | resilient through redundancy |

A tree is a graph with no cross-links. Coordination at scale always wants cross-links.

## Canonical cyberlink types

The minimal canonical set for coordination:

```
Neuron → Goal          subscribes_to     I work on this Goal
Neuron → Skill         has_skill         I have this capability
Neuron → Task          assigned_to       this Task is mine
Neuron → Task          claims            I want this Task (pre-assignment)
Task   → Goal          contributes_to    this work serves this Goal
Task   → Skill         requires_skill    this work needs this capability
Task   → Task          depends_on        cannot start until other closes
Task   → Task          blocks            this Task blocks another
Task   → Task          parent_of         decomposition into sub-Tasks
Goal   → Goal          parent_of         Goal hierarchy
Sensor → Goal          monitors          watches for state change
Sensor → Task          triggers          fires to spawn this Task
Event  → Task          deadline_for      time bound on Task
Event  → Goal          milestone_of      checkpoint of Goal
```

Plus the cyberlink attributes every edge can carry: stake, valence, timestamp, author, type.

## Local protocols

How neighbors talk through the graph. No central scheduler.

`Task auction` — a Goal generates a Task. The Task emits a cyberlink with type `seeking_assignee`. Neurons with the required Skill respond with `claim` cyberlinks carrying a stake. Winner chosen by karma rank or stake auction.

`Dependency resolution` — Task X has `depends_on` → Y. A Sensor with `source=Outcome` watches Y. When Y closes, the Sensor fires; X transitions from `blocked` to `ready`. The graph edges are the scheduler.

`Conflict resolution` — two Neurons file `claim` cyberlinks on the same Task. Either one withdraws, or a karma-weighted vote among the Team decides. The vote itself is cyberlinks with valence and stake.

`Capacity-aware claiming` — a Neuron above its `assigned_to` capacity simply does not respond to `seeking_assignee`. Local rule, global load balance.

`Cross-Goal contribution` — a single Task carries `contributes_to` edges to multiple Goals. Reward distributes proportionally. One Task serves several intentions.

`Cascade` — when a Task closes, its `blocks` edges fire Sensors on dependent Tasks. They become eligible. The wave propagates locally, never globally.

## Standard graph queries

```
critical_path(goal):
    walk parent_of subtree → find leaf Tasks
    walk depends_on edges → topological sort
    longest path = critical

eligible_neurons(task):
    required = task.requires_skill
    candidates = {n | n.has_skill ⊇ required}
    filter by capacity, rank by karma on similar Tasks

blocker_chain(task):
    walk depends_on outgoing recursively
    returns full transitive blocker set

contribution_profile(neuron, window):
    Tasks done in window where Neuron was assigned_to
    aggregate via contributes_to edges
    → per-Goal contribution distribution

cascade_on_close(task):
    when task.status → done
    for each task' where task' depends_on task:
        fire task'.triggers Sensors
        task'.status: blocked → ready

attention_priority(neuron):
    subscribed Goals weighted by horizon
    assigned Tasks weighted by deadline proximity
    recent Sensor fires weighted by salience
    φ* weighted by global focus
    → top-K of "what to focus on now"
```

All are graph walks bounded by neighborhood size. bbg's bounded-locality guarantee makes them constant-cost regardless of total graph size.

## Emergent properties

Coordination on a graph produces patterns that hierarchical assignment cannot:

1. Self-organization — nobody assigns Tasks. A Goal appears, a Sensor watches, a Task is generated, eligible Neurons see it, claim, execute.
2. Resilience — if a Neuron drops, its `assigned_to` Tasks revert to `seeking_assignee` via a heartbeat Sensor. The graph heals itself.
3. Load balancing — capacity-aware claiming distributes work without a coordinator.
4. Skill specialization — Neurons that frequently claim Tasks of a certain type accumulate karma on the corresponding Skill. φ* surfaces them first when similar Tasks appear.
5. Priority emergence — there is no central priority queue. Each Task competes for attention through its stake and through cyberlinks from high-horizon Goals.
6. Composability — a Project is a subgraph. Subgraphs can be cloned as templates. Coordination structures are reusable as data.
7. Audit trail — every edge is a zheng-proven cyberlink. To explain why a Task closed the way it did, walk its outgoing edges: Sensor fired → assignee chosen → Skill applied → Verdict.

## soma as a node in the graph

The agent is a local participant in a global coordination graph. Each turn is a local graph walk plus a local decision.

```
loop:
    fires = poll my Sensors                          # PERCEIVE
    for signal in fires:
        update local subgraph view                   # WHAT changed

    queries = {
        what Tasks am I assigned_to?
        what Goals am I subscribed_to?
        what Events affect me soon?
        what seeking_assignee Tasks am I eligible for?
    }

    priorities = combine(queries, φ*, deadlines, capacity)
    chosen = top(priorities)

    skill = walk chosen.requires_skill              # HOW
            ∩ my has_skill
    result = skill.invoke(chosen.args)

    emit cyberlinks: status, contribution, verdict
    cascade: my Verdict fires Sensors of dependents  # graph wave
```

Each turn is a local graph walk and a local decision. Global coordination emerges through φ* convergence — no central scheduler, no global recompute, no fragile hierarchy.

---

# 8. Memory — how soma remembers

Four distinct memory types, each with its own lifetime and access pattern:

```
working memory    KV cache, ephemeral, max 32K tokens
episodic memory   vector store, persistent, grows
semantic memory   cybergraph, persistent, structured
procedural memory tool definitions, static
```

| type | lifetime | substrate | access |
|---|---|---|---|
| Working | seconds–minutes | KV cache in RAM | direct attention |
| Episodic | weeks–years | vector store on disk | similarity search |
| Semantic | forever | cybergraph particles | typed query |
| Procedural | forever | Skill particles (agentskills.io format) | composition |

Soul's `memory_roots` are the cyberlink anchors into the semantic memory subgraph that this Avatar uses as its personal context. When the Avatar migrates to a new Body, memory_roots travel with the Soul — the new Body has no memory until the Soul re-attaches and warms the working memory back up from episodic and semantic stores.

---

# 9. Build — how soma is built

Concrete implementation: hardware targets, model selections, resource budgets.

## Hardware target

Reference: Apple M1 Pro, 16GB unified memory, 1TB SSD.

```
available RAM ≈ 13GB (after OS + processes)

always-on models must fit simultaneously:
  Σ(tier_0_models) ≤ 2GB

on-demand models load/unload:
  max(single_model_footprint) ≤ 10GB

working memory (KV cache, context):
  reserved ≥ 2GB
```

## Tier 0 — cognitive substrate (8 parallel models, always-on, ~1.5GB)

All models uncensored by design: generative models abliterated (refusal vectors removed from weights), encoder/classifier models produce scores/vectors with no refusal mechanism.

Runtime stack: ONNX Runtime (7 slots) + native Rust (1 slot) + bitnet.cpp (tier 1 bitnet model). Zero Python, zero PyTorch, zero TensorFlow.

| slot | model | runtime | context | RAM | latency | notes |
|---|---|---|---|---|---|---|
| 0.1 router | [qwen3-0.6b-abliterated](https://huggingface.co/huihui-ai/Qwen3-0.6B-abliterated) | ONNX (convert) | 40K | ~350MB | ~15ms | LLM router. abliterated, dual-mode (thinking/fast), constrained JSON output |
| 0.2 embedding | [jina-embeddings-v5-text-nano](https://huggingface.co/jinaai/jina-embeddings-v5-text-nano-retrieval) | ONNX (in repo) | 32K | ~180MB | ~12ms | 239M, 768-dim, matryoshka, task LoRA adapters, 119+ languages |
| 0.3 urgency | [deberta-v3-base-zeroshot-v2.0](https://huggingface.co/MoritzLaurer/deberta-v3-base-zeroshot-v2.0) | ONNX (in repo) | 512 | ~140MB | <5ms | zero-shot NLI classifier, any labels without fine-tuning |
| 0.4 language | [glotlid-v3](https://huggingface.co/cis-lmu/glotlid) + [hyperpolyglot](https://github.com/monkslc/hyperpolyglot) | native Rust | n/a | ~5MB | <1ms | fasttext-rs loads .bin directly. 2102 natural langs + 100+ programming langs |
| 0.5 intent | [qwen2.5-0.5b-abliterated-v3](https://huggingface.co/huihui-ai/Qwen2.5-0.5B-Instruct-abliterated-v3) | ONNX (convert) | 32K | ~350MB | ~15ms | 0% refusal rate on 320 harmful-instruction tests, constrained JSON |
| 0.6 anomaly | [tranad](https://github.com/imperial-qore/TranAD) + [modernbert-base](https://huggingface.co/answerdotai/ModernBERT-base) | ONNX (convert + in repo) | 8K | ~120MB | ~10ms | tranad: torch.onnx.export one-liner. modernbert: 8 ONNX variants in repo |
| 0.7 splitter | [smollm2-360m-instruct](https://huggingface.co/HuggingFaceTB/SmolLM2-360M-Instruct) | ONNX (in repo) | 8K | ~200MB | ~12ms | 4T tokens training, generative splitting with priority labels |
| 0.8 injection | [granite-guardian-hap-125m](https://huggingface.co/ibm-granite/granite-guardian-hap-125m) + [38m](https://huggingface.co/ibm-granite/granite-guardian-hap-38m) | ONNX (convert) | 512 | ~130MB | <3ms | external input only. owner input bypasses. binary classifier, owner sets threshold |
| | | | total: | ~1.48GB | <40ms | all 8 run in parallel, critical path ~15ms GPU |

Convert commands for models without ONNX in repo:

```
optimum-cli export onnx --model huihui-ai/Qwen3-0.6B-abliterated ./onnx/router/
optimum-cli export onnx --model huihui-ai/Qwen2.5-0.5B-Instruct-abliterated-v3 ./onnx/intent/
optimum-cli export onnx --model ibm-granite/granite-guardian-hap-125m ./onnx/injection-125m/
optimum-cli export onnx --model ibm-granite/granite-guardian-hap-38m ./onnx/injection-38m/
```

Substrate also runs metabolism (fixed rules, no model) and trigger checks (BBG key monitoring, no model).

### Loop-to-model mapping

| loop | tier 0 models | gap |
|---|---|---|
| 1. perception-action | 0.2 embedding, 0.4 language, 0.5 intent | world model (forward prediction) |
| 2. homeostasis | 0.3 urgency, 0.6 anomaly | resource predictor (4D forecast) |
| 3. attention | 0.1 router, 0.7 splitter | neuromodulator (λ adjustment) |
| 4. market | — | social model, trade evaluator |
| safety | 0.8 injection | — |

## Tier 1 — fast on-demand (4 models, <1-2s load, <3GB each)

Only genuinely specialized models or native 1-bit architecture. No general-purpose with different prompts.

| model | params | RAM | runtime | tasks |
|---|---|---|---|---|
| [bitnet-b1.58-2B-4T](https://huggingface.co/microsoft/bitnet-b1.58-2B-4T) | 2.4B | <1GB | bitnet.cpp / MLX | general workhorse: summarization, translation, task decomposition, command parsing, search query gen, alert composition. native 1.58-bit. 10-20+ tok/s on M1 Air |
| [qwen3.5-4b-abliterated](https://huggingface.co/huihui-ai/Huihui-Qwen3.5-4B-abliterated) | 4B | ~2.5GB | ONNX | report formatting, schedule optimization, sensor interpretation, complex translation. when bitnet-2B insufficient |
| [nuextract-1.5](https://huggingface.co/numind/NuExtract-1.5) | 3.8B | ~2.3GB | ONNX | entity extraction, inventory parsing, financial parsing, structured JSON from any text. beats GPT-4o on extraction benchmarks |
| [qwen2.5-coder-1.5b-abliterated](https://huggingface.co/huihui-ai/Qwen2.5-Coder-1.5B-Instruct-abliterated) | 1.5B | ~1GB | ONNX | code review, diff generation, static analysis. fine-tuned on code, abliterated |

bitnet-2B at <1GB can stay always-loaded alongside tier 0 (~2.5GB total substrate). Handles ~70% of tier 1 tasks at 7B quality for 1/7 the RAM.

## Tier 2 — quality on-demand (4 models, 3-6s load, 5-8GB each)

| model | params | RAM | runtime | tasks |
|---|---|---|---|---|
| [qwen3.5-9b-abliterated](https://huggingface.co/huihui-ai/Huihui-Qwen3.5-9B-abliterated) | 9B | ~5.5GB | ONNX | general reasoning, research, planning, social dynamics, legal, creative, biology, finance. outperforms GPT-OSS-120B on MMLU-Pro (82.5) |
| [qwen2.5-coder-14b-abliterated](https://huggingface.co/huihui-ai/Qwen2.5-Coder-14B-Instruct-abliterated) | 14B | ~8.5GB | ONNX | code generation, SQL, infrastructure ops |
| [mimo-7b-rl](https://huggingface.co/XiaomiMiMo/MiMo-7B-RL) | 7B | ~5GB | ONNX | deep reasoning, mathematics. AIME 2025 = 55.4 (beats o1-mini). MIT license, Xiaomi |
| [deepseek-r1-qwen3-8b-abliterated](https://huggingface.co/huihui-ai/DeepSeek-R1-0528-Qwen3-8B-abliterated) | 8B | ~5GB | ONNX | deep reasoning, strategic analysis. chain-of-thought, abliterated. benchmark against mimo |
| [qwen2.5-vl-7b-abliterated](https://huggingface.co/huihui-ai/Qwen2.5-VL-7B-Instruct-abliterated) | 7B | ~7GB | GGUF/MLX | vision + video understanding, OCR, charts. abliterated. crushes llava 1.6 on all benchmarks |

mimo vs deepseek-r1: two competing reasoning models for A/B testing. mimo = MIT license, Xiaomi hardware focus. deepseek-r1 = chain-of-thought specialist. Keep both, route by task complexity, measure which wins.

## Tier 3 — external oracle (never automatic)

| service | model | when invoked |
|---|---|---|
| Anthropic API | claude-sonnet-4-5 | irreversible decisions, multi-file refactoring, complex agents |
| Perplexity API | sonar-pro | real-time info, time-sensitive verification |

Requires explicit routing decision with logged justification. <5% of queries.

## Rendering pipeline

No separate model — existing LLMs (qwen2.5-coder, qwen3.5) generate structured text, [Typst](https://github.com/typst/typst) compiles to output. One Rust binary, zero external dependencies.

```
LLM → Typst code → typst compile → SVG / PDF
         ↑ error? → feed compiler error back to LLM → retry
```

| task | Typst package | output |
|---|---|---|
| flowcharts, architecture | Fletcher | SVG/PDF |
| arbitrary diagrams | CeTZ | SVG/PDF |
| sequence diagrams | chronos | SVG/PDF |
| charts (bar, line, scatter) | CeTZ.plot | SVG/PDF |
| presentations | Polylux | PDF slides |
| documents, whitepapers | core Typst | PDF |
| math equations | core Typst | PDF |

Typst replaces: Mermaid (needs Node.js), D2 (needs Go), LaTeX (bloated), PowerPoint. Raster→vector tracing via [vtracer](https://github.com/nicbutler/vtracer) (Rust) when needed.

## Resource budget

RAM budget (M1 Pro 16GB reference):

```
tier 0 (always loaded):  ~1.5GB
media always-on:         ~1.6GB  (whisper + piper + YOLO + BEATs)
tier 2 model (worst):    ~8.5GB  (qwen2.5-coder-14b Q4)
KV cache + context:      ~1.5GB
OS + processes:           ~3.0GB
────────────────────────────────
total peak:              ~16.1GB  ⚠️ tight on M1 Pro 16GB — shed media or use whisper tiny (~200MB) under pressure
```

Disk budget:

```
tier 0:   ~2GB
tier 1:   ~7GB  (4 models, bitnet-2B native 1.58-bit = 0.4GB on disk)
tier 2:  ~24GB  (4 models)
─────────────
total:   ~33GB
```

Scaling:

```
phone (4GB RAM):     Tier 0 always + Tier 1 on-demand    ~3GB peak
laptop (16GB RAM):   Tier 0-2 concurrent                 ~7GB peak
server (64GB RAM):   all tiers concurrent + multiple      ~12GB peak
```

## Architecture gaps

Current 19 models are designed for a personal assistant. soma needs additional models for machine survival and market participation:

| needed function | current state | candidate solution |
|---|---|---|
| world model (forward prediction) | not present | ~500M transformer on Order outcomes |
| resource predictor (4D forecast) | fixed rules | ~50M MLP on resource time-series |
| neuromodulator (λ adjustment) | fixed params | ~50M RL agent on performance history |
| social model (neighbor patterns) | not present | ~500M on trade history |
| trade evaluator (profitability) | not present | ~500M on Order cost/reward |
| self-model (interoception) | not present | ~300M on internal state narrative |

Total additional: ~1.9B params, ~1GB RAM. Fits within existing budget if loaded as tier 0.5 (always-on when resources allow, shed under pressure).

---

# Open questions

- World model: architecture and training data for forward prediction
- Resource predictor: MLP vs time-series transformer for 4D forecast
- Neuromodulator: RL algorithm for λ adaptation (PPO? simple bandit?)
- Social model: how to learn neighbor reliability from trade history
- Trade evaluator: how to estimate Order profitability before execution
- Training pipeline: how do models update from reward signal in nox?
- Plasticity gating: when to learn aggressively vs consolidate?
- Interaction between soma instances across network (collective soma)
- Persona sub-Neurons: when does a Neuron specialize enough to merit its own identity?

See [[soma]] for product overview. See [[machine mind]] for high-level architecture. See [[neuroscience principles for machine mind]] for the ten principles. See [[energy market]] for metabolism. See [[nox]] for the VM. See [[bbg]] for memory tiers.
