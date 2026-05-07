---
tags: cyber, cyb, draft, research
crystal-type: entity
crystal-domain: cyber
alias: CyberWasm, CyberRS, cyberwasm
---
# CyberWasm

A byte machine for sovereign computing.

---

## The Problem

[[cosmwasm]] is a transaction processor pretending to be a computing environment. Programs cannot live in memory. Programs cannot wake themselves up. Programs pay for abstractions they never asked for. Every call crosses a Go→CGo→Rust→Wasm bridge with JSON serialization at every boundary. The result is a system where conventional programs cannot run.

CyberWasm is the replacement. Not a module bolted onto a Go chain — a full Rust ABCI application. No Go. No bridge. No JSON. A machine where a HashMap is storage, a function call is a message, and a timer is a cron job.

The design is driven by six requirements. Every architectural decision in this document traces back to one of them.

---

## Six Requirements

```
1. Persistent processes     programs that live, maintain state in memory,
                            and can be suspended/resumed

2. Self-scheduling          programs that can trigger their own execution
                            without external relayers

3. Compact program overhead minimal per-program resource footprint so you
                            can run thousands of small programs

4. Sovereignty + IBC        own chain, interoperability with Cosmos ecosystem

5. Clean billing            storage rent (not one-time write cost), compute
                            metering that reflects reality

6. Minimal layering         as few abstraction layers as possible between
                            the program and the hardware
```

Everything below is organized around these six. Each section opens by naming which requirement it serves, and closes by showing what it replaces from CosmWasm.

---

## System Architecture: Two Tiers, One Consensus

This section establishes the system-level architecture that satisfies all six requirements simultaneously. The key insight: different computations have different optimal execution environments. Forcing everything through one VM is a compromise. Multiple execution engines sharing one consensus is the honest architecture.

### ABCI is 1:1

ABCI is a 1:1 protocol. One CometBFT instance connects to exactly one application. The reason is fundamental: CometBFT produces a single linear chain of blocks. Each block produces a single deterministic state transition. `Commit` returns one app hash — one root. There is no room in this protocol for multiple applications producing separate roots.

The [[cosmos-sdk]] solved this with modules — `x/bank`, `x/staking`, `x/wasm` are all different compute engines inside one process (`BaseApp`). But the SDK treats them as tightly coupled namespaces within a monolithic Go framework.

CyberWasm takes the same approach but cleaner: genuinely independent execution engines sharing a consensus layer, coordinated by a multiplexer.

### The multiplexer

The multiplexer IS the ABCI application. It routes transactions to execution engines, coordinates their state, and produces one Merkle root at commit. From CometBFT's perspective, nothing changed — it talks to one app and gets one hash back.

```
CometBFT → ABCI → Multiplexer (single Rust binary)
                     │
                     ├── CyberWasm Machine (Tier 2)
                     │   User-deployed persistent actor Wasm programs
                     │   Sandboxed, gas-metered, permissionless
                     │
                     ├── Graph Machine (Tier 1, native)
                     │   Cyberlink storage and rank computation
                     │   SIMD, multithreaded, memory-mapped
                     │
                     ├── Bank Machine (Tier 1, shared)
                     │   Token balances, transfers, minting
                     │   Callable from all machines
                     │
                     ├── Staking Machine (Tier 1, native)
                     │   Validators, delegation, slashing
                     │
                     ├── IBC Machine (Tier 1, shared)
                     │   Channels, packets, light clients (ibc-rs)
                     │   Any machine can send/receive IBC packets
                     │
                     └── [Future: EVM, ZK, AI Inference machines]
```

### Two execution tiers

Native Rust code (graph engine, bank, [[staking]]) runs alongside [[wasm]] programs. These are not the same thing. They have different trust models, capabilities, deployment mechanisms, and performance characteristics. They are two tiers of the same machine.

```
┌─────────────────────────────────────────────────────────────┐
│                    CometBFT (consensus)                      │
└──────────────────────────┬──────────────────────────────────┘
                           │ ABCI
┌──────────────────────────┴──────────────────────────────────┐
│                                                              │
│  TIER 1: Kernel — Native Rust Binaries (CyberRS)            │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌───────┐     │
│  │ Graph  │ │  Bank  │ │Staking │ │  IBC   │ │CyberW.│     │
│  │Engine  │ │        │ │        │ │        │ │Runtime│     │
│  └────────┘ └────────┘ └────────┘ └────────┘ └───┬───┘     │
│                                                   │         │
│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┼ ─ ─     │
│                                                   │         │
│  TIER 2: Userspace — Wasm Programs                │         │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐        │         │
│  │Prog │ │Prog │ │Prog │ │Prog │ │Prog │   ◄────┘         │
│  │  A  │ │  B  │ │  C  │ │  D  │ │ ... │                   │
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘                   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

This is an operating system. Tier 1 is the kernel. Tier 2 is userspace. The CyberWasm runtime itself is a Tier 1 component — the kernel's process scheduler and memory manager for Tier 2 programs.

### Tier comparison

| Property | Tier 1 (CyberRS) | Tier 2 (CyberWasm) |
|---|---|---|
| **Trust** | Trusted — bugs can halt/corrupt chain | Untrusted — sandbox prevents harm |
| **Deployment** | Compiled into binary, governance-upgraded | Permissionless, anyone deploys |
| **Execution** | Native speed, no sandbox | Wasm speed (near-native, sandboxed) |
| **State access** | Direct read/write to state store | Own persistent memory pages only |
| **Capabilities** | Unrestricted — threads, SIMD, mmap | ~50 host functions, nothing else |
| **Determinism** | Enforced by CyberRS lint discipline | Enforced by Wasm sandbox |
| **Metering** | No gas metering (system cost) | Compute + rent + bandwidth |
| **Upgrade** | Binary upgrade via governance | Deployer submits new code, instant |
| **Who writes** | Chain developers, audited | Anyone — devs, DAOs, users, AI agents |

### The Machine trait

Every Tier 1 component implements the `Machine` trait:

```rust
trait Machine {
    /// Deterministic state transition. CyberRS rules enforced.
    fn handle(&mut self, ctx: &BlockContext, msg: &[u8]) -> Result<Vec<u8>>;

    /// Produce Merkle root of this machine's state.
    fn commit(&self) -> Hash;

    /// Non-deterministic read-only query (not part of consensus).
    fn query(&self, path: &str, data: &[u8]) -> Result<Vec<u8>>;

    /// Called at the start of each block before transactions.
    fn begin_block(&mut self, ctx: &BlockContext) {}

    /// Called at the end of each block after all transactions.
    fn end_block(&mut self, ctx: &BlockContext) -> Vec<ValidatorUpdate> { vec![] }

    /// State export for genesis or snapshot.
    fn export(&self) -> Vec<u8>;

    /// State import from genesis or snapshot.
    fn import(&mut self, data: &[u8]) -> Result<()>;
}
```

The multiplexer holds all machines and orchestrates them:

```rust
impl AbciApplication for Multiplexer {
    fn finalize_block(&mut self, block: Block) -> FinalizeBlockResponse {
        // 1. Begin block for all machines
        self.graph.begin_block(&ctx);
        self.bank.begin_block(&ctx);
        self.cyberwasm.begin_block(&ctx);  // runs scheduled programs

        // 2. Route transactions to machines
        for tx in block.txs {
            match tx.route() {
                Route::CyberWasm(msg) => self.cyberwasm.handle(&ctx, &msg),
                Route::Graph(msg)     => self.graph.handle(&ctx, &msg),
                Route::Bank(msg)      => self.bank.handle(&ctx, &msg),
                Route::Staking(msg)   => self.staking.handle(&ctx, &msg),
                Route::Ibc(msg)       => self.ibc.handle(&ctx, &msg),
            };
        }

        // 3. End block
        let val_updates = self.staking.end_block(&ctx);
        self.cyberwasm.end_block(&ctx);  // drain async message queue
        FinalizeBlockResponse { validator_updates: val_updates, .. }
    }

    fn commit(&mut self) -> CommitResponse {
        let root = merkle_root([
            ("cyberwasm", self.cyberwasm.commit()),
            ("graph",     self.graph.commit()),
            ("bank",      self.bank.commit()),
            ("staking",   self.staking.commit()),
            ("ibc",       self.ibc.commit()),
        ]);
        CommitResponse { app_hash: root }
    }
}
```

### Cross-machine communication

Three mechanisms, in order of coupling:

**Shared state (read-only cross-access).** A CyberWasm program can read graph engine state. The graph engine can read CyberWasm program state. Neither can write to the other's namespace. The multiplexer provides each machine with a read handle to other machines' state.

**Async message passing.** A CyberWasm program sends a message to another machine via a cross-machine queue. The multiplexer drains the queue after all transactions in the block are processed. Raw bytes at the boundary — each machine encodes/decodes its own way.

**Synchronous bridge.** For performance-critical paths, a direct call through the multiplexer. The calling execution suspends, the target machine handles, result returns. Fast but creates coupling. Used sparingly.

Tier 1 machines can call each other directly — just Rust function calls, no serialization:

```rust
trait BankMachine {
    fn balance(&self, addr: &Address, denom: &str) -> u128;
    fn send(&mut self, from: &Address, to: &Address, coins: &[Coin]) -> Result<()>;
}

trait GraphMachine {
    fn create_link(&mut self, from: Cid, to: Cid, neuron: Address) -> Result<u64>;
    fn rank(&self, cid: &Cid) -> u64;
}
```

### State isolation

Each machine owns a namespace in the global Merkle tree:

```
Global Merkle tree
├── cyberwasm/           ← CyberWasm machine owns this subtree
├── graph/               ← Graph engine owns this subtree
├── evm/                 ← Future EVM machine
├── bank/                ← shared (all machines can call)
├── staking/
├── ibc/                 ← shared
└── bostrom/
```

One commitment covers all machines. One app hash for CometBFT. One proof tree for [[ibc]].

### The upgrade spectrum

Components can migrate between tiers as the system matures:

```
Fully native (Tier 1)                        Fully sandboxed (Tier 2)
◄──────────────────────────────────────────────────────────────────►

Graph Engine   Bank   Staking   Governance   DEX   User App
 native       native   native    program   program  program
 max perf     max perf  max perf  sandboxed  sandboxed  sandboxed
 gov upgrade  gov upgrade gov upgrade  permissionless  permissionless
```

[[governance]] starts as Tier 1 (hardcoded logic for bootstrapping). Once CyberWasm is stable, governance migrates to Tier 2 — self-modifying without a binary upgrade. A Tier 2 program that proves itself critical can be promoted to Tier 1 for native speed.

The Machine trait is the same for both tiers from the multiplexer's perspective. Migration between tiers is a pure implementation detail.

### DAG readiness

The linear ABCI model (one block, one state root) forces sequential execution. A DAG-based consensus produces a partial order — multiple vertices concurrently — enabling parallel execution of independent state domains.

CyberWasm's architecture prepares for this without depending on it:

- Programs already have isolated state domains (their own pages)
- The sync call mechanism defines cross-domain dependencies
- The message system separates "must happen now" (sync call) from "can happen whenever" (async send)
- Each machine with its own state namespace is a natural execution lane

If a DAG consensus layer appears — or gets built — each machine maps to a parallel lane. Cross-machine sync calls are the serialization points. Everything else runs concurrently. The ABCI boundary changes, but the programs don't.

```
DAG vertex set (future):
  ├── CyberWasm txs  ──→  CyberWasm machine ──┐
  ├── Graph txs      ──→  Graph machine      ──┼── merge → root
  ├── EVM txs        ──→  EVM machine        ──┤
  └── Cross-machine  ──→  serialized path    ──┘
```

The six requirements are DAG-agnostic. The first implementation is linear. The architecture is parallel-ready.

---

## CyberRS: Tier 1 Kernel Discipline

Tier 1 machines run native Rust — no Wasm sandbox to enforce determinism. A Tier 1 binary that uses `HashMap` (randomized iteration order in standard Rust) or reads system time will produce different results on different validators. Consensus breaks.

CyberRS is a constrained subset of Rust for writing deterministic kernel code. Not a new language — a set of rules enforced by a custom linter and a restricted standard library.

### Forbidden in CyberRS

```
std::collections::HashMap        → use BTreeMap (deterministic ordering)
std::collections::HashSet        → use BTreeSet
std::time::*                     → use block_time() from chain context
std::thread::*                   → no spawning threads during state transition
rand::*                          → use chain-provided randomness
std::fs::*, std::net::*          → no I/O during state transition
f32, f64                         → no floating point (or use softfloat)
panic!() in production paths     → must return Result, never panic
```

### Required in CyberRS

```
All iteration must be deterministic (sorted keys, indexed access)
All serialization must be canonical (borsh, not JSON)
All state mutations must go through the state store API
All external data must come from the chain context (block height, time, etc.)
Pure functions preferred — side effects explicit and auditable
```

### Enforcement

A `#![deny(cyberwasm::nondeterministic)]` attribute activates a custom lint pass catching: forbidden types, floating point, randomness, time-dependent logic, thread spawning during state transitions, non-canonical serialization.

The `cyberrs` crate re-exports safe alternatives:

```rust
use cyberrs::prelude::*;
// BTreeMap instead of HashMap
// BTreeSet instead of HashSet
// ChainTime instead of SystemTime
// DeterministicRng (seeded from chain randomness)
// CanonicalSerialize trait (borsh-based)
// StateStore trait (typed access to Merkle state)
```

### When determinism rules relax

The constraint applies to **state transitions** — the `handle()` and `commit()` path. Non-consensus contexts are unrestricted:

- **Query handling** (read-only, not replicated): threads, caching, approximate algorithms OK
- **Offchain computation**: anything
- **Block proposal preparation** (proposer-local): heuristics OK

The `Machine` trait encodes this split. Deterministic methods in one set, non-deterministic in another. The linter only enforces CyberRS rules on the deterministic methods.

### Graph engine: the Tier 1 showcase

[[bostrom/rank]] computation over millions of [[cyberlinks]] is a dense matrix operation. Running it inside Wasm pays a 3-10x penalty for sandboxing you don't need — it's not user-deployed code, it's the chain's core logic. As a Tier 1 machine:

```rust
#![deny(cyberwasm::nondeterministic)]
use cyberrs::prelude::*;

impl GraphEngine {
    fn recompute_rank_native(&mut self) {
        let adj = self.adjacency.mmap_view();
        let n = adj.node_count();
        let mut rank = vec![FIXED_POINT_ONE / n as u64; n];
        let mut new_rank = vec![0u64; n];

        // Power iteration — SIMD, rayon for parallel
        // Deterministic: same graph → same result (fixed-point, no float)
        for _iter in 0..RANK_ITERATIONS {
            new_rank.fill(0);
            for (node, neighbors) in adj.iter() {
                let share = rank[node] / neighbors.len() as u64;
                for &neighbor in neighbors {
                    new_rank[neighbor] += share;
                }
            }
            std::mem::swap(&mut rank, &mut new_rank);
        }

        for (node, &r) in rank.iter().enumerate() {
            self.rank.insert(node_cid(node), r);
        }
    }
}
```

Native speed. Fixed-point arithmetic. SIMD-accelerated. Deterministic. No Wasm overhead. A Tier 2 program reads the result:

```rust
// Tier 2 Wasm program
let rank = cyberwasm::query_machine(GRAPH, "rank", &my_cid)?;
```

Heavy computation is Tier 1. Creative application logic is Tier 2. Each at the right level.

---

## CyberWasm: Tier 2 Userspace

Everything below describes Tier 2 — sandboxed, gas-metered, permissionless Wasm programs. This is the user-facing execution environment.

### Requirement 1: Persistent Processes

*Programs that live, maintain state in memory, and can be suspended/resumed.*

In CosmWasm, a contract is a stateless handler. It wakes up, loads state from a KV store, computes, saves state, and dies. In CyberWasm, a program is a persistent actor. Its Wasm linear memory survives across calls.

#### What this looks like in code

```rust
use std::collections::HashMap;

struct State {
    registry: HashMap<String, Address>,
    counter: u64,
}

static mut STATE: Option<State> = None;

#[init]
fn init(_params: &[u8]) {
    unsafe {
        STATE = Some(State {
            registry: HashMap::new(),
            counter: 0,
        });
    }
}

#[handle("register")]
fn register(name: &[u8]) -> Result<Vec<u8>> {
    let state = unsafe { STATE.as_mut().unwrap() };
    let name = String::from_utf8_lossy(name).to_string();
    state.registry.insert(name, cyberwasm::caller());
    state.counter += 1;
    Ok(state.counter.to_le_bytes().to_vec())
}
```

No `deps.storage.set()`. No serde. No JSON. The HashMap IS the storage.

#### How it works: lazy pages

Wasm linear memory is divided into 64KB pages. The runtime tracks three page states:

```
┌────────┬────────┬────────┬────────┬────────┬─────┐
│ Page 0 │ Page 1 │ Page 2 │ Page 3 │ Page 4 │ ... │
│ loaded │ DIRTY  │  not   │ DIRTY  │ loaded │     │
│(clean) │        │ loaded │        │(clean) │     │
└────────┴───┬────┴────────┴───┬────┴────────┴─────┘
             │                 │
             ▼                 ▼
        write to store    write to store
```

At execution start: zero pages loaded. Pages load on demand. At execution end: only dirty pages written back. Cost proportional to mutation, not total state.

Three implementation strategies, in order of preference:

**mmap-backed memory.** Wasmtime uses mmap for linear memory. Map program pages to the state store. OS handles page faults natively. Near-zero overhead. Requires wasmtime memory management integration.

**Instrumented access.** Inject page-boundary checks at compile time. ~10-15% overhead. Gear Protocol's approach. Works with any runtime.

**Eager loading.** Load all pages at start. Simple, correct, wasteful. Acceptable as Phase 0 implementation.

#### Memory limits

Maximum linear memory per program: 4GB (Wasm32 limit). Practical limit set by rent economics. Maximum pages at deployment: configurable (default 256 = 16MB). Growable via `memory.grow`, subject to gas and rent.

#### Stable memory (phase 2)

Separate memory region surviving code upgrades. Accessed via explicit host functions:

```rust
cyberwasm::stable_read(offset, buf);
cyberwasm::stable_write(offset, data);
cyberwasm::stable_size() -> u64;
cyberwasm::stable_grow(pages) -> Result<u64>;
```

Maximum 64GB. Same page-based persistence and rent model.

#### Program lifecycle

```
Deploy → Active (has gas reserve, executes normally)
           │ gas reserve depleted
           ▼
         Frozen (pages retained, no execution, rent clock stops)
           │ grace period expires (default 30 days)
           ▼
         Evicted (pages garbage collected, Merkle root retained)
           │ address reserved indefinitely
           ▼
         Tombstone (can redeploy new code to same address)
```

#### Program upgrades

New Wasm binary submitted by deployer or admin. Linear memory preserved as-is — new code must be compatible with existing layout (program's responsibility). The `upgrade` handler runs after code swap for migration. Atomic: if handler fails, old code remains.

#### What this replaces from CosmWasm

| CosmWasm | CyberWasm |
|---|---|
| Instance created fresh per call | Instance persists, memory survives |
| State via `deps.storage` KV store | State lives in Wasm heap natively |
| Every structure serialized/deserialized | Data structures just exist in memory |
| `cosmwasm_std::Map`, `Item`, `IndexedMap` | `HashMap`, `BTreeMap`, anything |
| State access crosses FFI boundary | State access is a Wasm memory read/write |

---

### Requirement 2: Self-Scheduling

*Programs that can trigger their own execution without external relayers.*

In CosmWasm, a contract only executes when someone sends it a transaction. Periodic work requires off-chain relayer bots. CyberWasm programs schedule their own future execution.

```rust
#[init]
fn init(_params: &[u8]) {
    cyberwasm::schedule_periodic(10, "tick");   // every 10 blocks
}

#[handle("tick")]
fn tick() -> Result<Vec<u8>> {
    recompute_index();
    Ok(vec![])
}
```

One-time and delayed scheduling:

```rust
cyberwasm::schedule_at(1_000_000, "migrate");
cyberwasm::schedule_delayed(Duration::hours(1), "cleanup");
cyberwasm::cancel_schedule(id);
```

The runtime maintains a schedule table. During `FinalizeBlock`, before user transactions, due schedules execute. Gas from the program's reserve — not from any user. If reserve can't cover a call, it's skipped. Full depletion → freeze.

Scheduled execution has lower priority than user transactions. Ordering is deterministic by address.

```rust
let reserve = cyberwasm::gas_reserve_balance();
let burn_rate = cyberwasm::gas_burn_rate();
let blocks_remaining = reserve / burn_rate;
```

| CosmWasm | CyberWasm |
|---|---|
| External relayer bot | Program schedules itself |
| Relayer must be funded and monitored | Gas reserve is the only requirement |
| No cron, no timers | `schedule_periodic`, `schedule_at`, `schedule_delayed` |
| Cron requires governance whitelisting | Any program can self-schedule |

---

### Requirement 3: Compact Program Overhead

*Minimal per-program resource footprint so you can run thousands of small programs.*

CosmWasm contracts are fat. A CW20 token is 200-400KB. Every contract embeds serde, serde_json, cosmwasm_std, schema generation. CyberWasm eliminates all of it.

| Bloat source | CosmWasm overhead |
|---|---|
| serde + serde_json | 50-100 KB |
| cosmwasm_std | 30-60 KB |
| cosmwasm_schema | ~20 KB |
| JSON schema generation | 10-20 KB |
| String error types | 10-30 KB |
| Entry point boilerplate | 5-10 KB |
| **Total before logic** | **125-240 KB** |

CyberWasm SDK contributes under 5KB. Three Wasm exports:

```
cyberwasm_init(params_ptr, params_len) → i32
cyberwasm_upgrade(params_ptr, params_len) → i32
cyberwasm_dispatch(handler_ptr, handler_len, payload_ptr, payload_len) → i32
```

Expected binary sizes:
```
Minimal program (counter, echo):                 1 - 5 KB
Simple program (token, name registry):           15 - 50 KB
Medium program (DEX orderbook, AMM):             50 - 150 KB
Complex program (graph indexer, DeFi):           150 - 500 KB
```

Per-program state overhead:
```
Code hash:          32 bytes
Admin address:      32 bytes
Gas reserve:        16 bytes
Page count:         4 bytes
Metadata:           ~100-500 bytes
Pages:              64KB × active pages
Schedule entries:   ~50 bytes each
```

Code deduplication: Wasm binary stored once under `code/{code_hash}`. Deploy 1000 instances of a token: code once (~30KB), pages 1000 times.

Node binary target: under 50MB. Cosmos SDK Go binaries: 80-150MB.

| CosmWasm | CyberWasm |
|---|---|
| 200-400 KB simple token | 15-50 KB simple token |
| 125-240 KB mandatory overhead | Under 5 KB SDK overhead |
| 5 entry points | 3 entry points |
| JSON encoding | Raw bytes, program chooses format |

---

### Requirement 4: Sovereignty + IBC

*Own chain, interoperability with Cosmos ecosystem.*

#### ABCI boundary

CometBFT handles consensus, networking, block production. The multiplexer IS the ABCI application. CometBFT sends four things:

```
                    ABCI (protobuf over socket)
CometBFT ◄──────────────────────────────────────► Multiplexer (Rust binary)
consensus                                          state machine
networking                                         program execution
block production                                   state commitment
peer discovery                                     IBC
mempool                                            bank, staking, governance
```

The entire system — multiplexer, all Tier 1 machines, all Tier 2 programs — can be developed, tested, and iterated independently of consensus.

#### IBC integration

IBC operates on state proofs. The multiplexer writes to a Merkle-committed state store and produces verifiable proofs. IBC module is Rust using `ibc-rs`.

Programs interact with IBC through host functions:

```rust
cyberwasm::ibc_send_packet(port, channel, data, timeout) → Result<u64>
```

IBC callbacks are program handlers:

```rust
#[handle("ibc_recv_packet")]
fn on_recv(packet: &[u8]) -> Result<Vec<u8>> { ... }

#[handle("ibc_ack_packet")]
fn on_ack(ack: &[u8]) -> Result<Vec<u8>> { ... }

#[handle("ibc_timeout_packet")]
fn on_timeout(packet: &[u8]) -> Result<Vec<u8>> { ... }
```

Persistent memory (Requirement 1) means programs maintain context across asynchronous IBC round-trips without external state management.

#### IBC-provable program state

Any data in program persistent memory is IBC-provable:

```
global_root → cyberwasm/programs/{addr}/page_root → page_tree → page content
```

#### State commitment structure

```
Global Merkle tree
├── bank/balances/{address}/{denom}    → u128
├── staking/...
├── ibc/...
├── cyberwasm/
│   ├── code/{code_hash}               → Wasm bytes
│   ├── programs/{address}/
│   │   ├── code_hash                  → [u8; 32]
│   │   ├── gas_reserve                → u128
│   │   ├── page_root                  → [u8; 32]
│   │   ├── schedule/...
│   │   └── metadata
│   └── scheduler/...
├── graph/
│   ├── links/...
│   └── rank/...
└── bostrom/...
```

Each machine owns its subtree. Combined root covers all machines.

#### Core machines (Tier 1)

- **Bank** — token transfers, balances, supply. Shared, callable by all machines.
- **Auth** — accounts, sequence numbers, signatures.
- **Staking** — validators, delegation, slashing.
- **IBC** — channels, packets, light clients (ibc-rs). Shared.
- **CyberWasm runtime** — the Tier 2 execution engine.
- **Graph engine** — cyberlinks, rank computation. Native for performance.

#### Self-hosted modules (Tier 2)

- Governance
- Distribution
- Fee management
- Upgrade coordination

The chain's own governance logic runs on its own VM.

| CosmWasm (on Cosmos SDK) | CyberWasm |
|---|---|
| Go ABCI app wrapping a Wasm VM | Rust ABCI app with multiplexer |
| Cosmos SDK Go modules | Rust-native Tier 1 + self-hosted Tier 2 |
| IBC via Go ibc-go | IBC via Rust ibc-rs |
| Two languages, two runtimes | One language, one binary |

---

### Requirement 5: Clean Billing

*Storage rent (not one-time write cost), compute metering that reflects reality.*

CosmWasm charges once to write, then state persists forever at zero cost. CyberWasm bills three resources.

#### Compute

Per Wasm instruction via wasmtime's built-in fuel system. No binary modification at upload. Checked at branch points, not every instruction.

Host function costs:
```
cyberwasm::call              → 1000 gas + callee's consumption
cyberwasm::send              → 500 gas + 1 gas/byte
cyberwasm::sha256            → 100 gas + 1 gas/64 bytes
cyberwasm::secp256k1_verify  → 3000 gas
cyberwasm::bank_send         → 2000 gas
cyberwasm::ibc_send_packet   → 5000 gas + 1 gas/byte
```

#### Memory rent

Per page per block. Charged continuously from gas reserve.

```
rent_per_block = num_pages × RENT_PER_PAGE_PER_BLOCK
```

Governance parameter. Target: $1-5 per GB per year. Memory rent is natural selection: compact programs survive.

#### Message bandwidth

Per byte of inter-program messages:
```
message_gas = BASE_MESSAGE_GAS + payload_bytes × GAS_PER_BYTE
```

#### Gas reserve

Every program has a reserve account. Debited for rent, scheduled execution, async messages. Credited by deposits, fee splits from callers, governance subsidy.

```rust
cyberwasm::deposit_to_reserve(program_address, amount);
let blocks_remaining = cyberwasm::gas_reserve_balance() / cyberwasm::gas_burn_rate();
```

Reserve → zero → freeze (pages retained, rent stops). Grace period → eviction (pages GC'd).

User transaction gas covers compute and I/O of that call. Users pay for what they use. Programs pay for what they hold.

| CosmWasm | CyberWasm |
|---|---|
| One-time gas to write, free to hold forever | Continuous rent proportional to state |
| Instruction injection modifies binary | Wasmtime fuel, no modification |
| No program self-funding | Gas reserve: programs fund themselves |
| No pressure to clean up | Rent creates compact-or-die pressure |

---

### Requirement 6: Minimal Layering

*As few abstraction layers as possible between program and hardware.*

#### CosmWasm: five layers deep

```
Hardware → OS → Go runtime (GC, green threads)
  → Cosmos SDK → x/wasm → CGo bridge (JSON)
    → wasmvm → wasmer → CONTRACT
```

Nine boundary crossings for one `storage.get()`.

#### CyberWasm: two layers deep

```
Hardware → OS → CyberWasm (single Rust binary)
  ├── wasmtime → PROGRAM
  ├── state store (RocksDB, in-process)
  ├── message router (in-process)
  └── IBC, bank, staking (in-process)
```

Three boundary crossings per page access. No CGo. No JSON. No GC.

#### Message system: real dispatch

**Synchronous calls.**

```rust
let result: Vec<u8> = cyberwasm::call(target, "method", &payload)?;
```

Runtime suspends caller, loads target with persisted memory, runs handler, returns result. Bounded stack (default 10). Reentrancy disabled default, opt-in.

**Asynchronous messages.**

```rust
cyberwasm::send(target, "handler", &data);
cyberwasm::send_with_reply(target, "handler", &data, "callback");
cyberwasm::send_delayed(target, "handler", &data, blocks(100));
```

Async queues drain after user txs. Gas from sender's reserve.

#### Entry point surface

Three Wasm exports:

```
cyberwasm_init(params_ptr, params_len) → i32
cyberwasm_upgrade(params_ptr, params_len) → i32
cyberwasm_dispatch(handler_ptr, handler_len, payload_ptr, payload_len) → i32
```

#### The syscall table: ~50 host functions

The complete interface between a Tier 2 program and the kernel.

**Identity and context:**
```
self_address, caller, block_height, block_time, tx_hash
```

**Inter-program:**
```
call, call_with_gas, send, send_with_reply, send_delayed
```

**Cross-machine (routed through multiplexer):**
```
query_machine, call_machine
```

**Scheduling:**
```
schedule_periodic, schedule_at, schedule_delayed, cancel_schedule
```

**Gas and economics:**
```
gas_remaining, gas_reserve_balance, gas_burn_rate, deposit_to_reserve
```

**Cryptography (host-accelerated):**
```
sha256, sha512, keccak256, blake2b256, blake3
ed25519_verify, ed25519_sign
secp256k1_verify, secp256k1_recover
bls12_381_verify
```

**Bank:**
```
bank_send, bank_balance, bank_total_supply, bank_mint, bank_burn
```

**IBC:**
```
ibc_send_packet, ibc_channel_open, ibc_channel_close
```

**Stable memory:**
```
stable_read, stable_write, stable_size, stable_grow
```

[[bostrom]]-specific:
```
create_cyberlink, delete_cyberlink
query_rank, query_backlinks, query_outlinks, query_bandwidth
```

No filesystem. No sockets. No threads.

The syscalls split into two categories. **Intra-machine** (program ↔ CyberWasm runtime): call, send, schedule, gas management. **Cross-machine** (program ↔ other Tier 1 machines via multiplexer): bank_send, create_cyberlink, ibc_send_packet. The split is invisible to the developer — the SDK wraps both — but architecturally it's the right separation.

New machines can be added without modifying CyberWasm. An EVM machine registers on the multiplexer bus. Programs call it:

```rust
cyberwasm::call_machine(EVM, contract_addr, calldata)?;
```

No change to existing programs.

| CosmWasm | CyberWasm |
|---|---|
| Go + CGo + Rust + Wasm (4 layers) | Rust + Wasm (2 layers) |
| 9 boundary crossings per state access | 3 boundary crossings per page access |
| JSON between all components | Raw bytes |
| Submessage queue + reply callbacks | Direct sync calls + real async |
| Go GC pauses execution | No GC |

---

## SDK

A Rust crate. Thin. Does NOT depend on serde, serde_json, or cosmwasm_std. Under 5 KB compiled contribution.

```rust
// Macros
#[init]                      // initialization handler
#[upgrade]                   // upgrade handler
#[handle("name")]            // message handler
#[scheduled("name")]         // scheduled execution handler

// Host functions (typed wrappers)
cyberwasm::call(addr, handler, data) -> Result<Vec<u8>>
cyberwasm::send(addr, handler, data)
cyberwasm::caller() -> Address
cyberwasm::block_height() -> u64
cyberwasm::bank_send(to, denom, amount) -> Result<()>

// Types
Address([u8; 32])
Coin { denom: String, amount: u128 }
Link { from: Cid, to: Cid, neuron: Address }
Cid(Vec<u8>)

// Optional serialization (not required)
cyberwasm::encoding::{encode_borsh, decode_borsh}
cyberwasm::encoding::{encode_proto, decode_proto}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use cyberwasm_testing::*;

    #[test]
    fn test_register() {
        let mut env = TestEnv::new();
        let prog = env.deploy("my_program.wasm", b"init_params");
        env.call(&prog, "register", b"alice", &addr("user1")).unwrap();
        let result = env.call(&prog, "lookup", b"alice", &addr("user1")).unwrap();
        assert_eq!(result, addr("user1").as_bytes());
    }
}
```

Standard `cargo test`. No node needed.

---

## CosmWasm Migration

### Compatibility shim

Existing .wasm binaries deploy unmodified. The shim intercepts CosmWasm host imports (`db_read`, `db_write`, etc.) and redirects to CyberWasm equivalents. KV operations hit a HashMap in persistent memory.

```
┌─────────────────────────────────────┐
│ CosmWasm Compatibility Shim         │
│                                     │
│  Intercepts CosmWasm host imports   │
│  Redirects to CyberWasm equivalents │
│  KV ops → HashMap in persistent mem │
│                                     │
│  ┌─────────────────────────────┐    │
│  │ Original CosmWasm Contract  │    │
│  │ (unmodified .wasm binary)   │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

Shimmed contracts run faster than on real CosmWasm (no CGo bridge) but do NOT benefit from persistent memory.

### Migration timeline

1. Deploy CyberWasm alongside existing chain. Old CosmWasm untouched.
2. Migrate existing contracts via shim. Same behavior, better performance.
3. New programs deploy natively.
4. Deprecate CosmWasm.

---

## Security

**Sandboxing (Tier 2):** Wasm sandbox. No host OS access. All interaction through ~50 host functions. Memory bounded to program's own linear memory.

**Determinism (Tier 1):** CyberRS lint discipline. BTreeMap not HashMap. Fixed-point not float. No system time, no random, no threads during state transitions. Machine trait encodes the boundary between deterministic and non-deterministic methods.

**Determinism (Tier 2):** No floating point. No system time (block_time only). No random memory layout. No threads. Wasmtime strict determinism mode.

**Resource limits (governance-configurable):**
```
Max memory per program:       4 GB
Max call stack depth:         10
Max message payload:          1 MB
Max Wasm binary:              2 MB
Max pages at deployment:      256 (16 MB)
Gas limit per block:          governance
Gas limit per transaction:    sender
```

---

## Relationship to go-cyber Upgrade Plan

CyberWasm is a parallel effort to the [[go-cyber]] dependency upgrade plan. They are complementary, not competing.

### go-cyber upgrade plan: three phases of Go work

The go-cyber codebase (currently SDK v0.47 / CometBFT v0.37 / CosmWasm 1.x) has its own upgrade roadmap:

**Phase 0 (no consensus change, current SDK v0.47):** 12 items including graph streaming gRPC, cyb desktop app (Rust/tauri), IPFS sidecar, embedded dashboard, CPU rank optimization, graph inference (embeddings via Burn + llm via llama.cpp). ~13,000-16,000 new LOC. Predominantly Go with Rust for training (Burn) and desktop (tauri). Zero fork required.

**Phase 1 (SDK v0.50 chain upgrade):** Remove x/liquidity module (eliminates SDK fork), migrate all modules to ABCI 2.0 / FinalizeBlock, eliminate Cosmos SDK fork, snapshot extensions, height index, multi-chain binary, personal networks, Inter-Knowledge Protocol. The single largest piece of work is rebasing the forked SDK.

**Phase 2 (SDK v0.53 + CosmWasm 3.0):** IBC Eureka (Ethereum connectivity), unordered transactions, wgpu rank prototype.

### Where CyberWasm fits

CyberWasm development is independent of the Go upgrade path. It proceeds as a separate Rust codebase:

```
go-cyber Phase 0 (Go, current stack)     CyberWasm Phase 0 (Rust, standalone)
         │                                          │
go-cyber Phase 1 (SDK v0.50 upgrade)     CyberWasm Phase 1 (core runtime)
         │                                          │
go-cyber Phase 2 (SDK v0.53 + CW 3.0)    CyberWasm Phase 2 (chain integration)
         │                                          │
         └──── converge ────────────────► CyberWasm Phase 3 (mainnet migration)
```

The go-cyber upgrades keep the existing chain alive, performant, and compatible with the Cosmos ecosystem. CyberWasm builds the replacement in parallel. They converge when CyberWasm is production-ready and a governance proposal swaps the binary.

The convergence is a standard Cosmos chain upgrade via Cosmovisor — the same mechanism used for every prior upgrade (v3→v4→v7). At the upgrade height, validators swap from the Go binary to the Rust binary. Genesis state exports from go-cyber, imports into CyberWasm.

### What Phase 0 of each means

**go-cyber Phase 0:** Ship improvements on the current Go stack. Graph streaming, desktop app, inference, packaging. No consensus change. No fork.

**CyberWasm Phase 0:** Build the Rust ABCI skeleton. Wasmtime integration. Eager page loading. Single handler dispatch. Minimal host functions. In-memory state store. Basic SDK. Test suite. Pure Rust, zero contact with go-cyber codebase.

Both can proceed with independent teams on independent timelines.

---

## Roadmap

### Phase 0: Prototype (months 1-3)
Rust ABCI skeleton with CometBFT integration. Wasmtime with fuel metering. Eager page loading (Strategy 3). Single handler dispatch. Minimal host functions: block context, bank_send, bank_balance. In-memory state store (testing only). Basic SDK with `#[handle]` macro. Multiplexer skeleton with Machine trait. Test suite: deploy, call, persist, recover.

All pure Rust. No fork required. No contact with go-cyber.

### Phase 1: Core Runtime (months 3-6)
Persistent state store (RocksDB + Merkle tree). Lazy page loading (Strategy 2 — instrumented). Synchronous inter-program calls. Async message queue. Scheduler (periodic + oneshot). Gas reserve and memory rent. Full bank machine. Graph machine stub. CyberRS linter (initial rules). CosmWasm compatibility shim. Testnet deployment.

### Phase 2: Chain Integration (months 6-9)
IBC machine (ibc-rs). Auth machine. Basic staking machine. Graph engine with rank computation (native, SIMD). Bostrom host functions (cyberlinks, rank queries). Program upgrade mechanism. Stable memory. mmap page loading (Strategy 1). Cross-machine communication protocol. Security audit.

### Phase 3: Production (months 9-12)
Governance as Tier 2 CyberWasm program. Full Bostrom state migration from Go chain. CosmWasm contract migration via shim. Performance benchmarking. Documentation and developer onboarding. Mainnet upgrade proposal via governance.

### Phase 4: Self-Hosting (months 12+)
Core modules (bank, staking) migrate to Tier 2 programs. Runtime becomes minimal kernel. Cross-chain program deployment via IBC. Wasm component model support. Future machines (EVM, ZK, AI inference).

---

## Open Design Questions

1. **Floating point.** Remove entirely or allow with NaN canonicalization?
2. **Memory growth.** Growth during execution? Rent implications?
3. **Reentrancy.** Disabled default + opt-in sufficient?
4. **Scheduler fairness.** Ordering among concurrent schedules?
5. **Grace period.** 30 days freeze before eviction — right number?
6. **Program keys.** Threshold crypto or HKDF derivation?
7. **Runtime pinning.** Pin wasmtime version via governance?
8. **Cross-chain deploy.** Program code via IBC packet?
9. **CyberRS scope.** How strict should the linter be? Formal verification path?
10. **Machine registration.** Static (compile-time) or dynamic (governance-added)?

---

*CyberWasm. A byte machine for sovereign computing.*