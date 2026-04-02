---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .cyb model, model format, cyb model spec
---

# cyb-model — neural network in a [[cyb-format]] container

a complete neural network: architecture, weights, vocabulary, chat template, benchmarks — in one .cyb file ready for inference.

## required parts

| part | format | content |
|------|--------|---------|
| config | toml | architecture parameters |
| program | nox | forward pass (compiles to hardware) |
| weights | safetensors | tensor data |

## optional parts

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

[[parts]]
name = "config"
type = "model"
format = "toml"

[[parts]]
name = "program"
type = "model"
format = "nox"

[[parts]]
name = "vocab"
type = "model"
format = "toml"

[[parts]]
name = "chat"
type = "model"
format = "toml"

[[parts]]
name = "sampling"
type = "model"
format = "toml"

[[parts]]
name = "eval"
type = "model"
format = "toml"

[[parts]]
name = "card"
type = "model"
format = "md"

[[parts]]
name = "weights"
type = "model"
format = "safetensors"
size = 1200000000

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

## weight tensor naming

HuggingFace convention (canonical):

```
model.embed_tokens.weight
model.layers.{i}.self_attn.q_proj.weight
model.layers.{i}.self_attn.k_proj.weight
model.layers.{i}.self_attn.v_proj.weight
model.layers.{i}.self_attn.o_proj.weight
model.layers.{i}.mlp.gate_proj.weight
model.layers.{i}.mlp.up_proj.weight
model.layers.{i}.mlp.down_proj.weight
model.layers.{i}.input_layernorm.weight
model.layers.{i}.post_attention_layernorm.weight
model.norm.weight
lm_head.weight
```

import pipeline normalizes GGUF names (`blk.{i}.attn_q.weight`) to HF convention during .cyb creation.

## quantization

per-tensor quantization declared in tensor_index within the weights:

| method | bits/weight | use for |
|--------|:-:|---------|
| f16 | 16 | small critical models (router, intent) |
| q8_0 | 8.5 | math, vision (quality sensitive) |
| q4_0 | 4.5 | general LLMs (standard) |
| ternary | 1.58 | bitnet (native) |

KV cache compression via [[TurboQuant]] (PolarQuant + QJL) at runtime — not stored in .cyb.

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
