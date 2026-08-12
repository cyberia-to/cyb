---
tags: cyb, core
alias: cyb pipeline, toolchain
---
the full cycle of [[intelligence]] — from [[cyberlink]] to [[inference]] and back

```
link → graph → .cyb → compile → inference
 ↑                                    │
 └────── reverse (model → graph) ─────┘
```

## stages

### link
[[neurons]] create and edit [[cyberlinks]] — the atomic units of [[knowledge]]. every link is a signed, staked assertion binding two [[particles]]

tools: [[cyb]] browser (react/, bevy/), scripting (nu/)

### render
the [[cybergraph]] visualized — pages, navigation, search, graph minimap

tools: [[optica]] publisher, [[cyb]] browser

### format
.cyb — single-file container for graph + weights + config. the universal exchange format between all stages

sections: config (TOML), graph IR (nodes + edges), tensor index, tensor data (Q4/Q8/F16/F32)

canonical tensor naming: HuggingFace style (`model.layers.0.self_attn.q_proj.weight`)

### reverse
extract computation graph from trained models. ONNX protobuf → IR nodes. safetensors → weight tensors. GGUF → quantized weights. all formats converge into one Graph

loaders: safetensors, GGUF, ONNX, .cyb

### compile
graph optimization and weight quantization at import time. F16/F32 weights → Q4_0 (4-bit, 3.5x compression). tensor name normalization. config extraction

pipeline: `cyb-llm import` → canonicalize + quantize + pack → .cyb

### inference
.cyb → GPU compute → tokens. three backend paths reading the same .cyb:

| backend | hardware | speed | status |
|---------|----------|-------|--------|
| Metal (MSL) | Apple GPU | 242 tok/s | production |
| wgpu (WGSL) | any GPU | 45 tok/s | production |
| ANE (CoreML) | Apple Neural Engine | TBD | planned |

decode loop: embed → (norm → QKV → RoPE → KV cache → attention → O proj → residual → norm → FFN → residual) × layers → LM head → sample

## the cycle

inference produces new [[knowledge]]. [[neurons]] observe the output, create new [[cyberlinks]], and the graph grows. the pipeline is a loop — not a line

## invariants

one format (.cyb) for all stages. one set of tensor names (HF canonical). one quantization (Q4_0) for all backends. no format conversion at runtime — all conversion happens at import

discover all [[concepts]]
