// Fused Attention Decode — one workgroup per head
// Computes: softmax(Q·K^T / scale) · V
// Uses subgroupMax/subgroupAdd for fast SIMD reductions
//
// Q: [num_heads * head_dim]
// K, V: [num_heads * total_seq * head_dim]
// Output: [num_heads * head_dim]
//
// Each workgroup handles one attention head.
// Threads cooperate on dot products and reductions.
// Uses online softmax (single pass for max + exp_sum).

enable subgroups;

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

// Shared memory for scores and cross-subgroup reduction scratch
var<workgroup> scores: array<f32, 2048>;  // max total_seq we support
var<workgroup> wg_partial: array<f32, 8>; // 8 subgroups max

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let head = wg_id.x;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WG_SIZE / sg_size;

    if (head >= params.num_heads) { return; }

    let q_base = head * params.head_dim;
    let kv_base = head * params.total_seq * params.head_dim;

    // Step 1: Compute attention scores (parallel over total_seq)
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

    // Step 2: Subgroup max reduction
    local_max = subgroupMax(local_max);

    // Cross-subgroup max reduction
    if (sg_id == 0u) {
        wg_partial[sg_idx] = local_max;
    }
    workgroupBarrier();

    if (sg_idx == 0u && sg_id < num_sgs) {
        local_max = wg_partial[sg_id];
        local_max = subgroupMax(local_max);
    }

    // Broadcast global_max
    if (tid == 0u) {
        wg_partial[0] = local_max;
    }
    workgroupBarrier();
    let global_max = wg_partial[0];

    // Step 3: Compute exp(score - max) and sum (parallel)
    var local_sum: f32 = 0.0;
    t = tid;
    while (t < params.total_seq) {
        let e = exp(scores[t] - global_max);
        scores[t] = e;  // reuse scores array for exp values
        local_sum += e;
        t += WG_SIZE;
    }

    // Subgroup sum reduction
    local_sum = subgroupAdd(local_sum);

    // Cross-subgroup sum reduction
    if (sg_id == 0u) {
        wg_partial[sg_idx] = local_sum;
    }
    workgroupBarrier();

    if (sg_idx == 0u && sg_id < num_sgs) {
        local_sum = wg_partial[sg_id];
        local_sum = subgroupAdd(local_sum);
    }

    // Broadcast exp_total
    if (tid == 0u) {
        wg_partial[0] = local_sum;
    }
    workgroupBarrier();
    let exp_total = wg_partial[0];

    // Normalize scores to softmax weights
    t = tid;
    while (t < params.total_seq) {
        scores[t] = scores[t] / exp_total;
        t += WG_SIZE;
    }
    workgroupBarrier();

    // Step 4: Weighted sum of V (parallel over head_dim)
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
