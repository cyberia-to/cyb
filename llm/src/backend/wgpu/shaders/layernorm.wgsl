// Layer Normalization — mean subtraction + variance normalization + learned weight/bias
// For BERT/encoder models
// output = (input - mean) / sqrt(var + eps) * weight + bias
// Each workgroup normalizes one row (position)

enable subgroups;

const WORKGROUP_SIZE: u32 = 256u;

struct Params {
    hidden: u32,
    eps: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight: array<f32>;
@group(0) @binding(2) var<storage, read> bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

var<workgroup> wg_partial_sum: array<f32, 8>;
var<workgroup> wg_partial_sq: array<f32, 8>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let pos = wg_id.x;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WORKGROUP_SIZE / sg_size;
    let base = pos * params.hidden;

    // Phase 1: compute mean
    var partial_sum: f32 = 0.0;
    var i = tid;
    while (i < params.hidden) {
        partial_sum += input[base + i];
        i += WORKGROUP_SIZE;
    }

    partial_sum = subgroupAdd(partial_sum);
    if (sg_id == 0u) {
        wg_partial_sum[sg_idx] = partial_sum;
    }
    workgroupBarrier();

    if (sg_idx == 0u) {
        if (sg_id < num_sgs) {
            partial_sum = wg_partial_sum[sg_id];
        } else {
            partial_sum = 0.0;
        }
        partial_sum = subgroupAdd(partial_sum);
    }
    if (tid == 0u) {
        wg_partial_sum[0] = partial_sum;
    }
    workgroupBarrier();
    let mean = wg_partial_sum[0] / f32(params.hidden);

    // Phase 2: compute variance
    var partial_var: f32 = 0.0;
    i = tid;
    while (i < params.hidden) {
        let diff = input[base + i] - mean;
        partial_var += diff * diff;
        i += WORKGROUP_SIZE;
    }

    partial_var = subgroupAdd(partial_var);
    if (sg_id == 0u) {
        wg_partial_sq[sg_idx] = partial_var;
    }
    workgroupBarrier();

    if (sg_idx == 0u) {
        if (sg_id < num_sgs) {
            partial_var = wg_partial_sq[sg_id];
        } else {
            partial_var = 0.0;
        }
        partial_var = subgroupAdd(partial_var);
    }
    if (tid == 0u) {
        wg_partial_sq[0] = partial_var;
    }
    workgroupBarrier();
    let variance = wg_partial_sq[0] / f32(params.hidden);

    let inv_std = 1.0 / sqrt(variance + params.eps);

    // Phase 3: normalize
    i = tid;
    while (i < params.hidden) {
        let normalized = (input[base + i] - mean) * inv_std;
        output[base + i] = normalized * weight[i] + bias[i];
        i += WORKGROUP_SIZE;
    }
}
