// Q8_0 VecMat — signed int8 dequantization
// Our Q8 format: packed u32 (4 signed int8 values) + separate f32 scales
// Each block = 32 elements, scale = f32
// int8 range: -128..127, zero point = 0

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

// Extract signed int8 from a u32 at byte position
fn extract_i8(packed: u32, byte_idx: u32) -> f32 {
    let unsigned = (packed >> (byte_idx * 8u)) & 0xFFu;
    if (unsigned >= 128u) {
        return f32(unsigned) - 256.0;
    }
    return f32(unsigned);
}

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

    var sums: array<f32, 4>;
    sums[0] = 0.0; sums[1] = 0.0; sums[2] = 0.0; sums[3] = 0.0;

    let block_size_val = params.k / params.num_blocks;
    // Each u32 holds 4 int8 values = 4 elements
    // u32s per block = block_size / 4 = 32 / 4 = 8

    var u32_idx = tid;
    while (u32_idx < params.u32s_per_row) {
        // Determine which block and position within block
        let elem_idx = u32_idx * 4u;
        let block_idx = elem_idx / block_size_val;
        let col = elem_idx;

        for (var r = 0u; r < NR; r++) {
            let row = base_row + r;
            if (row >= params.n) { break; }

            let packed = packed_weights[row * params.u32s_per_row + u32_idx];
            let scale = scales[row * params.num_blocks + block_idx];

            let b0 = extract_i8(packed, 0u) * scale;
            let b1 = extract_i8(packed, 1u) * scale;
            let b2 = extract_i8(packed, 2u) * scale;
            let b3 = extract_i8(packed, 3u) * scale;

            var a0: f32 = 0.0;
            var a1: f32 = 0.0;
            var a2: f32 = 0.0;
            var a3: f32 = 0.0;
            if (col < params.k) { a0 = activation[col]; }
            if (col + 1u < params.k) { a1 = activation[col + 1u]; }
            if (col + 2u < params.k) { a2 = activation[col + 2u]; }
            if (col + 3u < params.k) { a3 = activation[col + 3u]; }

            sums[r] += a0 * b0 + a1 * b1 + a2 * b2 + a3 * b3;
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
