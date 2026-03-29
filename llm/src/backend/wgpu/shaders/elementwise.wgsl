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
