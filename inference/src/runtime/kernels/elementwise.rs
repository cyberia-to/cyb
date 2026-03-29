//! Element-wise GPU kernels: add, mul, sigmoid, silu

use cubecl::prelude::*;
use cubecl::wgpu::WgpuRuntime;
use crate::runtime::tensor::{Client, GpuTensor};

#[cube(launch_unchecked)]
fn mul_kernel(a: &Tensor<Line<f32>>, b: &Tensor<Line<f32>>, out: &mut Tensor<Line<f32>>) {
    if ABSOLUTE_POS >= out.len() { terminate!(); }
    out[ABSOLUTE_POS] = a[ABSOLUTE_POS] * b[ABSOLUTE_POS];
}

#[cube(launch_unchecked)]
fn sigmoid_kernel(input: &Tensor<Line<f32>>, out: &mut Tensor<Line<f32>>) {
    if ABSOLUTE_POS >= out.len() { terminate!(); }
    let x = f32::cast_from(input[ABSOLUTE_POS]);
    out[ABSOLUTE_POS] = Line::cast_from(1.0 / (1.0 + f32::exp(-x)));
}

/// Fused SiLU gate: output = sigmoid(gate) * gate * up
/// gate and up are same-shape tensors
#[cube(launch_unchecked)]
fn silu_mul_kernel(
    gate: &Tensor<Line<f32>>,
    up: &Tensor<Line<f32>>,
    out: &mut Tensor<Line<f32>>,
) {
    if ABSOLUTE_POS >= out.len() { terminate!(); }
    let g = f32::cast_from(gate[ABSOLUTE_POS]);
    let sig = 1.0 / (1.0 + f32::exp(-g));
    let u = f32::cast_from(up[ABSOLUTE_POS]);
    out[ABSOLUTE_POS] = Line::cast_from(g * sig * u);
}

pub fn mul(client: &Client, a: &GpuTensor, b: &GpuTensor) -> GpuTensor {
    let output = GpuTensor::empty_f32(client, a.shape.clone());
    let n = a.numel();
    let cube_dim = CubeDim::new(client, n);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, n, cube_dim);
    unsafe {
        mul_kernel::launch_unchecked::<WgpuRuntime>(
            client, cube_count, cube_dim,
            a.as_arg(), b.as_arg(), output.as_arg(),
        ).expect("mul failed");
    }
    output
}

pub fn sigmoid(client: &Client, input: &GpuTensor) -> GpuTensor {
    let output = GpuTensor::empty_f32(client, input.shape.clone());
    let n = input.numel();
    let cube_dim = CubeDim::new(client, n);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, n, cube_dim);
    unsafe {
        sigmoid_kernel::launch_unchecked::<WgpuRuntime>(
            client, cube_count, cube_dim,
            input.as_arg(), output.as_arg(),
        ).expect("sigmoid failed");
    }
    output
}

/// Fused SiLU(gate) * up — single kernel, 3 ops in 1
pub fn silu_mul(client: &Client, gate: &GpuTensor, up: &GpuTensor) -> GpuTensor {
    let output = GpuTensor::empty_f32(client, gate.shape.clone());
    let n = gate.numel();
    let cube_dim = CubeDim::new(client, n);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, n, cube_dim);
    unsafe {
        silu_mul_kernel::launch_unchecked::<WgpuRuntime>(
            client, cube_count, cube_dim,
            gate.as_arg(), up.as_arg(), output.as_arg(),
        ).expect("silu_mul failed");
    }
    output
}
