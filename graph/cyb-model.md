---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .model, model format, cyb model spec
---

# .model — neural network in [[format]]

.model is a [[format]]-compatible extension. a .model file IS a .cyb file — same three rules, same parsing. the extension tells tools and humans: this container holds a neural network.

one file = architecture + weights + vocabulary + chat template + benchmarks. ready for inference.

## five files

| name | format | what it does |
|------|--------|-------------|
| config | toml | metadata: name, license, parameters, languages |
| program | nox | entire pipeline: input → output (18 instructions, asm) |
| tensors | toml | tensor index: names, shapes, dtypes, offsets |
| vocab | toml | full vocabulary: tokens + merge rules |
| weights | tensors | raw weight data (binary, page-aligned) |

five files. that is it. model runs.

program is [[nox]] assembly (18 instructions). describes the ENTIRE pipeline — not just forward pass. input formatting, tokenization, embedding, forward, sampling, decoding — all in one program. no separate chat template, no separate sampling config, no separate preprocessor. the program IS the behavior.

[[trident]] compiles high-level architecture descriptions into nox. human writes trident, compiler produces nox, .model stores nox. like C → compiler → machine code → .exe.

## optional files

| name | format | what it adds |
|------|--------|-------------|
| source | trident | human-readable source of the nox program |
| eval | toml | benchmark results |
| card | md | model documentation |

## example

```
[cyb]
types = ["model"]
name = "qwen3-0.6b-abliterated"
parameters = "0.6B"
license = "Apache-2.0"
languages = ["en", "zh", "ru"]

[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
method = "abliteration"

[[files]]
name = "config"
format = "toml"

[[files]]
name = "program"
format = "nox"

[[files]]
name = "tensors"
format = "toml"

[[files]]
name = "vocab"
format = "toml"

[[files]]
name = "eval"
format = "toml"

[[files]]
name = "card"
format = "md"

[[files]]
name = "weights"
format = "tensors"
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
context_length = 32768

[tokenizer]
type = "bpe"
vocab_size = 151936
bos_id = 151643
eos_id = 151645
pad_id = 151643

~~~program
; qwen3-0.6b-abliterated — full inference pipeline
; compiled from trident, 18 nox instructions

; input: raw text → output: generated text

; step 1: format input
chatml_format bos=151643 eos=151645

; step 2: tokenize
bpe_encode vocab=151936

; step 3: embed + forward (28 layers)
token_embed dim=1024 vocab=151936
rope theta=1e6 head_dim=64
transformer_decode layers=28 hidden=1024 heads=16 kv_heads=8 ffn=swiglu:3072 norm=rmsnorm:1e-6 attn=flash

; step 4: sample
linear vocab=151936
sample top_p=0.9 temperature=0.7

; step 5: decode
bpe_decode

~~~tensors
"model.embed_tokens.weight" = { shape = [151936, 1024], dtype = "f16", offset = 0, size = 311361536 }
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], dtype = "q4_0", offset = 311361536, size = 1179648 }
"model.layers.0.self_attn.k_proj.weight" = { shape = [1024, 1024], dtype = "q4_0", offset = 312541184, size = 589824 }
"model.layers.0.input_layernorm.weight" = { shape = [1024], dtype = "f32", offset = 313131008, size = 4096 }

~~~vocab
[tokens]
0 = "<unk>"
1 = "<s>"
2 = "</s>"
3 = "▁the"
4 = "▁of"
# ... 151936 tokens total

[merges]
0 = ["▁", "t"]
1 = ["▁t", "h"]
# ... 151387 merges total

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
<1.2GB tensor binary data>
```

## config

metadata about the model. NOT architecture — architecture is in the nox program.

| field | type | description |
|-------|------|-------------|
| model_type | string | qwen3, llama, bitnet, mimo, whisper, yolo |
| architecture | string | HF class: Qwen3ForCausalLM, LlamaForCausalLM |
| parameters | string | model size: "0.6B", "7B", "14B" |
| license | string | SPDX: Apache-2.0, MIT |
| languages | string[] | supported languages |
| context_length | u32 | recommended working context |
| max_position_embeddings | u32 | architectural max (RoPE limit) |

config is pure metadata. architecture params (hidden_size, heads, layers) are in the nox program. if you need them for display — parse the nox.

## nox program

the entire inference pipeline as [[nox]] assembly. 18 instructions. input → output in one program.

[[trident]] compiles high-level descriptions into nox. human writes trident, compiler outputs nox, .model stores nox.

```
human writes trident → trident compiler → nox assembly → stored in .model → runtime executes
```

the nox program defines EVERYTHING: input formatting, tokenization, embedding, forward pass, sampling, decoding. no separate chat template file. no separate sampling config. the program IS the behavior.

### why nox, not ONNX

| | ONNX | nox |
|--|------|-----|
| abstraction | static graph (nodes + edges) | assembly (18 instructions) |
| size | millions of nodes | ~15 instructions |
| flash attention | cannot express | native instruction |
| pipeline | forward pass only | entire input → output |
| change sampling | rewrite application code | edit one instruction in program |
| new architecture | new ONNX operators (committee) | new nox program (anyone) |
| hardware | runtime rewrites graph | [[trident]] compiles to 28 targets |

### example programs

LLM (text → text):
```nox
chatml_format bos=151643 eos=151645
bpe_encode vocab=151936
token_embed dim=1024 vocab=151936
rope theta=1e6 head_dim=64
transformer_decode layers=28 hidden=1024 heads=16 kv_heads=8 ffn=swiglu:3072 norm=rmsnorm:1e-6 attn=flash
linear vocab=151936
sample top_p=0.9 temperature=0.7
bpe_decode
```

YOLO (image → detections):
```nox
image_resize 640
image_normalize mean=0.485,0.456,0.406 std=0.229,0.224,0.225
cnn_detect backbone=yolov11_nano classes=80
nms threshold=0.45
bbox_decode
```

whisper (audio → text):
```nox
audio_resample rate=16000
mel_spectrogram n_fft=400 n_mels=80
transformer_encode layers=12 hidden=768 heads=12
transformer_decode layers=12 hidden=768 heads=12 search=beam:5
bpe_decode
```

flux (text → image):
```nox
clip_tokenize
clip_encode hidden=768
diffusion_dit steps=50 scheduler=flow_matching layers=24 hidden=3072
vae_decode channels=3 spatial=1024
```

## tensor index

separate `~~~tensors` file inside the container. TOML format. one entry per tensor:

```toml
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], dtype = "q4_0", offset = 311361536, size = 1179648 }
```

per-tensor fields:
- `shape` — dimensions
- `dtype` — data type
- `offset` — byte offset from start of `~~~weights` binary zone
- `size` — byte count

## weights binary zone

raw concatenated tensor data after `~~~weights`. page-aligned per tensor (4096 bytes) for zero-copy mmap to GPU/NVMe/AMX via [[unimem]].

4096-byte alignment satisfies all hardware:

| hardware | required alignment | 4096 ≥ |
|----------|:-:|:-:|
| CPU cache line | 64 | yes |
| NEON | 16 | yes |
| AVX-512 | 64 | yes |
| Metal GPU | 256 | yes |
| CUDA | 256 | yes |
| NVMe DMA | 4096 | yes |
| AMX | 64 | yes |

overhead: ~4KB padding per tensor × ~300 tensors = ~1.2MB. negligible vs GB of weights.

## dtypes

| dtype | bits/value | block_size | description |
|-------|:-:|:-:|-------------|
| f32 | 32 | 1 | full precision (norms, biases) |
| f16 | 16 | 1 | half precision (small critical models) |
| bf16 | 16 | 1 | brain float |
| q8_0 | 8.5 | 32 | 8-bit block quantized (math, vision) |
| q4_0 | 4.5 | 32 | 4-bit block quantized (general LLMs) |
| q4_k | 4.5 | 256 | 4-bit K-quant (better quality) |
| ternary | 1.58 | — | 1.58-bit (bitnet native) |

block quantization (q4_0, q8_0): each block of `block_size` values has one f16 scale factor + packed integers.

## tensor naming

HuggingFace convention is canonical:

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

import converts GGUF names to HF at pack time:

| GGUF | HF |
|------|-----|
| token_embd.weight | model.embed_tokens.weight |
| output_norm.weight | model.norm.weight |
| output.weight | model.lm_head.weight |
| blk.{i}.attn_q.weight | model.layers.{i}.self_attn.q_proj.weight |
| blk.{i}.attn_k.weight | model.layers.{i}.self_attn.k_proj.weight |
| blk.{i}.attn_v.weight | model.layers.{i}.self_attn.v_proj.weight |
| blk.{i}.attn_output.weight | model.layers.{i}.self_attn.o_proj.weight |
| blk.{i}.ffn_gate.weight | model.layers.{i}.mlp.gate_proj.weight |
| blk.{i}.ffn_up.weight | model.layers.{i}.mlp.up_proj.weight |
| blk.{i}.ffn_down.weight | model.layers.{i}.mlp.down_proj.weight |
| blk.{i}.attn_norm.weight | model.layers.{i}.input_layernorm.weight |
| blk.{i}.ffn_norm.weight | model.layers.{i}.post_attention_layernorm.weight |

## vocab

full vocabulary in TOML. human-readable, grep-able. 151K tokens = ~6MB, parses in 91ms.

```toml
type = "bpe"
vocab_size = 151936
bos_id = 151643
eos_id = 151645
pad_id = 151643

[tokens]
0 = "<unk>"
1 = "<s>"
2 = "</s>"
3 = "▁the"
4 = "▁of"
151935 = "▁сверхразум"

[merges]
0 = ["▁", "t"]
1 = ["▁t", "h"]
2 = ["th", "e"]
```

`grep "сверхразум" model.model` → finds token ID. try that with tokenizer.json.

## lineage

```toml
[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
method = "abliteration + q4_0"
```

when stored as [[particles]] in [[hemera]], each step in the chain (base → finetune → quantize → abliterate) is content-addressable and verifiable.

## runtime load

```
file.model
  → parse frontmatter (TOML)
  → compile nox program via trident → hardware kernels (cached)
  → read ~~~tensors → tensor map
  → mmap ~~~weights into unimem Layout (zero-copy)
  → inference ready
```

see [[llm]] for memory architecture, [[unimem]] for zero-copy pipeline.
