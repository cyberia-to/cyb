// Q4 Vector × Matrix multiply
// Input: activation vector [1, K] (f32)
// Weights: packed 4-bit [N, K/block_size, block_size/2] (u32 packed)
// Scales: per-block [N, num_blocks] (f32)
// Output: [1, N] (f32)
//
// Each workgroup computes one output row (one N).
// Threads within workgroup cooperatively reduce over K dimension.

struct Params {
    n: u32,          // number of output rows
    k: u32,          // input dimension
    block_size: u32, // quantization block size (32 or 128)
    num_blocks: u32, // k / block_size
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> weights_packed: array<u32>; // 4 bytes = 8 nibbles = 8 weights
@group(0) @binding(2) var<storage, read> scales: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

const WORKGROUP_SIZE: u32 = 256u;

var<workgroup> shared_sum: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let row = wid.x; // output row index (N dimension)
    if (row >= params.n) { return; }

    let tid = lid.x;
    let half_bs = params.block_size / 2u;

    // Each thread accumulates a partial sum over K
    // Thread tid processes elements: tid, tid+WORKGROUP_SIZE, tid+2*WORKGROUP_SIZE, ...
    var partial_sum: f32 = 0.0;

    // Total packed bytes per row = num_blocks * half_bs
    let bytes_per_row = params.num_blocks * half_bs;
    // We pack 4 bytes into one u32, so u32s per row = bytes_per_row / 4
    let u32s_per_row = bytes_per_row / 4u;

    // Process packed u32 values
    // Each u32 contains 4 bytes = 8 nibbles = 8 weights
    var u32_idx = tid;
    while (u32_idx < u32s_per_row) {
        let packed_u32 = weights_packed[row * u32s_per_row + u32_idx];

        // Byte position within row
        let byte_offset = u32_idx * 4u;

        // Process 4 bytes (8 weights) from this u32
        for (var b = 0u; b < 4u; b++) {
            let byte_pos = byte_offset + b;
            let byte_val = (packed_u32 >> (b * 8u)) & 0xFFu;

            // Which block does this byte belong to?
            let block_idx = byte_pos / half_bs;
            let within_block = byte_pos % half_bs;

            // Get scale for this block
            let scale = scales[row * params.num_blocks + block_idx];

            // Column indices in the K dimension
            let col = block_idx * params.block_size + within_block * 2u;

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

    // Workgroup reduction
    shared_sum[tid] = partial_sum;
    workgroupBarrier();

    // Tree reduction
    var stride = WORKGROUP_SIZE / 2u;
    while (stride > 0u) {
        if (tid < stride) {
            shared_sum[tid] += shared_sum[tid + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
    }

    // Write result
    if (tid == 0u) {
        output[row] = shared_sum[0];
    }
}
