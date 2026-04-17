//! honeycrisp — Apple Silicon turbo backend.
//!
//! Stack: Metal + ANE + AMX + NEON + unimem (IOSurface zero-copy)
//! via the aruminium crate.
//!
//! Spec: reference/runtime/architecture.md#honeycrisp
//!
//! v1 skeleton: currently routes to CPU reference. Native honeycrisp
//! kernels are added incrementally with per-op tier 1 tests.
//! Only compiled on macOS (aruminium is macOS-only).

#[cfg(target_os = "macos")]
use crate::backend::{Backend, BackendError, BackendKind};
#[cfg(target_os = "macos")]
use crate::cpu::CpuBackend;
#[cfg(target_os = "macos")]
use crate::dtype::DType;
#[cfg(target_os = "macos")]
use crate::op::Op;
#[cfg(target_os = "macos")]
use crate::tensor::Tensor;

/// Apple Silicon backend. Uses Metal+ANE+AMX+NEON+unimem via aruminium.
#[cfg(target_os = "macos")]
pub struct HoneycrispBackend {
    cpu: CpuBackend,
    // aruminium device, pipelines, unimem layout go here as kernels are added.
}

#[cfg(target_os = "macos")]
impl HoneycrispBackend {
    pub fn new() -> Result<Self, BackendError> {
        Ok(Self {
            cpu: CpuBackend::new(),
        })
    }
}

#[cfg(target_os = "macos")]
impl Backend for HoneycrispBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Honeycrisp
    }

    fn supports(&self, op: &Op, inputs: &[&Tensor]) -> bool {
        self.cpu.supports(op, inputs)
    }

    fn execute(&self, op: &Op, inputs: &[&Tensor]) -> Result<Vec<Tensor>, BackendError> {
        self.cpu.execute(op, inputs)
    }

    fn upload(
        &self,
        bytes: &[u8],
        shape: Vec<usize>,
        dtype: DType,
    ) -> Result<Tensor, BackendError> {
        self.cpu.upload(bytes, shape, dtype)
    }

    fn download_f32(&self, t: &Tensor) -> Result<Vec<f32>, BackendError> {
        self.cpu.download_f32(t)
    }
}
