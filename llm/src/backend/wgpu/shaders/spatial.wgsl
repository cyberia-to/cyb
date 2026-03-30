// Spatial operations: Interpolate (nearest, bilinear), PixelShuffle, PixelUnshuffle, PatchEmbed

// === Nearest-Neighbor Interpolation (Upsampling) ===
// input: [batch, channels, in_h, in_w]
// output: [batch, channels, out_h, out_w]
struct InterpolateNearestParams {
    channels: u32,
    in_h: u32,
    in_w: u32,
    out_h: u32,
    out_w: u32,
    batch_size: u32,
    total_output: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> interp_nearest_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> interp_nearest_output: array<f32>;
@group(0) @binding(2) var<uniform> interp_nearest_params: InterpolateNearestParams;

@compute @workgroup_size(256)
fn interpolate_nearest_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= interp_nearest_params.total_output) { return; }

    let out_spatial = interp_nearest_params.out_h * interp_nearest_params.out_w;
    let out_per_batch = interp_nearest_params.channels * out_spatial;
    let in_spatial = interp_nearest_params.in_h * interp_nearest_params.in_w;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let ch = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / interp_nearest_params.out_w;
    let ow = spatial % interp_nearest_params.out_w;

    // Map output coords to input coords (nearest neighbor)
    let scale_h = f32(interp_nearest_params.in_h) / f32(interp_nearest_params.out_h);
    let scale_w = f32(interp_nearest_params.in_w) / f32(interp_nearest_params.out_w);
    let ih = min(u32(f32(oh) * scale_h), interp_nearest_params.in_h - 1u);
    let iw = min(u32(f32(ow) * scale_w), interp_nearest_params.in_w - 1u);

    let in_idx = batch * interp_nearest_params.channels * in_spatial
               + ch * in_spatial + ih * interp_nearest_params.in_w + iw;

    interp_nearest_output[out_idx] = interp_nearest_input[in_idx];
}

// === Area Interpolation (Downsampling) ===
// Each output pixel is the average of a rectangular region of input pixels
// Used for resolution matching and downsampling
// input: [batch, channels, in_h, in_w]
// output: [batch, channels, out_h, out_w]
struct InterpolateAreaParams {
    channels: u32,
    in_h: u32,
    in_w: u32,
    out_h: u32,
    out_w: u32,
    batch_size: u32,
    total_output: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> interp_area_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> interp_area_output: array<f32>;
@group(0) @binding(2) var<uniform> interp_area_params: InterpolateAreaParams;

@compute @workgroup_size(256)
fn interpolate_area_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= interp_area_params.total_output) { return; }

    let out_spatial = interp_area_params.out_h * interp_area_params.out_w;
    let out_per_batch = interp_area_params.channels * out_spatial;
    let in_spatial = interp_area_params.in_h * interp_area_params.in_w;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let ch = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / interp_area_params.out_w;
    let ow = spatial % interp_area_params.out_w;

    // Compute the input region that maps to this output pixel
    let scale_h = f32(interp_area_params.in_h) / f32(interp_area_params.out_h);
    let scale_w = f32(interp_area_params.in_w) / f32(interp_area_params.out_w);

    let ih_start = u32(floor(f32(oh) * scale_h));
    let ih_end = min(u32(ceil(f32(oh + 1u) * scale_h)), interp_area_params.in_h);
    let iw_start = u32(floor(f32(ow) * scale_w));
    let iw_end = min(u32(ceil(f32(ow + 1u) * scale_w)), interp_area_params.in_w);

    let base = batch * interp_area_params.channels * in_spatial + ch * in_spatial;

    var sum: f32 = 0.0;
    var count: f32 = 0.0;
    for (var ih = ih_start; ih < ih_end; ih++) {
        for (var iw = iw_start; iw < iw_end; iw++) {
            sum += interp_area_input[base + ih * interp_area_params.in_w + iw];
            count += 1.0;
        }
    }

    interp_area_output[out_idx] = sum / max(count, 1.0);
}

// === Bilinear Interpolation ===
// input: [batch, channels, in_h, in_w]
// output: [batch, channels, out_h, out_w]
struct InterpolateBilinearParams {
    channels: u32,
    in_h: u32,
    in_w: u32,
    out_h: u32,
    out_w: u32,
    batch_size: u32,
    total_output: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> interp_bilinear_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> interp_bilinear_output: array<f32>;
@group(0) @binding(2) var<uniform> interp_bilinear_params: InterpolateBilinearParams;

@compute @workgroup_size(256)
fn interpolate_bilinear_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= interp_bilinear_params.total_output) { return; }

    let out_spatial = interp_bilinear_params.out_h * interp_bilinear_params.out_w;
    let out_per_batch = interp_bilinear_params.channels * out_spatial;
    let in_spatial = interp_bilinear_params.in_h * interp_bilinear_params.in_w;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let ch = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / interp_bilinear_params.out_w;
    let ow = spatial % interp_bilinear_params.out_w;

    let scale_h = f32(interp_bilinear_params.in_h) / f32(interp_bilinear_params.out_h);
    let scale_w = f32(interp_bilinear_params.in_w) / f32(interp_bilinear_params.out_w);

    // Continuous source coordinates
    let src_h = f32(oh) * scale_h;
    let src_w = f32(ow) * scale_w;

    let h0 = u32(floor(src_h));
    let w0 = u32(floor(src_w));
    let h1 = min(h0 + 1u, interp_bilinear_params.in_h - 1u);
    let w1 = min(w0 + 1u, interp_bilinear_params.in_w - 1u);

    let fh = src_h - floor(src_h);
    let fw = src_w - floor(src_w);

    let base = batch * interp_bilinear_params.channels * in_spatial + ch * in_spatial;

    let v00 = interp_bilinear_input[base + h0 * interp_bilinear_params.in_w + w0];
    let v01 = interp_bilinear_input[base + h0 * interp_bilinear_params.in_w + w1];
    let v10 = interp_bilinear_input[base + h1 * interp_bilinear_params.in_w + w0];
    let v11 = interp_bilinear_input[base + h1 * interp_bilinear_params.in_w + w1];

    // Bilinear interpolation
    let val = v00 * (1.0 - fh) * (1.0 - fw)
            + v01 * (1.0 - fh) * fw
            + v10 * fh * (1.0 - fw)
            + v11 * fh * fw;

    interp_bilinear_output[out_idx] = val;
}

// === Pixel Shuffle ===
// Rearrange (batch, C*r^2, H, W) -> (batch, C, H*r, W*r)
// Pure index mapping, no computation
struct PixelShuffleParams {
    channels_out: u32,   // C (output channels)
    in_h: u32,
    in_w: u32,
    out_h: u32,          // H * r
    out_w: u32,          // W * r
    upscale_factor: u32,
    batch_size: u32,
    total_output: u32,
}

@group(0) @binding(0) var<storage, read> ps_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> ps_output: array<f32>;
@group(0) @binding(2) var<uniform> ps_params: PixelShuffleParams;

@compute @workgroup_size(256)
fn pixel_shuffle_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= ps_params.total_output) { return; }

    let out_spatial = ps_params.out_h * ps_params.out_w;
    let out_per_batch = ps_params.channels_out * out_spatial;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let oc = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / ps_params.out_w;
    let ow = spatial % ps_params.out_w;

    let r = ps_params.upscale_factor;
    // Map output (oc, oh, ow) to input (ic, ih, iw)
    let ih = oh / r;
    let iw = ow / r;
    let sub_h = oh % r;
    let sub_w = ow % r;
    let ic = oc * r * r + sub_h * r + sub_w;

    let in_spatial = ps_params.in_h * ps_params.in_w;
    let channels_in = ps_params.channels_out * r * r;
    let in_idx = batch * channels_in * in_spatial + ic * in_spatial + ih * ps_params.in_w + iw;

    ps_output[out_idx] = ps_input[in_idx];
}

// === Pixel Unshuffle ===
// Rearrange (batch, C, H, W) -> (batch, C*r^2, H/r, W/r)
// Inverse of pixel shuffle
struct PixelUnshuffleParams {
    channels_in: u32,    // C (input channels)
    in_h: u32,
    in_w: u32,
    out_h: u32,          // H / r
    out_w: u32,          // W / r
    downscale_factor: u32,
    batch_size: u32,
    total_output: u32,
}

@group(0) @binding(0) var<storage, read> pus_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> pus_output: array<f32>;
@group(0) @binding(2) var<uniform> pus_params: PixelUnshuffleParams;

@compute @workgroup_size(256)
fn pixel_unshuffle_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= pus_params.total_output) { return; }

    let r = pus_params.downscale_factor;
    let channels_out = pus_params.channels_in * r * r;
    let out_spatial = pus_params.out_h * pus_params.out_w;
    let out_per_batch = channels_out * out_spatial;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let oc = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / pus_params.out_w;
    let ow = spatial % pus_params.out_w;

    // Map output (oc, oh, ow) back to input (ic, ih, iw)
    let ic = oc / (r * r);
    let sub = oc % (r * r);
    let sub_h = sub / r;
    let sub_w = sub % r;
    let ih = oh * r + sub_h;
    let iw = ow * r + sub_w;

    let in_spatial = pus_params.in_h * pus_params.in_w;
    let in_idx = batch * pus_params.channels_in * in_spatial + ic * in_spatial + ih * pus_params.in_w + iw;

    pus_output[out_idx] = pus_input[in_idx];
}

// === Patch Embedding ===
// Conv2d with kernel=stride=patch_size (non-overlapping patches)
// input: [batch, in_channels, H, W]
// weight: [embed_dim, in_channels, patch_size, patch_size]
// bias: [embed_dim]
// output: [batch, num_patches, embed_dim] where num_patches = (H/p) * (W/p)
struct PatchEmbedParams {
    in_channels: u32,
    embed_dim: u32,
    in_h: u32,
    in_w: u32,
    patch_size: u32,
    num_patches_h: u32,
    num_patches_w: u32,
    total_output: u32,
    batch_size: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> pe_input: array<f32>;
@group(0) @binding(1) var<storage, read> pe_weight: array<f32>;
@group(0) @binding(2) var<storage, read> pe_bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> pe_output: array<f32>;
@group(0) @binding(4) var<uniform> pe_params: PatchEmbedParams;

@compute @workgroup_size(256)
fn patch_embed_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= pe_params.total_output) { return; }

    let num_patches = pe_params.num_patches_h * pe_params.num_patches_w;
    let out_per_batch = num_patches * pe_params.embed_dim;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let patch_idx = rem / pe_params.embed_dim;
    let ed = rem % pe_params.embed_dim;

    let ph = patch_idx / pe_params.num_patches_w;
    let pw = patch_idx % pe_params.num_patches_w;

    let in_spatial = pe_params.in_h * pe_params.in_w;
    let p = pe_params.patch_size;

    var sum: f32 = pe_bias[ed];

    // Conv2d with kernel=stride=patch_size
    for (var ic = 0u; ic < pe_params.in_channels; ic++) {
        for (var kh = 0u; kh < p; kh++) {
            for (var kw = 0u; kw < p; kw++) {
                let ih = ph * p + kh;
                let iw = pw * p + kw;
                let in_idx = batch * pe_params.in_channels * in_spatial
                           + ic * in_spatial + ih * pe_params.in_w + iw;
                let w_idx = ed * pe_params.in_channels * p * p
                          + ic * p * p + kh * p + kw;
                sum += pe_input[in_idx] * pe_weight[w_idx];
            }
        }
    }

    pe_output[out_idx] = sum;
}

// === Unpatchify ===
// Reconstruct spatial tensor from patch sequence
// Inverse of patch embedding: [batch, num_patches, channels * patch_size^2]
// -> [batch, channels, H, W]
// Each patch covers a patch_size x patch_size region
struct UnpatchifyParams {
    channels: u32,
    patch_size: u32,
    num_patches_h: u32,
    num_patches_w: u32,
    out_h: u32,
    out_w: u32,
    batch_size: u32,
    total_output: u32,
}

@group(0) @binding(0) var<storage, read> unpatch_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> unpatch_output: array<f32>;
@group(0) @binding(2) var<uniform> unpatch_params: UnpatchifyParams;

@compute @workgroup_size(256)
fn unpatchify_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= unpatch_params.total_output) { return; }

    let out_spatial = unpatch_params.out_h * unpatch_params.out_w;
    let out_per_batch = unpatch_params.channels * out_spatial;

    let batch = out_idx / out_per_batch;
    let rem = out_idx % out_per_batch;
    let ch = rem / out_spatial;
    let spatial = rem % out_spatial;
    let oh = spatial / unpatch_params.out_w;
    let ow = spatial % unpatch_params.out_w;

    let p = unpatch_params.patch_size;

    // Which patch does this pixel belong to?
    let ph = oh / p;
    let pw = ow / p;
    let local_h = oh % p;
    let local_w = ow % p;

    let patch_idx = ph * unpatch_params.num_patches_w + pw;
    let num_patches = unpatch_params.num_patches_h * unpatch_params.num_patches_w;
    let patch_elements = unpatch_params.channels * p * p;

    // Input layout: [batch, num_patches, channels * p * p]
    // Within each patch: [channels, p, p] flattened
    let in_idx = batch * num_patches * patch_elements
               + patch_idx * patch_elements
               + ch * p * p + local_h * p + local_w;

    unpatch_output[out_idx] = unpatch_input[in_idx];
}
