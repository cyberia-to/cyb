// RMS Normalization — each workgroup normalizes one row
// Uses subgroupAdd for fast SIMD reduction

enable subgroups;

const WORKGROUP_SIZE: u32 = 256u;

struct Params {
    hidden: u32,
    eps: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

// Only need cross-subgroup scratch (8 subgroups max)
var<workgroup> wg_partial: array<f32, 8>;

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

    var sum_sq: f32 = 0.0;
    var i = tid;
    while (i < params.hidden) {
        let val = input[base + i];
        sum_sq += val * val;
        i += WORKGROUP_SIZE;
    }

    // Subgroup reduction — instant SIMD sum
    sum_sq = subgroupAdd(sum_sq);

    // Cross-subgroup reduction
    if (sg_id == 0u) {
        wg_partial[sg_idx] = sum_sq;
    }
    workgroupBarrier();

    if (sg_idx == 0u && sg_id < num_sgs) {
        sum_sq = wg_partial[sg_id];
        sum_sq = subgroupAdd(sum_sq);
    }

    // Broadcast result via shared memory
    if (tid == 0u) {
        wg_partial[0] = sum_sq;
    }
    workgroupBarrier();
    let total_sum_sq = wg_partial[0];

    let rms = sqrt(total_sum_sq / f32(params.hidden) + params.eps);

    i = tid;
    while (i < params.hidden) {
        output[base + i] = input[base + i] / rms * weight[i];
        i += WORKGROUP_SIZE;
    }
}
