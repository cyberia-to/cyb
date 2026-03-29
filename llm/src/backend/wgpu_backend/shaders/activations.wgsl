// Compound activation functions: GeGLU, SwiGLU, GLU, PReLU

// === GeGLU ===
// Split input in half, apply gelu to first half, multiply by second half
// Input: gate [dim], up [dim] — output: [dim]
// gelu(gate) * up
@group(0) @binding(0) var<storage, read> geglu_gate: array<f32>;
@group(0) @binding(1) var<storage, read> geglu_up: array<f32>;
@group(0) @binding(2) var<storage, read_write> geglu_out: array<f32>;

@compute @workgroup_size(256)
fn geglu_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let x = geglu_gate[idx];
    // GELU approximation (tanh-based)
    let c = 0.7978845608; // sqrt(2/pi)
    let inner = c * (x + 0.044715 * x * x * x);
    let gelu_val = 0.5 * x * (1.0 + tanh(inner));
    geglu_out[idx] = gelu_val * geglu_up[idx];
}

// === SwiGLU ===
// output = silu(gate) * up = sigmoid(gate) * gate * up
// Same as silu_mul_kernel in elementwise.wgsl, provided here as explicit alias
@group(0) @binding(0) var<storage, read> swiglu_gate: array<f32>;
@group(0) @binding(1) var<storage, read> swiglu_up: array<f32>;
@group(0) @binding(2) var<storage, read_write> swiglu_output: array<f32>;

@compute @workgroup_size(256)
fn swiglu_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let g = swiglu_gate[idx];
    let sig = 1.0 / (1.0 + exp(-g));
    swiglu_output[idx] = g * sig * swiglu_up[idx];
}

// === GLU ===
// Split input in half, sigmoid first half, multiply by second half
// Input: gate [dim], value [dim] — output: [dim]
// output = sigmoid(gate) * value
@group(0) @binding(0) var<storage, read> glu_gate: array<f32>;
@group(0) @binding(1) var<storage, read> glu_value: array<f32>;
@group(0) @binding(2) var<storage, read_write> glu_out: array<f32>;

@compute @workgroup_size(256)
fn glu_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let sig = 1.0 / (1.0 + exp(-glu_gate[idx]));
    glu_out[idx] = sig * glu_value[idx];
}

// === PReLU ===
// Like leaky_relu but slope is a per-channel tensor
// Input: [n], slopes: [n], output: [n]
// output = x > 0 ? x : x * slope[i]
@group(0) @binding(0) var<storage, read> prelu_in: array<f32>;
@group(0) @binding(1) var<storage, read> prelu_slopes: array<f32>;
@group(0) @binding(2) var<storage, read_write> prelu_out: array<f32>;

@compute @workgroup_size(256)
fn prelu_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let x = prelu_in[idx];
    if (x > 0.0) {
        prelu_out[idx] = x;
    } else {
        prelu_out[idx] = x * prelu_slopes[idx];
    }
}
