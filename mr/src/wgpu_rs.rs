//! wgpu+rs — portable GPU backend with CPU fallback.
//!
//! Spec: reference/runtime/architecture.md#wgpurs
//!
//! v1 skeleton: currently routes everything to the CPU reference library.
//! Native wgpu kernels for matmul, rmsnorm, sdpa etc are added incrementally
//! with per-op tier 1 tests verifying output against CPU.

use crate::backend::{Backend, BackendError, BackendKind};
use crate::cpu::CpuBackend;
use crate::dtype::DType;
use crate::op::Op;
use crate::tensor::Tensor;

/// Portable GPU backend. Falls back to CPU for unimplemented ops.
pub struct WgpuRsBackend {
    /// CPU reference, used both as fallback and as the default impl until
    /// wgpu kernels are wired up.
    cpu: CpuBackend,
    // wgpu device/queue will go here once kernels are added.
}

impl WgpuRsBackend {
    pub fn new() -> Result<Self, BackendError> {
        Ok(Self {
            cpu: CpuBackend::new(),
        })
    }
}

impl Backend for WgpuRsBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::WgpuRs
    }

    fn supports(&self, op: &Op, inputs: &[&Tensor]) -> bool {
        // No native wgpu kernels yet — everything falls back to CPU.
        // As kernels are added, return true for supported ops here.
        // Invariant: if supports returns true, execute must produce within-eps output.
        self.cpu.supports(op, inputs)
    }

    fn execute(&self, op: &Op, inputs: &[&Tensor]) -> Result<Vec<Tensor>, BackendError> {
        // Until native wgpu kernels land, delegate to CPU.
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
