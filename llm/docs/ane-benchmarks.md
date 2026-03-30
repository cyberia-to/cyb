# ANE backend benchmarks

results from the ane driver + cyb/llm ANE backend on M1 Pro.

## Qwen3-0.6B inference

| metric | result | vs CoreML path |
|--------|--------|---------------|
| prefill (4 tokens) | 363ms | 1.09x faster |
| decode | 82.7ms/tok | 1.06x faster |
| throughput | 12.1 tok/s | — |

all 12 ANE kernels (3 forward + 9 backward) compile and run on
hardware. training loop with AdamW and cosine LR schedule verified.

## hardware

tested on M1 Pro only. M2/M3/M4 benchmarks welcome.

## how to reproduce

```bash
# from cyb/llm with ane driver installed

# compile all 12 Qwen3-0.6B kernels on ANE
cargo run --example ane/compile_kernels

# benchmark
cargo run --release --example ane/bench

# download and convert Qwen3-0.6B weights
cargo run --release -p ane-tools --bin convert_hf

# inference
cargo run --release --example ane/infer -- --ckpt ane_qwen3_06b_dyn_ckpt.bin

# train from scratch
cargo run --release --example ane/train -- --scratch --steps 100
```
