# Model lifecycle monopolization

Every competitor owns one segment. HuggingFace owns the registry.
Ollama owns the runtime. Nobody owns the format that connects them.
cyb owns the format, and from format gravity the entire lifecycle follows.

The thesis: whichever system becomes the lingua franca for model
representation captures every economic layer above it — distribution,
registry, attribution, and monetization — because those layers can only
function on a common unit. The .cyb format is that unit.

## Format gravity

`.cyb` is a single CID-addressed file: architecture + weights + tokenizer +
quantization, all in one. It is the first format that can flow through
every stage of the model lifecycle without conversion loss:

```
  cybergraph live state         HF / GGUF / ONNX / safetensors
          │                                  │
          ▼  compile (mc)      reverse (import) ▼
          └──────────────► .cyb / .model ◄────────┘
                                  │
                          runtime (mr)
                    cpu · wgpu+rs · honeycrisp
                                  │
                           tokens / images
                                  │
                   model-as-neuron writes cyberlinks
                           back into graph
```

Five engines, one format:

- **import** — reverse: HF / GGUF / ONNX / safetensors → .cyb
- **mc** — compile: cybergraph snapshot → .cyb (CT-0 spec)
- **mr** — runtime: .cyb → tokens, three backends
- **cyb-llm** — CLI + HTTP serve + router around mr
- **cyb browser** — render: graph IR, weights, traces from .cyb

No format is neutral. Every format that succeeds creates gravity: tools
build around it, distribution optimizes for it, registries index it.
The gravity of .cyb is that it is the only format both directions flow
through — compile from graph AND reverse from transformer. Competitors
cannot adopt it without shipping the closed cycle themselves.

## Displacement map

| Segment | HuggingFace | Ollama | Bittensor | cyb |
|---------|-------------|--------|-----------|-----|
| Format | scattered files | GGUF | — | .cyb (single CID, quant included) |
| Runtime | none | llama.cpp | cloud | Rust + honeycrisp / wgpu+rs / cpu |
| Distribution | HTTP | HTTP | — | p2p swarm, BAO dedup |
| Registry | centralized hub | none | none | .model on Bostrom (CyberRank) |
| Compile graph → model | none | none | none | mc (CT-0) |
| Reverse model → graph | none | none | none | import |
| Attribution | none | none | staker weights | model + graph neurons earn |
| Lock-in | HF URL | Modelfile | subnet | none — CID is portable |

HuggingFace is GitHub for models — a registry with no runtime.
Ollama is Docker for models — a runtime with no registry.
Both are one-way and centralized.

cyb is the lifecycle: any transformer reverses into the cybergraph,
any cybergraph slice compiles back to a transformer, and the same .cyb
runs on commodity, Apple Silicon, and deterministic hardware.
Neither HF nor Ollama can replicate the bidirectionality without
building mc + import + the cybergraph — i.e., the whole system.

## Lock-in cascade

Each phase captures a market segment and makes defection from the next
phase more expensive:

```
Phase 0: runtime moat     ← switch for verified speed + RAM savings
Phase 1: bridge moat      ← switch for the only bidirectional graph ↔ model path
Phase 2: distribution     ← invite for faster downloads (swarm beats HTTP at scale)
Phase 3: app store        ← publish for storefront + CyberRank-rated discovery
Phase 4: attribution      ← earn usage-weighted reputation that cannot be purchased
Later:   monetization     ← once the graph carries real economic weight
```

Phase 0 creates a performance reason to use cyb over Ollama.
Phase 1 creates a capability no one else has — the bridge is the moat.
Phases 2–4 are network effects: each model published, each fetch seeded,
each inference attributed makes the platform more valuable for the next
user without any additional engineering.

The manifest discipline applies throughout: four models made perfect and
fast before any new family is added. A moat built on four verified models
is stronger than a demo that claims ten.

## Why the bridge is the moat

Without the bridge (mc + import), cyb is another inference runtime
competing on tok/s benchmarks. With the bridge, cyb is the only system
where:

- a cybergraph becomes a transformer — knowledge crystallizes into weights
- a transformer becomes a cybergraph — closed weights become inspectable nodes
- attribution flows in both directions — model neurons and graph neurons both earn
- the .cyb format is the ledger — render, compile, reverse, run, distribute, register all read it

The bridge means the cybergraph accumulates economic weight over time:
every model ever compiled from it or reversed into it links back. That
weight compounds. Competitors building runtimes or registries are upstream
of this compounding; they can be commoditized by it.

## Phases

### Phase 0 — runtime moat (5 sessions)

Four manifest models verified correct and faster than Ollama on both
honeycrisp (Apple Silicon) and wgpu+rs (cross-platform):

| Model | Role | Acceptance |
|-------|------|------------|
| qwen3-0.6b-abl | router (classifier) | 50/50 prompts vs Ollama, > 256 tok/s |
| qwen2.5-coder-1.5b-abl | code small | 50/50 prompts verified, tok/s at bandwidth ceiling |
| qwen2.5-coder-14b-abl | code large | loads < 10 s, > 22 tok/s honeycrisp, 5 GB RAM |
| gemma-4-31b | general | loads, 50/50 verified, 10 GB RAM |

Deliverable: `brew install cyb-llm && cyb-llm fetch tier0 && cyb-llm serve`
routes traffic through qwen3-0.6b to whichever model fits the task.

### Phase 1 — bridge moat (4 sessions)

mc: 8-pass compile from cybergraph snapshot → .cyb (CT-0 spec).
import reverse: any HF / GGUF / ONNX model → .graph.
Round-trip: HF → import → .graph → mc → .cyb → mr output ε-equivalent to source.

This phase has no competitor. It cannot be replicated without shipping
the whole system.

### Phase 2 — distribution moat (2 sessions)

BAO content-addressing splits .cyb into 256 KB CID chunks.
P2P swarm: every downloader seeds. Qwen family shows 60 %+ chunk dedup
across sizes — every shared chunk fetched once across the network.
Target: 70 B model < 10 min swarm vs 30 + min HTTP.

### Phase 3 — registry moat (3 sessions)

`.model` NFT on Bostrom = listing with CyberRank.
Every version is an immutable CID. Every listing links to its source
graph or source HF repo — provenance is on-chain.
Discovery: semantic query over listings + the cybergraph itself.
CyberRank for the `.model` namespace is weighted by real inference,
not benchmarks or marketing.

### Phase 4 — attribution moat (2 sessions)

During inference, model neurons write cyberlinks:

```
user:         question_CID → answer_CID    (user's knowledge)
model_neuron: answer_CID   → model_CID     (model attribution)
graph_neuron: model_CID    → source_graph  (compile-source attribution)
```

No protocol changes. Model and source-graph are regular neurons earning
regular CyberRank. Reputation accumulates from real usage and cannot
be purchased. This makes the leaderboard the first honest model
ranking: usage-weighted, on-chain, permanent.

## Ground state (2026-05-28)

Runtime:

| Model | Load | Run | Verified | Blocker |
|-------|------|-----|----------|---------|
| qwen3-0.6b-abl | ✓ | ✓ | ✗ | forward-pass bug, suspect QK-norm |
| qwen2.5-coder-1.5b-abl | ✓ | ✓ | ✗ | no golden comparison yet |
| qwen2.5-coder-14b-abl | ✗ | — | — | load timeout, needs fused Q4_K matmul |
| gemma-4-31b | ✗ | — | — | Gemma extensions absent (softcapping, sliding window, K=V) |

Bridge: mc has .graph reader + .model writer scaffolding; CT-0 passes 1–8 not yet implemented.
import: loads safetensors / GGUF / ONNX into Weights table; reverse extraction is design-stage.

Phase 0 cannot ship until all four manifest models produce verified-correct
output on both honeycrisp and wgpu+rs and beat their Ollama baselines on tok/s.
