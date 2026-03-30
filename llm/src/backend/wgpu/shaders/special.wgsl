enable subgroups;

// Special operations: Sinusoidal timestep embedding, Noise schedule

// === Sinusoidal Timestep Embedding ===
// Computes sin/cos embedding from timestep for diffusion models
// output[i] = sin(timestep * freq[i]) for i < dim/2
// output[i] = cos(timestep * freq[i-dim/2]) for i >= dim/2
// freq[i] = exp(-log(10000) * i / (dim/2))
struct SinusoidalEmbedParams {
    dim: u32,
    half_dim: u32,
    timestep: f32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read_write> sinusoidal_output: array<f32>;
@group(0) @binding(1) var<uniform> sinusoidal_params: SinusoidalEmbedParams;

@compute @workgroup_size(256)
fn sinusoidal_embed_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= sinusoidal_params.dim) { return; }

    let half = sinusoidal_params.half_dim;
    let log_10000 = 9.210340372; // ln(10000)

    if (idx < half) {
        // sin component
        let freq = exp(-log_10000 * f32(idx) / f32(half));
        sinusoidal_output[idx] = sin(sinusoidal_params.timestep * freq);
    } else {
        // cos component
        let freq = exp(-log_10000 * f32(idx - half) / f32(half));
        sinusoidal_output[idx] = cos(sinusoidal_params.timestep * freq);
    }
}

// === Noise Schedule ===
// Compute sigma from timestep for diffusion models
// Supports linear and cosine schedules
// For linear: sigma = sqrt(beta_t / (1 - alpha_cumprod_t))
// Simplified: sigma = timestep * max_sigma (for flow-matching)
struct NoiseScheduleParams {
    num_steps: u32,
    max_sigma: f32,
    schedule_type: u32,  // 0 = linear, 1 = cosine, 2 = flow_matching
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> noise_timesteps: array<f32>;
@group(0) @binding(1) var<storage, read_write> noise_sigmas: array<f32>;
@group(0) @binding(2) var<uniform> noise_params: NoiseScheduleParams;

@compute @workgroup_size(256)
fn noise_schedule_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= noise_params.num_steps) { return; }

    let t = noise_timesteps[idx];

    if (noise_params.schedule_type == 0u) {
        // Linear schedule: sigma increases linearly
        noise_sigmas[idx] = t * noise_params.max_sigma;
    } else if (noise_params.schedule_type == 1u) {
        // Cosine schedule: alpha_cumprod = cos(pi/2 * t)^2
        let alpha_cumprod = cos(1.5707963 * t) * cos(1.5707963 * t);
        let sigma_sq = (1.0 - alpha_cumprod) / alpha_cumprod;
        noise_sigmas[idx] = sqrt(sigma_sq);
    } else {
        // Flow matching: sigma = t (direct interpolation)
        noise_sigmas[idx] = t;
    }
}

// === Flow Step ===
// Simple normalizing flow step (VITS/TTS)
// Forward pass: y = (x - mean) * exp(log_scale) + bias
// Each thread processes one element
struct FlowStepParams {
    n: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> flow_input: array<f32>;
@group(0) @binding(1) var<storage, read> flow_mean: array<f32>;
@group(0) @binding(2) var<storage, read> flow_log_scale: array<f32>;
@group(0) @binding(3) var<storage, read> flow_bias: array<f32>;
@group(0) @binding(4) var<storage, read_write> flow_output: array<f32>;
@group(0) @binding(5) var<uniform> flow_params: FlowStepParams;

@compute @workgroup_size(256)
fn flow_step_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= flow_params.n) { return; }
    flow_output[idx] = (flow_input[idx] - flow_mean[idx]) * exp(flow_log_scale[idx]) + flow_bias[idx];
}

// === Learned Positional Embedding Lookup ===
// output[pos, dim] = pos_table[pos, dim]
// pos_table: [max_seq_len, embed_dim]
// output: [seq_len, embed_dim]
struct PosEmbedParams {
    embed_dim: u32,
    seq_len: u32,
    offset: u32,  // starting position (for incremental decoding)
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> pos_table: array<f32>;
@group(0) @binding(1) var<storage, read_write> pos_output: array<f32>;
@group(0) @binding(2) var<uniform> pos_embed_params: PosEmbedParams;

@compute @workgroup_size(256)
fn pos_embed_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= pos_embed_params.seq_len * pos_embed_params.embed_dim) { return; }

    let pos = idx / pos_embed_params.embed_dim;
    let dim = idx % pos_embed_params.embed_dim;
    let table_pos = pos + pos_embed_params.offset;

    pos_output[idx] = pos_table[table_pos * pos_embed_params.embed_dim + dim];
}

// === Quantize (f32 -> Q4/Q8 on GPU) ===
// Block quantization: each block of block_size elements gets one scale
// For Q8: round(x / scale) clamped to [-128, 127], packed as i8
// For Q4: round(x / scale) clamped to [-8, 7], packed as nibbles into u32
// This kernel computes scales and quantized values
struct QuantizeParams {
    n: u32,
    block_size: u32,
    num_blocks: u32,
    bits: u32,  // 4 or 8
}

@group(0) @binding(0) var<storage, read> quant_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> quant_packed: array<u32>;
@group(0) @binding(2) var<storage, read_write> quant_scales: array<f32>;
@group(0) @binding(3) var<uniform> quant_params: QuantizeParams;

@compute @workgroup_size(256)
fn quantize_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let block_idx = gid.x;
    if (block_idx >= quant_params.num_blocks) { return; }

    let start = block_idx * quant_params.block_size;
    let end = min(start + quant_params.block_size, quant_params.n);

    // Find max absolute value in block for scale
    var max_abs: f32 = 0.0;
    for (var i = start; i < end; i++) {
        max_abs = max(max_abs, abs(quant_input[i]));
    }

    if (quant_params.bits == 8u) {
        let scale = max_abs / 127.0;
        quant_scales[block_idx] = scale;
        let inv_scale = select(0.0, 1.0 / scale, scale > 0.0);

        // Pack 4 int8 values per u32
        let u32s_per_block = quant_params.block_size / 4u;
        for (var j = 0u; j < u32s_per_block; j++) {
            let base = start + j * 4u;
            var packed: u32 = 0u;
            for (var k = 0u; k < 4u; k++) {
                let idx = base + k;
                if (idx < end) {
                    let q = clamp(i32(round(quant_input[idx] * inv_scale)), -128, 127);
                    packed = packed | (u32(q & 0xFF) << (k * 8u));
                }
            }
            quant_packed[block_idx * u32s_per_block + j] = packed;
        }
    } else {
        // Q4: 4-bit quantization
        let scale = max_abs / 7.0;
        quant_scales[block_idx] = scale;
        let inv_scale = select(0.0, 1.0 / scale, scale > 0.0);

        // Pack 8 int4 values per u32
        let u32s_per_block = quant_params.block_size / 8u;
        for (var j = 0u; j < u32s_per_block; j++) {
            let base = start + j * 8u;
            var packed: u32 = 0u;
            for (var k = 0u; k < 8u; k++) {
                let idx = base + k;
                if (idx < end) {
                    let q = clamp(i32(round(quant_input[idx] * inv_scale)), -8, 7);
                    packed = packed | (u32(q & 0xF) << (k * 4u));
                }
            }
            quant_packed[block_idx * u32s_per_block + j] = packed;
        }
    }
}

// === Cross-Attention ===
// Like self-attention but Q from one tensor, K/V from another
// Q: [num_heads * head_dim]
// K, V: [num_heads * src_seq * head_dim] (from encoder/conditioning)
// Output: [num_heads * head_dim]



const WG_SIZE: u32 = 256u;

struct CrossAttentionParams {
    head_dim: u32,
    src_seq: u32,    // source sequence length (encoder output)
    num_heads: u32,
    scale: f32,
}

@group(0) @binding(0) var<storage, read> cross_q: array<f32>;
@group(0) @binding(1) var<storage, read> cross_k: array<f32>;
@group(0) @binding(2) var<storage, read> cross_v: array<f32>;
@group(0) @binding(3) var<storage, read_write> cross_output: array<f32>;
@group(0) @binding(4) var<uniform> cross_params: CrossAttentionParams;

var<workgroup> cross_scores: array<f32, 2048>;
var<workgroup> cross_wg_partial: array<f32, 8>;

@compute @workgroup_size(256)
fn cross_attention_kernel(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let head = wg_id.x;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WG_SIZE / sg_size;

    if (head >= cross_params.num_heads) { return; }

    let q_base = head * cross_params.head_dim;
    let kv_base = head * cross_params.src_seq * cross_params.head_dim;

    // Step 1: Compute attention scores
    var local_max: f32 = -1000000.0;
    var t = tid;
    while (t < cross_params.src_seq) {
        var dot: f32 = 0.0;
        for (var d = 0u; d < cross_params.head_dim; d++) {
            dot += cross_q[q_base + d] * cross_k[kv_base + t * cross_params.head_dim + d];
        }
        dot *= cross_params.scale;
        cross_scores[t] = dot;
        local_max = max(local_max, dot);
        t += WG_SIZE;
    }

    // Step 2: Subgroup max reduction
    local_max = subgroupMax(local_max);
    if (sg_id == 0u) { cross_wg_partial[sg_idx] = local_max; }
    workgroupBarrier();
    if (sg_idx == 0u && sg_id < num_sgs) {
        local_max = cross_wg_partial[sg_id];
        local_max = subgroupMax(local_max);
    }
    if (tid == 0u) { cross_wg_partial[0] = local_max; }
    workgroupBarrier();
    let global_max = cross_wg_partial[0];

    // Step 3: Compute exp(score - max) and sum
    var local_sum: f32 = 0.0;
    t = tid;
    while (t < cross_params.src_seq) {
        let e = exp(cross_scores[t] - global_max);
        cross_scores[t] = e;
        local_sum += e;
        t += WG_SIZE;
    }

    local_sum = subgroupAdd(local_sum);
    if (sg_id == 0u) { cross_wg_partial[sg_idx] = local_sum; }
    workgroupBarrier();
    if (sg_idx == 0u && sg_id < num_sgs) {
        local_sum = cross_wg_partial[sg_id];
        local_sum = subgroupAdd(local_sum);
    }
    if (tid == 0u) { cross_wg_partial[0] = local_sum; }
    workgroupBarrier();
    let exp_total = cross_wg_partial[0];

    // Normalize to softmax weights
    t = tid;
    while (t < cross_params.src_seq) {
        cross_scores[t] = cross_scores[t] / exp_total;
        t += WG_SIZE;
    }
    workgroupBarrier();

    // Step 4: Weighted sum of V
    var d = tid;
    while (d < cross_params.head_dim) {
        var weighted_sum: f32 = 0.0;
        for (var tt = 0u; tt < cross_params.src_seq; tt++) {
            weighted_sum += cross_scores[tt] * cross_v[kv_base + tt * cross_params.head_dim + d];
        }
        cross_output[q_base + d] = weighted_sum;
        d += WG_SIZE;
    }
}

// === Flash Attention (Tiled) ===
// Tiled attention that avoids materializing the full attention matrix
// Processes K/V in blocks of TILE_SIZE, maintaining running softmax
// Q: [num_heads * head_dim]
// K, V: [num_heads * total_seq * head_dim]
// Output: [num_heads * head_dim]
//
// Algorithm: online softmax with tiling
// For each tile of K/V:
//   1. Compute Q*K^T for tile
//   2. Update running max and exp_sum
//   3. Rescale previous accumulated V and add new V contribution

const TILE_SIZE: u32 = 64u;

struct FlashAttentionParams {
    head_dim: u32,
    total_seq: u32,
    num_heads: u32,
    scale: f32,
}

@group(0) @binding(0) var<storage, read> flash_q: array<f32>;
@group(0) @binding(1) var<storage, read> flash_k: array<f32>;
@group(0) @binding(2) var<storage, read> flash_v: array<f32>;
@group(0) @binding(3) var<storage, read_write> flash_output: array<f32>;
@group(0) @binding(4) var<uniform> flash_params: FlashAttentionParams;

// Tile scratch space
var<workgroup> flash_tile_scores: array<f32, 64>;  // TILE_SIZE
var<workgroup> flash_wg_partial: array<f32, 8>;

@compute @workgroup_size(256)
fn flash_attention_kernel(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let head = wg_id.x;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WG_SIZE / sg_size;

    if (head >= flash_params.num_heads) { return; }

    let q_base = head * flash_params.head_dim;
    let kv_base = head * flash_params.total_seq * flash_params.head_dim;
    let num_tiles = (flash_params.total_seq + TILE_SIZE - 1u) / TILE_SIZE;

    // Running softmax state — accumulated in registers
    // We store the accumulated output in shared memory
    var running_max: f32 = -1000000.0;
    var running_sum: f32 = 0.0;

    // Process tiles
    for (var tile = 0u; tile < num_tiles; tile++) {
        let tile_start = tile * TILE_SIZE;
        let tile_end = min(tile_start + TILE_SIZE, flash_params.total_seq);
        let tile_len = tile_end - tile_start;

        // Compute attention scores for this tile
        var tile_max: f32 = -1000000.0;
        var t = tid;
        while (t < tile_len) {
            let seq_pos = tile_start + t;
            var dot: f32 = 0.0;
            for (var d = 0u; d < flash_params.head_dim; d++) {
                dot += flash_q[q_base + d] * flash_k[kv_base + seq_pos * flash_params.head_dim + d];
            }
            dot *= flash_params.scale;
            flash_tile_scores[t] = dot;
            tile_max = max(tile_max, dot);
            t += WG_SIZE;
        }

        // Reduce tile_max across workgroup
        tile_max = subgroupMax(tile_max);
        if (sg_id == 0u) { flash_wg_partial[sg_idx] = tile_max; }
        workgroupBarrier();
        if (sg_idx == 0u && sg_id < num_sgs) {
            tile_max = flash_wg_partial[sg_id];
            tile_max = subgroupMax(tile_max);
        }
        if (tid == 0u) { flash_wg_partial[0] = tile_max; }
        workgroupBarrier();
        tile_max = flash_wg_partial[0];

        // Compute exp(score - tile_max) and tile sum
        var tile_sum: f32 = 0.0;
        t = tid;
        while (t < tile_len) {
            let e = exp(flash_tile_scores[t] - tile_max);
            flash_tile_scores[t] = e;
            tile_sum += e;
            t += WG_SIZE;
        }

        tile_sum = subgroupAdd(tile_sum);
        if (sg_id == 0u) { flash_wg_partial[sg_idx] = tile_sum; }
        workgroupBarrier();
        if (sg_idx == 0u && sg_id < num_sgs) {
            tile_sum = flash_wg_partial[sg_id];
            tile_sum = subgroupAdd(tile_sum);
        }
        if (tid == 0u) { flash_wg_partial[0] = tile_sum; }
        workgroupBarrier();
        tile_sum = flash_wg_partial[0];

        // Online softmax correction: rescale previous output
        let prev_max = running_max;
        running_max = max(running_max, tile_max);
        let correction = exp(prev_max - running_max);
        let tile_correction = exp(tile_max - running_max);
        running_sum = running_sum * correction + tile_sum * tile_correction;

        // Normalize tile scores
        workgroupBarrier();
        t = tid;
        while (t < tile_len) {
            flash_tile_scores[t] = flash_tile_scores[t] * tile_correction;
            t += WG_SIZE;
        }
        workgroupBarrier();

        // Update output: rescale old output and add new tile contribution
        var d = tid;
        while (d < flash_params.head_dim) {
            // Rescale previous accumulated output
            if (tile > 0u) {
                flash_output[q_base + d] = flash_output[q_base + d] * correction;
            } else {
                flash_output[q_base + d] = 0.0;
            }

            // Add weighted V from this tile
            for (var tt = 0u; tt < tile_len; tt++) {
                flash_output[q_base + d] += flash_tile_scores[tt]
                    * flash_v[kv_base + (tile_start + tt) * flash_params.head_dim + d];
            }
            d += WG_SIZE;
        }
        workgroupBarrier();
    }

    // Final normalization: divide by total sum
    var d2 = tid;
    while (d2 < flash_params.head_dim) {
        flash_output[q_base + d2] = flash_output[q_base + d2] / running_sum;
        d2 += WG_SIZE;
    }
}
