---
tags: cyb, core, cybos, nox
crystal-type: entity
crystal-domain: cyber
alias: Order
---

the execution unit in [[cybos]]. input = [[cyberlink]]. output = [[cyberlink]]. memory: flat noun array, hash-consed, bulk-freed.

```
Order = {
    id:      H(formula, object),
    neuron:  NeuronId,
    formula: Noun,
    object:  Noun,
    budget:  [4]F_p,
}
```

## budget

four resource dimensions. not one-dimensional gas.

| dimension | what it bounds |
|-----------|---------------|
| compute | CPU cycles for [[nox]] reduction |
| memory | peak noun allocation |
| bandwidth | network reads/writes |
| energy | Joules consumed |

[[soma]] checks all four before accepting an Order. any exceeded → reject.

## lifecycle

```
1. Order arrives (from signal queue or trigger)
2. soma.accept_order() → check energy + 4 budgets
3. nox reduces formula(object) → execution trace
4. zheng proves trace → zheng proof
5. result = cyberlink (Order output IS a cyberlink)
6. soma.complete() → release resources, update state
```

## identity

Order id = H(formula, object). deterministic. same formula + same object = same id. this enables [[memoization]]: if axon(formula, object) exists in [[cybergraph]], skip execution.

## processes

process = chain of Orders sharing state through [[bbg]]. triggers fire the next step. no continuation in ISA.

```
Order_1 → writes state to BBG → trigger fires → Order_2 → ...
```

see [[cyb/soma]] for scheduling. see [[nox]] for execution. see [[zheng]] for proofs.
