//! WgpuBackend — pure wgpu compute shader backend
//!
//! Ports the working inference pipeline from cyb-inference.
//! Uses hardcoded forward pass for transformer decoders (optimized).

pub mod pipelines;
pub mod alloc;
pub mod dispatch;
pub mod model;
pub mod graph_model;

use std::collections::HashMap;
use std::sync::Arc;

use pipelines::Pipelines;

use crate::backend::Backend;
use crate::ir::Graph;

/// wgpu-based compute backend
pub struct WgpuBackend {
    pub pipelines: Arc<Pipelines>,
}

impl WgpuBackend {
    /// Initialize GPU runtime — creates wgpu device and compiles all shaders
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            }),
        )
        .expect("No GPU adapter found");

        log::info!("GPU: {}", adapter.get_info().name);

        let mut limits = wgpu::Limits::default();
        limits.max_buffer_size = 4u64 << 30; // 4GB
        limits.max_storage_buffer_binding_size = u32::MAX; // ~4GB

        let mut features = wgpu::Features::empty();
        if adapter.features().contains(wgpu::Features::SUBGROUP) {
            features |= wgpu::Features::SUBGROUP;
            log::info!("SUBGROUP OPS ENABLED");
        }

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("cyb-llm"),
                required_features: features,
                required_limits: limits,
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("Failed to create GPU device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let pipelines = Arc::new(Pipelines::new(device, queue));

        log::info!("All WGSL compute shaders compiled");

        Self { pipelines }
    }
}

impl Backend for WgpuBackend {
    fn name(&self) -> &str {
        "wgpu"
    }

    fn execute(
        &mut self,
        _graph: &Graph,
        _inputs: &HashMap<String, Vec<f32>>,
    ) -> Result<HashMap<String, Vec<f32>>, String> {
        // The optimized forward pass lives in model::NativeModel.
        // This trait method is for generic graph execution.
        // For now, return an error — callers should use NativeModel directly.
        Err("Use NativeModel::forward() for optimized transformer execution".to_string())
    }
}
