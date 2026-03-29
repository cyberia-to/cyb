// Argmax — find the index of maximum value
// Uses subgroupMax for fast SIMD max reduction
// Input: array of f32 [N]
// Output: [1] u32 (index of max)

enable subgroups;

const WG_SIZE: u32 = 256u;

struct Params {
    n: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

// Cross-subgroup scratch for (value, index) pairs
var<workgroup> wg_vals: array<f32, 8>;
var<workgroup> wg_idxs: array<u32, 8>;

@compute @workgroup_size(256)
fn main(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WG_SIZE / sg_size;

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

    // Subgroup max reduction — instant SIMD max
    let sg_max = subgroupMax(local_max);

    // The first lane in the subgroup whose value equals sg_max writes the result.
    // Use subgroupMin to pick the lowest sg_id among winners (deterministic).
    var winner_sg_id = WG_SIZE; // sentinel
    if (local_max == sg_max) {
        winner_sg_id = sg_id;
    }
    winner_sg_id = subgroupMin(winner_sg_id);

    if (sg_id == winner_sg_id) {
        wg_vals[sg_idx] = local_max;
        wg_idxs[sg_idx] = local_idx;
    }
    workgroupBarrier();

    // Cross-subgroup reduction (first subgroup only, num_sgs entries)
    if (sg_idx == 0u && sg_id < num_sgs) {
        local_max = wg_vals[sg_id];
        local_idx = wg_idxs[sg_id];

        let global_max = subgroupMax(local_max);

        var final_winner = WG_SIZE;
        if (local_max == global_max) {
            final_winner = sg_id;
        }
        final_winner = subgroupMin(final_winner);

        if (sg_id == final_winner) {
            output[0] = local_idx;
        }
    }
}
