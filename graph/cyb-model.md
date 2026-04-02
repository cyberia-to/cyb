---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .model, model format, cyb model spec
---

# .model — neural network in [[format]]

.model is a [[format]]-compatible extension. a .model file IS a .cyb file — same three rules, same parsing. the extension tells tools and humans: this container holds a neural network.

one file. ready for inference.

## required files

| name | format | what it does |
|------|--------|-------------|
| card | md | what this model is, how to use |
| config | toml | all parameters: architecture, tokenizer, sampling, chat |
| program | trident or rs | entire pipeline: input → output (reads params from config) |
| tensors | toml | tensor index: names, shapes, encodings, offsets |
| vocab | toml | full vocabulary: tokens + merge rules (empty for non-text models) |
| eval | toml | benchmark results (updatable by user for routing) |
| weights | tensors | raw weight data (binary, page-aligned) |

no optional files. everything is required. vocab is empty `{}` for models without tokenizer (YOLO, BEATs).

program reads all params from config — one program works for any model of the same architecture. change config → different model, same program.

two supported program languages:

| format | path | use for |
|--------|------|---------|
| trident | trident → [[nox]] → STARK proof | provable inference, field arithmetic |
| rs | Rust → native binary | fast inference, [[acpu]]/[[aruminium]]/[[rane]] |

a .model can contain both programs (as `program` and `program-native`). runtime picks based on need. two implementations = correctness verification.

## example

```
[cyb]
types = ["model"]
name = "qwen3-0.6b-abliterated"

[[files]]
name = "card"
format = "md"

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
name = "weights"
format = "tensors"
size = 1200000000

~~~card
# qwen3-0.6b-abliterated

0.6B parameter model for routing and intent classification.
soma tier 0 — always on, <15ms latency.

abliterated: refusal vectors removed from weights.
0% refusal rate on 320 harmful-instruction tests.

source: huihui-ai/Qwen3-0.6B-abliterated
license: Apache 2.0

~~~config
model_type = "qwen3"
parameters = 600000000
license = "Apache-2.0"
languages = ["en", "zh", "ru"]

[architecture]
hidden_size = 1024
num_attention_heads = 16
num_key_value_heads = 8
head_dim = 64
num_hidden_layers = 28
intermediate_size = 3072
vocab_size = 151936
context_length = 32768
max_position_embeddings = 40960
rope_theta = 1000000
rms_norm_eps = 1

[tokenizer]
type = "bpe"
bos_id = 151643
eos_id = 151645
pad_id = 151643

[sampling]
temperature = 700
top_p = 900
scale = 1000

[chat]
format = "chatml"
bos_token = "<|endoftext|>"
eos_token = "<|im_end|>"

[lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
method = "abliteration"

~~~program
module model.pipeline

use vm.io.io
use vm.core.convert
use std.nn.tensor

pub fn forward(input_addr: Field, output_addr: Field, seq_len: Field,
               cfg: Config) {
    let a: Architecture = cfg.architecture
    let tokens: Field = io.bpe_encode(input_addr, seq_len, a.vocab_size)
    let hidden: Field = tensor.embed(tokens, seq_len, a.hidden_size,
                                     a.vocab_size)

    for layer in 0..a.num_hidden_layers bounded 128 {
        let l: Field = convert.as_field(layer)
        hidden = tensor.rmsnorm(hidden, seq_len, a.hidden_size, a.rms_norm_eps)
        let q: Field = tensor.matvec(hidden, l,
                                     a.num_attention_heads * a.head_dim,
                                     a.hidden_size)
        let k: Field = tensor.matvec(hidden, l,
                                     a.num_key_value_heads * a.head_dim,
                                     a.hidden_size)
        let v: Field = tensor.matvec(hidden, l,
                                     a.num_key_value_heads * a.head_dim,
                                     a.hidden_size)
        let attn: Field = tensor.flash_attention(q, k, v,
                                                 a.num_attention_heads,
                                                 a.num_key_value_heads,
                                                 a.head_dim, seq_len)
        hidden = tensor.residual_add(hidden, attn, a.hidden_size)
        hidden = tensor.rmsnorm(hidden, seq_len, a.hidden_size, a.rms_norm_eps)
        hidden = tensor.swiglu(hidden, l, a.intermediate_size, a.hidden_size)
    }

    let logits: Field = tensor.linear(hidden, a.vocab_size, a.hidden_size)
    let token: Field = tensor.sample_top_p(logits, a.vocab_size,
                                           cfg.sampling.top_p,
                                           cfg.sampling.temperature)
    io.bpe_decode(token, output_addr)
}

~~~tensors
"model.embed_tokens.weight" = { shape = [151936, 1024], encoding = "u16", offset = 0, size = 311361536 }
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], encoding = "q4", offset = 311361536, size = 1179648 }
"model.layers.0.self_attn.k_proj.weight" = { shape = [1024, 1024], encoding = "q4", offset = 312541184, size = 589824 }
"model.layers.0.input_layernorm.weight" = { shape = [1024], encoding = "u32", offset = 313131008, size = 4096 }

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

~~~weights
<1.2GB tensor binary data>
```

## card

first thing you see. markdown. what the model is, how to use it, where it came from.

## config

everything about the model in structured TOML. program reads params from config — one program works for any model of the same architecture.

frontmatter `[cyb]` = minimal container metadata (types, name). config = everything else.

| top-level | type | description |
|-----------|------|-------------|
| model_type | string | qwen3, llama, bitnet, mimo, whisper, yolo |
| parameters | field | parameter count |
| license | string | SPDX identifier |
| languages | string[] | supported languages |

| section | fields | description |
|---------|--------|-------------|
| [architecture] | hidden_size, num_attention_heads, num_key_value_heads, head_dim, num_hidden_layers, intermediate_size, vocab_size, context_length, max_position_embeddings, rope_theta, rms_norm_eps | what program reads |
| [tokenizer] | type, bos_id, eos_id, pad_id | tokenizer params |
| [sampling] | temperature, top_p, scale | integers with scale (700/1000 = 0.7) |
| [chat] | format, bos_token, eos_token | chat formatting |
| [lineage] | source, method | provenance ([[hemera]] verifiable) |

all numeric values are integers (field elements). no floats.

## program

the entire inference pipeline as source code. reads all params from config — NOT hardcoded.

| | trident | rs |
|--|---------|-----|
| compiles to | [[nox]] (18 instructions) | native binary |
| proof | STARK witness every execution | none |
| speed | field arithmetic | native hardware ([[acpu]]/[[aruminium]]/[[rane]]) |
| std lib | `std.nn.tensor` (dot, matvec, relu) | full Rust ecosystem |

### why not ONNX

| | ONNX | trident/rs |
|--|------|-----------|
| size | millions of nodes | ~30 lines |
| flash attention | cannot express | `tensor.flash_attention()` |
| type system | none | Field, U32, Bool / Rust types |
| pipeline | forward pass only | entire input → output |
| parametric | no (frozen shapes) | yes (reads config) |
| proof | not possible | every trident execution = STARK |
| hardware | runtime rewrites graph | compiles to 28 targets |

## tensors

TOML index. one entry per tensor:

```toml
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], encoding = "q4", offset = 311361536, size = 1179648 }
```

fields: `shape`, `encoding`, `offset` (bytes from `~~~weights`), `size` (bytes). tensor names follow HuggingFace convention.

## vocab

full vocabulary in TOML. fast to parse. empty `{}` for non-text models.

## eval

live benchmark results. user updates after testing. routing reads eval to pick the best model.

## weights

raw concatenated tensor data. page-aligned per tensor (4096 bytes) for zero-copy mmap via [[unimem]].

no floats. all weights are integers. float models are converted at import time.

| encoding | bits/value | block_size | description |
|----------|:-:|:-:|-------------|
| u32 | 32 | 1 | full precision (norms, biases) |
| u16 | 16 | 1 | half precision |
| q8 | 8.5 | 32 | 8-bit block quantized |
| q4 | 4.5 | 32 | 4-bit block quantized |
| ternary | 1.58 | — | 1.58-bit (bitnet, [[kuro]]) |

### q4 layout

```
block of 32 values = 18 bytes:
  [0..1]    u16 scale (little-endian)
  [2..17]   32 × 4-bit packed (low nibble first)
dequantize: value[i] = (nibble[i] - 8) * scale / 8
```

### q8 layout

```
block of 32 values = 34 bytes:
  [0..1]    u16 scale (little-endian)
  [2..33]   32 × signed int8
dequantize: value[i] = int8[i] * scale / 127
```

### ternary layout

```
32 values = 8 bytes:
  2 bits per value: 00 = 0, 01 = +1, 10 = -1
matmul: +1 = add, -1 = subtract, 0 = skip.
```

### float → integer at import

| source | target | method |
|--------|--------|--------|
| float32 | u32 | `round(value * 65536)` |
| float16 | u16 | `round(value * 256)` |
| GGUF Q4_0 | q4 | direct copy |
| GGUF Q8_0 | q8 | direct copy |

## runtime load

```
file.model
  → parse frontmatter
  → read ~~~card (display)
  → read ~~~config → params
  → compile ~~~program(config) → hardware kernels (cached)
  → read ~~~tensors → tensor map
  → read ~~~vocab → tokenizer
  → read ~~~eval → routing data
  → mmap ~~~weights into unimem (zero-copy)
  → inference ready
```

see [[llm]] for memory architecture, [[unimem]] for zero-copy pipeline.
