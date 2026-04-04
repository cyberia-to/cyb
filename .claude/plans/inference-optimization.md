# Inference Optimization

Status: approved
Created: 2026-04-01, updated 2026-04-04
Baseline: Metal 242 tok/s @ 50 tok context (Qwen3-0.6B Q4, M1 Pro)
Ollama baseline: 214 tok/s same model same hardware

## Done

- fused_norm_q4 shader + dispatch wired in wgpu decode loop
- fused_skip_norm shader + dispatch wired in wgpu decode loop
- Q4 quantization in import pipeline (F16→Q4 at pack time)
- .cyb stores Q4 weights (3.5× smaller files)
- HF tensor name normalization in import

## Current blocker

Metal backend reads Q4 from .cyb Graph — needs load_from_cyb that uses Graph directly instead of safetensors file on disk.

## Tier 1: dispatch reduction (wgpu)

| What | Saves | Status |
|------|-------|--------|
| fused_norm_q4 (norm+Q4 matmul) | 56 dispatches | done |
| fused_skip_norm (add+norm) | 28 dispatches | done |
| Port fused_qkv from Metal | 56 dispatches | pending |
| Port fused_gate_up from Metal | 28 dispatches | pending |
| Port fused_rope from Metal | 28 dispatches | pending |
| Total | 593 → ~400 dispatches | partial |

## Tier 2: batched forward + speculative

Prerequisite for all multi-token techniques. Biggest potential gain.

- Batched forward with causal mask in attention_encode
- Prompt lookup decoding (free draft from n-gram match, lossless)
- Speculative decode with tiny draft model (lossless)
- Target: 2-3× throughput on short context

## Tier 3: long context

KV cache dominates at seq > 500. TurboQuant already built.

| Context | Without TurboQuant | With TurboQuant | Gain |
|---------|-------------------|-----------------|------|
| 1K | ~146 tok/s | ~433 tok/s | 3× |
| 4K | ~43 tok/s | ~685 tok/s | 16× |

- Wire TurboQuant into decode attention path (QJL scoring, skip kv_expand)
- KV prefix cache for system prompts (20× TTFT improvement)
- H2O eviction for infinite streaming

## Tier 4: zero-copy + heterogeneous

- Unified buffer allocator (storageModeShared on Metal)
- ANE draft model for parallel speculative decode
- CPU-side async TurboQuant pipeline
- Fork/CoW KV for multi-turn sharing

## Quality

13/15 techniques lossless. Only TurboQuant (<0.5% PPL) and aggressive H2O eviction have measurable quality impact.
