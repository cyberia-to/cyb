// Q4 VecMat — llama.cpp-style dequant optimization
// Key tricks from ggml-metal.metal:
// 1. Zero point (-8) factored out: d * (sumy * -8 + acc)
// 2. Scale applied ONCE per block, not per element
// 3. Subgroup reduction for instant SIMD sum

enable subgroups;

const WG_SIZE: u32 = 256u;
const NR: u32 = 4u;

struct Params {
    n: u32,
    k: u32,
    num_blocks: u32,
    u32s_per_row: u32,
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> packed_weights: array<u32>;
@group(0) @binding(2) var<storage, read> scales: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

var<workgroup> wg_partial: array<f32, 32>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let wg_idx = wg_id.y * num_wg.x + wg_id.x;
    let base_row = wg_idx * NR;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WG_SIZE / sg_size;

    // Accumulate: raw dot product (without zero point) and activation sum per block
    var sums: array<f32, 4>;
    sums[0] = 0.0; sums[1] = 0.0; sums[2] = 0.0; sums[3] = 0.0;

    let half_bs = params.k / params.num_blocks / 2u;
    let block_size_val = params.k / params.num_blocks;

    var u32_idx = tid;
    while (u32_idx < params.u32s_per_row) {
        let byte_offset = u32_idx * 4u;

        for (var b = 0u; b < 4u; b++) {
            let byte_pos = byte_offset + b;
            let block_idx = byte_pos / half_bs;
            let within_block = byte_pos % half_bs;
            let col = block_idx * block_size_val + within_block * 2u;

            // Read activation (shared across all rows)
            var act0: f32 = 0.0;
            var act1: f32 = 0.0;
            if (col < params.k) { act0 = activation[col]; }
            if (col + 1u < params.k) { act1 = activation[col + 1u]; }

            // Pre-compute activation sum for this block pair (for zero-point factoring)
            let act_sum = act0 + act1;

            for (var r = 0u; r < NR; r++) {
                let row = base_row + r;
                if (row >= params.n) { break; }

                let packed = packed_weights[row * params.u32s_per_row + u32_idx];
                let byte_val = (packed >> (b * 8u)) & 0xFFu;
                let scale = scales[row * params.num_blocks + block_idx];

                // llama.cpp trick: accumulate raw nibble × activation, subtract zero point via sum
                // raw_dot = lo_nibble * act0 + hi_nibble * act1
                // Corrected: dot = scale * (raw_dot - 8 * (act0 + act1))
                let lo = f32(byte_val & 0xFu);
                let hi = f32((byte_val >> 4u) & 0xFu);
                let raw_dot = lo * act0 + hi * act1;
                sums[r] += scale * (raw_dot - 8.0 * act_sum);
            }
        }

        u32_idx += WG_SIZE;
    }

    // Subgroup + cross-subgroup reduction
    for (var r = 0u; r < NR; r++) {
        sums[r] = subgroupAdd(sums[r]);
    }

    if (sg_id == 0u) {
        for (var r = 0u; r < NR; r++) {
            wg_partial[sg_idx * NR + r] = sums[r];
        }
    }
    workgroupBarrier();

    if (sg_idx == 0u) {
        if (sg_id < num_sgs) {
            for (var r = 0u; r < NR; r++) {
                sums[r] = wg_partial[sg_id * NR + r];
            }
        } else {
            for (var r = 0u; r < NR; r++) {
                sums[r] = 0.0;
            }
        }
        for (var r = 0u; r < NR; r++) {
            sums[r] = subgroupAdd(sums[r]);
        }
        if (sg_id == 0u) {
            for (var r = 0u; r < NR; r++) {
                let row = base_row + r;
                if (row < params.n) {
                    output[row] = sums[r];
                }
            }
        }
    }
}
