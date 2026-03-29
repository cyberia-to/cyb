//! Backend trait + detection
//!
//! Pluggable compute backends. Currently only wgpu.

pub mod wgpu_backend;

use std::collections::HashMap;

use crate::ir::Graph;

/// Backend trait — execute a model graph on a compute device
pub trait Backend {
    /// Backend name for logging
    fn name(&self) -> &str;

    /// Execute forward pass, returning logits
    fn execute(
        &mut self,
        graph: &Graph,
        inputs: &HashMap<String, Vec<f32>>,
    ) -> Result<HashMap<String, Vec<f32>>, String>;
}

/// Detect best available backend
pub fn detect_backend() -> Box<dyn Backend> {
    log::info!("Detecting best available backend...");
    Box::new(wgpu_backend::WgpuBackend::new())
}

/// Get the wgpu backend pipelines for direct model loading
pub fn create_wgpu_backend() -> wgpu_backend::WgpuBackend {
    wgpu_backend::WgpuBackend::new()
}
