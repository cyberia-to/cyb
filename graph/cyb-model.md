---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .model, model format, cyb model spec
---

# .model — neural network in [[format]]

.model is a [[format]]-compatible extension. a .model file IS a .cyb file — same three rules, same parsing. the extension tells tools and humans: this container holds a neural network.

one file = program + weights + vocabulary + documentation. ready for inference.

## five files

| name | format | what it does |
|------|--------|-------------|
| card | md | what this model is, how to use |
| config | toml | metadata: name, license, parameters, languages |
| program | trident | entire pipeline: input → output (compiles to [[nox]]) |
| tensors | toml | tensor index: names, shapes, encodings, offsets |
| vocab | toml | full vocabulary: tokens + merge rules |
| eval | toml | benchmark results (updatable by user) |
| weights | tensors | raw weight data (binary, page-aligned) |

seven files. model runs. first thing you see is the card. eval is live — user updates it with real benchmarks, routing uses it to pick the best model for the task.

program is [[trident]] source code. describes the ENTIRE pipeline — not just forward pass. input formatting, tokenization, embedding, forward, sampling, decoding — all in one program. no separate chat template, no separate sampling config, no separate preprocessor. the program IS the behavior.

[[trident]] compiles to [[nox]] (18-instruction VM) which executes on any hardware. trident has `std.nn.tensor` (dot, matvec, relu, scale), type system (Field, U32, Bool, Digest), generics, modules. every execution produces a STARK proof.

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
format = "trident"

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
module model.pipeline

// qwen3-0.6b-abliterated — full inference pipeline
// input: raw text → output: generated text

use vm.io.io
use vm.core.field
use std.nn.tensor

pub fn forward(input_addr: Field, output_addr: Field, seq_len: Field) {
    // step 1: tokenize (BPE, vocab=151936)
    let tokens_addr: Field = io.bpe_encode(input_addr, seq_len, 151936)

    // step 2: embed
    let hidden_addr: Field = tensor.embed(tokens_addr, seq_len, 1024, 151936)

    // step 3: transformer decode (28 layers)
    for layer in 0..28 bounded 28 {
        let l: Field = convert.as_field(layer)
        hidden_addr = tensor.rmsnorm(hidden_addr, seq_len, 1024, 0.000001)
        let q: Field = tensor.matvec(hidden_addr, l, 2048, 1024)
        let k: Field = tensor.matvec(hidden_addr, l, 1024, 1024)
        let v: Field = tensor.matvec(hidden_addr, l, 1024, 1024)
        let attn: Field = tensor.flash_attention(q, k, v, 16, 8, 64, seq_len)
        hidden_addr = tensor.residual_add(hidden_addr, attn, 1024)
        hidden_addr = tensor.rmsnorm(hidden_addr, seq_len, 1024, 0.000001)
        hidden_addr = tensor.swiglu(hidden_addr, l, 3072, 1024)
    }

    // step 4: output projection + sample
    let logits: Field = tensor.linear(hidden_addr, 151936, 1024)
    let token: Field = tensor.sample_top_p(logits, 151936, 0.9, 0.7)

    // step 5: decode
    io.bpe_decode(token, output_addr)
}

~~~tensors
"model.embed_tokens.weight" = { shape = [151936, 1024], encoding = "f16", offset = 0, size = 311361536 }
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], encoding = "q4_0", offset = 311361536, size = 1179648 }
"model.layers.0.self_attn.k_proj.weight" = { shape = [1024, 1024], encoding = "q4_0", offset = 312541184, size = 589824 }
"model.layers.0.input_layernorm.weight" = { shape = [1024], encoding = "f32", offset = 313131008, size = 4096 }

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

## trident program

the entire inference pipeline as [[trident]] source code. compiles to [[nox]] (18-instruction VM). every execution produces a STARK proof.

trident has modules (`use std.nn.tensor`), types (`Field`, `U32`, `Bool`), generics, `//` comments. `std.nn.tensor` provides `dot`, `matvec`, `relu`, `scale`, `flash_attention`. trident compiles to 28 hardware targets.

the program defines EVERYTHING: tokenization, embedding, forward pass, sampling, decoding. no separate chat template. no separate sampling config. the program IS the behavior.

### why not ONNX

| | ONNX | trident |
|--|------|---------|
| abstraction | static graph (nodes + edges) | typed language with modules |
| size | millions of nodes | ~30 lines |
| flash attention | cannot express | `tensor.flash_attention()` |
| type system | none | Field, U32, Bool, Digest, generics |
| pipeline | forward pass only | entire input → output |
| proof | not possible | every execution = STARK witness |
| hardware | runtime rewrites graph | compiles to 28 targets |

## tensor index

separate `~~~tensors` file inside the container. TOML format. one entry per tensor:

```toml
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], encoding = "q4_0", offset = 311361536, size = 1179648 }
```

per-tensor fields:
- `shape` — dimensions
- `encoding` — data type
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

## encodings

no floats. all weights are integers. float models are converted to fixed-point integers at import time. this is not a lossy compromise — production inference already runs on integers (Q4/Q8 quantization IS integer arithmetic). we make it explicit.

integers are [[nox]] field elements. field arithmetic on GPU int tensor cores = provable inference at native hardware speed. no sixth algebra needed — integer inference IS [[nebu]] (F_p).

| encoding | bits/value | block_size | type | description |
|----------|:-:|:-:|------|-------------|
| u32 | 32 | 1 | integer | full precision (norms, biases) |
| u16 | 16 | 1 | integer | half precision (small critical models) |
| q8 | 8.5 | 32 | block int | 8-bit quantized (math, vision sensitive) |
| q4 | 4.5 | 32 | block int | 4-bit quantized (general LLMs) |
| ternary | 1.58 | — | trit | 1.58-bit (bitnet, [[kuro]] native) |

### u32 layout

```
one value = 4 bytes, little-endian unsigned integer
range: 0 .. 4294967295
fixed-point: 16 integer bits + 16 fractional bits
actual_value = stored_value / 65536.0
```

used for: norms, biases — tensors that were float32 in the original model.

### u16 layout

```
one value = 2 bytes, little-endian unsigned integer
range: 0 .. 65535
fixed-point: 8 integer bits + 8 fractional bits
actual_value = stored_value / 256.0
```

used for: embedding tables, small model weights that were float16 in the original model.

### q4 layout (4-bit block quantized)

```
block of 32 values = 18 bytes:
  [0..1]    u16 scale factor (little-endian)
  [2..17]   32 × 4-bit values packed into 16 bytes
            each byte = two 4-bit values (low nibble first)

dequantize: value[i] = (nibble[i] - 8) * scale / 8
```

one tensor = ceil(num_elements / 32) blocks, concatenated. compatible with GGUF Q4_0.

### q8 layout (8-bit block quantized)

```
block of 32 values = 34 bytes:
  [0..1]    u16 scale factor (little-endian)
  [2..33]   32 × signed int8 values

dequantize: value[i] = int8[i] * scale / 127
```

one tensor = ceil(num_elements / 32) blocks, concatenated. compatible with GGUF Q8_0.

### ternary layout (1.58-bit, bitnet)

```
32 values = 8 bytes:
  each value is 2 bits: 00 = 0, 01 = +1, 10 = -1
  16 values packed per u32 (little-endian)

matmul: no multiply needed. +1 = add, -1 = subtract, 0 = skip.
```

used for: [[kuro]] (F₂) native models. XOR + popcount on binary hardware.

### float → integer conversion at import

| source | target | method |
|--------|--------|--------|
| float32 norms/biases | u32 | `round(value * 65536)` |
| float16 weights | u16 | `round(value * 256)` |
| bfloat16 weights | u16 | `round(value * 256)` |
| GGUF Q4_0 | q4 | direct copy (same layout) |
| GGUF Q8_0 | q8 | direct copy (same layout) |

Q4/Q8 weights are already integers — zero conversion. only norms and biases (originally float) need fixed-point conversion. for a typical 7B model: ~300 norm tensors × 4KB each = ~1.2MB to convert. the rest (99.9% of data) copies directly.

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
