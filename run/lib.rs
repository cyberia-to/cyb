//! run — universal, portable, spec-driven inference runtime.
//!
//! Arch: arch/decoder (causal decoder), future: encoder, diffusion.
//! Backends: backend/cpu (reference), backend/wgpu (portable GPU),
//!           backend/honeycrisp (Apple Silicon turbo).
//! Future backend: nox (convergent VM).
//!
//! Spec: reference/runtime/ in the repo root.

pub mod arch;
pub mod backend;
pub mod bench;
pub mod dtype;
pub mod format;
pub mod generate;
pub mod ir;
pub mod manifest;
pub mod op;
pub mod tensor;
pub mod tokenizer;

pub use backend::{Backend, BackendError, BackendKind};
pub use dtype::DType;
pub use format::{read_model_file, LoadedModel, ModelFile, TensorMeta};
pub use op::{Op, PoolMode, InterpolateMode, SampleMethod};
pub use tensor::{Tensor, Shape};
pub use tokenizer::{Tokenizer, ChatMessage};
