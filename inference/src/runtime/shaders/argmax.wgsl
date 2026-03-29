// Argmax — find the index of maximum value
// Uses parallel reduction in shared memory
// Input: array of f32 [N]
// Output: [1] u32 (index of max)

const WG_SIZE: u32 = 256u;

struct Params {
    n: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

// Shared memory for reduction: (value, index) pairs
var<workgroup> shared_vals: array<f32, 256>;
var<workgroup> shared_idxs: array<u32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let tid = local_id.x;

    // Each thread finds max in its chunk
    var local_max: f32 = -1000000000.0;
    var local_idx: u32 = 0u;

    var i = tid;
    while (i < params.n) {
        let val = input[i];
        if (val > local_max) {
            local_max = val;
            local_idx = i;
        }
        i += WG_SIZE;
    }

    shared_vals[tid] = local_max;
    shared_idxs[tid] = local_idx;
    workgroupBarrier();

    // Tree reduction
    for (var stride = WG_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            if (shared_vals[tid + stride] > shared_vals[tid]) {
                shared_vals[tid] = shared_vals[tid + stride];
                shared_idxs[tid] = shared_idxs[tid + stride];
            }
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        output[0] = shared_idxs[0];
    }
}
