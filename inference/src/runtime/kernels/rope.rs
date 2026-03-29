//! Rotary Position Embeddings (RoPE) kernel

use cubecl::prelude::*;
use cubecl::wgpu::WgpuRuntime;
use crate::runtime::tensor::{Client, GpuTensor};

/// Apply RoPE in-place: x = [batch, heads, seq, head_dim]
/// cos/sin: [seq, head_dim/2] (sliced for current positions)
/// Splits head_dim into halves, applies rotation
#[cube(launch_unchecked)]
fn rope_kernel(
    x: &Tensor<Line<f32>>,
    cos_cache: &Tensor<Line<f32>>,
    sin_cache: &Tensor<Line<f32>>,
    output: &mut Tensor<Line<f32>>,
    #[comptime] half_dim: u32,
    #[comptime] seq_len: u32,
    #[comptime] pos_offset: u32,
) {
    // Each thread handles one element
    // Total elements = batch * heads * seq * head_dim
    let idx = ABSOLUTE_POS;
    if idx >= output.len() { terminate!(); }

    let head_dim = half_dim * 2;
    let d = (idx % head_dim as usize) as u32; // position within head_dim
    let seq_idx = ((idx / head_dim as usize) % seq_len as usize) as u32;

    if d < half_dim {
        // First half: x1 * cos - x2 * sin
        let cos_idx = seq_idx * half_dim + d;
        let sin_idx = cos_idx;
        let x1 = f32::cast_from(x[idx]);
        let x2_idx = idx + half_dim as usize;
        let x2 = f32::cast_from(x[x2_idx]);
        let c = f32::cast_from(cos_cache[cos_idx as usize]);
        let s = f32::cast_from(sin_cache[sin_idx as usize]);
        output[idx] = Line::cast_from(x1 * c - x2 * s);
    } else {
        // Second half: x1 * sin + x2 * cos
        let d2 = d - half_dim;
        let cos_idx = seq_idx * half_dim + d2;
        let sin_idx = cos_idx;
        let x2 = f32::cast_from(x[idx]);
        let x1_idx = idx - half_dim as usize;
        let x1 = f32::cast_from(x[x1_idx]);
        let c = f32::cast_from(cos_cache[cos_idx as usize]);
        let s = f32::cast_from(sin_cache[sin_idx as usize]);
        output[idx] = Line::cast_from(x1 * s + x2 * c);
    }
}

/// Apply RoPE to tensor [batch, heads, seq, head_dim]
/// cos_cache, sin_cache: full cache [max_seq, head_dim/2]
/// pos_offset: starting position for this sequence
pub fn apply_rope(
    client: &Client,
    x: &GpuTensor,
    cos_cache: &GpuTensor,
    sin_cache: &GpuTensor,
    head_dim: usize,
    seq_len: usize,
    pos_offset: usize,
) -> GpuTensor {
    let half_dim = head_dim / 2;
    let output = GpuTensor::empty_f32(client, x.shape.clone());
    let n = x.numel();

    // Slice cos/sin for current positions
    let cos_slice = slice_rows(client, cos_cache, pos_offset, seq_len, half_dim);
    let sin_slice = slice_rows(client, sin_cache, pos_offset, seq_len, half_dim);

    let cube_dim = CubeDim::new(client, n);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, n, cube_dim);

    unsafe {
        rope_kernel::launch_unchecked::<WgpuRuntime>(
            client, cube_count, cube_dim,
            x.as_arg(),
            cos_slice.as_arg(),
            sin_slice.as_arg(),
            output.as_arg(),
            half_dim as u32,
            seq_len as u32,
            pos_offset as u32,
        ).expect("rope failed");
    }

    output
}

/// Slice rows from [max_seq, dim] → [seq_len, dim] starting at offset
fn slice_rows(client: &Client, cache: &GpuTensor, offset: usize, seq_len: usize, dim: usize) -> GpuTensor {
    // For now, use a copy kernel. TODO: zero-copy with offset handle
    let output = GpuTensor::empty_f32(client, vec![seq_len, dim]);
    let n = seq_len * dim;
    let cube_dim = CubeDim::new(client, n);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, n, cube_dim);
    let offset_buf = GpuTensor::from_u32(client, &[(offset * dim) as u32], vec![1]);

    unsafe {
        slice_kernel::launch_unchecked::<WgpuRuntime>(
            client, cube_count, cube_dim,
            cache.as_arg(), output.as_arg(),
            offset_buf.as_arg(),
        ).expect("slice failed");
    }
    output
}

#[cube(launch_unchecked)]
fn slice_kernel(
    src: &Tensor<Line<f32>>,
    dst: &mut Tensor<Line<f32>>,
    offset_buf: &Tensor<Line<u32>>,
) {
    if ABSOLUTE_POS >= dst.len() { terminate!(); }
    let off = u32::cast_from(offset_buf[0]) as usize;
    dst[ABSOLUTE_POS] = src[ABSOLUTE_POS + off];
}
