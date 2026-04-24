//! Execution backends.
//!
//! Trait + implementations: cpu (portable reference), wgpu (GPU),
//! honeycrisp (Apple Silicon turbo). Adding a backend = new submodule here.
//!
//! Spec: reference/runtime/execution.md, reference/runtime/architecture.md

pub mod cpu;
pub mod wgpu;

#[cfg(target_os = "macos")]
pub mod honeycrisp;

use crate::dtype::DType;
use crate::op::Op;
use crate::tensor::Tensor;
use thiserror::Error;

/// Three backends + cpu reference library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    /// Pure-Rust CPU reference. Always correct, slow.
    /// Not user-facing; used internally by wgpu+rs for op fallback.
    Cpu,
    /// Portable wgpu GPU + CPU fallback.
    WgpuRs,
    /// Apple Silicon turbo (Metal + ANE + AMX + NEON + unimem).
    Honeycrisp,
    /// Future: trident-compiled bytecode on deterministic VM.
    Nox,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendKind::Cpu => "cpu",
            BackendKind::WgpuRs => "wgpu+rs",
            BackendKind::Honeycrisp => "honeycrisp",
            BackendKind::Nox => "nox",
        }
    }
}

/// Errors surfaced by backends. Structured, actionable.
#[derive(Error, Debug)]
pub enum BackendError {
    #[error("backend {backend} does not support op {op} with dtype {input_dtype:?}")]
    UnsupportedOp {
        backend: &'static str,
        op: &'static str,
        input_dtype: DType,
    },

    #[error("shape mismatch in {op}: expected {expected:?}, got {got:?}")]
    ShapeMismatch {
        op: &'static str,
        expected: Vec<usize>,
        got: Vec<usize>,
    },

    #[error("invalid input to {op}: {reason}")]
    InvalidInput { op: &'static str, reason: String },

    #[error("out of memory on {backend}: requested {requested_bytes} bytes")]
    OutOfMemory {
        backend: &'static str,
        requested_bytes: usize,
    },

    #[error("backend {backend} init failed: {reason}")]
    BackendInit {
        backend: &'static str,
        reason: String,
    },

    #[error("dtype {dtype:?} not implemented in {backend} (blocker: {blocker})")]
    UnsupportedDtype {
        backend: &'static str,
        dtype: DType,
        blocker: &'static str,
    },

    #[error("context overflow: requested position {pos}, max {max}")]
    ContextOverflow { pos: usize, max: usize },

    #[error("NaN/Inf detected in {op} output (layer {layer}, position {pos})")]
    NonFiniteOutput {
        op: &'static str,
        layer: usize,
        pos: usize,
    },

    #[error("tensor: {0}")]
    Tensor(#[from] crate::tensor::TensorError),

    #[error("internal: {0}")]
    Internal(String),
}

/// What every backend must implement.
///
/// The CPU reference library implements ALL ops in f32.
/// GPU backends implement what they support; missing ops route to CPU.
pub trait Backend: Send + Sync {
    /// Identity for logging and dispatch.
    fn kind(&self) -> BackendKind;

    /// True if this backend has a native implementation of `op` for the
    /// given input tensor shapes/dtypes.
    ///
    /// If false, the dispatcher falls back to CPU reference.
    fn supports(&self, op: &Op, inputs: &[&Tensor]) -> bool;

    /// Execute one op. Inputs are guaranteed to be on this backend's memory
    /// (caller uploads first if needed).
    ///
    /// Returns output tensor(s), or a structured error.
    fn execute(&self, op: &Op, inputs: &[&Tensor]) -> Result<Vec<Tensor>, BackendError>;

    /// Upload host bytes to backend memory.
    fn upload(
        &self,
        bytes: &[u8],
        shape: Vec<usize>,
        dtype: DType,
    ) -> Result<Tensor, BackendError>;

    /// Move a host tensor to backend memory. Default implementation
    /// uploads raw bytes; backends may override for zero-copy paths.
    fn to_backend(&self, t: &Tensor) -> Result<Tensor, BackendError> {
        match &t.data {
            crate::tensor::TensorData::Host(bytes) => {
                self.upload(bytes.as_slice(), t.shape.clone(), t.dtype)
            }
            crate::tensor::TensorData::Backend(_) => Ok(t.clone()),
        }
    }

    /// Download tensor to host memory as F32.
    /// For quantized inputs, dequantizes to F32 on download.
    fn download_f32(&self, t: &Tensor) -> Result<Vec<f32>, BackendError>;
}
