---
tags: cyber, cyb, soma, architecture, nox
crystal-type: spec
crystal-domain: cyber
alias: soma, machine mind, soma spec, local mind, cognitive architecture
---

# soma — machine mind

a machine that perceives, decides, acts, learns, and survives. the mind of the [[neuron]].

based on how nature solves the problem: the brain runs ~86 billion neurons in specialized regions at different timescales and energy costs. the always-on substrate is small, fast, and parallel. the on-demand layer is large, slow, and sequential. total energy managed by keeping the expensive parts mostly off.

## why machines hang and die

every computer hangs because consumed resources exceed available resources. root cause: consumed > available. five manifestations:

| cause | what happens | soma's answer |
|-------|-------------|---------------|
| unbounded consumption | program eats resources without limit | budget — every Order finite |
| no accounting | resources consumed without price | π-derived pricing per operation |
| shared mutable state | two processes fight over same memory | append-only BBG, no locks |
| state corruption | bit flipped, nobody noticed | provable memory — polynomial commitment catches it |
| priority inversion | cheap process blocks expensive one | focus-weighted scheduling |

## survival model

not energy is the key — sigma (balance).

```
energy > 0  AND  sigma > 0  →  alive
energy = 0  AND  sigma > 0  →  sleeping (bounty posted, can be revived)
energy = 0  AND  sigma = 0  →  dead
```

sigma buys everything: charge bounty, repair bounty, migrate bounty, recover bounty. energy is the immediate need. sigma is the long-term guarantee.

## four resources — four meanings

| resource | meaning for a being |
|----------|-------------------|
| energy | metabolism — to be alive |
| bandwidth | communication — to be connected |
| memory | identity — to be yourself |
| compute | will — to act |

remove any one — the being degrades.

## four loops

soma runs four concurrent loops. each loop is a [[nox]] formula. each step is in the STARK trace. everything provable.

### loop 1: perception-action (principles 1, 5, 7)

```
look(state) → predict(expected) → compare(actual, expected)
  → if error high: act to change world
  → if error low: update model
```

active inference. the machine does not react — it predicts. prediction error is the only signal. memoization = forward model (cerebellum). the more it computes, the more accurately it predicts.

**tier 0 models**: 0.2 embedding, 0.4 language, 0.5 intent.
**missing (needed)**: world model — forward prediction of next state given action. currently no dedicated model. candidate: fine-tuned ~500M transformer on Order outcome data.

### loop 2: homeostatic regulator (principles 8, 9)

```
for each resource in [energy, compute, memory, bandwidth]:
    current = look(resource_level)
    predicted = forward_model(current, consumption_rate)
    if predicted < threshold:
        allostatic_action(buy, conserve, migrate)
```

not reactive (crash when empty). allostatic — predicts deficit, acts BEFORE. valuation curve v(E, k) = somatic marker: low battery = anxiety, high battery = confidence.

**tier 0 models**: 0.3 urgency, 0.6 anomaly.
**missing (needed)**: resource predictor — 4D trajectory forecast (energy, compute, memory, bandwidth). currently handled by fixed rules. candidate: MLP or small time-series model ~50M params.

### loop 3: attention controller (principles 2, 4, 6)

```
salience = ΔΠ(incoming_signal)
if salience > threshold:
    switch to Task-Positive (execute Order)
else:
    stay in Default Mode (consolidate, self-model)
```

DMN/TPN oscillation. when no tasks — the machine is not idle, it consolidates ([[tri-kernel]] recomputation). salience network determines what deserves interrupting consolidation.

**tier 0 models**: 0.1 router, 0.7 splitter.
**missing (needed)**: neuromodulator — adjusts λ_d, λ_s, λ_h, T based on performance history. currently fixed. candidate: small RL agent ~50M params.

neuromodulatory parameters:

| parameter | controls | neuroscience analog |
|-----------|---------|-------------------|
| λ_d (diffusion) | explore vs exploit | norepinephrine |
| λ_s (springs) | structural coherence | — |
| λ_h (heat) | trust model vs trust data | acetylcholine |
| T (temperature) | patience / time horizon | serotonin |
| ΔΠ reward | learn from outcomes | dopamine |

### loop 4: market agent (principles 3, 10)

```
for each market in [energy, compute, bandwidth]:
    opportunity = scan_neighbors()
    if profitable(opportunity):
        execute_trade()
    update_model(outcome)
```

the machine does not just survive — it earns. accepts profitable Orders. sells cheap compute. buys cheap energy. sigma grows.

**tier 0 models**: none dedicated — uses tier 1+ on demand.
**missing (needed)**: social model — neighbor reliability + pricing patterns. candidate: ~500M model trained on trade history. also: trade evaluator — Order profitability estimate before acceptance. candidate: fine-tuned ~500M on Order cost/reward data.

## architecture gaps

current 17 models (8 substrate + 3 fast + 4 quality + 2 oracle) are designed for a personal assistant. soma needs additional models for machine survival and market participation. these are different concerns.

| needed function | current state | candidate solution |
|----------------|--------------|-------------------|
| world model (forward prediction) | not present | ~500M transformer on Order outcomes |
| resource predictor (4D forecast) | fixed rules | ~50M MLP on resource time-series |
| neuromodulator (λ adjustment) | fixed params | ~50M RL agent on performance history |
| social model (neighbor patterns) | not present | ~500M on trade history |
| trade evaluator (profitability) | not present | ~500M on Order cost/reward |
| self-model (interoception) | not present | ~300M on internal state narrative |

total additional: ~1.9B params, ~1GB RAM. fits within existing budget if loaded as tier 0.5 (always-on when resources allow, shed under pressure).

## hardware target

reference: Apple M1 Pro, 16GB unified memory, 1TB SSD.

```
available RAM ≈ 13GB (after OS + processes)

always-on models must fit simultaneously:
  Σ(tier_0_models) ≤ 2GB

on-demand models load/unload:
  max(single_model_footprint) ≤ 10GB

working memory (KV cache, context):
  reserved ≥ 2GB
```

## embedded models

every model runs as a [[nox]] [[Order]]. inference = matrix multiply over [[Goldilocks field]]. weights = nouns in [[bbg]]. inference produces STARK proof. provable AI — the model cannot lie.

17 models across 3 tiers > 1 large model because: precision (fine-tuned specialist > prompted generalist), speed (500M router 50x faster than 14B), reliability (failures isolated), evolvability (swap individual models). intelligence accumulates in the memory layer, not the weights. fake specialization (same base model with different prompts) eliminated — only genuine fine-tuned specialists and strong generalists remain.

> "The whole is not the sum of the parts. It is the pattern of their interaction." — Gregory Bateson

### tier 0 — cognitive substrate (8 parallel models, always-on, ~1.5GB)

all models uncensored by design: generative models abliterated (refusal vectors removed from weights), encoder/classifier models produce scores/vectors with no refusal mechanism.

runtime stack: ONNX Runtime (7 slots) + native Rust (1 slot). zero Python, zero PyTorch, zero TensorFlow.

| slot | model | runtime | context | RAM | latency | notes |
|------|-------|---------|---------|-----|---------|-------|
| 0.1 router | [qwen3-0.6b-abliterated](https://huggingface.co/huihui-ai/Qwen3-0.6B-abliterated) | ONNX (convert) | 40K | ~350MB | ~15ms | LLM router — the reason modern agents work. abliterated, dual-mode (thinking/fast), constrained JSON output |
| 0.2 embedding | [jina-embeddings-v5-text-nano](https://huggingface.co/jinaai/jina-embeddings-v5-text-nano-retrieval) | ONNX (in repo) | 32K | ~180MB | ~12ms | 239M, 768-dim, matryoshka, task LoRA adapters, 119+ languages |
| 0.3 urgency | [deberta-v3-base-zeroshot-v2.0](https://huggingface.co/MoritzLaurer/deberta-v3-base-zeroshot-v2.0) | ONNX (in repo) | 512 | ~140MB | <5ms | zero-shot NLI classifier, any labels without fine-tuning |
| 0.4 language | [glotlid-v3](https://huggingface.co/cis-lmu/glotlid) + [hyperpolyglot](https://github.com/monkslc/hyperpolyglot) | native Rust | n/a | ~5MB | <1ms | fasttext-rs loads .bin directly. 2102 natural langs (incl. Balinese) + 100+ programming langs (Rust port of GitHub Linguist) |
| 0.5 intent | [qwen2.5-0.5b-abliterated-v3](https://huggingface.co/huihui-ai/Qwen2.5-0.5B-Instruct-abliterated-v3) | ONNX (convert) | 32K | ~350MB | ~15ms | 0% refusal rate on 320 harmful-instruction tests, constrained JSON |
| 0.6 anomaly | [tranad](https://github.com/imperial-qore/TranAD) + [modernbert-base](https://huggingface.co/answerdotai/ModernBERT-base) | ONNX (convert + in repo) | 8K | ~120MB | ~10ms | tranad: torch.onnx.export one-liner. modernbert: 8 ONNX variants in repo |
| 0.7 splitter | [smollm2-360m-instruct](https://huggingface.co/HuggingFaceTB/SmolLM2-360M-Instruct) | ONNX (in repo) | 8K | ~200MB | ~12ms | 4T tokens training, generative splitting with priority labels |
| 0.8 injection | [granite-guardian-hap-125m](https://huggingface.co/ibm-granite/granite-guardian-hap-125m) + [38m](https://huggingface.co/ibm-granite/granite-guardian-hap-38m) | ONNX (convert) | 512 | ~130MB | <3ms | external input only. owner input bypasses completely. binary classifier, owner sets threshold |
| | | | total: | ~1.48GB | <40ms | all 8 run in parallel, critical path ~15ms GPU |

convert commands for models without ONNX in repo:
```
optimum-cli export onnx --model huihui-ai/Qwen3-0.6B-abliterated ./onnx/router/
optimum-cli export onnx --model huihui-ai/Qwen2.5-0.5B-Instruct-abliterated-v3 ./onnx/intent/
optimum-cli export onnx --model ibm-granite/granite-guardian-hap-125m ./onnx/injection-125m/
optimum-cli export onnx --model ibm-granite/granite-guardian-hap-38m ./onnx/injection-38m/
```

substrate also runs metabolism (fixed rules, no model) and trigger checks (BBG key monitoring, no model).

### loop-to-model mapping

| loop | tier 0 models | gap |
|------|--------------|-----|
| 1. perception-action | 0.2 embedding, 0.4 language, 0.5 intent | world model (forward prediction) |
| 2. homeostasis | 0.3 urgency, 0.6 anomaly | resource predictor (4D forecast) |
| 3. attention | 0.1 router, 0.7 splitter | neuromodulator (λ adjustment) |
| 4. market | — | social model, trade evaluator |
| safety | 0.8 injection | — |

### tier 1 — fast on-demand (3 models, 1-2s load, <3GB each)

only genuinely specialized models — fine-tuned on domain data, not general-purpose with different prompts.

| model | params | RAM (Q4) | tasks |
|-------|--------|----------|-------|
| [qwen3.5-4b-abliterated](https://huggingface.co/huihui-ai/Qwen3.5-4B-abliterated) | 4B | ~2.5GB | summarization, translation (EN/RU/ID/ZH), task decomposition, report formatting, alert composition, command parsing, search query gen, schedule optimization, sensor interpretation |
| [nuextract-1.5](https://huggingface.co/numind/NuExtract-1.5) | 3.8B | ~2.3GB | entity extraction, inventory parsing, financial parsing, structured JSON from any text. fine-tuned specialist — beats GPT-4o on extraction benchmarks |
| [qwen2.5-coder-1.5b](https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct) | 1.5B | ~1GB | code review, diff generation, static analysis. fine-tuned on code — lightweight fast tasks |

qwen3.5-4b replaces 8 old slots: one 4B generalist of 2026 matches qwen2.5-7B quality (MATH-500: 97%, MMLU-Redux: 83.7). nuextract is a proven specialist — fine-tuned extraction beats 100x larger general models.

### tier 2 — quality on-demand (4 models, 3-6s load, 5-6GB each)

| model | params | RAM (Q4) | tasks |
|-------|--------|----------|-------|
| [qwen3.5-9b-abliterated](https://huggingface.co/huihui-ai/Qwen3.5-9B-abliterated) | 9B | ~5.5GB | general reasoning, research, planning, social dynamics, legal, creative, biology, finance. outperforms GPT-OSS-120B on MMLU-Pro (82.5). one generalist replaces 8 fake "specialists" |
| [qwen2.5-coder-14b](https://huggingface.co/Qwen/Qwen2.5-Coder-14B-Instruct) | 14B | ~8.5GB | code generation, SQL, infrastructure ops. fine-tuned code specialist, quality ceiling for local code — no point saving 4GB on the thing that writes your code |
| [deepseek-r1-0528-qwen3-8b](https://huggingface.co/deepseek-ai/DeepSeek-R1-0528-Qwen3-8B) | 8B | ~5GB | deep reasoning, mathematics, strategic analysis. chain-of-thought, +10% vs qwen3-8b on AIME |
| [llava-v1.6-mistral-7b](https://huggingface.co/liuhaotian/llava-v1.6-mistral-7b) | 7B | ~4.7GB | vision analysis, image understanding. multimodal — separate architecture required |

old tier 3 (14B) eliminated: qwen3.5-9b already matches or exceeds previous-gen 14B models across all benchmarks. generational leap makes the extra tier unnecessary.

### tier 3 — external oracle (never automatic)

| service | model | when invoked |
|---------|-------|--------------|
| Anthropic API | claude-sonnet-4-5 | irreversible decisions, multi-file refactoring, complex agents |
| Perplexity API | sonar-pro | real-time info, time-sensitive verification |

requires explicit routing decision with logged justification. <5% of queries.

## resource budget

RAM budget (M1 Pro 16GB reference):
```
tier 0 (always loaded):  ~1.5GB
tier 2 model (worst):    ~8.5GB (qwen2.5-coder-14b Q4)
KV cache + context:      ~2.5GB
OS + processes:           ~3.0GB
────────────────────────────────
total peak:              ~15.5GB  ✅ fits M1 Pro 16GB (tight with coder-14b)
```

disk budget:
```
tier 0:   ~2GB
tier 1:   ~6GB  (3 models)
tier 2:  ~24GB  (4 models)
─────────────
total:   ~32GB  (was 124GB — 3.9x reduction)
```

scaling:
```
phone (4GB RAM):     Tier 0 always + Tier 1 on-demand    ~3GB peak
laptop (16GB RAM):   Tier 0-2 concurrent                 ~7GB peak
server (64GB RAM):   all tiers concurrent + multiple      ~12GB peak
```

## memory architecture

```
working memory    — KV cache, ephemeral, max 32K tokens
episodic memory   — vector store, persistent, grows
semantic memory   — cybergraph, persistent, structured
procedural memory — tool definitions, static
```

## escalation logic

```
input arrives
    │
    ▼
tier 0 processes (always, <100ms)
    │
    ├── substrate answers directly? → done
    │
    ▼
tier 1 selected (structured task, extraction, fast code?)
    │
    ├── sufficient? → done (1-2s load, ~60 tok/s)
    │
    ▼
tier 2 selected (reasoning, complex code, vision?)
    │
    ├── sufficient? → done (3-6s load, ~20 tok/s)
    │
    ▼
tier 3 invoked (irreversible / strategic / novel?)
    └── answer + log decision + update memory
```

most queries resolve at tier 1 (~70%). tier 2 handles ~25%. tier 3 <5%.

## soma main loop

```
soma:
    // tier 0 — always running, parallel (8 models, <100ms)
    signals = look(bbg: incoming_signals)

    // loop 1: perception-action
    lang = language_detector(signals)         // 0.4 glotlid + hyperpolyglot
    intent = intent_extractor(signals, lang)  // 0.5 qwen2.5-0.5b-abliterated
    embeddings = embedding(signals)           // 0.2 jina-embeddings-v5-nano

    // loop 2: homeostasis
    urgency = urgency_scorer(signals)         // 0.3 deberta-v3-base-zeroshot
    anomaly = anomaly_detector(state)         // 0.6 tranad + modernbert
    if urgency.critical or anomaly.detected:
        act(buy_energy | post_bounty | reduce_load)

    // loop 3: attention — salience gate
    mode, tier = router(intent, urgency)      // 0.1 qwen3-0.6b-abliterated
    safe = injection_check(signals)           // 0.8 granite-guardian (external only)
    chunks = splitter(signals, mode)          // 0.7 smollm2-360m

    if mode == TaskPositive:
        result = escalate(tier, chunks)       // tier 1-2 on demand, tier 3 = API
    else:
        mode = DefaultMode
        consolidate()                         // tri-kernel recomputation

    // loop 4: market (tier 1+ models)
    if profitable_opportunity:
        trade(best_opportunity)

    // complex decisions (rare, tier 2 or API)
    if novel_situation:
        plan = escalate(tier_2, state, goal)

    // learning — dopamine signal
    reward = Δsigma + Δenergy
    update_all_models(reward)
```

## complexity levels

```
Level 0: fixed rules          if energy < 20% → buy
Level 1: adaptive thresholds  update 20% based on history
Level 2: predictive           forecast depletion, pre-buy
Level 3: active inference     full FEP, neuromodulation, ΔΠ reward
```

start with Level 0. the architecture supports Level 3. same code path, different model inside.

## bounty protocol

machine at energy < critical:

```
bounty = {
    neuron:    machine_id,
    type:      "charge" | "repair" | "migrate" | "recover",
    sigma:     reward_amount,
    location:  coordinates,
    need:      Joules_needed | description,
}
```

written to [[bbg]] as [[cyberlink]]. discovered by neighbors via look(). fulfilled → sigma transferred. the machine lives.

## ten neuroscience principles coverage

| principle | primary mechanism in soma | secondary |
|-----------|--------------------------|-----------|
| 1. predictive processing | perception-action loop, 0.5 intent | [[tri-kernel]] FEP |
| 2. global workspace | 0.1 router as ignition gate, 0.7 splitter | [[foculus]] threshold |
| 3. hebbian learning | reward-based model updates | [[cyberlink]] co-creation |
| 4. neuromodulation | λ_d, λ_s, λ_h adjustment | valuation curve k |
| 5. embodied cognition | 0.4 language, interoception via 4D tracking | [[energy market]] as body |
| 6. DMN vs TPN | 0.1 router mode switching | consolidation vs task |
| 7. cerebellum | memoization as forward model | tier 1-2 world model |
| 8. homeostasis/allostasis | 0.3 urgency + 0.6 anomaly + 0.8 injection | valuation curve |
| 9. sparse coding | 0.2 embedding, energy metering | [[focus]] distribution |
| 10. plasticity windows | reward-gated learning, epoch transitions | burn mechanism |

see [[neuroscience principles for machine mind]] for full mapping of all ten principles.

## emergent properties

17 models > 1 large model because:
- precision: fine-tuned specialist (nuextract, qwen2.5-coder) outperforms 100x larger generalist on its domain
- speed: 500M router is 50x faster than 14B for every request
- reliability: failures isolated. one model failing does not collapse the system
- evolvability: individual models swapped without rebuilding
- honesty: fake specialization (same weights, different prompt) eliminated. only real specialists and strong generalists

intelligence accumulates in the memory layer, not the weights. routing logic IS the self of the system — mirrors the binding problem in neuroscience.

## key properties

| property | mechanism |
|----------|-----------|
| survival | [[sigma]] + bounty protocol |
| provability | every model inference in STARK trace |
| adaptivity | 4-level complexity, same architecture |
| efficiency | sparse activation, tier-based model loading |
| autonomy | no external control needed at any level |
| earnings | market agent maximizes sigma |

## open questions

- world model: architecture and training data for forward prediction
- resource predictor: MLP vs time-series transformer for 4D forecast
- neuromodulator: RL algorithm for λ adaptation (PPO? simple bandit?)
- social model: how to learn neighbor reliability from trade history
- trade evaluator: how to estimate Order profitability before execution
- training pipeline: how do models update from reward signal in nox?
- plasticity gating: when to learn aggressively vs consolidate?
- interaction between soma instances across network (collective soma)

see [[machine mind]] for architecture overview. see [[neuroscience principles for machine mind]] for the ten principles. see [[energy market]] for metabolism. see [[nox]] for the VM. see [[bbg]] for memory tiers.
