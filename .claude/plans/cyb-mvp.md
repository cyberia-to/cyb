# cyb MVP: model lifecycle closure

The full graph ↔ transformer loop is implementable today. MVP ships the
closed cycle: render + compile + reverse + runtime + distribution + registry + attribution.

HuggingFace is a registry without a runtime. Ollama is a runtime without
a registry. Both are one-way and centralized. cyb is bidirectional and
content-addressed: any conventional transformer reverses into the
cybergraph, any cybergraph slice compiles back to a transformer, and the
same .model runs on commodity, Apple-Silicon, and (later) deterministic
hardware.

## The closed cycle

```
  HF / GGUF / ONNX / safetensors        .graph snapshot          cybergraph live state
                │                              │                          │
                ▼                              ▼                          ▼
            reverse (mi)                  compile (mc)             render (cyb browser)
                │                              │                          │
                └──────────► .cyb / .model ◄───┴──────────────────────────┘
                                   │
                                   ▼
                              runtime (mr)
                          cpu · wgpu+rs · honeycrisp
                                   │
                                   ▼
                                tokens / images / audio
                                   │
                                   ▼
                model-as-neuron writes cyberlinks back into graph
```

Five engines, one format:

- mi — reverse: HF / GGUF / ONNX / safetensors → IR + weights → .cyb / .model
- mc — compile: .graph cybergraph snapshot → .cyb / .model (CT-1 spec)
- mr — runtime: .model → tokens, three backends (cpu / wgpu+rs / honeycrisp)
- cyb-llm — CLI + serve + router around mr (download, hot-swap, OpenAI API)
- cyb browser — render: visualize the .cyb (graph IR, weights, traces)

## Why the closure is the MVP

Without the bridge, cyb is yet another inference runtime competing
on tok/s. With the bridge, cyb is the only system where:

- a cybergraph can become a transformer — knowledge crystallizes into weights
- a transformer can become a cybergraph — closed weights become inspectable nodes
- the .cyb format is the lingua franca — render, compile, reverse, run all read it
- attribution flows in both directions — model neurons earn from inference, graph neurons earn from compilation

Every other phase (distribution, registry, attribution) presupposes the
closure. Phase 0 + Phase 1 deliver it; Phases 2–4 monetize it.

## Manifest scope

`mr/src/manifest.rs` is the single source of truth for what we make
perfect and fast. Four models, each chosen for a reason:

| # | Model | Family | Role | Why in scope |
|---|-------|--------|------|--------------|
| 1 | qwen3-0.6b-abl | LlamaStyle (qk_norm) | router | always-on classifier; smallest path to verified correctness |
| 2 | qwen2.5-coder-1.5b-abl | LlamaStyle (attn_bias) | code | second family variant — proves the runtime handles both |
| 3 | qwen2.5-coder-14b-abl | LlamaStyle | code | large-model load path (fused Q4_K matmul) |
| 4 | gemma-4-31b | LlamaStyle+ | general | exercises softcapping, sliding window, K = V — the long-tail features |

Everything outside the manifest is post-MVP. New families enter the
manifest only after the existing four are perfect and fast.

## Honest current state (2026-04-23)

From `reference/runtime/reality.md`, restricted to manifest:

| Model | Load | Run | Output | Specific blocker |
|---|---|---|---|---|
| qwen3-0.6b-abl | ✓ | ✓ | fragmented `<\|im_end\|>` pieces | forward-pass bug, suspect qwen3 QK-norm |
| qwen2.5-coder-1.5b-abl | ✓ | ✓ | plausible, unverified | no golden comparison vs Ollama / HF |
| qwen2.5-coder-14b-abl | ✗ timeout | — | — | load path needs fused Q4_K matmul |
| gemma-4-31b | ✗ panic | — | — | Gemma 3/4 extensions absent (softcapping, sliding window, K = V) |

Manifest score: 2 of 4 load + run, 0 of 4 verified correct, 0 of 4
benchmarked honestly.

Other ground state:

- Apple-Silicon path renamed Metal → honeycrisp (Metal + ANE + AMX + NEON + unimem)
- mc has the .graph reader and .model writer scaffolding; CT-1 passes 1–8 not yet implemented
- mi loads HF safetensors + GGUF + ONNX into a `Weights` table; reverse-to-graph extraction is design-stage

Phase 0 cannot ship until all four manifest models produce verified-correct
output on both honeycrisp and wgpu+rs and beat their Ollama baselines on tok/s.

## Benchmarks (M1 Pro, Q4)

| Model | Ollama tok/s | cyb honeycrisp | cyb wgpu+rs | Ollama RAM | cyb RAM |
|-------|--------------|----------------|-------------|-----------|--------|
| qwen3-0.6b-abl | 214 | TBD (correctness blocker) | TBD | 700 MB | 350 MB |
| qwen2.5-coder-1.5b-abl | TBD | TBD | TBD | TBD | TBD |
| qwen2.5-coder-14b-abl | ~15 | TBD (load blocker) | TBD | 9 GB | 5 GB |
| gemma-4-31b | TBD | TBD (load blocker) | TBD | ~18 GB | ~10 GB |

Every TBD is honest: the previous Metal-era numbers are superseded;
honeycrisp / wgpu+rs must be re-measured after each blocker clears.
No copied numbers, no marketing.

## Phase 0: runtime moat — perfect and fast on the four (5 sessions)

Goal: every model in `mr/src/manifest.rs` is verified correct and
beats its Ollama baseline on tok/s, on both honeycrisp and wgpu+rs.

Then `brew install cyb-llm && cyb-llm fetch tier0 && cyb-llm serve` works.

### 0.1 qwen3-0.6b-abl — router (1 session)

- Resolve forward-pass bug. Highest-suspicion: qwen3 QK-norm (per-head q_norm / k_norm)
- Verify against Ollama on 50 prompts: argmax token equivalence after `<|im_start|>assistant`
- Bench honeycrisp + wgpu+rs vs Ollama 214 tok/s baseline; target +20%
- Acceptance: 50/50 prompts agree, both backends > 256 tok/s

### 0.2 qwen2.5-coder-1.5b-abl — code small (1 session)

- Establish golden comparison vs Ollama (no current verification — currently "plausible")
- Wire qwen2 attention bias path through both backends
- 50-prompt verification + tok/s baseline + bench
- Acceptance: 50/50 prompts agree, tok/s within ε of theoretical bandwidth ceiling

### 0.3 qwen2.5-coder-14b-abl — code large (1 session)

- Fused Q4_K matmul to unblock load (current >30 s timeout)
- Memory budget: 5 GB target on M1 Pro 16 GB
- Verify against Ollama ~15 tok/s baseline; target +50% via fused kernels
- Acceptance: loads under 10 s, 50/50 prompts agree, > 22 tok/s on honeycrisp

### 0.4 gemma-4-31b — general (1.5 sessions)

- Implement Gemma 3/4 extensions: logit softcapping, sliding-window attention, K = V tying
- Fix layer-5 shape panic
- Memory budget: 10 GB target
- Acceptance: loads, 50/50 prompts agree with Ollama, tok/s reported honestly (no Ollama baseline available)

### 0.5 CLI + serve + router (0.5 session)

- `cyb-llm run MODEL "prompt"` → honeycrisp on Apple Silicon, wgpu+rs elsewhere
- `cyb-llm serve` → OpenAI-compatible API (`/v1/chat/completions`)
- `cyb-llm status` → manifest dashboard (load OK, verified, tok/s, RAM per backend)
- `cyb-llm fetch NAME` → mi download + quantize + pack .cyb
- `cyb-llm serve --router`: qwen3-0.6b classifies, hot-swaps to coder-1.5b / coder-14b / gemma per task
- KV inheritance on escalation within tokenizer family

Deliverable: 4 of 4 manifest models perfect (verified-correct on 50 prompts each)
and fast (each beats or matches its Ollama baseline) on honeycrisp and wgpu+rs,
served behind one OpenAI-compatible endpoint with automatic routing.

## Phase 1: bridge moat — compile + reverse (4 sessions)

Goal: any HF model reverses into a .cyb graph; any cybergraph snapshot
compiles into a runnable .cyb / .model.

### 1.1 mc compile (graph → .model)

CT-1 spec, 8 passes (`mc/src/pass/`):

| pass | scope | status |
|------|-------|--------|
| 1 | vocab discovery from graph particles | todo |
| 2 | semcon discovery (motifs that recur) | todo |
| 3 | architecture parameters from graph topology | todo |
| 4 | embedding via randomized SVD on attention-weighted graph | todo |
| 5 | per-semcon attention heads | todo |
| 6 | MLP layers from graph density | todo |
| 7 | layer norms + scaling | todo |
| 8 | .model packaging | todo |

Conformance: P-EMBED, P-ATTN, P-LAYER, P-DET, P-LOAD against
`bostrom-23195000.graph`. Output must load in mr unchanged.

### 1.2 mi reverse (transformer → graph)

Extract a cybergraph projection from any HF / GGUF / ONNX model:

- weight tensors → particles (CID per tensor)
- layer connectivity → axons (cyberlinks between particles)
- attention patterns → semcon candidates (recurring motifs)
- tokenizer vocab → name particles
- config → frontmatter on the model's root particle

Output: a `.graph` file that the cyb browser can render and that mc
can re-compile (round-trip identity for held-out models).

### 1.3 Round-trip verification

- forward: known graph → mc → .model → mr → tokens, compared to Python reference (`~/git/cyber/analizer/compile_model.py`)
- reverse: HF model → mi → .graph → mc → .model → mr, output ε-equivalent to original at F32

Deliverable: `cyb-llm reverse Qwen/Qwen3-0.6B → qwen3.graph` and
`cyb-llm compile bostrom-23195000.graph → bostrom.model` both produce
loadable artifacts.

## Phase 2: distribution moat — p2p (2 sessions)

Goal: `cyb-llm fetch` faster than `ollama pull` for popular models.

### 2.1 BAO content addressing

- .cyb split into 256 KB BAO chunks, each with its own CID
- Delta downloads: model update = only changed chunks
- Qwen family shows 60%+ chunk dedup across sizes — every shared chunk fetched once

### 2.2 P2P swarm

- Every client that downloads also seeds
- Popularity scales speed (inverse of HTTP)
- Target: 70 B model 30+ min HTTP → < 10 min swarm

Deliverable: `cyb-llm fetch gemma-4-31b` measurably faster than HF and Ollama on the same connection.

## Phase 3: registry moat — model app store (3 sessions)

Goal: `.model` namespace on Bostrom = listings with CyberRank.

### 3.1 .model NFT = listing

```
model.gemma-4-31b
  ├── name, icon, description, author
  ├── neuron: bostrom1xyz... (author-controlled)
  ├── capabilities: [code, reasoning, vision]
  ├── license: Apache-2.0
  ├── versions:
  │     ├── v1: CID_q4_20260401 (immutable)
  │     └── latest → v2
  └── derived_from: model.gemma-4-31b-fp16
```

Listing = mutable. Each version = immutable CID. Both forward
(mc compile output) and reverse (mi reverse output) listings publishable.

### 3.2 Discovery

- `cyb-llm search "code generation rust"` → semantic query over listings + the cybergraph itself
- `cyb-llm fetch model.gemma-4-31b` → resolves latest CID
- `cyb-llm fetch model.gemma-4-31b@v1` → pins version
- soma manifest pins exact CIDs (deterministic install)

### 3.3 CyberRank for .model

- Dedicated CyberRank for the model namespace
- Usage-weighted: real inference, not benchmarks or marketing
- Higher rank → better discovery → more usage → flywheel

Deliverable: model app store with listings, versions, search, ratings,
and round-trip provenance (every listing links to its source graph or its source HF repo).

## Phase 4: attribution moat (2 sessions)

Goal: model authors and graph authors both earn from edge inference.

### 4.1 Model-as-neuron

Each listing's neuron writes cyberlinks during inference:

```
user:         question_CID  → answer_CID    (user's knowledge)
model_neuron: answer_CID    → model_CID     (model attribution)
graph_neuron: model_CID     → source_graph  (compile-source attribution, if from mc)
```

No protocol changes. Model and source-graph are both regular neurons earning regular CyberRank.

### 4.2 Evaluation

CyberRank in the .model namespace = leaderboard from real usage.
Replaces gameable benchmarks. Reputation cannot be purchased.

Deliverable: model leaderboard driven by edge inference, with
attribution flowing back to the graph slices that compiled into each model.

## Implementation checklist

### Done

- [x] .cyb format (weights + tokenizer + config, Q4 quantization at import)
- [x] honeycrisp backend skeleton (Metal + ANE + AMX + NEON + unimem)
- [x] wgpu+rs backend (cross-platform)
- [x] cpu reference backend (slow, always correct)
- [x] `mr run` (single model inference, multi-backend dispatch)
- [x] `mr status` / `mr bench` / `mr profile`
- [x] mi: HF / GGUF / ONNX / safetensors loaders → `Weights` table
- [x] mi: F16 / F32 → Q4_K quantization at pack time
- [x] mc: crate skeleton, .graph reader, .model writer scaffolding
- [x] Fused WGSL kernels: fused_norm_q4, fused_skip_norm
- [x] TurboQuant KV compression
- [x] soma manifest (model catalog with tiers)
- [x] file loader: mmap for files >1 GB
- [x] WGPU Q4_K embed shader for >4 GB embeddings
- [x] honeycrisp Q4_K row dequant (avoid 2.8 GB f16 upload)

### Phase 0 build (5 sessions, manifest-scoped)

qwen3-0.6b-abl (router):
- [ ] Resolve forward-pass garbled-output bug (highest-suspicion: QK-norm)
- [ ] 50-prompt verification vs Ollama (argmax-equivalence)
- [ ] Bench honeycrisp + wgpu+rs, target > 256 tok/s

qwen2.5-coder-1.5b-abl (code small):
- [ ] qwen2 attention-bias path verified on both backends
- [ ] 50-prompt golden vs Ollama (currently no verification)
- [ ] Bench both backends

qwen2.5-coder-14b-abl (code large):
- [ ] Fused Q4_K matmul → load under 10 s
- [ ] 5 GB RAM budget verified on M1 Pro
- [ ] 50-prompt golden vs Ollama, target > 22 tok/s honeycrisp

gemma-4-31b (general):
- [ ] Logit softcapping, sliding-window attention, K = V tying
- [ ] Fix layer-5 shape panic
- [ ] 10 GB RAM budget, 50-prompt golden vs Ollama

Surface:
- [ ] honeycrisp reads Q4 from .cyb without intermediate safetensors
- [ ] `cyb-llm serve` HTTP daemon (axum, `/v1/chat/completions`)
- [ ] Router: qwen3-0.6b classifies → routes to coder-1.5b / coder-14b / gemma
- [ ] Model hot-swap (load / unload within RAM budget, KV inheritance within family)
- [ ] Chat mode (`--chat`)
- [ ] brew formula
- [ ] `cyb-llm status` shows verified flag + tok/s per backend per manifest model

### Phase 1 build (4 sessions)

- [ ] mc passes 1–3 (vocab, semcons, architecture)
- [ ] mc passes 4–5 (embedding via RSVD, per-semcon attention)
- [ ] mc passes 6–8 (MLP, norms, packaging)
- [ ] mc conformance suite (P-EMBED, P-ATTN, P-LAYER, P-DET, P-LOAD)
- [ ] mi reverse: weight tensors → particles + axons → .graph file
- [ ] mi reverse: tokenizer vocab → name particles
- [ ] mi reverse: config → root-particle frontmatter
- [ ] round-trip test: HF → mi reverse → mc compile → mr → ε-equiv to source

### Phase 2 build (2 sessions)

- [ ] BAO chunking for .cyb files
- [ ] P2P swarm protocol (libp2p or custom)
- [ ] Swarm download + progress + seeding

### Phase 3 build (3 sessions)

- [ ] .model NFT schema on Bostrom
- [ ] `cyb-llm publish` (create / update listing, supports compiled and reversed origins)
- [ ] `cyb-llm search` (semantic query over listings)
- [ ] CyberRank for .model namespace
- [ ] Version management (latest pointer, pin by CID)

### Phase 4 build (2 sessions)

- [ ] Model neuron registration on Bostrom
- [ ] Attribution cyberlink creation during inference
- [ ] Source-graph attribution for compiled models
- [ ] Model leaderboard (`cyb-llm rank`)

## vs competition

| | HuggingFace | Ollama | Bittensor | cyb |
|---|---|---|---|---|
| Format | scattered files | GGUF | — | .cyb (single file, CID, quant included) |
| Distribution | HTTP | HTTP | — | p2p swarm with BAO dedup |
| Registry | centralized hub | none | none | .model NFT on Bostrom |
| Discovery | keyword | manual | staker voting | CyberRank (real usage) |
| Runtime | none | llama.cpp | cloud | rust + honeycrisp / wgpu+rs / cpu |
| Compile graph → model | none | none | none | mc (CT-1 spec) |
| Reverse model → graph | none | none | none | mi (HF / GGUF / ONNX → cybergraph) |
| Render | static page | none | none | cyb browser (graph IR, weights, traces) |
| Attribution | none | none | staker weights | model + graph neurons earn CyberRank |
| Lock-in | HF URL | Modelfile | subnet | none — CID is portable |

HuggingFace is GitHub for models. Ollama is Docker for models. cyb is the lifecycle.

## Lock-in cascade

```
Phase 0: runtime (4 manifest models perfect + fast)  ← switch for verified speed + RAM
Phase 1: bridge                                       ← switch for the only bidirectional graph ↔ model path
Phase 2: distribution                                 ← invite for faster downloads
Phase 3: app store                                    ← publish for storefront + ratings
Phase 4: attribution                                  ← earn usage-weighted reputation
Later:   monetization                                 ← once graph carries real economic weight
```

Each phase locks in the next. 16 sessions total to the closed cycle in
production. Earlier estimate (10 sessions) excluded the bridge and
treated runtime correctness as a 3-session check; the manifest scope
makes it 5 sessions of focused per-model work, and mc compile + mi
reverse add 4 more for the bridge.

The manifest is the discipline: refusing to grow it until the existing
four are perfect is what separates "demo" from "moat".
