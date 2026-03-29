// Fused Attention Decode — one workgroup per head
// Computes: softmax(Q·K^T / scale) · V
//
// Q: [num_heads * head_dim]
// K, V: [num_heads * total_seq * head_dim]
// Output: [num_heads * head_dim]
//
// Each workgroup handles one attention head.
// Threads cooperate on dot products and reductions.
// Uses online softmax (single pass for max + exp_sum).

const WG_SIZE: u32 = 256u;

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

// Shared memory for scores, max, exp_sum
var<workgroup> scores: array<f32, 2048>;  // max total_seq we support
var<workgroup> shared_max: array<f32, 256>;
var<workgroup> shared_sum: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let head = wg_id.x;
    let tid = local_id.x;

    if (head >= params.num_heads) { return; }

    let q_base = head * params.head_dim;
    let kv_base = head * params.total_seq * params.head_dim;

    // Step 1: Compute attention scores (parallel over total_seq)
    // Each thread computes scores for positions: tid, tid+WG_SIZE, ...
    var local_max: f32 = -1000000.0;
    var t = tid;
    while (t < params.total_seq) {
        var dot: f32 = 0.0;
        for (var d = 0u; d < params.head_dim; d++) {
            dot += q[q_base + d] * k[kv_base + t * params.head_dim + d];
        }
        dot *= params.scale;
        scores[t] = dot;
        local_max = max(local_max, dot);
        t += WG_SIZE;
    }

    // Step 2: Parallel max reduction
    shared_max[tid] = local_max;
    workgroupBarrier();

    for (var stride = WG_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            shared_max[tid] = max(shared_max[tid], shared_max[tid + stride]);
        }
        workgroupBarrier();
    }
    let global_max = shared_max[0];

    // Step 3: Compute exp(score - max) and sum (parallel)
    var local_sum: f32 = 0.0;
    t = tid;
    while (t < params.total_seq) {
        let e = exp(scores[t] - global_max);
        scores[t] = e;  // reuse scores array for exp values
        local_sum += e;
        t += WG_SIZE;
    }

    shared_sum[tid] = local_sum;
    workgroupBarrier();

    for (var stride = WG_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            shared_sum[tid] += shared_sum[tid + stride];
        }
        workgroupBarrier();
    }
    let exp_total = shared_sum[0];

    // Normalize scores to softmax weights
    workgroupBarrier();
    t = tid;
    while (t < params.total_seq) {
        scores[t] = scores[t] / exp_total;
        t += WG_SIZE;
    }
    workgroupBarrier();

    // Step 4: Weighted sum of V (parallel over head_dim)
    // Each thread handles dimensions: tid, tid+WG_SIZE, ...
    var d = tid;
    while (d < params.head_dim) {
        var weighted_sum: f32 = 0.0;
        for (var tt = 0u; tt < params.total_seq; tt++) {
            weighted_sum += scores[tt] * v[kv_base + tt * params.head_dim + d];
        }
        output[q_base + d] = weighted_sum;
        d += WG_SIZE;
    }
}
