//! run — universal, portable, spec-driven inference runtime.
//!
//! Arch: arch/decoder (causal decoder), future: encoder, diffusion.
//! Backends: backend/cpu (reference), backend/wgpu (portable GPU),
//!           backend/honeycrisp (Apple Silicon turbo).
//! Future backend: nox (convergent VM).
//!
//! Spec: specs/ in the repo root.

pub mod arch;
pub mod backend;
pub mod bench;
pub mod core;
pub mod format;
pub mod generate;
pub mod ir;
pub mod manifest;
pub mod tokenizer;

pub use backend::{Backend, BackendError, BackendKind};
pub use generate::ModelRunner;
pub use ir::GraphRunner;
pub use core::{DType, Op, PoolMode, InterpolateMode, SampleMethod, Tensor, Shape};
pub use format::{read_model_file, LoadedModel, ModelFile, TensorMeta};
pub use tokenizer::{Tokenizer, ChatMessage};
