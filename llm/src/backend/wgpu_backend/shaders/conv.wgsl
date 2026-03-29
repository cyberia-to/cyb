// Convolution kernels: Conv2d, Conv1d, DepthwiseConv, Pool
// Uses subgroups for pooling reductions

enable subgroups;

const WORKGROUP_SIZE: u32 = 256u;

// === Conv2d ===
// 2D convolution with padding and stride
// Each thread computes one output element
// input: [batch, in_channels, in_h, in_w]
// weight: [out_channels, in_channels/groups, kh, kw]
// bias: [out_channels]
// output: [batch, out_channels, out_h, out_w]
struct Conv2dParams {
    in_channels: u32,
    out_channels: u32,
    in_h: u32,
    in_w: u32,
    out_h: u32,
    out_w: u32,
    kernel_h: u32,
    kernel_w: u32,
    stride_h: u32,
    stride_w: u32,
    pad_h: u32,
    pad_w: u32,
    groups: u32,
    batch_size: u32,
    total_output: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> conv2d_input: array<f32>;
@group(0) @binding(1) var<storage, read> conv2d_weight: array<f32>;
@group(0) @binding(2) var<storage, read> conv2d_bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> conv2d_output: array<f32>;
@group(0) @binding(4) var<uniform> conv2d_params: Conv2dParams;

@compute @workgroup_size(256)
fn conv2d_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= conv2d_params.total_output) { return; }

    // Decompose flat index into (batch, out_channel, out_h, out_w)
    let out_spatial = conv2d_params.out_h * conv2d_params.out_w;
    let out_per_batch = conv2d_params.out_channels * out_spatial;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let oc = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / conv2d_params.out_w;
    let ow = spatial % conv2d_params.out_w;

    let group = oc / (conv2d_params.out_channels / conv2d_params.groups);
    let in_channels_per_group = conv2d_params.in_channels / conv2d_params.groups;
    let in_channel_start = group * in_channels_per_group;

    let in_spatial = conv2d_params.in_h * conv2d_params.in_w;

    var sum: f32 = conv2d_bias[oc];

    for (var ic = 0u; ic < in_channels_per_group; ic++) {
        let in_c = in_channel_start + ic;
        for (var kh = 0u; kh < conv2d_params.kernel_h; kh++) {
            for (var kw = 0u; kw < conv2d_params.kernel_w; kw++) {
                let ih_signed = i32(oh * conv2d_params.stride_h + kh) - i32(conv2d_params.pad_h);
                let iw_signed = i32(ow * conv2d_params.stride_w + kw) - i32(conv2d_params.pad_w);

                if (ih_signed >= 0 && ih_signed < i32(conv2d_params.in_h) &&
                    iw_signed >= 0 && iw_signed < i32(conv2d_params.in_w)) {
                    let ih = u32(ih_signed);
                    let iw = u32(iw_signed);

                    let in_idx = batch * conv2d_params.in_channels * in_spatial
                               + in_c * in_spatial
                               + ih * conv2d_params.in_w + iw;

                    // weight: [out_channels, in_channels_per_group, kh, kw]
                    let w_idx = oc * in_channels_per_group * conv2d_params.kernel_h * conv2d_params.kernel_w
                              + ic * conv2d_params.kernel_h * conv2d_params.kernel_w
                              + kh * conv2d_params.kernel_w + kw;

                    sum += conv2d_input[in_idx] * conv2d_weight[w_idx];
                }
            }
        }
    }

    conv2d_output[out_idx] = sum;
}

// === Conv1d ===
// 1D convolution (kernel only in width dimension)
// input: [batch, in_channels, length]
// weight: [out_channels, in_channels/groups, kernel_size]
// bias: [out_channels]
struct Conv1dParams {
    in_channels: u32,
    out_channels: u32,
    in_length: u32,
    out_length: u32,
    kernel_size: u32,
    stride: u32,
    padding: u32,
    groups: u32,
    batch_size: u32,
    total_output: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> conv1d_input: array<f32>;
@group(0) @binding(1) var<storage, read> conv1d_weight: array<f32>;
@group(0) @binding(2) var<storage, read> conv1d_bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> conv1d_output: array<f32>;
@group(0) @binding(4) var<uniform> conv1d_params: Conv1dParams;

@compute @workgroup_size(256)
fn conv1d_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= conv1d_params.total_output) { return; }

    let out_per_batch = conv1d_params.out_channels * conv1d_params.out_length;
    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let oc = rem / conv1d_params.out_length;
    let ol = rem % conv1d_params.out_length;

    let group = oc / (conv1d_params.out_channels / conv1d_params.groups);
    let in_channels_per_group = conv1d_params.in_channels / conv1d_params.groups;
    let in_channel_start = group * in_channels_per_group;

    var sum: f32 = conv1d_bias[oc];

    for (var ic = 0u; ic < in_channels_per_group; ic++) {
        let in_c = in_channel_start + ic;
        for (var k = 0u; k < conv1d_params.kernel_size; k++) {
            let il_signed = i32(ol * conv1d_params.stride + k) - i32(conv1d_params.padding);
            if (il_signed >= 0 && il_signed < i32(conv1d_params.in_length)) {
                let il = u32(il_signed);
                let in_idx = batch * conv1d_params.in_channels * conv1d_params.in_length
                           + in_c * conv1d_params.in_length + il;
                let w_idx = oc * in_channels_per_group * conv1d_params.kernel_size
                          + ic * conv1d_params.kernel_size + k;
                sum += conv1d_input[in_idx] * conv1d_weight[w_idx];
            }
        }
    }

    conv1d_output[out_idx] = sum;
}

// === Depthwise Convolution ===
// groups = channels, each output channel uses one input channel
// input: [batch, channels, length]
// weight: [channels, 1, kernel_size]
// bias: [channels]
struct DepthwiseConvParams {
    channels: u32,
    in_length: u32,
    out_length: u32,
    kernel_size: u32,
    stride: u32,
    padding: u32,
    batch_size: u32,
    total_output: u32,
}

@group(0) @binding(0) var<storage, read> dw_input: array<f32>;
@group(0) @binding(1) var<storage, read> dw_weight: array<f32>;
@group(0) @binding(2) var<storage, read> dw_bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> dw_output: array<f32>;
@group(0) @binding(4) var<uniform> dw_params: DepthwiseConvParams;

@compute @workgroup_size(256)
fn depthwise_conv_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= dw_params.total_output) { return; }

    let out_per_batch = dw_params.channels * dw_params.out_length;
    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let ch = rem / dw_params.out_length;
    let ol = rem % dw_params.out_length;

    var sum: f32 = dw_bias[ch];

    for (var k = 0u; k < dw_params.kernel_size; k++) {
        let il_signed = i32(ol * dw_params.stride + k) - i32(dw_params.padding);
        if (il_signed >= 0 && il_signed < i32(dw_params.in_length)) {
            let il = u32(il_signed);
            let in_idx = batch * dw_params.channels * dw_params.in_length
                       + ch * dw_params.in_length + il;
            let w_idx = ch * dw_params.kernel_size + k;
            sum += dw_input[in_idx] * dw_weight[w_idx];
        }
    }

    dw_output[out_idx] = sum;
}

// === Pooling (Max or Avg, 2D) ===
// Each thread computes one output element
// input: [batch, channels, in_h, in_w]
// output: [batch, channels, out_h, out_w]
struct PoolParams {
    channels: u32,
    in_h: u32,
    in_w: u32,
    out_h: u32,
    out_w: u32,
    kernel_h: u32,
    kernel_w: u32,
    stride_h: u32,
    stride_w: u32,
    batch_size: u32,
    total_output: u32,
    mode: u32, // 0 = max, 1 = avg
}

@group(0) @binding(0) var<storage, read> pool_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> pool_output: array<f32>;
@group(0) @binding(2) var<uniform> pool_params: PoolParams;

@compute @workgroup_size(256)
fn pool_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= pool_params.total_output) { return; }

    let out_spatial = pool_params.out_h * pool_params.out_w;
    let out_per_batch = pool_params.channels * out_spatial;
    let in_spatial = pool_params.in_h * pool_params.in_w;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let ch = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / pool_params.out_w;
    let ow = spatial % pool_params.out_w;

    let base = batch * pool_params.channels * in_spatial + ch * in_spatial;

    if (pool_params.mode == 0u) {
        // Max pooling
        var max_val: f32 = -1000000.0;
        for (var kh = 0u; kh < pool_params.kernel_h; kh++) {
            for (var kw = 0u; kw < pool_params.kernel_w; kw++) {
                let ih = oh * pool_params.stride_h + kh;
                let iw = ow * pool_params.stride_w + kw;
                if (ih < pool_params.in_h && iw < pool_params.in_w) {
                    max_val = max(max_val, pool_input[base + ih * pool_params.in_w + iw]);
                }
            }
        }
        pool_output[out_idx] = max_val;
    } else {
        // Average pooling
        var sum: f32 = 0.0;
        var count: f32 = 0.0;
        for (var kh = 0u; kh < pool_params.kernel_h; kh++) {
            for (var kw = 0u; kw < pool_params.kernel_w; kw++) {
                let ih = oh * pool_params.stride_h + kh;
                let iw = ow * pool_params.stride_w + kw;
                if (ih < pool_params.in_h && iw < pool_params.in_w) {
                    sum += pool_input[base + ih * pool_params.in_w + iw];
                    count += 1.0;
                }
            }
        }
        pool_output[out_idx] = sum / count;
    }
}
