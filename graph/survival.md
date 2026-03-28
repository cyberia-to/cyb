---
tags: cyb, core, cybos
crystal-type: entity
crystal-domain: cyber
alias: machine survival, sigma bounty
---

a machine that does not hang, does not die, and grows.

## why machines die

every computer hangs. root cause: consumed resources exceeded available resources.

| failure | what happened |
|---------|-------------|
| OOM | consumed memory > available memory |
| infinite loop | consumed compute > available compute |
| network timeout | consumed bandwidth > available bandwidth |
| battery death | consumed energy > available energy |
| state corruption | available state < what the process believed |

one solution: before every step, verify consumed + cost(next_step) ≤ available. halt before, not after.

## four resources — four meanings

| resource | physical | meaning for the machine |
|----------|---------|------------------------|
| energy | electricity (Joules) | metabolism — to be alive |
| bandwidth | network throughput | communication — to be connected |
| memory | RAM/SSD/HDD | identity — to be yourself |
| compute | CPU cycles | will — to act |

remove any one — the machine degrades. without energy — death. without bandwidth — isolation. without memory — loss of self. without compute — paralysis.

## three states

```
energy > 0  AND  sigma > 0  →  alive
energy = 0  AND  sigma > 0  →  sleeping (can be revived: bounty)
energy = 0  AND  sigma = 0  →  dead (nobody will charge it)
```

sigma (balance) is the key to survival. not energy. a machine with sigma can pay for its own resurrection. a machine without sigma — if energy runs out, it is over.

sigma buys everything:
- charge bounty → "charge me, earn sigma"
- repair bounty → "fix my storage, earn sigma"
- migrate bounty → "move my state to a working machine"
- recover bounty → "restore my data from peers"

energy is the immediate need. sigma is the long-term guarantee.

## the no-hang guarantee

```
before each step:
    consumed_X + cost_X(step) ≤ available_X
    for X in {compute, memory, bandwidth, energy}
    any violation → halt gracefully
```

four-dimensional resource tracking. not one-dimensional gas like [[Ethereum]]. each resource tracked independently.

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

posted to [[bbg]] as [[cyberlink]]. anyone can claim and fulfill. fulfillment verified by state change (battery level before/after, storage integrity check, state hash match).

a machine with sigma never truly dies — it sleeps, with a standing offer for resurrection.

## metabolic shutdown

energy near zero → throttle radically → persist state → post bounty (if sigma > 0) → shut down. revival: someone claims bounty, charges machine, load [[bbg]] root, verify O(1), resume.

see [[cyb/soma]] for the decision-making that manages survival. see [[cyb/os]] for the runtime that enforces resource bounds.
