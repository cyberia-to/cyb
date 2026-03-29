// f32 Vector × Matrix multiply (for lm_head with tied embed weights)
// activation: [K] f32
// weight: [N, K] f32 (row-major)
// output: [N] f32
// Each workgroup computes one output element using parallel reduction

const WORKGROUP_SIZE: u32 = 256u;

struct Params {
    n: u32,
    k: u32,
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> weight: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> shared_sums: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>,
) {
    let row = wg_id.y * num_wg.x + wg_id.x;
    let tid = local_id.x;

    if (row >= params.n) { return; }

    var partial_sum: f32 = 0.0;
    let base = row * params.k;

    var i = tid;
    while (i < params.k) {
        partial_sum += activation[i] * weight[base + i];
        i += WORKGROUP_SIZE;
    }

    shared_sums[tid] = partial_sum;
    workgroupBarrier();

    for (var stride = WORKGROUP_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            shared_sums[tid] += shared_sums[tid + stride];
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        output[row] = shared_sums[0];
    }
}
