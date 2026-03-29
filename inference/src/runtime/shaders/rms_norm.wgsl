// RMS Normalization — each workgroup normalizes one row

const WORKGROUP_SIZE: u32 = 256u;

struct Params {
    hidden: u32,
    eps: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> shared_sums: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let pos = wg_id.x;
    let tid = local_id.x;
    let base = pos * params.hidden;

    var sum_sq: f32 = 0.0;
    var i = tid;
    while (i < params.hidden) {
        let val = input[base + i];
        sum_sq += val * val;
        i += WORKGROUP_SIZE;
    }

    shared_sums[tid] = sum_sq;
    workgroupBarrier();

    for (var stride = WORKGROUP_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            shared_sums[tid] += shared_sums[tid + stride];
        }
        workgroupBarrier();
    }

    let rms = sqrt(shared_sums[0] / f32(params.hidden) + params.eps);

    i = tid;
    while (i < params.hidden) {
        output[base + i] = input[base + i] / rms * weight[i];
        i += WORKGROUP_SIZE;
    }
}
