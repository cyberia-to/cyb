# Graph IR

Intermediate representation for computation graphs. Used by the
graph executor path ([architecture.md](architecture.md)) to execute
any model expressible as a composition of ops from [ops.md](ops.md).

A `.model` file may optionally include an `~~~graph` section
containing a serialized graph. The graph executor loads this and
walks it. If absent, only curated codepaths can run the model.

## Graph

```rust
pub struct Graph {
    /// Nodes in topological order — executing in order is a valid schedule.
    pub nodes: Vec<Node>,
    /// Named inputs that the caller must provide (e.g. "tokens", "timestep").
    pub inputs: Vec<TensorSpec>,
    /// Named outputs the graph produces (e.g. "logits", "image").
    pub outputs: Vec<String>,
    /// Weight tensor metadata (name → shape, dtype, weights-section offset).
    pub weights: HashMap<String, TensorSpec>,
}

pub struct Node {
    pub id: u32,                    // unique within graph
    pub op: Op,                     // from ops.md
    pub inputs: Vec<String>,        // tensor names (weight or intermediate)
    pub outputs: Vec<String>,       // tensor names produced
    pub attrs: Attrs,               // op-specific parameters
    pub backend_hint: Option<BackendHint>,  // prefer this backend if available
}

pub struct TensorSpec {
    pub shape: Vec<i64>,            // -1 for dynamic dims
    pub dtype: DType,
    pub offset: Option<u64>,        // byte offset in weights section (for weights)
    pub size: Option<u64>,
}

pub enum Attrs {
    None,
    Int(i64),
    Float(f32),
    String(String),
    Ints(Vec<i64>),
    Floats(Vec<f32>),
    Strings(Vec<String>),
    Map(HashMap<String, Attrs>),
}
```

## Node inputs / outputs

Tensor names use `model.` prefix for weights (matching [tensor.md](tensor.md)),
arbitrary identifiers for intermediates:

```
node 5:
  op: Matmul
  inputs: ["layer_0_normed", "model.layers.0.self_attn.q_proj.weight"]
  outputs: ["layer_0_q"]
```

Intermediate names are scoped to the graph; no naming collision
guarantee across models.

## Node attributes

Each op carries its non-tensor parameters directly inside the `Op` enum
variant (see `core/op.rs`). The `Node.attrs` field (`HashMap<String, AttrValue>`)
holds supplementary free-form hints (e.g. fusion hints) — it is *not* the
primary store for op parameters.

`AttrValue` variants (matching `ir/graph.rs`):
- `AttrValue::Int(i64)`
- `AttrValue::Float(f32)`
- `AttrValue::String(String)`
- `AttrValue::Ints(Vec<i64>)`
- `AttrValue::Floats(Vec<f32>)`
- `AttrValue::Bool(bool)`

## Serialization (binary)

The `~~~graph` section stores the graph as a length-prefixed binary
blob. Little-endian throughout.

```
graph_section:
  u32             num_nodes
  [node]          nodes × num_nodes
  u32             num_inputs
  [tensor_spec]   inputs × num_inputs
  u32             num_outputs
  [string]        outputs × num_outputs
  # weights are indexed via tensors.toml, not duplicated here

node:
  u32             id
  u16             op_tag              # from the op table below
  var             op_payload          # op-specific fields
  u32             num_inputs
  [string]        inputs × num_inputs
  u32             num_outputs
  [string]        outputs × num_outputs
  u8              has_backend_hint
  u8?             backend_hint        # if has_backend_hint

string:
  u32             len
  u8[len]         utf8_bytes

tensor_spec:
  u32             rank
  i64[rank]       shape
  u8              dtype_tag
  u8              has_offset
  u64?            offset, size        # if has_offset
```

## Op tags

Stable u16 values for serialization. Never reused. New ops append.

```
0   Matmul
1   Add
2   Mul
3   Sub
4   Div
5   Transpose
6   Reshape
7   Permute
8   Concat
9   Split
10  Chunk
11  Clamp
12  NanToNum
20  Sdpa
21  SdpaCross
22  SdpaWindow
23  KvCache
24  KvCompress
25  KvDecompress
26  Rope
27  SinusoidalEmbed
28  RelativePosEmbedding
40  RmsNorm
41  LayerNorm
42  BatchNorm
43  GroupNorm
44  InstanceNorm
45  AdaLN
60  Silu
61  Gelu
62  GeGlu
63  SwiGlu
64  Glu
65  Relu
66  LeakyRelu
67  PRelu
68  Sigmoid
69  Tanh
70  Softmax
80  Conv1d
81  Conv2d
82  Conv3d
83  ConvTranspose2d
84  CausalConv1d
85  DepthwiseConv
86  Pool
100 Interpolate
101 PixelShuffle
102 PixelUnshuffle
103 PatchEmbed
104 Unpatchify
120 TokenEmbed
121 PosEmbed
140 NoiseSchedule
141 FlowStep
142 Quantize
143 Dequantize
144 Sample
160 LoraApply
161 Kron
162 MatrixInverse
180 FusedNormMatmul
181 FusedSkipNorm
182 FusedSwiGlu
183 FlashAttention
199 Argmax
```

## Stateful execution

The `GraphExecutor` is stateful across `run()` calls:

- **`KvCache` op**: inputs `[K_new, V_new]`, outputs `[K_full, V_full]`.
  The executor appends `K_new` and `V_new` rows to per-layer accumulators
  and returns the full accumulated `[seq, kv_heads*head_dim]` tensors.
  Accumulators survive across `run()` calls (i.e. across generation steps).

- **`pos` auto-injection**: before each `run()`, the executor inserts
  `"pos"` = `Tensor::from_f32([1], [past_seq_len as f32])` into the
  working tensors. Rope nodes consume this as their position argument.
  `past_seq_len` starts at 0 and increments by 1 each `run()` call.

- **`reset()`**: clears all KV accumulators and resets `past_seq_len` to 0.
  Equivalent to starting a new conversation.

## Weight loading

Two categories of weights:

- **Matmul weights** (embedding, projections): passed as quantized
  `&Tensor` to `backend.quant_matmul()`. The backend handles
  dequantization internally — no pre-dequant needed for correctness
  or performance.

- **Norm / Rope / other non-matmul weights**: always small (shape `[H]`).
  Dequantize to f32 once at executor prepare time and store as f32
  `Tensor` in the weights map. This avoids per-step dequant overhead
  for kernels that don't have a quantized path.

## Op payload encoding

Op-specific fields after `op_tag`. Examples:

```
Rope:
  u32 head_dim
  u32 rope_dim    # rotated dims (≤ head_dim); rope_dim == head_dim for full rotation
  f32 base

RmsNorm, LayerNorm, BatchNorm, GroupNorm, InstanceNorm:
  f32 eps
  # GroupNorm also: u32 num_groups
  # BatchNorm also: f32 momentum

Sdpa:
  u32 num_heads
  u32 kv_heads
  u32 head_dim
  u8  causal

Reshape, Permute, Transpose, Chunk, Split, Concat:
  # see struct definitions in ops.md; serialize their fields directly

Pool:
  u8  mode        # 0=max, 1=avg
  u32 kernel_h
  u32 kernel_w
  u32 stride_h
  u32 stride_w
  u32 padding_h
  u32 padding_w

Conv1d:
  u32 kernel
  u32 stride
  u32 padding
  u32 dilation
  u32 groups

Conv2d:
  u32 kernel_h, kernel_w
  u32 stride_h, stride_w
  u32 padding_h, padding_w
  u32 dilation_h, dilation_w
  u32 groups

Softmax:
  i32 dim         # negative allowed (e.g. -1 = last dim)

Sample:
  u8  method      # 0=greedy, 1=temp, 2=topk, 3=topp, 4=minp
  f32 temperature
  f32 p            # for topp / minp
  u32 k            # for topk

# Ops with no payload: Add, Mul, Sub, Div, Silu, Relu, Sigmoid, Tanh,
# Argmax, Kron, MatrixInverse → op_payload is empty.
```

## Serialization (alternative: JSON/TOML)

For debuggability, graphs may also be stored as TOML in the
`~~~graph` section. Format is the same structure, human-readable.
Binary is canonical; TOML is a convenience for inspection.

## Walking the graph

Execution algorithm:

```
load graph from .model
upload weights to backend (or defer)
alloc intermediates map: name → Tensor

for node in graph.nodes:       # already topologically sorted
    inputs = node.inputs.iter().map(|name| intermediates[name].or_weight(name))
    backend = pick_backend(node.op, node.backend_hint, inputs)
    outputs = backend.execute(node.op, inputs, node.attrs)
    for (name, t) in node.outputs.zip(outputs):
        intermediates.insert(name, t)
    free_if_no_future_use(intermediates, node.id)

return intermediates[graph.outputs[0]]
```

### Backend selection

For each node:
1. If `backend_hint` is set and that backend `supports(op)`, use it.
2. Else, try default backend for the platform (honeycrisp on macOS,
   wgpu+rs elsewhere).
3. If default doesn't support, fall back to CPU library.
4. Never silently fail — error if no backend accepts the op.

### Tensor lifecycle

Free an intermediate once all its consumers have run. Simple
liveness analysis: pre-compute `last_use[name]`, free after node
`last_use[name]`.

### Fused op detection

Optional pre-pass: scan the graph for recognized fusion patterns
(rmsnorm → matmul, matmul → silu, add → rmsnorm) and replace with
fused ops ([execution.md](execution.md) lists patterns). Fusion is
always optional; unfused graph must produce same output within ε.

## Shape inference

The graph validates at load time. For each node, given input shapes:
compute output shape per the op's math in [ops.md](ops.md).
Mismatch = error.

Dynamic shapes (`shape[i] = -1`) are resolved at first invocation
from actual input sizes.

## Invariants

- Graph is acyclic. A topological order exists and is pre-computed.
- Every tensor consumed by a node was produced by an earlier node
  or is a weight/input.
- No unused intermediates (dead code) — validation may warn.
- Sum of `inputs` tensor sizes + weights + intermediates fits
  declared max memory for the target backend. Else error pre-execution.
