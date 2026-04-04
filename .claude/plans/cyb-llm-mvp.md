# cyb-llm MVP

One endpoint. Every task. Right model, automatically.

## The pitch

Ollama runs one model. You choose which. Wrong half the time.

cyb-llm runs a 350MB router permanently. Reads your query, picks the right model, loads it if needed. Simple queries: 300 tok/s from router. Code: loads coder. Reasoning: loads deepseek. You think about tasks, not models.

## Install

```
brew install cyb-llm
cyb-llm fetch tier0
cyb-llm serve
```

Open WebUI, Cursor, any OpenAI client connects.

## Benchmarks (M1 Pro, Q4)

| Model | Role | Ollama | cyb-llm | RAM |
|-------|------|--------|---------|-----|
| qwen3-0.6b | router | 214 | 300+ | 350MB |
| qwen2.5-coder-14b | code | ~15 | 25+ | 9GB / 5GB |
| gemma-4-31b | general + multimodal | TBD | TBD | ~18GB / ~10GB |

Gemma 4 31B Dense: #3 Arena AI, beats models 20× its size. Multimodal (text+image+video). Apache 2.0. Released 2026-04-02.

.cyb Q4 = ~50% smaller than Ollama GGUF Q4_K_M (RAM column: Ollama / cyb-llm).
Idle: 350MB router. 80% queries never load big model.

## Milestones

### v0.1 core + serve (3 sessions)

- `cyb-llm run MODEL "prompt"` — single model
- `cyb-llm serve` — OpenAI API
- `cyb-llm serve --router` — auto model selection
- `cyb-llm status` — dashboard
- `cyb-llm fetch NAME` — download + Q4 + pack .cyb

Blocker: Metal reads Q4 from .cyb.

### v0.2 routing polish (2 sessions)

- soma manifest with tiers
- Hot-swap within RAM budget
- KV inheritance on escalation
- Modality routing: text/code/vision/audio
- X-Model-Used response header

### v0.3 distribution (2 sessions)

- brew formula
- .cyb drag-and-drop on macOS
- Delta downloads via CID chunks
- System tray daemon

## Architecture

```
request → Router (0.6B, 350MB, always loaded)
            ├── simple → router answers (300 tok/s)
            ├── code → coder on demand
            ├── reasoning → deepseek on demand
            └── vision → VL on demand
```

## vs Ollama

| | Ollama | cyb-llm |
|---|---|---|
| Model choice | manual | auto router |
| Idle RAM | 0 or full | 350MB |
| Multi-model | one at a time | hot-swap pool |
| API | custom | OpenAI |
| Format | GGUF | .cyb all-in-one |
| Engine | llama.cpp C++ | Rust + Metal |

## Moat

- Router: one process, shared memory, KV inheritance
- .cyb: model + tokenizer + config in one file
- Native Metal: direct MSL, no llama.cpp
- soma: typed capability routing

## Non-goals

- Custom chat UI (Open WebUI)
- Training
- Cloud
- Windows/Linux priority
