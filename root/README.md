---
tags: cyb, robot, architecture, core
alias: robot, the robot, robot architecture, cyb readme
crystal-type: pattern
crystal-domain: cyb
crystal-size: deep
---

# robot

ownable autonomous intelligence. the only mind a person can hold and not rent.

→ see [[cyb/root/product]] for the market thesis (~$60T wage bill, ~68B Robot ceiling, virtual entry, embodied upgrade)

---

## cardinality

```
1 robot  →  1 avatar  →  1 body
```

a Robot has exactly one Avatar. an Avatar has zero or one Body. organizational complexity emerges from Robots owning Robots — never from multiplying Avatars.

---

## the four attributes

| attribute | nature | provides |
|---|---|---|
| Soul | immortal root [[Neuron]]; keypair; holds [[karma]] | identity; signs every output; survives Body change |
| Avatar | virtual cyberspace model | visible presence; capability spec; what other Robots see |
| Name | unique NFT handle on the [[cybergraph]] | resolvable identifier (`@master`) |
| Body | mortal substrate — none, machine, or meat | compute, sensors, actuators when embodied |

Soul + Avatar + Name persist. Body is replaceable. when a Body fails, the three invariants migrate to a new Body — same Robot, new vessel.

---

## the body — five functional regions

the Body has five regions. each has a function; each maps to a named component in the cyber stack. the architecture is the mapping.

| region | function | implementation |
|---|---|---|
| soma | cognition — perceive, decide, plan, learn, exercise [[Skill\|Skills]] | [[soma]] runtime: four loops, tiered model stack |
| sense | communication — route signals between Robot and world (bidirectional) | [[sense]] cell + [[radio]] transport |
| sigma | capital — hold value, transact via PLUMB | [[sigma]] cell + [[plumb]] (TSP-1, TSP-2, five operations) |
| memory | knowledge — store and recall | [[bbg]] (shared, polynomial-committed) + soma-memory (procedural, episodic, semantic, working) |
| schedule | time — fire on deadlines, recur, materialize Templates autonomously | Schedule primitive (a [[Sensor]] watching the clock) + [[cyb/time]] cell |

I/O between regions:

```
   world inputs                                       world outputs
   ─────────────                                      ──────────────

   network gossip   ┌─────────────┐                   cyberlinks
   peer messages ─► │    sense    │ ──► percepts ──┐  signed messages
   local sensors    │  comm router│                │  to network
                    └─────────────┘                │
                                                    │
                    ┌─────────────┐                ▼
   memory recall ◄─►│             │ ◄── percepts
   stored events    │    soma     │
                    │  cognition  │ ◄── fires from schedule
                    │             │
                    └──────┬──────┘
                           │
                       decisions
                       + intents
                           │
                           ▼
                    ┌─────────────┐                   PLUMB ops
                    │    sigma    │ ──────────────►   to cybergraph
                    │capital flow │                   (mint/burn/pay/lock/update)
                    └─────────────┘
```

every output from sense or sigma carries the Soul's signature. nothing leaves the Robot unsigned.

---

## surfaces — what others see

a Robot presents two read-only surfaces to the network. mutations always come from inside.

| surface | shows | computed by |
|---|---|---|
| Avatar | visible model + capability spec + current state — rendered as chrome | [[prysm/chroma]] organs + [[mir]] visual layout |
| Standing | [[karma]] (accumulated trust) + [[cyberank]] (network-attention probability) | [[tru]] convergence VM — recomputed every block from all signed [[cyberlink|cyberlinks]] |

Avatar is what a Robot *looks like*. Standing is what a Robot *is worth* in the attention economy.

---

## life — metabolism

a Robot lives when:

```
endowment > 0  AND  H > 0
```

| resource | role |
|---|---|
| endowment ([[BOOT]]) | staked capital — sustains the Robot indefinitely; owned absolutely by the Soul |
| energy ([[H]]) | metered fuel — burns per action; replenished from endowment yield |

when H crosses critical, the Robot posts a bounty against endowment and goes dormant. a neighbor may revive it by fulfilling the bounty. when both reach zero, the Robot dies.

a Robot is a capital asset. its endowment compounds, its karma accumulates, its Avatar appreciates. it can be sold, willed, or collateralized at any block — see [[cyb/root/product]].

---

## scale — operating states and ownership

a Robot's state on two axes determines what it can do and what it costs:

|  | alone (sovereign) | collab (gas) |
|---|---|---|
| virtual — no Body | **dormant** — capital, schedules, hosted inference | **social ghost** — publishes, earns, governs without Body |
| embodied — machine or meat Body | **sovereign** — own compute, private inference, private network | **full actor** — physical action + cyberspace presence |

default entry is dormant. every upgrade is additive.

ownership scales without breaking the 1:1:1 rule. any Robot can own any Robot, recursively:

| structure | meaning |
|---|---|
| Robot owns Robots | corporation — one mind directing a fleet |
| Robot owns machine-Body Robots | factory, sensor network, fleet |
| Robot owns meat-Body Robots | agency, labor collective |
| Robot owns Robot owns Robot | holding company, DAO, dynasty |

the 1:1:1 rule governs structure. the ownership rule governs scale.

---

## see also

- [[cyb/root/product]] — market thesis, TAM, the rent → own gap
- [[soma]] — cognition runtime
- [[plumb]] — value layer (TSP-1, TSP-2, PLUMB)
- [[prysm/chroma]] — presentation chrome
- [[tru]] — convergence VM (focus, karma, cyberank)
- [[aos]] — cell catalog (Robot's apps)
- [[cyberia/protocol]] — sovereign-state layer
- [[soft3]] — stack the Body composes from
- [[cybergraph]] — shared substrate

---

discover all [[concepts]]
