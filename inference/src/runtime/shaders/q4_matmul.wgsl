// Q4_0 Vector × Matrix Multiply — ported from llama.cpp concepts
// Each workgroup computes one output row using parallel reduction
//
// Weights: packed 4-bit (uint8 stored as u32, 8 weights per u32)
// Scales: f32 per block
// Activation: f32 vector [K]
// Output: f32 vector [N]
//
// Workgroup: 256 threads cooperatively reduce over K dimension
// Uses shared memory for final reduction

const WORKGROUP_SIZE: u32 = 256u;
const BLOCK_SIZE: u32 = 32u;      // Q4 quantization block size
const HALF_BLOCK: u32 = 16u;       // block_size / 2 (bytes per block)
const WEIGHTS_PER_U32: u32 = 8u;   // 4 bytes × 2 nibbles

struct Params {
    n: u32,              // output rows
    k: u32,              // input dimension
    num_blocks: u32,     // k / block_size
    u32s_per_row: u32,   // num_blocks * half_block / 4
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> packed_weights: array<u32>;
@group(0) @binding(2) var<storage, read> scales: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

var<workgroup> shared_sums: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let row = wg_id.x;
    let tid = local_id.x;

    if (row >= params.n) { return; }

    // Each thread processes a chunk of the K dimension
    // Thread tid handles u32 indices: tid, tid + WORKGROUP_SIZE, tid + 2*WORKGROUP_SIZE, ...
    var partial_sum: f32 = 0.0;

    var u32_idx = tid;
    while (u32_idx < params.u32s_per_row) {
        let packed = packed_weights[row * params.u32s_per_row + u32_idx];

        // Unpack 4 bytes (8 weights) from this u32
        for (var b = 0u; b < 4u; b++) {
            let byte_val = (packed >> (b * 8u)) & 0xFFu;
            let byte_pos = u32_idx * 4u + b;
            let block_idx = byte_pos / HALF_BLOCK;
            let within_block = byte_pos % HALF_BLOCK;
            let scale = scales[row * params.num_blocks + block_idx];
            let col = block_idx * BLOCK_SIZE + within_block * 2u;

            // Dequantize and multiply — 2 weights per byte
            if (col < params.k) {
                let lo = f32(byte_val & 0xFu) - 8.0;
                partial_sum += activation[col] * lo * scale;
            }
            if (col + 1u < params.k) {
                let hi = f32((byte_val >> 4u) & 0xFu) - 8.0;
                partial_sum += activation[col + 1u] * hi * scale;
            }
        }

        u32_idx += WORKGROUP_SIZE;
    }

    // Workgroup reduction via shared memory
    shared_sums[tid] = partial_sum;
    workgroupBarrier();

    // Tree reduction: 256 → 128 → 64 → 32 → 16 → 8 → 4 → 2 → 1
    for (var stride = WORKGROUP_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (tid < stride) {
            shared_sums[tid] += shared_sums[tid + stride];
        }
        workgroupBarrier();
    }

    // Thread 0 writes the result
    if (tid == 0u) {
        output[row] = shared_sums[0];
    }
}
