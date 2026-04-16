// Additional normalization kernels: BatchNorm, GroupNorm, InstanceNorm, AdaLN
// Uses subgroup ops for fast SIMD reductions

enable subgroups;

const WORKGROUP_SIZE: u32 = 256u;

// === Batch Normalization ===
// Normalizes over (N,H,W) for each channel, using running mean/var
// Each workgroup handles one channel
// input: [N, C, spatial...], running_mean: [C], running_var: [C], weight: [C], bias: [C]
// For inference: use running stats (no per-batch computation)
struct BatchNormParams {
    channels: u32,
    spatial_size: u32,  // H*W (or product of spatial dims)
    batch_size: u32,
    eps: f32,
}

@group(0) @binding(0) var<storage, read> bn_input: array<f32>;
@group(0) @binding(1) var<storage, read> bn_running_mean: array<f32>;
@group(0) @binding(2) var<storage, read> bn_running_var: array<f32>;
@group(0) @binding(3) var<storage, read> bn_weight: array<f32>;
@group(0) @binding(4) var<storage, read> bn_bias: array<f32>;
@group(0) @binding(5) var<storage, read_write> bn_output: array<f32>;
@group(0) @binding(6) var<uniform> bn_params: BatchNormParams;

@compute @workgroup_size(256)
fn batchnorm_kernel(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let channel = wg_id.x;
    let tid = local_id.x;

    if (channel >= bn_params.channels) { return; }

    let mean = bn_running_mean[channel];
    let variance = bn_running_var[channel];
    let inv_std = 1.0 / sqrt(variance + bn_params.eps);
    let w = bn_weight[channel];
    let b = bn_bias[channel];

    // Process all elements in this channel across batch and spatial dims
    let total = bn_params.batch_size * bn_params.spatial_size;
    var i = tid;
    while (i < total) {
        let batch_idx = i / bn_params.spatial_size;
        let spatial_idx = i % bn_params.spatial_size;
        let flat_idx = batch_idx * bn_params.channels * bn_params.spatial_size
                     + channel * bn_params.spatial_size
                     + spatial_idx;
        let normalized = (bn_input[flat_idx] - mean) * inv_std;
        bn_output[flat_idx] = normalized * w + b;
        i += WORKGROUP_SIZE;
    }
}

// === Group Normalization ===
// Split channels into groups, normalize each group independently
// Each workgroup handles one (batch, group) pair
// input: [N, C, spatial...], weight: [C], bias: [C]
struct GroupNormParams {
    channels: u32,
    spatial_size: u32,   // H*W
    num_groups: u32,
    channels_per_group: u32,
    eps: f32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> gn_input: array<f32>;
@group(0) @binding(1) var<storage, read> gn_weight: array<f32>;
@group(0) @binding(2) var<storage, read> gn_bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> gn_output: array<f32>;
@group(0) @binding(4) var<uniform> gn_params: GroupNormParams;

var<workgroup> gn_wg_partial: array<f32, 8>;

@compute @workgroup_size(256)
fn groupnorm_kernel(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let batch = wg_id.x;
    let group = wg_id.y;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WORKGROUP_SIZE / sg_size;

    let group_size = gn_params.channels_per_group * gn_params.spatial_size;
    let batch_offset = batch * gn_params.channels * gn_params.spatial_size;
    let group_channel_start = group * gn_params.channels_per_group;

    // Phase 1: compute mean
    var partial_sum: f32 = 0.0;
    var i = tid;
    while (i < group_size) {
        let c = group_channel_start + i / gn_params.spatial_size;
        let s = i % gn_params.spatial_size;
        let flat_idx = batch_offset + c * gn_params.spatial_size + s;
        partial_sum += gn_input[flat_idx];
        i += WORKGROUP_SIZE;
    }

    partial_sum = subgroupAdd(partial_sum);
    if (sg_id == 0u) { gn_wg_partial[sg_idx] = partial_sum; }
    workgroupBarrier();
    if (sg_idx == 0u) {
        if (sg_id < num_sgs) { partial_sum = gn_wg_partial[sg_id]; } else { partial_sum = 0.0; }
        partial_sum = subgroupAdd(partial_sum);
    }
    if (tid == 0u) { gn_wg_partial[0] = partial_sum; }
    workgroupBarrier();
    let mean = gn_wg_partial[0] / f32(group_size);

    // Phase 2: compute variance
    var partial_var: f32 = 0.0;
    i = tid;
    while (i < group_size) {
        let c = group_channel_start + i / gn_params.spatial_size;
        let s = i % gn_params.spatial_size;
        let flat_idx = batch_offset + c * gn_params.spatial_size + s;
        let diff = gn_input[flat_idx] - mean;
        partial_var += diff * diff;
        i += WORKGROUP_SIZE;
    }

    partial_var = subgroupAdd(partial_var);
    if (sg_id == 0u) { gn_wg_partial[sg_idx] = partial_var; }
    workgroupBarrier();
    if (sg_idx == 0u) {
        if (sg_id < num_sgs) { partial_var = gn_wg_partial[sg_id]; } else { partial_var = 0.0; }
        partial_var = subgroupAdd(partial_var);
    }
    if (tid == 0u) { gn_wg_partial[0] = partial_var; }
    workgroupBarrier();
    let variance = gn_wg_partial[0] / f32(group_size);
    let inv_std = 1.0 / sqrt(variance + gn_params.eps);

    // Phase 3: normalize with per-channel weight and bias
    i = tid;
    while (i < group_size) {
        let c = group_channel_start + i / gn_params.spatial_size;
        let s = i % gn_params.spatial_size;
        let flat_idx = batch_offset + c * gn_params.spatial_size + s;
        let normalized = (gn_input[flat_idx] - mean) * inv_std;
        gn_output[flat_idx] = normalized * gn_weight[c] + gn_bias[c];
        i += WORKGROUP_SIZE;
    }
}

// === Instance Normalization ===
// Normalize each (batch, channel) independently — groupnorm with groups=channels
// Each workgroup handles one (batch, channel) pair
struct InstanceNormParams {
    channels: u32,
    spatial_size: u32,
    eps: f32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> in_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> in_output: array<f32>;
@group(0) @binding(2) var<uniform> in_params: InstanceNormParams;

var<workgroup> in_wg_partial: array<f32, 8>;

@compute @workgroup_size(256)
fn instance_norm_kernel(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let batch = wg_id.x;
    let channel = wg_id.y;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WORKGROUP_SIZE / sg_size;

    let base = (batch * in_params.channels + channel) * in_params.spatial_size;

    // Phase 1: mean
    var partial_sum: f32 = 0.0;
    var i = tid;
    while (i < in_params.spatial_size) {
        partial_sum += in_input[base + i];
        i += WORKGROUP_SIZE;
    }

    partial_sum = subgroupAdd(partial_sum);
    if (sg_id == 0u) { in_wg_partial[sg_idx] = partial_sum; }
    workgroupBarrier();
    if (sg_idx == 0u) {
        if (sg_id < num_sgs) { partial_sum = in_wg_partial[sg_id]; } else { partial_sum = 0.0; }
        partial_sum = subgroupAdd(partial_sum);
    }
    if (tid == 0u) { in_wg_partial[0] = partial_sum; }
    workgroupBarrier();
    let mean = in_wg_partial[0] / f32(in_params.spatial_size);

    // Phase 2: variance
    var partial_var: f32 = 0.0;
    i = tid;
    while (i < in_params.spatial_size) {
        let diff = in_input[base + i] - mean;
        partial_var += diff * diff;
        i += WORKGROUP_SIZE;
    }

    partial_var = subgroupAdd(partial_var);
    if (sg_id == 0u) { in_wg_partial[sg_idx] = partial_var; }
    workgroupBarrier();
    if (sg_idx == 0u) {
        if (sg_id < num_sgs) { partial_var = in_wg_partial[sg_id]; } else { partial_var = 0.0; }
        partial_var = subgroupAdd(partial_var);
    }
    if (tid == 0u) { in_wg_partial[0] = partial_var; }
    workgroupBarrier();
    let variance = in_wg_partial[0] / f32(in_params.spatial_size);
    let inv_std = 1.0 / sqrt(variance + in_params.eps);

    // Phase 3: normalize (no weight/bias for instance norm)
    i = tid;
    while (i < in_params.spatial_size) {
        in_output[base + i] = (in_input[base + i] - mean) * inv_std;
        i += WORKGROUP_SIZE;
    }
}

// === Adaptive Layer Norm (AdaLN) ===
// output = (1 + scale) * layernorm(x) + shift
// Where scale and shift come from conditioning (e.g. timestep MLP)
// Each workgroup handles one position/token
struct AdaLNParams {
    hidden: u32,
    eps: f32,
}

@group(0) @binding(0) var<storage, read> adaln_input: array<f32>;
@group(0) @binding(1) var<storage, read> adaln_scale: array<f32>;
@group(0) @binding(2) var<storage, read> adaln_shift: array<f32>;
@group(0) @binding(3) var<storage, read> adaln_weight: array<f32>;
@group(0) @binding(4) var<storage, read> adaln_bias: array<f32>;
@group(0) @binding(5) var<storage, read_write> adaln_output: array<f32>;
@group(0) @binding(6) var<uniform> adaln_params: AdaLNParams;

var<workgroup> adaln_wg_partial: array<f32, 8>;

@compute @workgroup_size(256)
fn adaln_kernel(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let pos = wg_id.x;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WORKGROUP_SIZE / sg_size;
    let base = pos * adaln_params.hidden;

    // Phase 1: mean
    var partial_sum: f32 = 0.0;
    var i = tid;
    while (i < adaln_params.hidden) {
        partial_sum += adaln_input[base + i];
        i += WORKGROUP_SIZE;
    }

    partial_sum = subgroupAdd(partial_sum);
    if (sg_id == 0u) { adaln_wg_partial[sg_idx] = partial_sum; }
    workgroupBarrier();
    if (sg_idx == 0u) {
        if (sg_id < num_sgs) { partial_sum = adaln_wg_partial[sg_id]; } else { partial_sum = 0.0; }
        partial_sum = subgroupAdd(partial_sum);
    }
    if (tid == 0u) { adaln_wg_partial[0] = partial_sum; }
    workgroupBarrier();
    let mean = adaln_wg_partial[0] / f32(adaln_params.hidden);

    // Phase 2: variance
    var partial_var: f32 = 0.0;
    i = tid;
    while (i < adaln_params.hidden) {
        let diff = adaln_input[base + i] - mean;
        partial_var += diff * diff;
        i += WORKGROUP_SIZE;
    }

    partial_var = subgroupAdd(partial_var);
    if (sg_id == 0u) { adaln_wg_partial[sg_idx] = partial_var; }
    workgroupBarrier();
    if (sg_idx == 0u) {
        if (sg_id < num_sgs) { partial_var = adaln_wg_partial[sg_id]; } else { partial_var = 0.0; }
        partial_var = subgroupAdd(partial_var);
    }
    if (tid == 0u) { adaln_wg_partial[0] = partial_var; }
    workgroupBarrier();
    let variance = adaln_wg_partial[0] / f32(adaln_params.hidden);
    let inv_std = 1.0 / sqrt(variance + adaln_params.eps);

    // Phase 3: normalize then apply adaptive affine: (1 + scale) * norm + shift
    i = tid;
    while (i < adaln_params.hidden) {
        let normalized = (adaln_input[base + i] - mean) * inv_std;
        let ln = normalized * adaln_weight[i] + adaln_bias[i];
        adaln_output[base + i] = (1.0 + adaln_scale[i]) * ln + adaln_shift[i];
        i += WORKGROUP_SIZE;
    }
}
