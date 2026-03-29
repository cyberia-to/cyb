//! Pure cubecl GPU runtime — no burn dependency for inference
//! Direct kernel dispatch, zero-copy tensors, minimal overhead.

pub mod tensor;
pub mod kernels;

use cubecl::wgpu::{WgpuDevice, WgpuRuntime};
use cubecl::Runtime;
use tensor::{Client, GpuTensor};

/// GPU runtime context
pub struct GpuContext {
    pub client: Client,
    pub device: WgpuDevice,
}

impl GpuContext {
    /// Initialize GPU runtime
    pub fn new() -> Self {
        let device = WgpuDevice::default();
        let client = WgpuRuntime::client(&device);
        log::info!("GPU runtime initialized");
        Self { client, device }
    }

    /// Create f32 tensor on GPU
    pub fn tensor_f32(&self, data: &[f32], shape: Vec<usize>) -> GpuTensor {
        GpuTensor::from_f32(&self.client, data, shape)
    }

    /// Create empty f32 tensor
    pub fn empty_f32(&self, shape: Vec<usize>) -> GpuTensor {
        GpuTensor::empty_f32(&self.client, shape)
    }

    /// Read tensor to CPU
    pub fn read_f32(&self, tensor: &GpuTensor) -> Vec<f32> {
        tensor.to_f32(&self.client)
    }
}
