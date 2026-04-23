# mc — model compilation

Reference rust implementation of [CT-1](https://cyber.page/compiled-transformers-spec):
read a `.graph` cybergraph snapshot, write a `.model` transformer checkpoint.

```
.graph  ──►  mc  ──►  .model
```

No python. No pytorch. Output is loadable by `~/git/cyb/llm` runtime via mmap.

## install

```
cargo build --release
```

## usage

```
mc inspect bostrom-23195000.graph
mc compile bostrom-23195000.graph -o bostrom-23195000-ct1.model
```

End-to-end pipe:

```
curl -s https://node.bostrom.cybernode.ai/cyber/graph/snapshot?block=23195000 \
  | mc compile - -o bostrom-latest.model \
  && cyb-llm load bostrom-latest.model
```

## status

| phase | scope | status |
|-------|-------|--------|
| 0 | crate skeleton, .graph reader, .model writer scaffolding | done |
| 1 | passes 1–3: vocab, semcon discovery, architecture parameters | todo |
| 2 | passes 4–5: embedding (randomized SVD), per-semcon attention | todo |
| 3 | passes 6–8: MLP, norms, .model packaging | todo |
| 4 | conformance suite (P-EMBED, P-ATTN, P-LAYER, P-DET, P-LOAD) | todo |
| 5 | recompile bostrom-23195000.graph, P-LOAD against cyb-llm | todo |

## references

- [`compiled transformers spec`](https://cyber.page/compiled-transformers-spec) — CT-1 contract
- [`cyb-graph`](https://cyber.page/cyb-graph) — input format
- [`cyb-model`](https://cyber.page/cyb-model) — output format
- `~/git/cyber/analizer/compile_model.py` — python prototype, used as numerical reference
