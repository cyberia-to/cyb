// Element-wise operations: add, mul, SiLU gate (SwiGLU)

// === ADD ===
@group(0) @binding(0) var<storage, read> add_a: array<f32>;
@group(0) @binding(1) var<storage, read> add_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> add_out: array<f32>;

@compute @workgroup_size(256)
fn add_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    add_out[idx] = add_a[idx] + add_b[idx];
}

// === ADD with broadcast (bias [N] + activation [P*N]) ===
struct AddBcastParams {
    total: u32,
    stride: u32,  // N — bias repeats every stride elements
}

@group(0) @binding(0) var<storage, read> bcast_a: array<f32>;
@group(0) @binding(1) var<storage, read> bcast_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> bcast_out: array<f32>;
@group(0) @binding(3) var<uniform> bcast_params: AddBcastParams;

@compute @workgroup_size(256)
fn add_broadcast_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= bcast_params.total) { return; }
    bcast_out[idx] = bcast_a[idx] + bcast_b[idx % bcast_params.stride];
}

// === MUL ===
@group(0) @binding(0) var<storage, read> mul_a: array<f32>;
@group(0) @binding(1) var<storage, read> mul_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> mul_out: array<f32>;

@compute @workgroup_size(256)
fn mul_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    mul_out[idx] = mul_a[idx] * mul_b[idx];
}

// === Fused SiLU × gate (SwiGLU) ===
// output = silu(gate) * up = sigmoid(gate) * gate * up
@group(0) @binding(0) var<storage, read> gate: array<f32>;
@group(0) @binding(1) var<storage, read> up: array<f32>;
@group(0) @binding(2) var<storage, read_write> swiglu_out: array<f32>;

@compute @workgroup_size(256)
fn silu_mul_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let g = gate[idx];
    let sig = 1.0 / (1.0 + exp(-g));
    swiglu_out[idx] = g * sig * up[idx];
}

// === GELU activation (approximate, tanh-based) ===
// gelu(x) = 0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
@group(0) @binding(0) var<storage, read> gelu_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> gelu_out: array<f32>;

@compute @workgroup_size(256)
fn gelu_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let x = gelu_in[idx];
    let c = 0.7978845608; // sqrt(2/pi)
    let inner = clamp(c * (x + 0.044715 * x * x * x), -10.0, 10.0);
    gelu_out[idx] = 0.5 * x * (1.0 + tanh(inner));
}

// === SUB ===
@group(0) @binding(0) var<storage, read> sub_a: array<f32>;
@group(0) @binding(1) var<storage, read> sub_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> sub_out: array<f32>;

@compute @workgroup_size(256)
fn sub_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    sub_out[idx] = sub_a[idx] - sub_b[idx];
}

// === DIV ===
@group(0) @binding(0) var<storage, read> div_a: array<f32>;
@group(0) @binding(1) var<storage, read> div_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> div_out: array<f32>;

@compute @workgroup_size(256)
fn div_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    div_out[idx] = div_a[idx] / div_b[idx];
}

// === ReLU ===
@group(0) @binding(0) var<storage, read> relu_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> relu_out: array<f32>;

@compute @workgroup_size(256)
fn relu_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    relu_out[idx] = max(relu_in[idx], 0.0);
}

// === Leaky ReLU ===
struct LeakyReluParams {
    slope: f32,
}

@group(0) @binding(0) var<storage, read> leaky_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> leaky_out: array<f32>;
@group(0) @binding(2) var<uniform> leaky_params: LeakyReluParams;

@compute @workgroup_size(256)
fn leaky_relu_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let x = leaky_in[idx];
    if (x > 0.0) {
        leaky_out[idx] = x;
    } else {
        leaky_out[idx] = x * leaky_params.slope;
    }
}

// === Tanh ===
@group(0) @binding(0) var<storage, read> tanh_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> tanh_out: array<f32>;

@compute @workgroup_size(256)
fn tanh_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    tanh_out[idx] = tanh(tanh_in[idx]);
}

// === Clamp ===
struct ClampParams {
    min_val: f32,
    max_val: f32,
}

@group(0) @binding(0) var<storage, read> clamp_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> clamp_out: array<f32>;
@group(0) @binding(2) var<uniform> clamp_params: ClampParams;

@compute @workgroup_size(256)
fn clamp_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    clamp_out[idx] = clamp(clamp_in[idx], clamp_params.min_val, clamp_params.max_val);
}

// === Abs ===
@group(0) @binding(0) var<storage, read> abs_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> abs_out: array<f32>;

@compute @workgroup_size(256)
fn abs_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    abs_out[idx] = abs(abs_in[idx]);
}

// === Neg ===
@group(0) @binding(0) var<storage, read> neg_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> neg_out: array<f32>;

@compute @workgroup_size(256)
fn neg_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    neg_out[idx] = -neg_in[idx];
}

// === Sqrt ===
@group(0) @binding(0) var<storage, read> sqrt_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> sqrt_out: array<f32>;

@compute @workgroup_size(256)
fn sqrt_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    sqrt_out[idx] = sqrt(sqrt_in[idx]);
}

// === Exp ===
@group(0) @binding(0) var<storage, read> exp_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> exp_out: array<f32>;

@compute @workgroup_size(256)
fn exp_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    exp_out[idx] = exp(exp_in[idx]);
}

// === Sigmoid ===
@group(0) @binding(0) var<storage, read> sigmoid_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> sigmoid_out: array<f32>;

@compute @workgroup_size(256)
fn sigmoid_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    sigmoid_out[idx] = 1.0 / (1.0 + exp(-sigmoid_in[idx]));
}

// === NanToNum ===
// Replace NaN with 0, +inf with large number, -inf with large negative number
struct NanToNumParams {
    nan_val: f32,
    posinf_val: f32,
    neginf_val: f32,
    n: u32,
}

@group(0) @binding(0) var<storage, read> ntn_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> ntn_out: array<f32>;
@group(0) @binding(2) var<uniform> ntn_params: NanToNumParams;

@compute @workgroup_size(256)
fn nan_to_num_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= ntn_params.n) { return; }

    let x = ntn_in[idx];
    // WGSL: isnan() and isinf() are not available, so check manually
    // NaN: x != x
    // +inf: x > large_val
    // -inf: x < -large_val
    let large = 3.402823e+38; // near f32 max
    if (x != x) {
        ntn_out[idx] = ntn_params.nan_val;
    } else if (x > large) {
        ntn_out[idx] = ntn_params.posinf_val;
    } else if (x < -large) {
        ntn_out[idx] = ntn_params.neginf_val;
    } else {
        ntn_out[idx] = x;
    }
}

// === Embedding lookup ===
struct EmbedParams {
    hidden: u32,
    seq_len: u32,
}

@group(0) @binding(0) var<storage, read> embed_table: array<f32>;
@group(0) @binding(1) var<storage, read> token_ids: array<f32>;
@group(0) @binding(2) var<storage, read_write> embed_out: array<f32>;
@group(0) @binding(3) var<uniform> embed_params: EmbedParams;

@compute @workgroup_size(256)
fn embed_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= embed_params.seq_len * embed_params.hidden) { return; }

    let pos = idx / embed_params.hidden;
    let dim = idx % embed_params.hidden;
    let token_id = u32(token_ids[pos]);

    embed_out[idx] = embed_table[token_id * embed_params.hidden + dim];
}
