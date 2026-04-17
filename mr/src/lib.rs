//! mr (modelruntime) — universal, portable, spec-driven inference runtime.
//!
//! Three backends: wgpu+rs (portable), honeycrisp (Apple Silicon turbo),
//! nox (convergent VM, future).
//!
//! Spec: reference/runtime/ in the repo root.

pub mod backend;
pub mod cpu;
pub mod dtype;
pub mod op;
pub mod tensor;
pub mod wgpu_rs;

#[cfg(target_os = "macos")]
pub mod honeycrisp;

pub use backend::{Backend, BackendError, BackendKind};
pub use dtype::DType;
pub use op::{Op, PoolMode, InterpolateMode, SampleMethod};
pub use tensor::{Tensor, Shape};
