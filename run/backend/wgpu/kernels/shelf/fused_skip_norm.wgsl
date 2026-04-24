// Fused Skip Connection + RMSNorm
// output = rmsnorm(input + skip) * weight
// Also writes skip_output = input + skip (for next residual)
// Saves: 1 add dispatch + 1 buffer write/read

const WG_SIZE: u32 = 256u;

struct Params {
    hidden: u32,
    eps: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> skip: array<f32>;
@group(0) @binding(2) var<storage, read> weight: array<f32>;
@group(0) @binding(3) var<storage, read_write> normed_output: array<f32>;
@group(0) @binding(4) var<storage, read_write> skip_output: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

var<workgroup> shared_sums: array<f32, 256>;
var<workgroup> shared_added: array<f32, 4096>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let pos = wg_id.x;
    let tid = local_id.x;
    let base = pos * params.hidden;

    // Step 1: Compute input + skip and store in shared + skip_output
    var sum_sq: f32 = 0.0;
    var i = tid;
    while (i < params.hidden) {
        let added = input[base + i] + skip[base + i];
        shared_added[i] = added;
        skip_output[base + i] = added;
        sum_sq += added * added;
        i += WG_SIZE;
    }

    // Step 2: Reduce sum of squares
    shared_sums[tid] = sum_sq;
    workgroupBarrier();
    for (var stride = WG_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) { shared_sums[tid] += shared_sums[tid + stride]; }
        workgroupBarrier();
    }
    let rms = sqrt(shared_sums[0] / f32(params.hidden) + params.eps);

    // Step 3: Normalize
    i = tid;
    while (i < params.hidden) {
        normed_output[base + i] = shared_added[i] / rms * weight[i];
        i += WG_SIZE;
    }
}
