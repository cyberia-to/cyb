# Execution contract

How a loaded `.model` turns into output tokens / audio samples /
image pixels. The runtime's public API and internal dispatch rules.

## Load phase

```
model = cyb_llm::load(path)
```

Steps:

1. mmap the `.model` file (read-only).
2. Parse text sections (config, tensors, vocab).
3. Validate config schema and tensor completeness.
4. Identify execution path:
   - `model_type` matches curated family → curated
   - else if `graph` section present → graph
   - else → error
5. Instantiate curated family code OR graph executor.
6. Upload weights to backend (or defer to first forward for lazy GPU).
7. Pre-compute position caches (cos/sin for RoPE).

Load must be idempotent: loading same file twice produces equivalent
models (weights may or may not share GPU memory depending on backend).

## Forward phase

### Text generation

```
generator = model.generator()
for tokens in generator.generate(prompt, max_tokens=N, sampling=...):
    ...
```

Under the hood:

```
tokens = tokenizer.encode(apply_chat_template(prompt))
for t in tokens:
    logits = model.forward(&[t])        # prefill: one call per prompt token
for step in 0..max_tokens:
    next_token = sample(logits, sampling)
    if eos_tokens.contains(next_token): break
    emit(next_token)
    logits = model.forward(&[next_token])  # decode: append to KV
```

Prefill and decode use the same forward function with seq_len=1.
Optimization: bulk prefill (seq_len > 1) available but must produce
identical output to token-by-token (ε tolerance in [ops.md](ops.md)).

### Encoder (embeddings / classification)

```
h = model.encode(tokens)              # returns final hidden states
emb = pool(h, method)                  # cls / mean / max
# or
logits = model.classify(tokens)        # returns classification logits
```

### Whisper (ASR)

```
mel = audio_to_mel(samples, model.config)
h_enc = model.encoder(mel)
tokens = model.decode(h_enc, prompt_tokens=None)  # autoregressive decoder
text = tokenizer.decode(tokens)
```

### Diffusion (image/video)

```
latent = randn(shape=model.config.latent_shape)
for t in noise_schedule:
    noise_pred = model.forward(latent, t, text_emb)
    latent = denoise_step(latent, noise_pred, t)
image = vae.decode(latent)
```

### TTS

```
phonemes = text_frontend(text)
mel = model.text_to_mel(phonemes, speaker_id=...)
audio = model.mel_to_audio(mel)   # vocoder
```

## Backend dispatch

Each op dispatched to a backend. The chosen backend for a given op:

### Platform default

| Platform | Default |
|---|---|
| macOS + Apple Silicon | honeycrisp |
| macOS + Intel | wgpu+rs |
| Linux / Windows / Android / Web | wgpu+rs |

### Fallback chain

For a single op, given the chosen default backend:

1. Try default backend's `supports(op, inputs)`. If yes → execute there.
2. If default is honeycrisp and rejects, try wgpu+rs.
3. If wgpu+rs rejects, fall back to CPU reference library.
4. CPU reference implements every op — step 3 always succeeds.

The runtime may report per-op dispatch decisions to observability
so heavy CPU fallback is visible.

### User override

`cyb-llm run --backend=wgpu+rs|honeycrisp|nox` forces the entire
execution to a single backend. Override skips steps 1-2; if op is
not supported, fall-through is still to CPU library (step 3).

`--backend=nox` only works if model has `~~~graph` section (graph
executor is Tier 1 for nox, not curated).

## Backend trait

```rust
pub trait Backend: Send + Sync {
    /// Backend identity for logging and dispatch records.
    fn name(&self) -> &str;

    /// Return true if this backend implements the op for the given
    /// input dtypes/shapes. If not, caller falls back to CPU reference.
    fn supports(&self, op: &Op, inputs: &[TensorInfo]) -> bool;

    /// Execute one op. Inputs are already on the backend's memory
    /// (or shared via zero-copy for honeycrisp). Output may allocate
    /// or reuse from the frame allocator.
    fn execute(
        &self,
        op: &Op,
        inputs: &[Tensor],
        ctx: &mut ExecContext,
    ) -> Result<Vec<Tensor>, BackendError>;

    /// Allocate a tensor on this backend's memory.
    fn alloc(&self, shape: &[usize], dtype: DType) -> Result<Tensor, BackendError>;

    /// Upload f32/quantized bytes from host memory.
    fn upload(&self, data: &[u8], shape: &[usize], dtype: DType) -> Result<Tensor, BackendError>;

    /// Download to host memory for inspection.
    fn download_f32(&self, t: &Tensor) -> Vec<f32>;
}
```

## Memory lifecycle

### Weights (long-lived)

Allocated at load time. Persistent for the lifetime of the model
instance. Freed when model dropped.

### KV cache (per-conversation)

Allocated at conversation start, sized to max context. Appended to
on each decode step. Cleared on `reset_kv_cache()`.

### Activations (per-step)

Frame allocator: per-backend pool of buffers keyed by `(size, usage)`.
On each forward call, the allocator hands out buffers from the pool
or allocates new ones. At the start of the NEXT forward call, all
buffers from the previous call become available for reuse (they're
guaranteed unused by pending GPU work if dispatch was complete).

```rust
pub struct FrameAllocator {
    pool: HashMap<(u64 /* size */, BufferUsage), Vec<Buffer>>,
    in_use: Vec<(u64, BufferUsage, Buffer)>,
}

impl FrameAllocator {
    pub fn alloc(&mut self, size: u64, usage: BufferUsage) -> Buffer;
    pub fn reset(&mut self);  // called at start of next forward
}
```

Sizing policy:

- Buckets by exact byte size. No over-allocation to "next power of 2"
  — different shapes in different layers produce many bucket sizes,
  but reuse within a size is perfect.
- On reset, all in-use buffers move back to the pool. Pool grows
  monotonically to the steady-state size, then flat.
- Typical footprint for a 7B decoder at bs=1: ~200 MB of pool.

Separate pools per `BufferUsage` (STORAGE vs UNIFORM on wgpu;
IOSurface vs shared memory on honeycrisp).

See `llm/src/backend/wgpu/alloc.rs` for the existing wgpu
implementation. Honeycrisp allocator follows the same design with
IOSurface-backed buffers from aruminium.

### Position caches

Cos/sin cache for RoPE: allocated at load, sized to
`max_position_embeddings × half_dim`. Read-only during forward.

## Error semantics

Errors are structured, not strings:

```rust
pub enum ExecutionError {
    UnsupportedOp { op: String, backend: String, input_dtype: DType },
    ShapeMismatch { op: String, expected: Vec<usize>, got: Vec<usize> },
    MissingTensor { name: String, required_by: String },
    WeightFormatMismatch { name: String, declared: DType, actual_bytes: usize },
    OutOfMemory { backend: String, requested_bytes: usize },
    BackendInit { backend: String, reason: String },
    InvalidConfig { field: String, reason: String },
    Tokenizer { reason: String },
}
```

Every error is actionable — the user can tell which backend, which
op, which tensor. No `"internal error"` or generic panic.

Silent corruption is a BUG. If a backend returns wrong output without
erroring, that's higher severity than a missing-op error (which at
least surfaces).

## Invariants across paths

Running the same model with the same input through different paths
must produce outputs within the tolerances in [ops.md](ops.md#tolerance-summary):

```
output_curated  ≈ output_graph  ≈ output_nox  (within ε)
output_wgpu+rs  ≈ output_honeycrisp            (within ε)
```

A divergence larger than ε between paths or backends indicates a
bug in one of them. The CPU reference is the arbiter.

## Concurrency

### Multiple conversations

Each conversation has its own KV cache. Multiple cached conversations
can share one loaded model (same weights, separate KV).

Not supported in v1: batched inference (multiple prompts processed in
one forward pass). Future.

### Thread safety

`Backend` is `Send + Sync`. Running inference from multiple threads
on different models is safe. Running multiple concurrent forwards on
the same model is ALLOWED only if each has its own KV cache; shared
KV is a race.

## Observability

Every forward exposes timing breakdown (load-time, prefill-ms,
per-step-ms) for performance analysis. Backends log which op went
to which device (e.g., "layer 5 attention: honeycrisp.ane, FFN:
honeycrisp.gpu").

## Shutdown

```
drop(model)
```

Frees all GPU/backend memory. Must be idempotent. No leaks.

## Fused op policy

Fused ops ([ops.md](ops.md#10-fused-ops)) are optional optimizations.
Policy:

### Curated paths

Each curated family codepath explicitly emits fused ops where
beneficial. The author knows the architecture and writes fused
calls directly:
```rust
let (normed, res) = fused_skip_norm(hidden, attn_out, post_norm_w, eps);
```
No auto-detection. Curated code is hand-tuned; the author chose the
fusion.

### Graph path

The graph executor runs a **pre-execution fusion pass** that
rewrites the graph. Recognized patterns:

| Pattern | Fused op |
|---|---|
| `RmsNorm → Matmul` | `FusedNormMatmul` |
| `Add → RmsNorm` (both outputs used) | `FusedSkipNorm` |
| `Matmul → Silu` followed by `Matmul` + `Mul` | `FusedSwiGlu` |
| `Sdpa` (when backend has FlashAttention) | `FlashAttention` |

Detection is purely structural: a subgraph matches the pattern's
shape. If the backend reports `supports(FusedSkipNorm, ...)` is
true, the fused node replaces the sub-DAG. Else stays unfused.

Fusion is applied at graph-load time, once. Subsequent forwards use
the fused graph.

### Correctness

Every fused op must produce the same output as its unfused
equivalent within ε ([ops.md](ops.md#tolerance-summary)). Violation
is a backend bug — the graph executor can fall back to unfused.

## Extension points

Adding a new backend: implement `Backend` trait, register in dispatch
table. The runtime automatically routes supported ops. Curated
codepaths opt in per-family.

Adding a new op: add `Op` enum variant, math spec in [ops.md](ops.md),
CPU reference implementation, at least one backend implementation.
Version bump per [scope.md](scope.md).

Adding a new model family: write a curated codepath implementing the
graph from [arch.md](arch.md). Register the `model_type` string(s)
that dispatch to it.
