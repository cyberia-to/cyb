//! Backend trait + detection
//!
//! Pluggable compute backends: wgpu (cross-platform) + metal (Apple Silicon).

pub mod wgpu;

#[cfg(target_os = "macos")]
pub mod metal;

// ANE/CPU/Apple backends depend on unimem/acpu — enable when those crates are ready
// #[cfg(target_os = "macos")]
// pub mod ane;
// #[cfg(target_os = "macos")]
// pub mod cpu;
// #[cfg(target_os = "macos")]
// pub mod apple;

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
    Box::new(wgpu::WgpuBackend::new())
}

/// Get the wgpu backend pipelines for direct model loading
pub fn create_wgpu_backend() -> wgpu::WgpuBackend {
    wgpu::WgpuBackend::new()
}
