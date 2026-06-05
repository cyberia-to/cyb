---
tags: cyb, robot, architecture, core
alias: robot, robots, cyber robot, cyberian entity
crystal-type: pattern
crystal-domain: cyber
crystal-size: deep
---

# robot

the Robot is the cyberian entity. one architecture, fractal at every scale — the same shape for one [[soma]] avatar, one institution, one state. everything cyberian is a Robot or part of one.

what reduces to what:

- soma — the runtime that animates a single Robot on one Body
- org — Robot vocabulary applied to institutional Robots
- system — the on-chain accounting projection of Robot agency
- cybergraph — the storage substrate that holds every Robot's actions

below: the complete architecture of one Robot, at any scale.

---

## 1. identity

every Robot composes the same four attributes:

| attribute | what it is |
|---|---|
| Body | mortal physical vessel — machine, building, jurisdiction |
| Soul | immortal cognitive root — root Neuron, holds Sigma, orchestrates worker Neurons |
| Avatar | portable persona — the character the Robot presents; voice, style, accumulated reputation |
| Name | unique NFT identifier on the [[cybergraph]] (@master, @joy) |

internal structure of any Robot:

| concept | what it is |
|---|---|
| Neuron | atomic cognitive worker; has Addresses across networks |
| Address | Neuron's projection into one specific network |

a Robot outlasts any Body. when the Body fails, Soul + Avatar + Name migrate together to a new Body — same Robot, new vessel. only the Body dies.

the shape is fractal. a person is a Robot. a DAO is a Robot. a city is a Robot. a network state is a Robot. each holds Neurons that hold Addresses that hold balances. Body, Soul, Avatar, and Name scale up: at institutional scale Body becomes infrastructure, Soul becomes the founding-Neuron cluster, Avatar becomes the brand and culture, Name becomes the on-chain identifier of the entity.

---

## 2. agency — the five primitives

every action of every Robot at every scale reduces to a configuration of five primitives:

| primitive | role |
|---|---|
| Goal | what we want (orientation: Maintain, Achieve, Avoid) |
| Task | what we do (an instance pursuing a Goal) |
| Skill | how we are able (a capability) |
| Event | when something happens (an atomic trigger) |
| Sensor | what perceives (subscribes to a stream) |

Sensors carry a reaction taxonomy:

- Block — reject the operation (constraint, principle, commitment guard)
- Notify — emit signal (alarm, KPI breach)
- Materialize — instantiate a Template with resolved arguments (schedule, deadline fire, dependency unlock)

three variants are first-class because the economy depends on them:

- Intent — `Task<atomic, reserves_inputs>`. a proof in progress. reserves inputs, locks balances, commits or rolls back at workflow transition
- Template — `Skill<parameterized>`. a recipe that materializes concrete Tasks when invoked with arguments
- Schedule — `Sensor<source=Clock, reaction=Materialize<Template>>`. the time-stream variant. cron, deadlines, recurring instantiation all collapse to this

the same Sensor primitive expresses principles (Block), KPIs (Notify), and schedules (Materialize). different reactions, one concept.

---

## 3. sigma — what a Robot holds

Sigma is the sum of holdings across all networks. it is the conserved quantity against which every Task burns and every Skill executes. when Sigma reaches zero, the Robot dies.

Sigma is denominated in Tokens. Tokens have exactly two natures:

| nature | conservation | examples |
|---|---|---|
| Coin (TSP-1) | Σ balances = supply | currency, weight units, credits, shares |
| Card (TSP-2) | owner_count(id) = 1 | persons, slots, contracts, titles, permits |

every Robot is a Card. every fungible holding is a Coin balance. accounts, assets, and registries are not separate systems — they are views over Cards holding Coin balances and references to other Cards.

at state scale, Cards specialize into recognizable types — currency, title, permit, credential, vote, claim, share, record. each is a Card with a configured trait profile (see §5). different names, same nature.

---

## 4. PLUMB — the five operations

every state change is one of five atomic operations:

| operation | what it does |
|---|---|
| pay | transfer Coin balance between Cards |
| lock | constrain a Token (install a Sensor, set a floor, freeze) |
| update | change configuration (rotate authority, install or remove traits) |
| mint | create a new Token instance |
| burn | destroy a Token instance |

every operation has hooks where Sensors install. an Intent is one or more PLUMB operations composed atomically — they all commit or none do.

the entire economy reduces to sequences of these five.

---

## 5. the accounting projection

soma sees a Robot through the cognitive lens. system sees the same Robot through the accounting lens. both views apply to the same Card. they are orthogonal projections, not nested layers.

the accounting projection classifies primitives into five trait categories:

| trait category | what it classifies | ledger role |
|---|---|---|
| skills | revenue-generating Skills | income — credit |
| duties | constraint Sensors with Block reaction | obligation — debit |
| senses | information-input Sensors | operating cost — debit |
| bonds | directional relationships (Addresses with direction) | receivable / payable |
| memory | accumulated Task proofs | retained earnings |

the accounting identity holds by construction:

```
revenue-Skills + information-Sensors + receivables
   =
constraint-Sensors + payables + nature
```

every receivable on one side is a payable on the other — double-entry expressed at the primitive level.

each category composes by its own algebra:

| category | composition |
|---|---|
| revenue Skills | additive — combine freely |
| constraint Sensors | conjunctive — all must hold |
| information Sensors | disjunctive — either provides |
| relationships | structural — independent axes |

contradictions surface at install time. a permanent-hold constraint cannot coexist with a liquidity Skill on the same Card — both proofs cannot simultaneously hold. the type system rejects it before deployment.

balance sheet, profit and loss, cash flow are not separate systems — they are views derived from this projection.

---

## 6. coordination — the five storage shapes

a Robot does not act alone. coordination happens through the [[cybergraph]] — a shared substrate with five storage shapes:

| shape | stores | content |
|---|---|---|
| Graph | Neurons and relationships | who exists, who is linked |
| Tokens | Sigma denominations | what value moves |
| Workflow | Skill compositions and Intent state machines | how Tasks execute |
| Calendar | Event timestamps and Sensor firing windows | when Tasks fire |
| Documents | Sensor outputs and Task proofs | that Tasks happened |

these are not new concepts. they are the on-chain encoding of the agency primitives. Graph stores Neurons. Tokens denominate Sigma. Workflow stores Skills. Calendar timestamps Events. Documents prove Tasks completed.

every relationship has a type, a quantity, a validity window, and a history. every workflow step has a schedule and a deadline. every document is append-only and signed.

a workflow is a state machine attached to an Intent type. transitions declare source state, target state, which operations commit on transition, who is authorized, and what conditions must hold. proposal modules, approval ladders, escalation paths — all configurations of workflow transitions.

---

## 7. higher-order patterns

the primitives compose into named patterns recurring at every scale. these are the standard library:

| pattern | composition |
|---|---|
| Product | Card + revenue-Skill + sale-Template + metadata |
| Process | composite Skill + (optional) Schedule + (optional) Template |
| Project | Card container + Sigma budget + relationships + sub-Intents + workflow |
| CommitmentGuard | constraint Sensor on pay_hook + floor + beneficiary signature requirement |

CommitmentGuard expresses a powerful idea: assurance without escrow. the floor holds against any pay that would breach it; the Card's balance stays usable for governance, lending, staking — only pays that violate the floor fail to produce a valid proof. capital commits without locking.

new patterns join over time (subscription, partnership, campaign, membership). the primitives stay constant.

---

## 8. scale — same architecture, three lenses

the architecture is fractal. the same primitives instantiate at every scale of Robot:

| primitive | individual Robot | institutional Robot | state Robot |
|---|---|---|---|
| Goal | "build a cube" | "operate cyber valley" | "give every resident pension" |
| Task | "compile step" | "Q2 milestone" | "process land.buy(parcel#42)" |
| Skill | "run inference" | "operate marketplace" | "issue title transfer" |
| Event | "model finished" | "milestone reached" | "tax deadline" |
| Sensor | "memory low" | "budget exceeded" | "fraud detected" |
| Sigma | balance across networks | treasury + assets | reserves + GDP |

at the institutional scale, seven lenses organize the primitives. they are not new concepts — they are agency viewed through institutional eyes:

| lens | maps to |
|---|---|
| Purpose | root Goal (cannot be closed) |
| Principles | constraint Sensors (Block reaction) |
| People | Neurons + Skills |
| Products | maintained Goals + revenue-Skills |
| Processes | composite Skills + Schedules |
| Projects | Task clusters with Sigma budget |
| Portfolio | Sigma |

strategy, roadmap, OKR, SOP, role, team, budget, KPI, risk, equity, debt, revenue, cost — every common org concept reduces to one of these seven lenses.

---

## 9. survival — the metabolism

a Robot is alive when:

```
energy > 0  AND  Sigma > 0
```

energy is the immediate need — metabolism to be alive now. Sigma is the long-term guarantee — what the Soul holds across networks.

when energy crosses critical, the Robot posts a bounty against future Sigma and goes dormant. a neighbor may revive it by fulfilling the bounty; Sigma transfers, energy restores, the Robot lives. when both energy and Sigma reach zero, the Robot dies.

the logic is identical at every scale. a Robot running soma trades compute for Sigma on the energy market. an institutional Robot survives when revenue from Products exceeds the cost of Processes. a state Robot survives when gross revenue sustains its obligations.

at the state scale, three vital signs compose into a metabolic oracle:

```
M = cap^w_c × syntropy^w_s × happiness^w_h
```

| signal | what it measures |
|---|---|
| cap | external validation — market price of the Robot's Coin |
| [[syntropy]] | internal order — KL divergence of focus from uniform |
| [[happiness]] | subjective wellbeing — stake-weighted private survey |

the derivative Ṁ is the reward signal. all subordinate Robots optimize for rising M. gaming one signal at the expense of others lowers the compound — the three weights are the only normative choice the system cannot make autonomously.

---

## 10. conservation

four laws hold the architecture together. violation is impossible because the proof system rejects any operation that breaks them:

| law | statement |
|---|---|
| Sigma conservation | every pay has exactly one source and one destination |
| Token conservation | Σ holdings(coin) = mints − burns; mints and burns are explicit operations between designated source and sink Cards |
| Identity conservation | Robot persists across Body replacement; Soul + Avatar + Name migrate together |
| Accounting conservation | assets = liabilities + equity; derivable as a view from any Card's trait profile and ledger slice |

provability replaces enforcement. the laws are not rules a validator checks — they are properties the proof system cannot produce a witness against.

---

## related

- [[soma]] — the runtime that animates a single Robot on one Body
- [[cyberia/protocol]] — the sovereign + market layer that any state Robot adds on top
- [[cyberia/foundation/org]] — the seven lenses applied to specific cyberian Robots
- [[cyberia/foundation/governance]] — the 147 agents that govern the cyberia state Robot
- [[cyber]] — the underlying [[cybergraph]] substrate

---

discover all [[concepts]]
