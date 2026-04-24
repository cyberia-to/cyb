# Tokenizer

How text becomes token IDs and back. Covers vocabulary representation,
BPE merge algorithm, special tokens, chat templates.

## Model

Byte-Pair Encoding (BPE) is the canonical tokenizer. Vocabulary is a
map from token id (u32) to token string (bytes). Merges are an
ordered list of `(left, right)` pairs; earlier has higher priority.

Two flavors matter in practice:

- **Byte-level BPE** (GPT-2, Qwen, Mistral, Llama 3, Gemma): maps
  bytes 0..255 through a fixed byte-to-unicode table, then BPE
  operates on unicode code points. Each input byte deterministically
  becomes a visible character, so no data loss.
- **SentencePiece / Unigram** (Llama 2, older T5): different algorithm.
  Import converts to BPE where possible; otherwise model uses
  pre-tokenized input and bypasses merge (rare).

## Vocab representation in .model

From [format.md](format.md) — TOML sections in the `~~~vocab` section:

```toml
[tokenizer]
type = "bpe"                      # or "byte_level_bpe" explicitly
byte_fallback = false             # true for some Llama variants
add_bos_token = false
add_eos_token = false

[tokens]
0 = "<unk>"
1 = "<s>"
2 = "</s>"
...
151665 = "<|im_end|>"
...

[merges]
0 = ["Ġ", "t"]                   # Ġ = space (byte-level encoded)
1 = ["h", "e"]
...
```

Token strings are UTF-8. For byte-level BPE, strings already use
the byte-unicode mapping — NOT raw bytes. E.g. the space character
(0x20) is encoded as "Ġ" (U+0120) in the vocabulary. Encode/decode
passes through this mapping transparently.

Merges are `[left, right]` pairs. List order = priority.

## Byte-level encoding table

The standard GPT-2 byte-to-unicode mapping. Every byte 0..255 maps
to a unique printable code point:

```
Bytes 33..=126 (visible ASCII except space):   mapped to themselves
Bytes 161..=172 and 174..=255 (printable Latin-1 mostly): mapped to themselves
Bytes 0..=32, 127..=160, 173:  mapped to u+0100..u+013F range
                                  (offset so each gets a unique char)
```

Canonical mapping table is published with GPT-2 code; we ship the
table in the tokenizer library. Reverse mapping is exact.

## Encode algorithm

Given text → token ids:

```
1. Normalize (optional per-model — NFKC for some, none for most)
2. Pre-tokenize: split on whitespace/regex, keeping spaces.
   For byte-level, prepend a marker to each token that had leading
   space (e.g. "Ġhello").
3. For each pre-token string:
   a. Apply byte-level encoding (text → unicode string where each
      char represents one input byte).
   b. Split into characters (initial BPE state).
   c. Apply merges in priority order: find highest-priority pair
      (c_i, c_{i+1}) in merges; merge to single symbol; repeat
      until no merge applies.
   d. Look up resulting symbols in vocab → token ids.
4. Concatenate all token id lists.
5. Optionally prepend BOS, append EOS.
```

### Merge step (detailed)

```
symbols = list of chars in the pre-token
while True:
    best_pair = None
    best_priority = infinity
    for i in 0..len(symbols) - 1:
        pair = (symbols[i], symbols[i+1])
        if pair in merges and merges[pair] < best_priority:
            best_priority = merges[pair]
            best_pair = pair
            best_idx = i
    if best_pair is None:
        break
    # merge at best_idx
    symbols[best_idx] = symbols[best_idx] + symbols[best_idx+1]
    symbols.pop(best_idx+1)
    # continue searching — may create new merge opportunities
```

Cache by pre-token for speed: most words are repeated across inputs.

## Special tokens

Tokens matching `<|...|>` or `[...]` patterns are special: they do
NOT go through BPE and have fixed single-token IDs.

Before BPE, scan input for special-token strings and yield their
IDs directly. Between special tokens, normal BPE applies:

```
Input:    "<|im_start|>user\nHello<|im_end|>"
Split:    [special "<|im_start|>"] [text "user\nHello"] [special "<|im_end|>"]
Tokens:   [151644] [872, 198, ...] [151645]
```

Special token detection is greedy — prefer longest match at each
position.

### Registered special tokens

At load time, the tokenizer registers every vocab entry matching
`^<\|.+\|>$` or a model-specific list. Registered tokens are:
- Matched before BPE
- Decoded as their literal string (e.g. id 151645 → `"<|im_end|>"`)
- Skipped by `decode(..., skip_special_tokens=true)`

## Byte fallback

Some tokenizers (Llama 2 SentencePiece) fall back to raw byte tokens
for unknown characters. If `byte_fallback = true`, the vocab
includes tokens like `<0x00>`, `<0x01>`, ..., `<0xFF>` with IDs
3..258. Unknown characters emit their UTF-8 bytes as individual
byte-token IDs. Byte-level BPE doesn't need this — it never has
unknowns.

## Decode

Token IDs → text:

```
for id in ids:
    if id is registered special:
        if skip_special_tokens: continue
        else: emit string literal
    else:
        emit vocab[id] (byte-level encoded string)
concatenated = join all emitted strings
text = byte_level_decode(concatenated)
```

`byte_level_decode` reverses the byte-to-unicode mapping: each
char in the concatenated string maps back to one byte, producing
raw UTF-8.

## Chat templates

Each chat-trained model has a template that converts structured
conversation into a prompt string. Template lives in the `~~~chat`
section of `.model` (optional) or is inlined in the `~~~program`
section.

### Storage: `~~~chat`

```
~~~chat
format = "chatml"

template = """
{%- for msg in messages -%}
<|im_start|>{{ msg.role }}
{{ msg.content }}<|im_end|>
{% endfor -%}
{%- if add_generation_prompt -%}
<|im_start|>assistant
{%- endif -%}
"""

system_default = "You are a helpful assistant."

bos = ""                        # prepended to the rendered prompt
eos_sequences = ["<|im_end|>"]   # additional stop sequences besides EOS token
```

### Template engine

Jinja2-subset: `{% %}` control blocks, `{{ }}` interpolation, `{#
#}` comments. Supported control: `for`, `if`, `elif`, `else`, `endif`,
`endfor`. Supported operators: `==`, `!=`, `+`, `-` (string concat
and arithmetic). Filters: `trim`, `default`.

Template variables:
- `messages`: list of `{role, content}` dicts
- `add_generation_prompt`: bool (true when generating response)
- `system_default`: injected if first message isn't `system`

Output is a single string fed to the tokenizer.

### Common formats

- **chatml** (Qwen, OpenChat, etc): `<|im_start|>role\ncontent<|im_end|>\n`
- **llama-3-instruct**: `<|start_header_id|>role<|end_header_id|>\n\ncontent<|eot_id|>`
- **gemma**: `<start_of_turn>role\ncontent<end_of_turn>\n`
- **plain** (base models): no template, raw text

`format` field in `~~~chat` picks a built-in template; `template`
field overrides with custom Jinja.

## Stop sequences

Generation halts when any of these is produced:

1. EOS token id from `[tokenizer] eos_token_ids`
2. Any decoded substring in `eos_sequences` (chat template)
3. User-supplied stop sequences

Detection is post-decode: after each generated token, the accumulated
text is checked for any stop substring. Requires partial decode of
recent tokens.

## Tokenizer in the runtime

`cyb_llm::tokenizer::Tokenizer` struct owns the vocab, merges, special
tokens, and optionally the chat template. API:

```rust
impl Tokenizer {
    pub fn load(lm: &LoadedModel) -> Result<Self, TokenizerError>;

    pub fn encode(&self, text: &str, add_special: bool) -> Vec<u32>;
    pub fn decode(&self, ids: &[u32], skip_special: bool) -> String;

    pub fn apply_chat_template(
        &self,
        messages: &[ChatMessage],
        add_generation_prompt: bool,
    ) -> Result<String, TokenizerError>;

    pub fn eos_token_ids(&self) -> &[u32];
    pub fn bos_token_id(&self) -> Option<u32>;
}

pub struct ChatMessage {
    pub role: String,   // "system", "user", "assistant", "tool"
    pub content: String,
}
```

## Round-trip guarantee

For any canonical text input T and model M:
```
decode(encode(T)) == T
```
up to whitespace normalization documented per tokenizer. Byte-level
BPE guarantees exact byte-level round-trip when no normalization is
applied.

Tests in [test.md](test.md#tier-0-import-round-trip) verify this
for each imported model.

## Errors

- `TokenizerError::UnknownSpecialToken(name)` — template references
  a token not in vocab
- `TokenizerError::InvalidTemplate(line, reason)` — Jinja parse error
- `TokenizerError::IncompleteUtf8(position)` — decode produced
  truncated UTF-8 (shouldn't happen on complete token sequences)
