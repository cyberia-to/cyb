---
tags: cyb, robot, architecture, core
alias: robot, the robot, robot architecture, cyb readme
crystal-type: pattern
crystal-domain: cyb
crystal-size: deep
---

# robot

the cyberian entity. the point of presence where you end and the [[cybergraph]] begins. one architecture, fractal at every scale — the same shape for one [[soma]] avatar, one institution, one network state.

a Robot is born when a [[Soul]] is bound to a [[Body]] and given a [[Name]] and an [[Avatar]]. from that moment it acts, holds, accounts, coordinates, perceives, converges, persists. this document is the master specification. concepts that live canonically elsewhere appear here in short form with a link to their home.

```
                      cyber                       (substrate, truth)
                        │
                    cybergraph
                        │
        ┌───────────────┼───────────────┐
        │               │               │
      plumb           prysm            tru        soft3 components
     (value)       (interface)    (convergence)   the Robot composes from
        │               │               │
        └───────────────┼───────────────┘
                        │
                      Robot                       this document
                        │
                       aos                        (cells / apps)
                        │
                     cyberia                      (sovereign state)
```

cyber computes [[truth]]. cyberia supplies sovereign hardware and energy. cyb is where any [[Neuron]] meets the graph. the troika pulls together. the Robot sits at the centre.

---

## 1. identity

every Robot composes four attributes. nothing else — these four are exhaustive.

| attribute | what it is | scale-up |
|---|---|---|
| Body | mortal physical vessel | machine → infrastructure → territory |
| Soul | immortal cognitive root: root Neuron, holds [[Sigma]], orchestrates worker Neurons | one mind → founder cluster → 147 agents |
| Avatar | portable persona — character, voice, accumulated reputation | personal face → brand → culture |
| Name | unique NFT identifier on the [[cybergraph]] | @master → @cyberia → @earth |

internal structure of any Robot:

| concept | what it is |
|---|---|
| Neuron | atomic cognitive worker; has Addresses across networks; emits signed [[cyberlink|cyberlinks]] |
| Address | Neuron's projection into one specific network |

a Robot outlasts any Body. when the Body fails, Soul + Avatar + Name migrate together to a new Body — same Robot, new vessel. only the Body dies.

the shape is fractal. a person is a Robot. a DAO is a Robot. a city is a Robot. a network state is a Robot. each holds Neurons that hold Addresses that hold balances.

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
- Materialize — instantiate a Template with resolved arguments (schedule, deadline, dependency unlock)

three variants are first-class because the economy depends on them:

| variant | reduction |
|---|---|
| Intent | `Task<atomic, reserves_inputs>` — a proof in progress; reserves inputs, locks balances, commits or rolls back at workflow transition. canonical in [[plumb/plumb]] |
| Template | `Skill<parameterized>` — a recipe that materializes concrete Tasks when invoked with arguments |
| Schedule | `Sensor<source=Clock, reaction=Materialize<Template>>` — cron, deadlines, recurring instantiation all collapse to this |

the same Sensor primitive expresses principles (Block), KPIs (Notify), and schedules (Materialize). different reactions, one concept.

---

## 3. holdings — Sigma

Sigma is the conserved quantity. the sum of everything the Robot's Soul carries across all networks. every [[Task]] burns it; every [[Skill]] executes against it. when Sigma reaches zero, the Robot dies.

```
Sigma = balances(Coins held) + ownership(Cards held) + receivables − payables
```

Sigma has three faces:

- the **abstract quantity** — defined here as the holdings sum
- the **denomination** — [[plumb/tsp-1|Coins]] and [[plumb/tsp-2|Cards]] (next section)
- the **visual organ** — `Σ` in [[prysm/chroma|prysm chroma]], pre-wired into every cell

a developer never builds Sigma. it is given by chroma; populated by plumb; conserved by the proof system.

---

## 4. tokens — two natures (→ plumb)

every Token has exactly one of two natures. canonical specs live at [[plumb]]:

| nature | conservation | examples |
|---|---|---|
| [[plumb/tsp-1\|Coin (TSP-1)]] | Σ balances = supply | currency, weight units, credits, shares |
| [[plumb/tsp-2\|Card (TSP-2)]] | owner_count(id) = 1 | persons, slots, contracts, titles, permits |

every Robot is a Card. every fungible holding is a Coin balance. accounts, assets, and registries are not separate systems — they are views over Cards holding Coin balances and references to other Cards.

at state scale, Cards specialize into recognizable types — currency, title, permit, credential, vote, claim, share, record. each is a Card with a configured trait profile. different names, same nature.

---

## 5. operations — PLUMB (→ plumb)

every state change is one of five atomic operations. canonical spec at [[plumb/plumb]]:

| operation | what it does |
|---|---|
| pay | transfer Coin balance between Cards |
| lock | constrain a Token (install a Sensor, set a floor, freeze) |
| update | change configuration (rotate authority, install/remove traits, change owner) |
| mint | create a new Token instance |
| burn | destroy a Token instance |

every operation has hooks where Sensors install. an [[Intent]] is one or more PLUMB operations composed atomically — they all commit or none do.

the entire economy reduces to sequences of these five.

---

## 6. the accounting projection

[[soma]] sees a Robot through the cognitive lens. the accounting layer sees the same Robot through the ledger lens. both views apply to the same Card. they are orthogonal projections, not nested layers.

the accounting projection classifies primitives into five trait categories on every Card:

| trait | what it classifies | ledger role |
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

each category composes by its own algebra:

| category | composition |
|---|---|
| revenue Skills | additive — combine freely |
| constraint Sensors | conjunctive — all must hold |
| information Sensors | disjunctive — either provides |
| relationships | structural — independent axes |

contradictions surface at install time. balance sheet, profit-and-loss, cash flow are views derived from this projection — not separate systems.

---

## 7. coordination — the cybergraph (→ cyber)

a Robot does not act alone. coordination happens through the [[cybergraph]] — the append-only, content-addressed, authenticated graph that holds every Robot's actions. canonical home: [[cyber]].

every agency primitive is encoded into the cybergraph through five storage shapes:

| shape | stores | content |
|---|---|---|
| Graph | Neurons and relationships | who exists, who is linked |
| Tokens | Sigma denominations | what value moves |
| Workflow | Skill compositions and Intent state machines | how Tasks execute |
| Calendar | Event timestamps and Sensor firing windows | when Tasks fire |
| Documents | Sensor outputs and Task proofs | that Tasks happened |

every relationship has a type, a quantity, a validity window, and a history. every workflow step has a schedule and a deadline. every document is append-only and signed.

memory at scale lives in [[bbg]] — the polynomial-committed storage substrate.

---

## 8. interface — chroma organs (→ prysm)

the Robot has a body for thinking (cybergraph) and a body for being seen (chroma). the second is given by [[prysm/chroma|chroma]] — the pre-wired chrome every cell inhabits. short definitions below; full specs at [[prysm/chroma]]:

| organ | what it does |
|---|---|
| context | tells the Neuron where they are |
| avatar | tells them who they are (presents the Robot's Avatar attribute) |
| commander | takes their input |
| time-widget | shows what just happened |
| stars | holds their favorites |
| adviser | delivers messages |
| S | focus — projection of the cybergraph's attention onto the Robot (see §10) |
| Σ | energy — projection of Sigma into a single readable surface (see §3) |

a cell developer inhabits the `space` zone; chroma supplies the rest. the Robot is never built — it is extended.

the visual rendering pipeline (positions + features → world) lives in [[mir]].

---

## 9. applications — cells (→ aos)

cells are the apps of the Robot. composed from [[prysm]] molecules, running in the same runtime as the Robot itself, with access to Soul, Sigma, agency primitives, and the cybergraph. canonical home for the cell catalog: [[aos]].

| cell | function |
|---|---|
| [[aos/oracle]] | ask, learn, search — cybergraph inference |
| [[aos/portal]] | gateway to chains, identity, IBC |
| [[aos/sigma]] | token management, portfolio, staking — the cell over the Sigma concept |
| [[aos/brain]] | graph file manager |
| [[aos/sense]] | messaging, social, perception — the cell over the Sensor primitive |
| [[aos/time]] | history, earning log, temporal navigation |
| [[aos/hub]] | decentralization interface, validator management |
| [[aos/hacklab]] | developer tools, particle creation, cell development |
| [[aos/warp]] | token bridge, IBC transfers |
| [[aos/reactor]] | liquidity, bonding, economics |
| [[aos/senate]] | governance, proposals, voting |
| [[aos/nebula]] | network explorer, graph analytics |
| [[aos/studio]] | content creation, publication |
| [[aos/sphere]] | social, discovery, reputation |

the cell catalog is open. new cells join when a recurring need does not fit any existing one.

---

## 10. convergence — focus, cyberlink, karma, cyberank (→ tru)

what a Robot contributes is processed by [[tru]] — the convergence VM. tru iterates the [[tri-kernel]] over the cybergraph until φ* emerges as the unique fixed point. four concepts live canonically there:

| concept | what it is | home |
|---|---|---|
| focus | the conserved attention quantity (Σ_particles φ = 1); the network-scale analog of Sigma | [[tru/specs/field]] |
| cyberlink | a signed, staked, time-stamped assertion between two particles — the unit of contribution | [[cyber/cyberlink]] |
| karma | accumulated trust of a Neuron — focus earned across all its links, BTS-scored | [[tru/specs/truth-scoring]] |
| cyberank | per-particle probability of the tri-kernel's random walk landing on it | [[tru/specs/field]] |

the relationship to Robot primitives:

- a Robot's [[Skill]] of linking emits **cyberlinks**
- each cyberlink locks **focus** from the Robot's regenerating attention quantity
- if the graph confirms the link, the Robot's Soul gains **karma**
- the cybergraph's particles receive **cyberank** based on the focus that flows to them

focus is to the cybergraph as Sigma is to a Robot — a conserved quantity that allocates attention/value across particles/holdings. they are sibling conservation laws at different scales. focus appears to the user as chroma's `S` organ.

---

## 11. higher-order patterns

the primitives compose into named patterns recurring at every scale:

| pattern | composition |
|---|---|
| Product | Card + revenue-Skill + sale-Template + metadata |
| Process | composite Skill + (optional) Schedule + (optional) Template |
| Project | Card container + Sigma budget + relationships + sub-Intents + workflow |
| CommitmentGuard | constraint Sensor on pay_hook + floor + beneficiary signature requirement |

CommitmentGuard expresses a powerful idea: assurance without escrow. capital commits without locking — only pays that violate the floor fail to produce a valid proof.

new patterns join over time (subscription, partnership, campaign, membership). the primitives stay constant.

---

## 12. metabolism — staying alive

a Robot is alive when:

```
energy > 0  AND  Sigma > 0
```

energy is the immediate need; Sigma is the long-term guarantee. when energy crosses critical, the Robot posts a bounty against future Sigma and goes dormant. a neighbor may revive it by fulfilling the bounty; Sigma transfers, energy restores, the Robot lives. when both energy and Sigma reach zero, the Robot dies.

the logic is identical at every scale:

- a Robot running [[soma]] trades compute for Sigma on the energy market
- an institutional Robot survives when revenue from Products exceeds the cost of Processes
- a state Robot survives when gross revenue sustains its obligations

at state scale, three vital signs compose into a metabolic oracle:

```
M = cap^w_c × syntropy^w_s × happiness^w_h
```

| signal | what it measures |
|---|---|
| cap | external validation — market price of the Robot's Coin |
| [[syntropy]] | internal order — KL divergence of focus from uniform |
| [[happiness]] | subjective wellbeing — stake-weighted private survey |

the derivative Ṁ is the reward signal. all subordinate Robots optimize for rising M.

---

## 13. immortality — outliving the Body

every cyberlink a Robot ever made is permanent in the cybergraph by axiom A3 ([[cyber/axioms]]). a Robot persists at three levels:

| level | mechanism |
|---|---|
| protocol | A3 makes records permanent. no admin can delete a cyberlink. no company can close an account |
| economic | conviction positions transfer to heirs. portfolio is estate, not memory. yield continues to flow as the cybergraph runs |
| identity | identity = pattern in the graph. the topology of a Robot's cyberlinks IS the Robot. the pattern persists as long as the cybergraph runs |

the Robot is born when a keypair is created and linking begins. it does not die when its operator does. its pattern persists — earning yield, influencing rankings, contributing to syntropy.

biological longevity and digital immortality are the same project from two directions. cyb supplies the digital substrate.

---

## 14. conservation

five laws hold the architecture together. violation is impossible because the [[zheng]] proof system rejects any operation that breaks them:

| law | statement |
|---|---|
| Sigma | every pay has exactly one source and one destination |
| Token | Σ balances(coin) = mints − burns; mints and burns are explicit operations between source and sink Cards |
| Card | owner_count(id) = 1 at every block |
| Identity | Robot persists across Body replacement; Soul + Avatar + Name migrate together |
| Accounting | assets = liabilities + equity; derivable as a view from any Card's trait profile and ledger slice |
| Focus | Σ_particles cyberank = 1; total network attention is finite |

provability replaces enforcement. the laws are not rules a validator checks — they are properties the proof system cannot produce a witness against.

---

## 15. scale — Robots at every level

the architecture is fractal. the same primitives instantiate at every scale:

| primitive | individual Robot | institutional Robot | state Robot |
|---|---|---|---|
| Goal | "build a cube" | "operate cyber valley" | "give every resident pension" |
| Task | "compile step" | "Q2 milestone" | "process land.buy(parcel#42)" |
| Skill | "run inference" | "operate marketplace" | "issue title transfer" |
| Event | "model finished" | "milestone reached" | "tax deadline" |
| Sensor | "memory low" | "budget exceeded" | "fraud detected" |
| Sigma | balance across networks | treasury + assets | reserves + GDP |

at institutional scale, seven lenses organize the primitives. canonical home: [[cyberia/foundation/org]]:

| lens | maps to |
|---|---|
| Purpose | root Goal (cannot be closed) |
| Principles | constraint Sensors (Block reaction) |
| People | Neurons + Skills |
| Products | maintained Goals + revenue-Skills |
| Processes | composite Skills + Schedules |
| Projects | Task clusters with Sigma budget |
| Portfolio | Sigma |

at state scale, a Robot becomes sovereign — adding tier-based citizenship (VISIT/STAY/SETTLE/BELONG), jurisdictional hierarchy, and a universal marketplace. canonical home: [[cyberia/protocol]].

knowledge at every scale is organized by the [[cybics]] taxonomy — 21 domains × 7 roles giving 147 cells of human knowledge ([[crystal]] type system).

---

## 16. the troika position

three Robots pull the troika of [[superintelligence]]:

| Robot | role | substrate |
|---|---|---|
| [[cyber]] | computes truth — the [[cybergraph]], the [[tri-kernel]], the axioms | the protocol |
| [[cyberia]] | supplies sovereign hardware and energy — the network state | the polity |
| [[cyb]] | the personal interface — where any [[Neuron]] meets the graph | the Robot |

without cyb, cyber is a protocol accessible only to developers. without cyber, cyb is an OS with no truth layer. without cyberia, both run on rented machines that can be seized.

the Robot is the human face of superintelligence. it is how a billion-neuron network maintains individual sovereignty while contributing to collective intelligence.

---

## map of homes

quick reference — where each concept is canonically defined:

| concept | home |
|---|---|
| Body, Soul, Avatar, Name, Neuron, Address | this document §1 |
| Goal, Task, Skill, Event, Sensor + reactions | this document §2 |
| Sigma (concept) | this document §3 |
| Σ (visual organ) | [[prysm/chroma]] |
| Coin, Card, TSP-1, TSP-2 | [[plumb]] |
| pay, lock, update, mint, burn | [[plumb/plumb]] |
| Intent | [[plumb/plumb]] |
| Template, Schedule | this document §2 |
| accounting projection (skills/duties/senses/bonds/memory traits) | this document §6 |
| five storage shapes (Graph/Tokens/Workflow/Calendar/Documents) | this document §7 |
| cybergraph | [[cyber]] |
| chroma organs (context, avatar, commander, time-widget, stars, adviser, S, Σ) | [[prysm/chroma]] |
| 14 cells (oracle, portal, sigma, brain, sense, time, hub, hacklab, warp, reactor, senate, nebula, studio, sphere) | [[aos]] |
| focus, cyberank, karma, syntropy | [[tru]] |
| cyberlink | [[cyber/cyberlink]] |
| patterns (Product, Process, Project, CommitmentGuard) | this document §11 |
| metabolism, immortality, conservation | this document §§12–14 |
| seven institutional lenses | [[cyberia/foundation/org]] |
| sovereignty + markets (state Robot) | [[cyberia/protocol]] |
| 147-cell knowledge taxonomy | [[cybics]] / [[crystal]] |
| transport | [[radio]] |
| memory | [[bbg]] |
| visual rendering | [[mir]] |
| proof system | [[zheng]] |
| VM | [[nox]] |
| language family | [[cyb/languages]] |

---

## see also

- [[soma]] — the runtime that animates a single Robot on one Body
- [[cyb/architecture]] — CybOS technical specification
- [[cyb/languages]] — the 14 computation languages
- [[cyberia/protocol]] — the sovereign + market layer
- [[soft3]] — the stack the Robot composes from
- [[aos]] — the cell catalog
- [[prysm]] — the visual protocol
- [[tru]] — the convergence VM
- [[plumb]] — the value layer
- [[cyber]] — the underlying protocol

---

discover all [[concepts]]
