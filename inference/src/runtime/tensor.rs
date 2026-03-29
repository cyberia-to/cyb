//! GPU Tensor — cubecl Handle + shape metadata
//! Reshape is FREE (metadata only). All data stays on GPU.

use cubecl::wgpu::WgpuRuntime;
use cubecl::prelude::*;

pub type Client = cubecl::client::ComputeClient<WgpuRuntime>;
pub type Handle = cubecl::server::Handle;

/// GPU tensor: raw buffer handle + shape
/// No burn dependency. Zero-copy reshape.
#[derive(Clone, Debug)]
pub struct GpuTensor {
    pub handle: Handle,
    pub shape: Vec<usize>,
    pub strides: Vec<usize>,
    /// Element size in bytes (4 for f32, 4 for u32)
    pub elem_size: usize,
}

impl GpuTensor {
    /// Create from raw bytes on GPU
    pub fn from_slice(client: &Client, data: &[u8], shape: Vec<usize>, elem_size: usize) -> Self {
        let handle = client.create_from_slice(data);
        let strides = compute_strides(&shape);
        Self { handle, shape, strides, elem_size }
    }

    /// Create f32 tensor from float data
    pub fn from_f32(client: &Client, data: &[f32], shape: Vec<usize>) -> Self {
        Self::from_slice(client, bytemuck::cast_slice(data), shape, 4)
    }

    /// Create u32 tensor
    pub fn from_u32(client: &Client, data: &[u32], shape: Vec<usize>) -> Self {
        Self::from_slice(client, bytemuck::cast_slice(data), shape, 4)
    }

    /// Allocate empty f32 tensor
    pub fn empty_f32(client: &Client, shape: Vec<usize>) -> Self {
        let numel: usize = shape.iter().product();
        let handle = client.empty(numel * 4);
        let strides = compute_strides(&shape);
        Self { handle, shape, strides, elem_size: 4 }
    }

    /// Reshape — FREE, no GPU ops
    pub fn reshape(&self, new_shape: Vec<usize>) -> Self {
        let strides = compute_strides(&new_shape);
        Self {
            handle: self.handle.clone(),
            shape: new_shape,
            strides,
            elem_size: self.elem_size,
        }
    }

    /// Number of elements
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }

    /// Number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Read data back to CPU (only for output/debug — forces GPU sync!)
    pub fn to_f32(&self, client: &Client) -> Vec<f32> {
        let bytes = client.read_one(self.handle.clone());
        bytemuck::cast_slice(&bytes).to_vec()
    }

    /// Get TensorArg for cubecl kernel dispatch
    pub fn as_arg(&self) -> TensorArg<'_, WgpuRuntime> {
        TensorArg::Handle {
            handle: cubecl::frontend::TensorHandleRef {
                handle: &self.handle,
                strides: &self.strides,
                shape: &self.shape,
                elem_size: self.elem_size,
                runtime: std::marker::PhantomData,
            },
            line_size: 1,
        }
    }
}

fn compute_strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1usize; shape.len()];
    for i in (0..shape.len().saturating_sub(1)).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}
