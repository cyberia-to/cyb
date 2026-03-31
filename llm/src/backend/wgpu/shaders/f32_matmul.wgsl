// f32 batch Matrix × Matrix multiply
// activation: [P, K] f32  (P = batch/seq_len, 1 for decode VecMat)
// weight: [N, K] f32 (row-major)
// output: [P, N] f32
// wg_id.x = output row (0..N), wg_id.y = batch row (0..P)
// Each workgroup computes one (batch, output) element using subgroup reduction.

enable subgroups;

const WORKGROUP_SIZE: u32 = 256u;

struct Params {
    n: u32,
    k: u32,
    p: u32,
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> weight: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> wg_partial: array<f32, 8>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let out_row = wg_id.x;
    let batch_row = wg_id.y;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WORKGROUP_SIZE / sg_size;

    if (out_row >= params.n || batch_row >= params.p) { return; }

    var partial_sum: f32 = 0.0;
    let w_base = out_row * params.k;
    let a_base = batch_row * params.k;

    var i = tid;
    while (i < params.k) {
        partial_sum += activation[a_base + i] * weight[w_base + i];
        i += WORKGROUP_SIZE;
    }

    partial_sum = subgroupAdd(partial_sum);

    if (sg_id == 0u) {
        wg_partial[sg_idx] = partial_sum;
    }
    workgroupBarrier();

    if (sg_idx == 0u && sg_id < num_sgs) {
        partial_sum = wg_partial[sg_id];
        partial_sum = subgroupAdd(partial_sum);
    }

    if (tid == 0u) {
        output[batch_row * params.n + out_row] = partial_sum;
    }
}
