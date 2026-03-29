// Fused Attention for decode step (seq_len = 1)
// Computes: softmax(Q × K^T / sqrt(d)) × V in single kernel
//
// Q: [batch * heads * head_dim] (flattened, one position)
// K: [batch * heads * total_seq * head_dim] (full KV cache)
// V: [batch * heads * total_seq * head_dim]
// Output: [batch * heads * head_dim]
//
// Each workgroup handles one (batch, head, dim) output element
// Threads cooperate on the attention score reduction

struct Params {
    head_dim: u32,
    total_seq: u32,
    num_heads: u32,
    scale: f32,
}

@group(0) @binding(0) var<storage, read> q: array<f32>;
@group(0) @binding(1) var<storage, read> k: array<f32>;
@group(0) @binding(2) var<storage, read> v: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Each invocation handles one output element: (head, dim)
    let head_idx = gid.x / params.head_dim;
    let d = gid.x % params.head_dim;

    if (head_idx >= params.num_heads) { return; }

    let q_base = head_idx * params.head_dim;
    let kv_base = head_idx * params.total_seq * params.head_dim;

    // Step 1: compute attention scores and find max (numerically stable softmax)
    var max_score: f32 = -1e30;
    for (var t = 0u; t < params.total_seq; t++) {
        var dot: f32 = 0.0;
        for (var dd = 0u; dd < params.head_dim; dd++) {
            dot += q[q_base + dd] * k[kv_base + t * params.head_dim + dd];
        }
        dot *= params.scale;
        max_score = max(max_score, dot);
    }

    // Step 2: compute exp sum
    var exp_sum: f32 = 0.0;
    for (var t = 0u; t < params.total_seq; t++) {
        var dot: f32 = 0.0;
        for (var dd = 0u; dd < params.head_dim; dd++) {
            dot += q[q_base + dd] * k[kv_base + t * params.head_dim + dd];
        }
        exp_sum += exp(dot * params.scale - max_score);
    }

    // Step 3: weighted sum of V for this dimension
    var result: f32 = 0.0;
    for (var t = 0u; t < params.total_seq; t++) {
        var dot: f32 = 0.0;
        for (var dd = 0u; dd < params.head_dim; dd++) {
            dot += q[q_base + dd] * k[kv_base + t * params.head_dim + dd];
        }
        let attn_weight = exp(dot * params.scale - max_score) / exp_sum;
        result += attn_weight * v[kv_base + t * params.head_dim + d];
    }

    output[gid.x] = result;
}
