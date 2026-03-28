---
tags: cyb, ai, architecture
crystal-type: entity
crystal-domain: superhuman
---

four-tier cognitive architecture for [[cyb]]. every [[cybernode]] runs a local mind. models are replaceable — functions are not.

## tier 0 — cognitive substrate

always loaded. parallel. total ≤ 2GB. latency < 100ms.

these functions run on EVERY input before anything else:

| function | what it does | why required |
|----------|-------------|--------------|
| router | classify input → select tier + specialist | without routing, every query hits the heaviest model |
| embedder | text → vector | memory lookup, similarity, dedup |
| urgency | score 0-1 + category | triage: alert vs background vs ignore |
| language | detect language + confidence | multi-language environment |
| intent | raw text → structured intent | canonical form before downstream |
| anomaly | detect deviation from baseline | sensor streams, log streams — always watching |
| context splitter | long input → chunks + salience | manage context window for higher tiers |
| safety | first-pass content filter | never bypassed, always first |

## tier 1 — fast specialists

sequential. 1-2GB each. load < 2s. structured tasks only.

| function | input → output |
|----------|---------------|
| code review | source → bugs, style issues, suggestions |
| SQL generation | natural language → query |
| translation | text + lang_pair → translated text |
| summarization | long document → structured summary |
| entity extraction | unstructured → people, places, dates, quantities |
| inventory parsing | informal update → structured delta |
| sensor interpretation | raw telemetry → human event + recommendation |
| financial parsing | transactions → structured records |
| search query generation | intent → optimized search queries |
| task decomposition | goal → ordered subtasks with dependencies |
| report formatting | structured data → formatted output |
| alert composition | event + context → message with severity |
| command parsing | natural language → action JSON |
| memory retrieval | query → relevant memory chunks |
| diff generation | before/after → changelog |
| schedule optimization | tasks + constraints → schedule |

## tier 2 — domain reasoners

sequential. 5-6GB each. load 3-6s. multi-step reasoning within a domain.

| domain | when activated |
|--------|---------------|
| general reasoning | causal inference, multi-step logic |
| code generation | implementation from spec, debugging |
| research analysis | synthesis across sources, hypothesis |
| project planning | timeline, resources, dependencies |
| social dynamics | people, conflict, communication strategy |
| financial analysis | budgets, projections, risk |
| infrastructure ops | devops, nodes, servers |
| biology / ecology | plants, soil, growing systems |
| legal / compliance | contracts, regulatory, structure |
| creative / comms | long-form writing, narrative |
| mathematics | calculation, proof, quantitative |
| vision | image/video understanding |

## tier 3 — deep synthesis

sequential. 9-10GB each. load 8-12s. cross-domain. rare.

| function | when |
|----------|------|
| master coder | large codebase changes, architecture decisions |
| strategic reasoner | cross-domain decisions, long-horizon planning |
| deep generalist | novel problems beyond any tier 2 domain |
| synthesis writer | whitepapers, complex documents |

## tier 4 — external oracle

API call. <5% of queries. never automatic.

| function | when |
|----------|------|
| frontier model | irreversible decisions, genuine novelty |
| live search | time-sensitive facts, current events |

## escalation rule

most queries never leave tier 1. tier 3 ~10-15%. tier 4 <5%. escalate only when lower tier is insufficient. every tier 4 call logged with justification.

## memory

| layer | what | persistence |
|-------|------|-------------|
| working | KV cache, active context | ephemeral |
| episodic | every input/output vectorized | persistent, grows |
| semantic | [[cybergraph]] — particles, links, knowledge | persistent, structured |
| procedural | tool definitions, [[rune]] MCP servers | static |

## mapping to cyber stack

| tier | cyber equivalent |
|------|-----------------|
| tier 0 | [[nox]] jets (routing, embedding, classification) |
| tier 1 | compiled transformers from domain subgraphs |
| tier 2 | compiled transformers from full subgraphs |
| tier 3 | compiled transformer from full [[cybergraph]] |
| tier 4 | host::oracle jet (API dispatch) |
| routing | [[tru]] focus flow |
| memory | [[cybergraph]] (episodic = [[cyberlinks]], semantic = [[particles]]) |

the 42-model architecture IS what cyber compiles to on constrained hardware. see [[cyberia/local mind]] for concrete model selection and RAM budget.
