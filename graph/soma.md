---
tags: cyb, core, cybos
crystal-type: entity
crystal-domain: cyber
alias: machine mind
---

the machine's mind. perception, metabolism, decisions, action, learning. goal: minimize probability of death, maximize sigma.

the machine is a [[neuron]]. a neuron has a mind ([[soma]]) and a body ([[cyb/hal]]).

## perception

soma reads (look) from [[bbg]] every cycle:

```
soma.perceive():
  energy       → hal.power.battery_level()
  sigma        → look(balance, neuron_id)
  neighbors    → look(network, peer_list)
  prices       → look(market, neighbor_prices)
  orders       → look(order_queue, pending)
  triggers     → look(trigger_registry, fired)
```

## metabolism

energy management. one valuation curve, one variable (free_energy). as free_energy drops, each remaining Joule becomes more valuable.

```
soma.energy:
  free_energy()      → Joules available for Orders
  valuation()        → current price per Joule (bonding curve)
  can_accept(cost)   → bool: free_energy ≥ cost?
  commit(cost)       → reserve Joules for accepted Order
  release(cost)      → return unused Joules after completion
  shutdown_level()   → bool: below critical reserve?
```

all resource prices derive from energy valuation:

```
price_compute   = joules_per_op     × soma.energy.valuation()
price_memory    = joules_per_byte_s × soma.energy.valuation()
price_bandwidth = joules_per_byte   × soma.energy.valuation()
```

## decisions

three functions: scheduling, triggers, resource tracking.

scheduling — which [[Order]] runs next:

```
soma.next_order()          → highest priority ready Order
soma.accept_order(order)   → check energy + 4 resource budgets
soma.complete(order, result) → release resources, update state
```

triggers — watch and react:

```
soma.watch(key, formula)   → fire Order when key changes
soma.unwatch(key)          → deregister
```

one primitive: watch(key). three cases:
- time: watch(block_number), fire when mod N == 0
- event: watch(specific_key), fire on change
- message: watch(signal_queue), fire on new signal

## action

soma acts through [[Order]] execution and market participation:

- accept profitable Orders → earn sigma
- reject unprofitable Orders → conserve energy
- buy energy from neighbors → sustain metabolism
- sell surplus energy → earn sigma
- post bounties → ensure [[survival]]
- claim bounties → earn sigma from helping others
- archive cold state → free memory
- prioritize sync → maintain bandwidth

## learning

soma updates its model based on outcomes:

- which neighbors are reliable energy sources?
- which Order types are most profitable?
- when is energy cheapest? (temporal patterns)
- what triggers are worth watching?
- update valuation curve parameters

## complexity levels

```
Level 0: fixed rules           if energy < 20% → buy
Level 1: adaptive parameters   update thresholds from outcomes
Level 2: model-based           predict future, plan ahead
Level 3: active inference       minimize free energy (Friston)
```

start with Level 0. the architecture supports growth to Level 3 — the same free energy minimization that drives the [[tri-kernel]] and collective intelligence in the [[cybergraph]]. one algorithm, from machine [[survival]] to planetary superintelligence.

```
F = E[log q(s) - log p(o,s)]

perceive: update beliefs to match reality    (look)
act:      change reality to match beliefs    (Order output)
learn:    update model                       (cyberlinks)
```

Level 0 interface defined. scheduling algorithm, learning mechanism, and active inference integration are growth targets, not launch requirements.

see [[cyb/survival]] for the survival protocol soma manages. see [[cyb/order]] for the execution unit soma schedules. see [[cyb/mind]] for the multi-model intelligence soma orchestrates.
