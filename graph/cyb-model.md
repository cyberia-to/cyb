---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .model, model format, cyb model spec
---

# .model — neural network in [[cyb-format]]

.model is a [[cyb-format]]-compatible extension. a .model file IS a .cyb file — same three rules, same parsing, same CLI. the extension tells tools and humans: this container holds a neural network.

one file = architecture + weights + vocabulary + chat template + benchmarks. ready for inference.

## required files

| part | format | content |
|------|--------|---------|
| config | toml | architecture parameters |
| program | nox | forward pass (compiles to hardware) |
| weights | tensors | weight data (own format, see below) |

## optional files

| part | format | content |
|------|--------|---------|
| vocab | toml | tokenizer vocabulary + merge rules |
| chat | toml | chat template + special tokens |
| sampling | toml | default inference parameters |
| preprocess | toml | image/audio/video preprocessing |
| eval | toml | benchmark results |
| card | md | model documentation |

## example

```
[cyb]
version = 2
types = ["model"]
name = "qwen3-0.6b-abliterated"

[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
method = "abliteration"

[[files]]
name = "config"
type = "model"
format = "toml"

[[files]]
name = "program"
type = "model"
format = "nox"

[[files]]
name = "vocab"
type = "model"
format = "toml"

[[files]]
name = "chat"
type = "model"
format = "toml"

[[files]]
name = "sampling"
type = "model"
format = "toml"

[[files]]
name = "eval"
type = "model"
format = "toml"

[[files]]
name = "card"
type = "model"
format = "md"

[[files]]
name = "weights"
type = "model"
format = "tensors"
size = 1200000000

[files.tensors]
"model.embed_tokens.weight" = { shape = [151936, 1024], dtype = "f16", offset = 0, size = 311361536 }
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], dtype = "q4_0", offset = 311361536, size = 1179648 }

~~~config
model_type = "qwen3"
architecture = "Qwen3ForCausalLM"
hidden_size = 1024
num_attention_heads = 16
num_key_value_heads = 8
num_hidden_layers = 28
intermediate_size = 3072
vocab_size = 151936
max_position_embeddings = 40960
rope_theta = 1000000.0
rms_norm_eps = 0.000001
tie_word_embeddings = true

~~~program
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

~~~vocab
type = "bpe"
vocab_size = 151936
merges_count = 151387
bos_id = 151643
eos_id = 151645
pad_id = 151643

~~~chat
format = "chatml"
template = """
{%- for message in messages %}
<|im_start|>{{ message.role }}
{{ message.content }}<|im_end|>
{%- endfor %}
"""
bos_token = "<|endoftext|>"
eos_token = "<|im_end|>"

~~~sampling
temperature = 0.7
top_p = 0.9
top_k = 40
repetition_penalty = 1.1
max_tokens = 2048

~~~eval
[needle_in_haystack]
context = 104000
score = 0.991

[mmlu_pro]
score = 0.724

[humaneval]
pass_at_1 = 0.652

~~~card
# qwen3-0.6b-abliterated

0.6B parameter model for routing and intent classification.
soma tier 0 — always on, <15ms latency.

abliterated: refusal vectors removed from weights.
0% refusal rate on 320 harmful-instruction tests.

source: huihui-ai/Qwen3-0.6B-abliterated
license: Apache 2.0

~~~weights
<1.2GB safetensors binary data>
```

## config part

architecture parameters. tensor name convention: HuggingFace (canonical per [[llm]] model registry).

| field | type | description |
|-------|------|-------------|
| model_type | string | runtime dispatch key: qwen3, llama, bitnet, mimo, whisper, yolo, ... |
| architecture | string | HF class: Qwen3ForCausalLM, LlamaForCausalLM, ... |
| hidden_size | u32 | embedding dimension |
| num_attention_heads | u32 | query heads |
| num_key_value_heads | u32 | KV heads (GQA) |
| num_hidden_layers | u32 | transformer layers |
| intermediate_size | u32 | FFN hidden dimension |
| vocab_size | u32 | vocabulary size |
| max_position_embeddings | u32 | maximum context length |
| rope_theta | f32 | rotary position embedding base |
| rms_norm_eps | f32 | normalization epsilon |

## nox program

the forward pass as a [[nox]] program. replaces ONNX graph.

the runtime reads the program and compiles to optimal code for available hardware:
- Apple Silicon → [[acpu]] AMX + [[rane]] ANE + [[aruminium]] Metal
- NVIDIA → CUDA tensor cores
- wgpu → cross-platform shaders
- CPU → NEON/AVX2 SIMD

all on [[unimem]] zero-copy physical memory when available.

### why not ONNX

| | ONNX | nox program |
|--|------|-------------|
| size | millions of nodes | ~50 lines |
| flash attention | cannot express | `attn: flash_attention` |
| dynamic shapes | limited | native |
| hardware optimization | runtime rewrites graph | compiler generates optimal code |
| human readable | no (protobuf) | yes (in .cyb file) |

### supported architectures

| nox construct | models |
|--------------|--------|
| transformer_decoder | qwen, llama, mistral, deepseek, phi, bitnet |
| transformer_encoder | bert, deberta, modernbert, jina |
| encoder_decoder | whisper |
| cnn_detector | yolo |
| diffusion_dit | flux, wan2.2 |
| tts_vits | piper |
| tts_autoregressive | xtts |
| moe_decoder | mimo, mixtral |

## vocab part

embedded tokenizer. no external tokenizer.json needed.

```toml
type = "bpe"           # bpe | unigram | wordpiece | byte
vocab_size = 151936
merges_count = 151387
bos_id = 151643
eos_id = 151645
pad_id = 151643
```

for full vocabulary data (token strings + merge rules), the runtime loads from the vocabulary section of the weights or from a companion tokenizer embedded in the binary zone.

## preprocessors (multimodal)

```toml
[image]
image_size = 448
patch_size = 14
mean = [0.485, 0.456, 0.406]
std = [0.229, 0.224, 0.225]

[audio]
sample_rate = 16000
n_fft = 400
hop_length = 160
n_mels = 80
```

## weights format: tensors

own binary format. tensor index in TOML frontmatter, raw data in binary zone. no dependency on GGUF or safetensors parsers.

### tensor index

each tensor declared in frontmatter under `[files.tensors]`:

```toml
[[files]]
name = "weights"
format = "tensors"
size = 1200000000

[files.tensors]
"model.embed_tokens.weight" = { shape = [151936, 1024], dtype = "f16", offset = 0, size = 311361536 }
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], dtype = "q4_0", offset = 311361536, size = 1179648 }
"model.layers.0.self_attn.k_proj.weight" = { shape = [1024, 1024], dtype = "q4_0", offset = 312541184, size = 589824 }
"model.layers.0.input_layernorm.weight" = { shape = [1024], dtype = "f32", offset = 313131008, size = 4096 }
```

per-tensor fields:
- `shape` — dimensions
- `dtype` — data type (see table below)
- `offset` — byte offset from start of `~~~weights` binary zone
- `size` — byte count

binary zone after `~~~weights` is raw concatenated tensor data. 64-byte aligned per tensor (for AMX/DMA).

### dtypes

| dtype | bits/value | block_size | description |
|-------|:-:|:-:|-------------|
| f32 | 32 | 1 | full precision (norms, biases) |
| f16 | 16 | 1 | half precision (small critical models) |
| bf16 | 16 | 1 | brain float |
| q8_0 | 8.5 | 32 | 8-bit quantized (math, vision) |
| q4_0 | 4.5 | 32 | 4-bit quantized (general LLMs) |
| q4_k | 4.5 | 256 | 4-bit K-quant (better quality) |
| ternary | 1.58 | — | 1.58-bit (bitnet native) |

block quantization (q4_0, q8_0): each block of `block_size` values has one f16 scale factor + packed integers. byte layout matches GGUF block format for compatibility.

### tensor naming

HuggingFace convention is canonical. this is the only naming scheme .model files use:

```
model.embed_tokens.weight
model.layers.{i}.self_attn.q_proj.weight
model.layers.{i}.self_attn.k_proj.weight
model.layers.{i}.self_attn.v_proj.weight
model.layers.{i}.self_attn.o_proj.weight
model.layers.{i}.self_attn.q_norm.weight
model.layers.{i}.self_attn.k_norm.weight
model.layers.{i}.mlp.gate_proj.weight
model.layers.{i}.mlp.up_proj.weight
model.layers.{i}.mlp.down_proj.weight
model.layers.{i}.input_layernorm.weight
model.layers.{i}.post_attention_layernorm.weight
model.norm.weight
model.lm_head.weight
```

import pipeline converts other naming conventions (GGUF `blk.{i}.attn_q`, ONNX paths) to HF at pack time. runtime only sees HF names.

### import conversion table (GGUF → HF)

| GGUF | HF |
|------|-----|
| token_embd.weight | model.embed_tokens.weight |
| output_norm.weight | model.norm.weight |
| output.weight | model.lm_head.weight |
| blk.{i}.attn_norm.weight | model.layers.{i}.input_layernorm.weight |
| blk.{i}.attn_q.weight | model.layers.{i}.self_attn.q_proj.weight |
| blk.{i}.attn_k.weight | model.layers.{i}.self_attn.k_proj.weight |
| blk.{i}.attn_v.weight | model.layers.{i}.self_attn.v_proj.weight |
| blk.{i}.attn_output.weight | model.layers.{i}.self_attn.o_proj.weight |
| blk.{i}.ffn_norm.weight | model.layers.{i}.post_attention_layernorm.weight |
| blk.{i}.ffn_gate.weight | model.layers.{i}.mlp.gate_proj.weight |
| blk.{i}.ffn_up.weight | model.layers.{i}.mlp.up_proj.weight |
| blk.{i}.ffn_down.weight | model.layers.{i}.mlp.down_proj.weight |

### why own format instead of safetensors/GGUF

safetensors does not support block quantization (Q4, Q8). storing quantized tensors in safetensors loses dtype metadata — runtime cannot distinguish Q4 from raw U8.

GGUF supports quantization but brings its own naming convention, metadata format, and architectural assumptions. dependency on GGUF = dependency on llama.cpp decisions.

own format: tensor index in TOML (readable, editable), binary data with per-tensor dtype and alignment, HF naming canonical, zero external dependencies.

KV cache compression via TurboQuant (PolarQuant + QJL) happens at runtime — not stored in model file.

## model lineage

content-addressable provenance chain:

```toml
[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
method = "abliteration + q4_0"
```

the full chain: base model → fine-tune → quantize → abliterate. when stored as [[particles]] in [[hemera]], each step is content-addressable and verifiable.

## runtime load

```
model.cyb
  → parse frontmatter (TOML)
  → compile nox program → hardware kernels (cached)
  → mmap weights into unimem Layout (zero-copy)
  → inference ready
```

see [[llm]] for memory architecture, [[unimem]] for zero-copy pipeline, [[TurboQuant]] for KV cache compression.
