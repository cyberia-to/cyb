# cyb MVP: Model App Store

HuggingFace = registry without runtime. Ollama = runtime without registry.
First protocol that connects format → distribution → discovery → runtime → attribution wins.

## The vertical

```
.cyb format (quantization included, content-addressed)
    ↓
p2p distribution (BAO chunks, swarm download)
    ↓
model app store (.model NFT — listings, versions, capabilities)
    ↓
cyb-llm runtime (router → auto model selection → inference)
    ↓
attribution (model neuron creates cyberlinks alongside user)
    ↓
evaluation (CyberRank for .model namespace = usage-weighted ratings)
```

## Benchmarks (M1 Pro, Q4)

| Model | Role | Ollama | cyb-llm | RAM Ollama / cyb |
|-------|------|--------|---------|-----------------|
| qwen3-0.6b | router | 214 tok/s | 300+ | 700 / 350MB |
| qwen2.5-coder-14b | code | ~15 | 25+ | 9 / 5GB |
| gemma-4-31b | general | TBD | TBD | ~18 / ~10GB |

## Phase 0: runtime moat (3 sessions)

Ship: `brew install cyb-llm && cyb-llm fetch tier0 && cyb-llm serve`

### 0.1 Metal .cyb Q4 pipeline
- .cyb stores Q4 weights (quantize at import, not at load)
- Metal backend reads Q4 from .cyb Graph directly
- wgpu reads same Q4 (cross-platform fallback)
- Blocker: Metal load_from_cyb needs Graph→GPU path

### 0.2 CLI + serve
- `cyb-llm run MODEL "prompt"` → Metal inference
- `cyb-llm serve` → OpenAI-compatible API (/v1/chat/completions)
- `cyb-llm status` → model dashboard (tok/s, RAM, quality)
- `cyb-llm fetch NAME` → download HF → quantize → pack .cyb

### 0.3 Router
- Router model (0.6B, 350MB) always loaded
- Classifies: simple/code/reasoning/vision/audio
- `cyb-llm serve --router` → one endpoint, auto model selection
- 80% queries handled by router at 300 tok/s
- Hot-swap specialists within RAM budget
- KV inheritance on escalation (same tokenizer family)

Deliverable: benchmark vs Ollama + working serve + Open WebUI.

## Phase 1: distribution moat (2 sessions)

Ship: `cyb-llm fetch` faster than `ollama pull`.

### 1.1 BAO content addressing
- .cyb split into 256KB BAO chunks, each with CID
- Delta downloads: model update = only changed chunks
- Qwen family: 60%+ chunk dedup across sizes

### 1.2 P2P swarm
- Every client that downloads = seeds
- Popularity = speed (inverse of HTTP)
- 70B model: 30+ min HTTP → <10 min swarm

Deliverable: `cyb-llm fetch gemma-4-31b` demonstrably faster than HF/Ollama.

## Phase 2: registry moat — model app store (3 sessions)

Ship: `.model` namespace on Bostrom = model listings with CyberRank.

### 2.1 .model NFT = listing
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

Listing = mutable. Each version = immutable CID.

### 2.2 Discovery
- `cyb-llm search "code generation rust"` → semantic query
- `cyb-llm fetch model.gemma-4-31b` → resolves latest CID
- `cyb-llm fetch model.gemma-4-31b@v1` → pins version
- soma manifest pins exact CIDs (deterministic)

### 2.3 CyberRank for .model
- Dedicated CyberRank for model namespace
- Usage-weighted: real inference, not benchmarks or marketing
- Higher rank → better discovery → more usage → flywheel

Deliverable: model app store with listings, versions, search, ratings.

## Phase 3: attribution moat (2 sessions)

Ship: model authors earn reputation from edge inference.

### 3.1 Model-as-neuron
Each listing's neuron creates cyberlinks during inference:
```
user:         question_CID → answer_CID   (user's knowledge)
model_neuron: answer_CID   → model_CID    (attribution)
```
No protocol changes. Model = regular neuron. Earns regular CyberRank.

### 3.2 Evaluation
CyberRank in .model namespace = leaderboard from real usage.
Replaces gameable benchmarks. Non-purchasable reputation.

Deliverable: model leaderboard driven by real edge inference.

## Implementation checklist

### Exists (ready or 90%+)
- [x] .cyb format (weights + tokenizer + config, Q4 quantization at import)
- [x] Metal backend (242 tok/s qwen3-0.6b, native MSL)
- [x] wgpu backend (cross-platform fallback)
- [x] `cyb-llm run` (single model inference)
- [x] `cyb-llm status` (model dashboard with tok/s, quality)
- [x] `cyb-llm import` + `fetch` (download + quantize + pack .cyb)
- [x] Fused WGSL kernels (fused_norm_q4, fused_skip_norm)
- [x] TurboQuant KV compression
- [x] soma manifest (model catalog with tiers)

### Phase 0 build (3 sessions)
- [ ] Metal reads Q4 from .cyb Graph (current blocker)
- [ ] `cyb-llm serve` HTTP daemon (axum, /v1/chat/completions)
- [ ] Router: classify prompt → route to model
- [ ] Model hot-swap (load/unload within RAM budget)
- [ ] Chat mode (`--chat`)
- [ ] brew formula

### Phase 1 build (2 sessions)
- [ ] BAO chunking for .cyb files
- [ ] P2P swarm protocol (libp2p or custom)
- [ ] Swarm download + progress + seeding

### Phase 2 build (3 sessions)
- [ ] .model NFT schema on Bostrom
- [ ] `cyb-llm publish` (create/update listing)
- [ ] `cyb-llm search` (semantic query over listings)
- [ ] CyberRank for .model namespace
- [ ] Version management (latest pointer, pin by CID)

### Phase 3 build (2 sessions)
- [ ] Model neuron registration on Bostrom
- [ ] Attribution cyberlink creation during inference
- [ ] Model leaderboard (`cyb-llm rank`)

## vs competition

| | HuggingFace | Ollama | Bittensor | cyb |
|---|---|---|---|---|
| Format | scattered files | GGUF | — | .cyb (all-in-one, CID) |
| Distribution | HTTP | HTTP | — | p2p swarm |
| Registry | centralized hub | none | none | .model NFT |
| Discovery | keyword | manual | staker voting | CyberRank (usage) |
| Runtime | none | llama.cpp | cloud | Rust + Metal |
| Attribution | none | none | staker weights | .model neuron |
| Lock-in | HF URL | Modelfile | subnet | none (CID portable) |

HuggingFace = GitHub for models. We build the App Store.

## Lock-in cascade

```
Phase 0: format + runtime     ← switch for speed + RAM
Phase 1: distribution          ← invite for faster downloads
Phase 2: app store             ← publish for storefront + ratings
Phase 3: attribution           ← earn usage-weighted reputation
Later:   monetization           ← once graph has real data
Later:   compiled transformers  ← full lifecycle closure
```

Each phase locks in the next. 10 sessions total to app store.
