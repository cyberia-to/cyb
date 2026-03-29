pub mod onnx_proto;
pub mod graph;
pub mod ops;
pub mod quant;
pub mod generate;
pub mod hub;
pub mod runtime;

use burn::backend::Wgpu;

pub type Backend = Wgpu;

pub struct InferenceEngine {
    pub device: burn::backend::wgpu::WgpuDevice,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            device: burn::backend::wgpu::WgpuDevice::default(),
        }
    }
}
