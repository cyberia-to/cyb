// RMS Normalization — ported from llama.cpp
// output[i] = input[i] / rms * weight[i]
// rms = sqrt(mean(input^2) + eps)
//
// Each workgroup normalizes one position (one row of [batch*seq, hidden])
// Threads cooperatively compute sum of squares over hidden dimension

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
    let pos = wg_id.x;  // which position (row)
    let tid = local_id.x;
    let base = pos * params.hidden;

    // Parallel sum of squares
    var sum_sq: f32 = 0.0;
    var i = tid;
    while (i < params.hidden) {
        let val = input[base + i];
        sum_sq += val * val;
        i += WORKGROUP_SIZE;
    }

    // Reduction
    shared_sums[tid] = sum_sq;
    workgroupBarrier();

    for (var stride = WORKGROUP_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            shared_sums[tid] += shared_sums[tid + stride];
        }
        workgroupBarrier();
    }

    let rms = sqrt(shared_sums[0] / f32(params.hidden) + params.eps);

    // Normalize and scale
    i = tid;
    while (i < params.hidden) {
        output[base + i] = input[base + i] / rms * weight[i];
        i += WORKGROUP_SIZE;
    }
}
