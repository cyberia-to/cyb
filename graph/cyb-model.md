---
tags: cyber, cyb, core, soma, spec
crystal-type: spec
crystal-domain: cyber
alias: .model, model format, cyb model spec
---

# .model — neural network in [[format]]

.model is a [[format]]-compatible extension. a .model file IS a .cyb file — same three rules, same parsing. the extension tells tools and humans: this container holds a neural network.

one file = program + weights + vocabulary + documentation + benchmarks. ready for inference.

## required files

| name | format | what it does |
|------|--------|-------------|
| card | md | what this model is, how to use |
| config | toml | metadata: name, license, parameters, languages, context |
| program | trident or rs | entire pipeline: input → output |
| tensors | toml | tensor index: names, shapes, encodings, offsets |
| vocab | toml | full vocabulary: tokens + merge rules |
| eval | toml | benchmark results (updatable by user for routing) |
| weights | tensors | raw weight data (binary, page-aligned) |

no optional files. everything in .model is required.

program describes the ENTIRE pipeline — not just forward pass. tokenization, embedding, forward, sampling, decoding — all in one program. no separate chat template. no separate sampling config. the program IS the behavior.

two supported program languages:

| format | path | use for |
|--------|------|---------|
| trident | trident → [[nox]] → STARK proof | provable inference, field arithmetic |
| rs | Rust → native binary | fast inference, [[acpu]]/[[aruminium]]/[[rane]] |

both describe the same pipeline. both produce the same result. trident proves it. rs runs it fast. a .model can contain both — runtime picks based on need. two implementations = correctness verification.

## example

```
[cyb]
types = ["model"]
name = "qwen3-0.6b-abliterated"
parameters = 600000000
license = "Apache-2.0"
languages = ["en", "zh", "ru"]

[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
method = "abliteration"

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
context_length = 32768
max_position_embeddings = 40960

~~~program
module model.pipeline

// qwen3-0.6b-abliterated — full inference pipeline
// input: raw text → output: generated text

use vm.io.io
use std.nn.tensor

pub fn forward(input_addr: Field, output_addr: Field, seq_len: Field) {
    // tokenize
    let tokens: Field = io.bpe_encode(input_addr, seq_len, 151936)

    // embed
    let hidden: Field = tensor.embed(tokens, seq_len, 1024, 151936)

    // 28 transformer layers
    for layer in 0..28 bounded 28 {
        let l: Field = convert.as_field(layer)
        hidden = tensor.rmsnorm(hidden, seq_len, 1024, 0.000001)
        let q: Field = tensor.matvec(hidden, l, 2048, 1024)
        let k: Field = tensor.matvec(hidden, l, 1024, 1024)
        let v: Field = tensor.matvec(hidden, l, 1024, 1024)
        let attn: Field = tensor.flash_attention(q, k, v, 16, 8, 64, seq_len)
        hidden = tensor.residual_add(hidden, attn, 1024)
        hidden = tensor.rmsnorm(hidden, seq_len, 1024, 0.000001)
        hidden = tensor.swiglu(hidden, l, 3072, 1024)
    }

    // sample + decode
    let logits: Field = tensor.linear(hidden, 151936, 1024)
    let token: Field = tensor.sample_top_p(logits, 151936, 0.9, 0.7)
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

## config

metadata about the model. architecture params live in the program — config is for routing and display.

| field | type | description |
|-------|------|-------------|
| model_type | string | qwen3, llama, bitnet, mimo, whisper, yolo |
| context_length | field | recommended working context |
| max_position_embeddings | field | architectural max (RoPE limit) |

frontmatter `[cyb]` section holds: name, parameters, license, languages, lineage. config holds model-specific fields that the program needs at runtime.

## program

the entire inference pipeline as source code. two languages supported:

| | trident | rs |
|--|---------|-----|
| compiles to | [[nox]] (18 instructions) | native binary |
| proof | STARK witness every execution | none |
| speed | field arithmetic | native hardware ([[acpu]]/[[aruminium]]/[[rane]]) |
| std lib | `std.nn.tensor` (dot, matvec, relu) | full Rust ecosystem |

the program defines EVERYTHING: tokenization, embedding, forward pass, sampling, decoding. the program IS the behavior.

### why not ONNX

| | ONNX | trident/rs |
|--|------|-----------|
| size | millions of nodes | ~30 lines |
| flash attention | cannot express | `tensor.flash_attention()` |
| type system | none | Field, U32, Bool / Rust types |
| pipeline | forward pass only | entire input → output |
| proof | not possible | every trident execution = STARK |
| hardware | runtime rewrites graph | compiles to 28 targets |

## tensors

separate `~~~tensors` file inside the container. TOML format. one entry per tensor:

```toml
"model.layers.0.self_attn.q_proj.weight" = { shape = [2048, 1024], encoding = "q4", offset = 311361536, size = 1179648 }
```

fields: `shape`, `encoding`, `offset` (byte offset from `~~~weights`), `size` (byte count).

## weights

raw concatenated tensor data after `~~~weights`. page-aligned per tensor (4096 bytes) for zero-copy mmap via [[unimem]].

## encodings

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
matmul: +1 = add, -1 = subtract, 0 = skip. no multiply.
```

### float → integer at import

| source | target | method |
|--------|--------|--------|
| float32 | u32 | `round(value * 65536)` |
| float16 | u16 | `round(value * 256)` |
| GGUF Q4_0 | q4 | direct copy |
| GGUF Q8_0 | q8 | direct copy |

## tensor naming

HuggingFace convention is canonical. import converts GGUF names (`blk.{i}.attn_q`) to HF (`model.layers.{i}.self_attn.q_proj`) at pack time.

## vocab

full vocabulary in TOML. 151K tokens = ~6MB, parses in 91ms. `grep "сверхразум" model.model` → finds token ID.

## eval

live benchmark results. user updates after testing. routing reads eval to pick the best model for each task.

```toml
[needle_in_haystack]
context = 104000
score = 0.991

[mmlu_pro]
score = 0.724
```

## lineage

```toml
[cyb.lineage]
source = "huihui-ai/Qwen3-0.6B-abliterated"
method = "abliteration + q4"
```

when stored as [[particles]] in [[hemera]], each step is content-addressable and verifiable.

## runtime load

```
file.model
  → parse frontmatter
  → read ~~~card (display)
  → compile ~~~program → hardware kernels (cached)
  → read ~~~tensors → tensor map
  → mmap ~~~weights into unimem (zero-copy)
  → inference ready
```

see [[llm]] for memory architecture, [[unimem]] for zero-copy pipeline.
