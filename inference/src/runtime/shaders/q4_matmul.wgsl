// Q4 Tiled VecMat — processes NR=4 output rows per workgroup
// Amortizes activation vector read across 4 rows
// Each thread accumulates 4 partial sums, then reduces
//
// Weights: packed 4-bit (u32, 8 weights per u32)
// For Qwen3: K=1024 → 128 u32s per row, 256 threads → 2 iterations max

const WG_SIZE: u32 = 256u;
const NR: u32 = 4u;  // rows per workgroup

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

var<workgroup> shared_sums: array<f32, 1024>;  // WG_SIZE * NR

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>,
) {
    let wg_idx = wg_id.y * num_wg.x + wg_id.x;
    let base_row = wg_idx * NR;
    let tid = local_id.x;

    // Initialize partial sums for NR rows
    var sums: array<f32, 4>;
    sums[0] = 0.0; sums[1] = 0.0; sums[2] = 0.0; sums[3] = 0.0;

    // Each thread processes u32s: tid, tid+WG_SIZE, ...
    var u32_idx = tid;
    while (u32_idx < params.u32s_per_row) {
        // Compute column positions for this u32 (same for all rows)
        let byte_offset = u32_idx * 4u;

        // Process 4 bytes (8 weights)
        for (var b = 0u; b < 4u; b++) {
            let byte_pos = byte_offset + b;
            let block_idx = byte_pos / (params.k / params.num_blocks / 2u);
            let half_bs = params.k / params.num_blocks / 2u;
            let within_block = byte_pos % half_bs;
            let block_size = params.k / params.num_blocks;
            let col = block_idx * block_size + within_block * 2u;

            // Read activation ONCE for all rows
            var act0: f32 = 0.0;
            var act1: f32 = 0.0;
            if (col < params.k) { act0 = activation[col]; }
            if (col + 1u < params.k) { act1 = activation[col + 1u]; }

            // Process each row
            for (var r = 0u; r < NR; r++) {
                let row = base_row + r;
                if (row >= params.n) { break; }

                let packed = packed_weights[row * params.u32s_per_row + u32_idx];
                let byte_val = (packed >> (b * 8u)) & 0xFFu;
                let scale = scales[row * params.num_blocks + block_idx];

                if (col < params.k) {
                    let lo = f32(byte_val & 0xFu) - 8.0;
                    sums[r] += act0 * lo * scale;
                }
                if (col + 1u < params.k) {
                    let hi = f32((byte_val >> 4u) & 0xFu) - 8.0;
                    sums[r] += act1 * hi * scale;
                }
            }
        }

        u32_idx += WG_SIZE;
    }

    // Store partial sums to shared memory
    for (var r = 0u; r < NR; r++) {
        shared_sums[r * WG_SIZE + tid] = sums[r];
    }
    workgroupBarrier();

    // Tree reduction for each row
    for (var stride = WG_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            for (var r = 0u; r < NR; r++) {
                shared_sums[r * WG_SIZE + tid] += shared_sums[r * WG_SIZE + tid + stride];
            }
        }
        workgroupBarrier();
    }

    // Write results
    if (tid == 0u) {
        for (var r = 0u; r < NR; r++) {
            let row = base_row + r;
            if (row < params.n) {
                output[row] = shared_sums[r * WG_SIZE];
            }
        }
    }
}
