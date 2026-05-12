---
tags: cyb, core
alias: compiled runtime, unified runtime, agent runtime
---
unified compiled [[inference]] runtime — multiple models share one physical memory space, [[KV cache]] transfers between models without copies, agents scale to thousands

## the compatibility problem

models trained independently have incompatible embedding spaces. different vocabulary, different dimension, different basis. passing context between them requires text serialization — destroying zero-copy

## compilation eliminates it

from [[cyb/compile|Theorem 1]], the embedding matrix of any compiled model equals the eigenvectors of the [[focus]] covariance Σ_φ*

all models compiled from the same [[cybergraph]] share the same Σ_φ*, therefore the same eigenvector basis. models of different sizes are truncations:

```
router  (1B):  E[:, :512]
domain  (3B):  E[:, :2048]
general (7B):  E[:, :4096]
```

nesting: 512-dim ⊂ 2048-dim ⊂ 4096-dim. this is Matryoshka embedding — exact by construction, not an approximation

no projection matrices. no cross-model translation. the compatibility problem does not exist

## physical memory layout

one unified [[cyb/hardware|cyb-mem]] pool:

```
E_full      [vocab × 4096]  f16   ← all models read their slice
φ*          [vocab × 1]     f32   ← shared read-only
system_ctx  [text + CIDs]         ← immutable, same PA for all agents
group_KV    [seq × 4096]    f16   ← smaller models read [:D] slice
personal_KV [seq × D]             ← per-agent delta
weights     per-model, demand-paged from NVMe
```

physical copies: zero. every agent reads the same bytes at the same physical addresses

## KV cache transfer

standard: KV caches are model-specific, non-transferable

compiled: KV caches inherit across depth levels

```
router_1B processes 1000 tokens → KV [1000 × 512]
domain_3B inherits:
  KV[:, :512]  = router KV   (exact, same eigenspace)
  KV[:, 512:]  = zeros
  continues with no context loss
```

context never restarts. larger model inherits smaller model's work in correct representation

## dynamic depth

one agent escalates model depth mid-[[inference]] based on confidence:

```
token arrives → router_1B (AMX, D=512)
  confidence > 0.9 → answer directly
  confidence < 0.9 → escalate
    → domain_3B (Metal, D=2048), inherits KV
      confidence > 0.85 → answer
      confidence < 0.85 → escalate
        → general_7B (Metal+NVMe, D=4096), inherits KV
```

cost scales exactly with task complexity. simple tokens pay router cost. complex tokens pay full model cost. no restarts, no copies, no context loss between depth levels

## bandwidth amortization

standard: N agents × full model weights = N × bandwidth demand

unified: weights read once into L3/SLC cache → N agents read from cache

```
standard:   N agents × 14GB = N × 14GB bandwidth
unified:    14GB once → cache → N agents from cache
            bandwidth ≈ constant up to cache capacity
```

1000 concurrent agents on M1 Pro 32GB with 7B Q4 model: 22× bandwidth efficiency vs standard multi-process inference. the system becomes more efficient as agents increase — the opposite of standard [[inference]]

## agent as a pointer

```rust
struct Agent {
    id:           CID,
    model_depth:  usize,
    kv_slice:     PhysPageSlice,
    personal_kv:  PhysPage,
    goal:         GoalNodeRef,
}
```

five fields. no model-specific state. no context copies. the agent is a view into shared physical memory. memory cost: O(N × personal_context_size), not O(N × model_size)

## layered context

four context layers, each physically realized as shared [[cyb/hardware|cyb-mem]] pages:

| layer | scope | sharing |
|-------|-------|---------|
| system | one per machine | hardware profile, available models, global constraints |
| group | one per agent group | shared goal tree, collective [[memory]], KL divergence tracking |
| personal | one per agent | current task, last observations, output format |
| KV cache | one per agent | grows with [[inference]] |

## goal tree

three levels drive agent behavior:

terminal goals: never change — maximize [[cybergraph]] [[knowledge]] quality, minimize KL(φ*_human ∥ φ*_ai)

instrumental goals: change per task — extract implicit [[cyberlinks]] from [[inference]], verify claims before staking

immediate goals: current step — analyze subgraph diameter of domain X

all three levels visible simultaneously in [[cyb/context|context]]. optimization of immediate goals at the expense of terminal goals is the canonical failure mode

see [[cyb/hardware]] for the physical memory stack. see [[cyb/compile]] for why compilation produces compatible eigenspaces. see [[pipeline]] for the full .cyb cycle

discover all [[concepts]]
