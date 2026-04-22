// Ternary (BitNet) Vector x Matrix multiply
// activation: [K] f32
// weight: [N, K] ternary {-1, 0, +1} packed as 2 bits per value into u32
//   Each u32 holds 16 ternary values (2 bits each):
//     00 = 0, 01 = +1, 10 = -1, 11 = reserved
// scale: [N] f32 (per-row scale factor)
// output: [N] f32
//
// BitNet key insight: no multiplication needed for ternary weights.
// weight = +1 -> add activation
// weight = -1 -> subtract activation
// weight =  0 -> skip
// This makes the inner loop ~3x faster than f16 matmul.

enable subgroups;

const WORKGROUP_SIZE: u32 = 256u;

struct TernaryParams {
    n: u32,
    k: u32,
    u32s_per_row: u32,  // ceil(K / 16)
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> tern_activation: array<f32>;
@group(0) @binding(1) var<storage, read> tern_weight_packed: array<u32>;  // 2-bit packed ternary
@group(0) @binding(2) var<storage, read> tern_scale: array<f32>;          // per-row scale
@group(0) @binding(3) var<storage, read_write> tern_output: array<f32>;
@group(0) @binding(4) var<uniform> tern_params: TernaryParams;

var<workgroup> tern_wg_partial: array<f32, 8>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let row = wg_id.y * num_wg.x + wg_id.x;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WORKGROUP_SIZE / sg_size;

    if (row >= tern_params.n) { return; }

    var partial_sum: f32 = 0.0;
    let base = row * tern_params.u32s_per_row;

    // Each u32 contains 16 ternary values (2 bits each)
    var u32_idx = tid;
    while (u32_idx < tern_params.u32s_per_row) {
        let packed = tern_weight_packed[base + u32_idx];
        let k_base = u32_idx * 16u;

        // Unroll: process 16 ternary values from one u32
        for (var bit = 0u; bit < 16u; bit++) {
            let k_idx = k_base + bit;
            if (k_idx >= tern_params.k) { break; }

            let val = (packed >> (bit * 2u)) & 3u;
            // 00 = 0 (skip), 01 = +1 (add), 10 = -1 (subtract)
            if (val == 1u) {
                partial_sum += tern_activation[k_idx];
            } else if (val == 2u) {
                partial_sum -= tern_activation[k_idx];
            }
            // val == 0 or 3: skip (zero contribution)
        }

        u32_idx += WORKGROUP_SIZE;
    }

    // Subgroup reduction
    partial_sum = subgroupAdd(partial_sum);

    if (sg_id == 0u) {
        tern_wg_partial[sg_idx] = partial_sum;
    }
    workgroupBarrier();

    if (sg_idx == 0u) {
        if (sg_id < num_sgs) {
            partial_sum = tern_wg_partial[sg_id];
        } else {
            partial_sum = 0.0;
        }
        partial_sum = subgroupAdd(partial_sum);
    }

    if (tid == 0u) {
        // Apply per-row scale
        tern_output[row] = partial_sum * tern_scale[row];
    }
}
