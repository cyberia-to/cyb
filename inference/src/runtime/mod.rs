//! Pure wgpu GPU runtime — WGSL shaders, zero framework overhead
//! Ported from llama.cpp Metal shader algorithms

pub mod tensor;
pub mod kernels;
pub mod pipelines;
pub mod ops;

use std::sync::Arc;
use pipelines::Pipelines;

/// GPU runtime — wgpu device + shader pipelines
pub struct GpuRuntime {
    pub pipelines: Pipelines,
}

impl GpuRuntime {
    /// Initialize GPU runtime — creates wgpu device and compiles all shaders
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })).expect("No GPU adapter found");

        log::info!("GPU: {}", adapter.get_info().name);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("cyb-inference"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )).expect("Failed to create GPU device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let pipelines = Pipelines::new(device, queue);

        log::info!("All WGSL compute shaders compiled");

        Self { pipelines }
    }
}
