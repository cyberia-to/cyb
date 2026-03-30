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

// === Conv3d ===
// 3D convolution for video models (SVD, Wan2.2, HunyuanVideo VAE)
// input: [batch, in_channels, in_d, in_h, in_w]
// weight: [out_channels, in_channels/groups, kd, kh, kw]
// bias: [out_channels]
// output: [batch, out_channels, out_d, out_h, out_w]
struct Conv3dParams {
    in_channels: u32,
    out_channels: u32,
    in_d: u32,
    in_h: u32,
    in_w: u32,
    out_d: u32,
    out_h: u32,
    out_w: u32,
    kernel_d: u32,
    kernel_h: u32,
    kernel_w: u32,
    stride_d: u32,
    stride_h: u32,
    stride_w: u32,
    pad_d: u32,
    pad_h: u32,
    pad_w: u32,
    groups: u32,
    batch_size: u32,
    total_output: u32,
}

@group(0) @binding(0) var<storage, read> conv3d_input: array<f32>;
@group(0) @binding(1) var<storage, read> conv3d_weight: array<f32>;
@group(0) @binding(2) var<storage, read> conv3d_bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> conv3d_output: array<f32>;
@group(0) @binding(4) var<uniform> conv3d_params: Conv3dParams;

@compute @workgroup_size(256)
fn conv3d_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= conv3d_params.total_output) { return; }

    let out_spatial = conv3d_params.out_d * conv3d_params.out_h * conv3d_params.out_w;
    let out_per_batch = conv3d_params.out_channels * out_spatial;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let oc = rem / out_spatial;
    let spatial = rem % out_spatial;
    let od = spatial / (conv3d_params.out_h * conv3d_params.out_w);
    let spatial2 = spatial % (conv3d_params.out_h * conv3d_params.out_w);
    let oh = spatial2 / conv3d_params.out_w;
    let ow = spatial2 % conv3d_params.out_w;

    let group = oc / (conv3d_params.out_channels / conv3d_params.groups);
    let in_channels_per_group = conv3d_params.in_channels / conv3d_params.groups;
    let in_channel_start = group * in_channels_per_group;

    let in_spatial = conv3d_params.in_d * conv3d_params.in_h * conv3d_params.in_w;

    var sum: f32 = conv3d_bias[oc];

    for (var ic = 0u; ic < in_channels_per_group; ic++) {
        let in_c = in_channel_start + ic;
        for (var kd = 0u; kd < conv3d_params.kernel_d; kd++) {
            let id_signed = i32(od * conv3d_params.stride_d + kd) - i32(conv3d_params.pad_d);
            if (id_signed < 0 || id_signed >= i32(conv3d_params.in_d)) { continue; }
            let id = u32(id_signed);

            for (var kh = 0u; kh < conv3d_params.kernel_h; kh++) {
                let ih_signed = i32(oh * conv3d_params.stride_h + kh) - i32(conv3d_params.pad_h);
                if (ih_signed < 0 || ih_signed >= i32(conv3d_params.in_h)) { continue; }
                let ih = u32(ih_signed);

                for (var kw = 0u; kw < conv3d_params.kernel_w; kw++) {
                    let iw_signed = i32(ow * conv3d_params.stride_w + kw) - i32(conv3d_params.pad_w);
                    if (iw_signed < 0 || iw_signed >= i32(conv3d_params.in_w)) { continue; }
                    let iw = u32(iw_signed);

                    let in_idx = batch * conv3d_params.in_channels * in_spatial
                               + in_c * in_spatial
                               + id * conv3d_params.in_h * conv3d_params.in_w
                               + ih * conv3d_params.in_w + iw;

                    let kernel_spatial = conv3d_params.kernel_d * conv3d_params.kernel_h * conv3d_params.kernel_w;
                    let w_idx = oc * in_channels_per_group * kernel_spatial
                              + ic * kernel_spatial
                              + kd * conv3d_params.kernel_h * conv3d_params.kernel_w
                              + kh * conv3d_params.kernel_w + kw;

                    sum += conv3d_input[in_idx] * conv3d_weight[w_idx];
                }
            }
        }
    }

    conv3d_output[out_idx] = sum;
}

// === ConvTranspose2d ===
// Transposed 2D convolution (learned upsampling)
// input: [batch, in_channels, in_h, in_w]
// weight: [in_channels, out_channels, kh, kw]  (note: transposed)
// bias: [out_channels]
// output: [batch, out_channels, out_h, out_w]
// out_h = (in_h - 1) * stride_h - 2 * pad_h + kernel_h
struct ConvTranspose2dParams {
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
    batch_size: u32,
    total_output: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> ct2d_input: array<f32>;
@group(0) @binding(1) var<storage, read> ct2d_weight: array<f32>;
@group(0) @binding(2) var<storage, read> ct2d_bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> ct2d_output: array<f32>;
@group(0) @binding(4) var<uniform> ct2d_params: ConvTranspose2dParams;

@compute @workgroup_size(256)
fn conv_transpose2d_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= ct2d_params.total_output) { return; }

    let out_spatial = ct2d_params.out_h * ct2d_params.out_w;
    let out_per_batch = ct2d_params.out_channels * out_spatial;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let oc = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / ct2d_params.out_w;
    let ow = spatial % ct2d_params.out_w;

    let in_spatial = ct2d_params.in_h * ct2d_params.in_w;

    var sum: f32 = ct2d_bias[oc];

    // For transposed conv: for each input position that contributes to this output
    for (var ic = 0u; ic < ct2d_params.in_channels; ic++) {
        for (var kh = 0u; kh < ct2d_params.kernel_h; kh++) {
            // oh = ih * stride_h - pad_h + kh => ih = (oh + pad_h - kh) / stride_h
            let oh_plus_pad = i32(oh) + i32(ct2d_params.pad_h);
            let numerator_h = oh_plus_pad - i32(kh);
            if (numerator_h < 0 || numerator_h % i32(ct2d_params.stride_h) != 0) { continue; }
            let ih = u32(numerator_h / i32(ct2d_params.stride_h));
            if (ih >= ct2d_params.in_h) { continue; }

            for (var kw = 0u; kw < ct2d_params.kernel_w; kw++) {
                let ow_plus_pad = i32(ow) + i32(ct2d_params.pad_w);
                let numerator_w = ow_plus_pad - i32(kw);
                if (numerator_w < 0 || numerator_w % i32(ct2d_params.stride_w) != 0) { continue; }
                let iw = u32(numerator_w / i32(ct2d_params.stride_w));
                if (iw >= ct2d_params.in_w) { continue; }

                let in_idx = batch * ct2d_params.in_channels * in_spatial
                           + ic * in_spatial + ih * ct2d_params.in_w + iw;

                // Weight layout for transposed: [in_channels, out_channels, kh, kw]
                let w_idx = ic * ct2d_params.out_channels * ct2d_params.kernel_h * ct2d_params.kernel_w
                          + oc * ct2d_params.kernel_h * ct2d_params.kernel_w
                          + kh * ct2d_params.kernel_w + kw;

                sum += ct2d_input[in_idx] * ct2d_weight[w_idx];
            }
        }
    }

    ct2d_output[out_idx] = sum;
}

// === Causal Conv1d ===
// Causal 1D convolution with replicate padding (Wan video, Mamba)
// Only looks at past + current positions, never future
// input: [batch, channels, length]
// weight: [channels, 1, kernel_size]  (depthwise)
// bias: [channels]
// output: [batch, channels, length]  (same length due to causal padding)
struct CausalConv1dParams {
    channels: u32,
    length: u32,
    kernel_size: u32,
    batch_size: u32,
    total_output: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> cc1d_input: array<f32>;
@group(0) @binding(1) var<storage, read> cc1d_weight: array<f32>;
@group(0) @binding(2) var<storage, read> cc1d_bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> cc1d_output: array<f32>;
@group(0) @binding(4) var<uniform> cc1d_params: CausalConv1dParams;

@compute @workgroup_size(256)
fn causal_conv1d_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= cc1d_params.total_output) { return; }

    let out_per_batch = cc1d_params.channels * cc1d_params.length;
    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let ch = rem / cc1d_params.length;
    let pos = rem % cc1d_params.length;

    var sum: f32 = cc1d_bias[ch];

    // Causal padding: kernel_size - 1 positions of left padding
    let causal_pad = cc1d_params.kernel_size - 1u;

    for (var k = 0u; k < cc1d_params.kernel_size; k++) {
        let src_signed = i32(pos) - i32(causal_pad) + i32(k);
        var src_val: f32;
        if (src_signed < 0) {
            // Replicate padding: use first element
            src_val = cc1d_input[batch * cc1d_params.channels * cc1d_params.length
                                + ch * cc1d_params.length];
        } else if (u32(src_signed) >= cc1d_params.length) {
            src_val = 0.0;
        } else {
            let src = u32(src_signed);
            src_val = cc1d_input[batch * cc1d_params.channels * cc1d_params.length
                                + ch * cc1d_params.length + src];
        }
        sum += src_val * cc1d_weight[ch * cc1d_params.kernel_size + k];
    }

    cc1d_output[out_idx] = sum;
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
