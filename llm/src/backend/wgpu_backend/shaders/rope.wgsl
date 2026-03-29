// Rotary Position Embeddings — ported from llama.cpp kernel_rope_neox
// Applies rotation to Q/K vectors based on position
//
// Input x: [total_elements] (flattened [batch, heads, seq, head_dim])
// cos_cache, sin_cache: [seq_len * half_dim] (pre-sliced for current positions)
// Output: same shape as input with rotation applied

struct Params {
    half_dim: u32,      // head_dim / 2
    head_dim: u32,      // full head dimension
    seq_len: u32,       // number of positions
    total_elements: u32, // total elements in tensor
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> cos_cache: array<f32>;
@group(0) @binding(2) var<storage, read> sin_cache: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.total_elements) { return; }

    let d = idx % params.head_dim;                           // position within head_dim
    let seq_idx = (idx / params.head_dim) % params.seq_len;  // which sequence position

    if (d < params.half_dim) {
        // First half: x1 * cos - x2 * sin
        let cos_idx = seq_idx * params.half_dim + d;
        let x1 = input[idx];
        let x2 = input[idx + params.half_dim];
        output[idx] = x1 * cos_cache[cos_idx] - x2 * sin_cache[cos_idx];
    } else {
        // Second half: x1 * sin + x2 * cos
        let d2 = d - params.half_dim;
        let cos_idx = seq_idx * params.half_dim + d2;
        let x1 = input[idx - params.half_dim];
        let x2 = input[idx];
        output[idx] = x1 * sin_cache[cos_idx] + x2 * cos_cache[cos_idx];
    }
}
