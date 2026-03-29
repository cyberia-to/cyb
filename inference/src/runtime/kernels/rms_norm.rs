//! RMS Normalization kernel
//! output[i] = (input[i] / rms(input)) * weight[i]
//! where rms = sqrt(mean(input^2) + eps)

use cubecl::prelude::*;
use cubecl::wgpu::WgpuRuntime;
use crate::runtime::tensor::{Client, GpuTensor};

/// Fused RMSNorm: normalize + scale, optionally with skip connection
/// input: [batch, seq, hidden], weight: [hidden], skip: optional [batch, seq, hidden]
/// output: [batch, seq, hidden], skip_out: input + skip (if skip provided)
#[cube(launch_unchecked)]
fn rms_norm_kernel(
    input: &Tensor<Line<f32>>,
    weight: &Tensor<Line<f32>>,
    output: &mut Tensor<Line<f32>>,
    eps_buf: &Tensor<Line<f32>>,
    #[comptime] hidden: u32,
) {
    // Each thread handles one position (batch * seq)
    let pos = ABSOLUTE_POS;
    if pos >= output.len() / hidden as usize {
        terminate!();
    }

    let base = pos * hidden as usize;

    // Compute RMS
    let mut sum_sq: f32 = 0.0;
    let mut i: u32 = 0;
    while i < hidden {
        let val = f32::cast_from(input[base + i as usize]);
        sum_sq += val * val;
        i += 1;
    }

    let eps = f32::cast_from(eps_buf[0]);
    let rms = f32::sqrt(sum_sq / f32::cast_from(hidden) + eps);

    // Normalize and scale
    i = 0;
    while i < hidden {
        let val = f32::cast_from(input[base + i as usize]);
        let w = f32::cast_from(weight[i as usize]);
        output[base + i as usize] = Line::cast_from(val / rms * w);
        i += 1;
    }
}

/// Launch RMSNorm kernel
pub fn rms_norm(
    client: &Client,
    input: &GpuTensor,
    weight: &GpuTensor,
    eps: f32,
) -> GpuTensor {
    let shape = input.shape.clone();
    let hidden = *shape.last().unwrap();
    let positions = input.numel() / hidden;
    let output = GpuTensor::empty_f32(client, shape);

    let eps_buf = GpuTensor::from_f32(client, &[eps], vec![1]);

    let cube_dim = CubeDim::new(client, positions);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, positions, cube_dim);

    unsafe {
        rms_norm_kernel::launch_unchecked::<WgpuRuntime>(
            client,
            cube_count,
            cube_dim,
            input.as_arg(),
            weight.as_arg(),
            output.as_arg(),
            eps_buf.as_arg(),
            hidden as u32,
        ).expect("rms_norm kernel failed");
    }

    output
}

/// Fused skip + RMSNorm: output = rms_norm(input + skip) * weight
/// Returns (normalized, input + skip)
pub fn skip_rms_norm(
    client: &Client,
    input: &GpuTensor,
    skip: &GpuTensor,
    weight: &GpuTensor,
    eps: f32,
) -> (GpuTensor, GpuTensor) {
    // For now: add skip first, then rms_norm
    // TODO: fuse into single kernel
    let sum = elementwise_add(client, input, skip);
    let normed = rms_norm(client, &sum, weight, eps);
    (normed, sum)
}

/// Element-wise add (temporary, will be fused)
fn elementwise_add(client: &Client, a: &GpuTensor, b: &GpuTensor) -> GpuTensor {
    let output = GpuTensor::empty_f32(client, a.shape.clone());
    let n = a.numel();
    let cube_dim = CubeDim::new(client, n);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, n, cube_dim);

    unsafe {
        add_kernel::launch_unchecked::<WgpuRuntime>(
            client, cube_count, cube_dim,
            a.as_arg(), b.as_arg(), output.as_arg(),
        ).expect("add kernel failed");
    }

    output
}

#[cube(launch_unchecked)]
fn add_kernel(
    a: &Tensor<Line<f32>>,
    b: &Tensor<Line<f32>>,
    output: &mut Tensor<Line<f32>>,
) {
    if ABSOLUTE_POS >= output.len() { terminate!(); }
    output[ABSOLUTE_POS] = a[ABSOLUTE_POS] + b[ABSOLUTE_POS];
}
