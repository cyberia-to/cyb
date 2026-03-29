// Q4 Tiled VecMat — NR=4 output rows per workgroup
// Workgroup tree reduction (subgroups not yet available in WGSL/naga)

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

var<workgroup> shared_sums: array<f32, 1024>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>,
) {
    let wg_idx = wg_id.y * num_wg.x + wg_id.x;
    let base_row = wg_idx * NR;
    let tid = local_id.x;

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

            var act0: f32 = 0.0;
            var act1: f32 = 0.0;
            if (col < params.k) { act0 = activation[col]; }
            if (col + 1u < params.k) { act1 = activation[col + 1u]; }

            for (var r = 0u; r < NR; r++) {
                let row = base_row + r;
                if (row >= params.n) { break; }

                let packed = packed_weights[row * params.u32s_per_row + u32_idx];
                let byte_val = (packed >> (b * 8u)) & 0xFFu;
                let scale = scales[row * params.num_blocks + block_idx];

                if (col < params.k) {
                    sums[r] += act0 * (f32(byte_val & 0xFu) - 8.0) * scale;
                }
                if (col + 1u < params.k) {
                    sums[r] += act1 * (f32((byte_val >> 4u) & 0xFu) - 8.0) * scale;
                }
            }
        }

        u32_idx += WG_SIZE;
    }

    for (var r = 0u; r < NR; r++) {
        shared_sums[r * WG_SIZE + tid] = sums[r];
    }
    workgroupBarrier();

    for (var stride = WG_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            for (var r = 0u; r < NR; r++) {
                shared_sums[r * WG_SIZE + tid] += shared_sums[r * WG_SIZE + tid + stride];
            }
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        for (var r = 0u; r < NR; r++) {
            let row = base_row + r;
            if (row < params.n) {
                output[row] = shared_sums[r * WG_SIZE];
            }
        }
    }
}
