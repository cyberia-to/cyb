// Fused RMSNorm + Q4 VecMat — two ops in one dispatch
// Reads input once, normalizes in shared memory, then multiplies by Q4 weights
// Saves: 1 dispatch + 1 buffer write/read cycle
//
// Step 1: RMSNorm (workgroup cooperative reduction)
// Step 2: Q4 dot product using normalized values from shared memory

const WG_SIZE: u32 = 256u;
const NR: u32 = 4u;

struct Params {
    n: u32,           // matmul output rows
    k: u32,           // hidden size (both norm dimension and matmul K)
    num_blocks: u32,
    u32s_per_row: u32,
    eps: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;       // [hidden]
@group(0) @binding(1) var<storage, read> norm_weight: array<f32>; // [hidden]
@group(0) @binding(2) var<storage, read> packed_weights: array<u32>;
@group(0) @binding(3) var<storage, read> scales: array<f32>;
@group(0) @binding(4) var<storage, read_write> output: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

// Shared memory: normalized activation values + reduction scratch
var<workgroup> shared_normed: array<f32, 4096>;  // max hidden size (fused path disabled for >4096)
var<workgroup> shared_sums: array<f32, 1024>;    // WG_SIZE * NR

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>,
) {
    let wg_idx = wg_id.y * num_wg.x + wg_id.x;
    let base_row = wg_idx * NR;
    let tid = local_id.x;

    // === STEP 1: RMSNorm into shared memory ===

    // Compute sum of squares (parallel)
    var sum_sq: f32 = 0.0;
    var i = tid;
    while (i < params.k) {
        let val = input[i];
        sum_sq += val * val;
        i += WG_SIZE;
    }

    // Reduce sum_sq
    shared_sums[tid] = sum_sq;
    workgroupBarrier();
    for (var stride = WG_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) { shared_sums[tid] += shared_sums[tid + stride]; }
        workgroupBarrier();
    }
    let rms = sqrt(shared_sums[0] / f32(params.k) + params.eps);

    // Normalize and store in shared memory
    i = tid;
    while (i < params.k) {
        shared_normed[i] = input[i] / rms * norm_weight[i];
        i += WG_SIZE;
    }
    workgroupBarrier();

    // === STEP 2: Q4 matmul using shared_normed as activation ===

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

            // Read from SHARED memory (already normalized) — no global memory access!
            var act0: f32 = 0.0;
            var act1: f32 = 0.0;
            if (col < params.k) { act0 = shared_normed[col]; }
            if (col + 1u < params.k) { act1 = shared_normed[col + 1u]; }

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

    // Reduce matmul results
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
