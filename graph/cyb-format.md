---
tags: cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .cyb format, cyb format, model format
---

# .cyb — universal model format

one file = complete model. no external dependencies. content-addressable.

replaces ONNX, GGUF, safetensors, CoreML, TensorRT with a single format designed for [[nox]] execution across any hardware.

## why not ONNX

ONNX stores a static computation graph with frozen shapes. flash attention cannot be expressed as ONNX nodes. every serious runtime rewrites the ONNX graph before execution anyway. ONNX has 4000+ operators, protobuf bloat, vendor-specific extensions.

a 10-parameter [[nox]] program replaces millions of ONNX nodes, compiles to optimal hardware-specific code, and supports dynamic shapes natively.

## file structure

```
model.cyb
├── header (CBOR)
│   ├── magic, version
│   ├── arch: "minimal_graph_native"
│   ├── params, context_len, modality
│   ├── rope_theta, rope_scaling, rope_factor
│   ├── source_cid (CID of source graph)
│   ├── quant_method, quant_source_cid
│   ├── model_lineage (base_cid, finetune_cid)
│   └── sampling_defaults (temperature, top_p, top_k)
│
├── vocabulary
│   ├── token_strings (length-prefixed)
│   ├── token_scores (f32, BPE merge priorities)
│   ├── special_tokens (BOS, EOS, PAD ids)
│   └── BPE merge rules
│
├── chat_template (per format: llama3 | chatml | mistral)
│
├── preprocessors[] (one per modality beyond text)
│   ├── image: {image_size, patch_size, mean, std}
│   ├── audio: {sample_rate, n_fft, hop_length, n_mels}
│   └── video: {frame_rate, max_frames, + image config}
│
├── nox_program (~50 lines)
│   ├── preprocess: raw input → embeddings
│   └── forward: embeddings → logits
│       (replaces ONNX graph entirely)
│       (compiles to AMX/ANE/Metal at first run)
│
├── tensor_index
│   └── per tensor: name, shape, dtype, quant_params,
│                   offset, chunk_cids[]
│
└── tensor_data (BAO chunked, 256KB blocks)
    each chunk: independently addressable by CID
    enables: parallel download, partial load,
             automatic deduplication across model family
```

## header (CBOR)

CBOR (RFC 8949) — compact binary, self-describing, extensible. parsers exist in every language. no custom binary format to maintain.

key fields:

| field | type | purpose |
|-------|------|---------|
| magic | bytes | `CYB\x02` (version 2) |
| arch | string | architecture class: `transformer_decoder`, `transformer_encoder`, `cnn_detector`, `diffusion_dit`, `tts_vits` |
| params | object | hidden_size, num_heads, kv_heads, head_dim, num_layers, intermediate_size, vocab_size |
| context_len | u32 | maximum context window |
| modality | string[] | `["text"]`, `["text", "vision"]`, `["audio"]` |
| rope_theta | f32 | rotary position embedding base |
| rope_scaling | string | `null`, `dynamic`, `yarn` |
| source_cid | CID | content address of the original unquantized model |
| quant_method | string | `null`, `q4_0`, `q8_0`, `f16`, `ternary` |
| quant_source_cid | CID | content address of the pre-quant version |
| model_lineage | object | `{ base_cid, finetune_cid, abliteration: bool }` |
| sampling_defaults | object | `{ temperature, top_p, top_k, repetition_penalty }` |

CID fields enable content-addressable provenance. a quantized model links back to its source. a fine-tune links to its base. the entire model family is a DAG.

## vocabulary

embedded tokenizer — no external tokenizer.json needed.

```
vocabulary:
  type: bpe | unigram | wordpiece | byte
  tokens: [length-prefixed UTF-8 strings]
  scores: [f32 array — BPE merge priorities]
  special_tokens:
    bos: 1
    eos: 2
    pad: 0
  merges: [pair indices for BPE]
```

for models without tokenizer (YOLO, BEATs, whisper GGML), this section is empty.

## chat_template

Jinja2 template string for instruction-tuned models:

```
chat_template:
  format: chatml
  template: |
    {%- for message in messages %}
    <|im_start|>{{ message.role }}
    {{ message.content }}<|im_end|>
    {%- endfor %}
  bos_token: "<|endoftext|>"
  eos_token: "<|im_end|>"
```

## preprocessors

for multimodal models — describes how to convert raw input to tensors:

```
preprocessors:
  - modality: image
    image_size: 448
    patch_size: 14
    mean: [0.485, 0.456, 0.406]
    std: [0.229, 0.224, 0.225]
  - modality: audio
    sample_rate: 16000
    n_fft: 400
    hop_length: 160
    n_mels: 80
```

## nox_program

the forward pass described as a [[nox]] program. ~50 lines replaces the entire ONNX graph. compiles to AMX/ANE/Metal/wgpu at first run.

```nox
transformer_decoder {
  layers: 28
  hidden: 1024
  heads: 16
  kv_heads: 8
  head_dim: 64
  rope_theta: 1e6
  norm: rmsnorm(eps=1e-6)
  attn: flash_attention
  ffn: swiglu(intermediate=3072)
  embed: token_embed(vocab=151936)
  output: linear(vocab=151936)
}
```

the runtime reads this and generates optimal code for the target hardware:
- Apple Silicon → AMX matmul + ANE for tier 0 + Metal for attention
- NVIDIA → CUDA kernels with tensor cores
- wgpu → cross-platform compute shaders
- CPU → NEON/AVX2 SIMD

nox_program IS the model architecture. weights are just data. changing the program changes how weights are used — this enables architecture experimentation without retraining.

### why nox_program instead of graph IR

| | ONNX / graph IR | nox_program |
|--|-----------------|-------------|
| size | millions of nodes | ~50 lines |
| flash attention | cannot express | `attn: flash_attention` |
| dynamic shapes | limited | native |
| hardware optimization | runtime rewrites graph | compiler generates optimal code |
| human readable | no (protobuf) | yes |
| extensible | add ONNX operator (committee) | add nox primitive |

## tensor_index

per-tensor metadata:

```
tensor_index:
  - name: "model.embed_tokens.weight"
    shape: [151936, 1024]
    dtype: f16
    quant: null
    offset: 0
    size: 311361536
    chunk_cids: [bafy...abc, bafy...def, ...]
  - name: "model.layers.0.self_attn.q_proj.weight"
    shape: [2048, 1024]
    dtype: q4_0
    quant: { block_size: 32 }
    offset: 311361536
    size: 1179648
    chunk_cids: [bafy...ghi, ...]
```

tensor names follow HuggingFace convention (canonical per [[llm]] spec model registry).

`chunk_cids[]` — content addresses of BAO chunks containing this tensor's data. enables:
- parallel download from multiple peers
- partial model loading (load only needed layers)
- deduplication: if two models share a tensor (e.g. same embedding table), they reference the same CIDs

## tensor_data (BAO chunked)

raw weight bytes, split into 256KB BAO chunks. each chunk independently addressable by CID.

BAO (BLAKE3 Authenticated and Organized) provides:
- verified streaming: verify each chunk independently, no need to download entire file
- parallel download: fetch chunks from different sources simultaneously
- content addressing: chunk CID = BLAKE3 hash of chunk content
- [[hemera]] integration: chunks stored and transmitted via hemera protocol

256KB chunk size balances:
- network: small enough for efficient p2p transfer
- disk: large enough to avoid excessive index overhead
- verification: BLAKE3 hash per chunk is fast (~1GB/s)

## content-addressable model ecosystem

the CID fields create a verifiable model supply chain:

```
base_model.cyb (CID: bafy...base)
  │
  ├─ finetune.cyb (CID: bafy...ft, lineage.base_cid = bafy...base)
  │
  └─ quantized.cyb (CID: bafy...q4, quant_source_cid = bafy...base)
       │
       └─ abliterated.cyb (CID: bafy...abl, lineage.finetune_cid = bafy...q4)
```

anyone can verify: this quantized model came from that base model. this fine-tune was trained on that dataset. the abliteration was applied to that checkpoint. no trust required — verify the CID chain.

shared tensors between model variants are stored once. a family of 5 model variants (base + 4 quants) stores ~1.2x the base size, not 5x.

## import pipeline

```
HuggingFace repo (safetensors/GGUF/ONNX)
    │
    ▼
cyb-llm import (download + parse)
    │
    ├─ extract architecture → nox_program
    ├─ extract tokenizer → vocabulary
    ├─ extract chat template → chat_template
    ├─ normalize tensor names → HF convention
    ├─ quantize if needed (PolarQuant/Q4/Q8)
    │
    ▼
cyb-llm pack → model.cyb
    │
    ├─ CBOR header
    ├─ vocabulary + chat_template + preprocessors
    ├─ nox_program
    ├─ tensor_index with chunk_cids
    └─ BAO chunked tensor_data
```

## runtime load

```
model.cyb
    │
    ▼
read CBOR header → architecture params
    │
    ▼
compile nox_program → hardware-specific kernels
    │ (cached after first compilation)
    ▼
map tensor_data into unimem Layout (zero-copy)
    │ weights tape: pinned IOSurface, visible to CPU/GPU/ANE
    │ scratch tape: per-token activations
    │ history tape: KV cache (TurboQuant compressed)
    │
    ▼
inference: nox_program dispatches compiled kernels
    │ AMX for matmul, NEON for norms/activations,
    │ Metal for attention, ANE for tier 0 models
    ▼
result
```

## versioning

| version | magic | description |
|---------|-------|-------------|
| 1 | `CYB\x01` | initial: custom binary header + raw tensor data |
| 2 | `CYB\x02` | CBOR header + nox_program + BAO chunks + CIDs |

version 1 readers that encounter version 2 files return a clear error. version 2 readers can load version 1 files with a compatibility shim.
