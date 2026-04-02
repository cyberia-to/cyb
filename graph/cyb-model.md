---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .model, model format, cyb model spec
---

# .model — neural network in [[.cyb|format]]

.model follows the [[.cyb|format]] three rules. a .model file IS a .cyb file — same parsing, same tools. the extension tells humans and tools: this container holds a neural network.

one file. ready for inference.

## required files

| name | format | what it does |
|------|--------|-------------|
| card | md | what this model is, how to use |
| config | toml | all parameters: architecture, tokenizer, sampling, lineage |
| program | tri or rs | entire pipeline: input → output (reads params from config) |
| tensors | toml | tensor index: names, shapes, encodings, offsets |
| vocab | toml | full vocabulary: tokens + merge rules (empty for non-text models) |
| eval | toml | benchmark results (updatable by user for routing) |
| weights | tensors | raw weight data (binary, page-aligned) |

no optional files. everything is required. vocab is empty `{}` for models without tokenizer.

program reads all params from config — one program works for any model of the same architecture. change config → different model, same program.

two supported program languages:

| format | path | use for |
|--------|------|---------|
| [[trident]] | trident → [[nox]] → [[zheng]] proof | provable inference, field arithmetic |
| Rust | Rust → native binary | fast inference, [[acpu]]/[[aruminium]]/[[rane]] |

## frontmatter

```toml
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
format = "tri"

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
```

## card

first thing you see. markdown.

```markdown
~~~card
# qwen3-0.6b-abliterated

0.6B parameter model for routing and intent classification.
soma tier 0 — always on, <15ms latency.

abliterated: refusal vectors removed from weights.
0% refusal rate on 320 harmful-instruction tests.

source: huihui-ai/Qwen3-0.6B-abliterated
license: Apache 2.0
```

## config

everything about the model. program reads params from config — one program works for any model of the same architecture. all numeric values are integers. no floats.

```toml
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
rms_norm_eps = 1000000

[tokenizer]
type = "bpe"
bos_id = 151643
eos_id = 151645
pad_id = 151643

[sampling]
temperature = 700
top_p = 900
scale = 1000

[lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
method = "abliteration"
```

| section | what it holds |
|---------|---------------|
| top-level | model_type, parameters, license, languages |
| [architecture] | what program reads (hidden_size, heads, layers, etc.) |
| [tokenizer] | type, bos_id, eos_id, pad_id |
| [sampling] | integers with scale (700/1000 = 0.7) |
| [lineage] | provenance ([[hemera]] verifiable) |

integer conventions: rms_norm_eps stores 1/ε (1000000 → ε = one millionth). sampling uses explicit scale (700/1000 = 0.7). eval scores are per-mille (991 = 99.1%).

## program

the entire inference pipeline as source code. reads all params from config — not hardcoded. change config → different model, same program. all behavior lives here — chat formatting, sampling strategy, tokenization. to change how the model talks, change the program, not a config file.

```trident
~~~program
module model.pipeline

use vm.io.io
use vm.core.convert
use std.nn.tensor

// chat formatting — behavior is code, not config
pub fn format_chatml(messages: &[Message], cfg: Config) {
    for msg in messages {
        io.write_token(cfg.tokenizer.bos_id)
        io.write_string(msg.role)
        io.write_string("\n")
        io.write_string(msg.content)
        io.write_token(cfg.tokenizer.eos_id)
    }
}

pub fn forward(input: Field, output: Field, seq: Field, cfg: Config) {
    let a = cfg.architecture
    let s = cfg.sampling

    // tokenize + embed
    let tok = io.bpe_encode(input, seq, a.vocab_size)
    let h   = tensor.embed(tok, seq, a.hidden_size, a.vocab_size)

    // transformer layers
    for i in 0..a.num_hidden_layers bounded 128 {
        let l = convert.as_field(i)

        h = tensor.rmsnorm(h, seq, a.hidden_size, a.rms_norm_eps)

        let qd = a.num_attention_heads * a.head_dim
        let kd = a.num_key_value_heads * a.head_dim

        let q = tensor.matvec(h, l, qd, a.hidden_size)
        let k = tensor.matvec(h, l, kd, a.hidden_size)
        let v = tensor.matvec(h, l, kd, a.hidden_size)

        let att = tensor.flash_attention(
            q, k, v,
            a.num_attention_heads,
            a.num_key_value_heads,
            a.head_dim, seq
        )

        h = tensor.residual_add(h, att, a.hidden_size)
        h = tensor.rmsnorm(h, seq, a.hidden_size, a.rms_norm_eps)
        h = tensor.swiglu(h, l, a.intermediate_size, a.hidden_size)
    }

    // output + sample + decode
    let logits = tensor.linear(h, a.vocab_size, a.hidden_size)
    let token  = tensor.sample_top_p(logits, a.vocab_size, s.top_p, s.temperature)
    io.bpe_decode(token, output)
}
```

| | trident | rs |
|--|---------|-----|
| compiles to | [[nox]] (18 instructions) | native binary |
| proof | [[zheng]] witness every execution | none |
| speed | field arithmetic | native hardware ([[acpu]]/[[aruminium]]/[[rane]]) |
| std lib | `std.nn.tensor` | full Rust ecosystem |

### why not ONNX

| | ONNX | trident/rs |
|--|------|-----------|
| size | millions of nodes | ~30 lines |
| flash attention | cannot express | `tensor.flash_attention()` |
| parametric | no (frozen shapes) | yes (reads config) |
| proof | not possible | every trident execution = [[zheng]] proof |
| hardware | runtime rewrites graph | compiles to 28 targets |

## tensors

TOML index. one entry per tensor. tensor names follow HuggingFace convention.

```toml
~~~tensors
"model.embed_tokens.weight" = { shape = [151936, 1024], encoding = "u16", offset = 0, size = 311361536 }
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], encoding = "q4", offset = 311361536, size = 1179648 }
"model.layers.0.input_layernorm.weight" = { shape = [1024], encoding = "u32", offset = 313131008, size = 4096 }
```

## vocab

full vocabulary in TOML. fast to parse. empty `{}` for non-text models.

```toml
~~~vocab
[tokens]
0 = "<unk>"
1 = "<s>"
2 = "</s>"
3 = "▁the"
4 = "▁of"

[merges]
0 = ["▁", "t"]
1 = ["▁t", "h"]
```

## eval

live benchmark results. scores are per-mille (0–1000). user updates after testing. routing reads eval to pick the best model.

```toml
~~~eval
[needle_in_haystack]
context = 104000
score = 991

[mmlu_pro]
score = 724

[humaneval]
pass_at_1 = 652
```

## weights

raw concatenated tensor data. page-aligned per tensor (4096 bytes) for zero-copy load, e.g. via [[unimem]].

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

### import conversion

everything converts to five encodings. no exceptions.

| source | target | method |
|--------|--------|--------|
| float32 | u32 | `round(value * 65536)` |
| float16 / bfloat16 | u16 | `round(value * 256)` |
| GGUF Q4_0 | q4 | direct copy |
| GGUF Q4_1 / Q4_K / Q5_K | q4 | dequant → requant as q4 |
| GGUF Q8_0 | q8 | direct copy |
| GGUF Q6_K | q8 | dequant → requant as q8 |
| BitNet ternary | ternary | direct copy |

five encodings. like UTF-8 killed the encoding zoo.

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
  → load ~~~weights into unimem (zero-copy)
  → inference ready
```

see [[llm]] for memory architecture, [[unimem]] for zero-copy pipeline.
