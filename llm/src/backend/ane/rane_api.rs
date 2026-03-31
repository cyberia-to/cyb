//! `ane` — Pure Rust driver for Apple Neural Engine
//!
//! Compile MIL programs, load them into ANE hardware, and dispatch
//! compute kernels. Zero external dependencies — only macOS system frameworks.
//!
//! # Example
//!
//! ```no_run
//! use rane::{AneModel, AneSurface};
//!
//! let program = rane::mil::matmul(64, 64, 64);
//! let mut model = AneModel::compile(&program, &[])?;
//! model.load()?;
//!
//! let input = AneSurface::new(program.input_bytes())?;
//! let output = AneSurface::new(program.output_bytes())?;
//! model.run(&input, &output)?;
//! # Ok::<(), rane::AneError>(())
//! ```
//!
//! # Architecture
//!
//! ```text
//! MIL text → AneModel::compile() → load() → run() → unload()
//!   ↓              ↓                  ↓        ↓
//!   MIL         aned XPC           ANE SRAM  IOSurface I/O
//! ```
//!
//! All data passes through [`AneSurface`] — IOSurface-backed shared memory
//! with zero copies between CPU and ANE. Tensor data is fp16.

#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::missing_transmute_annotations
)]

#[doc(hidden)]
pub mod ffi;
pub mod mil;
pub mod model;
pub mod surface;

pub use mil::{build_weight_blob, gen_dyn_matmul, mil_footer, mil_header, MilProgram};
pub use model::AneModel;
pub use surface::{cvt_f16_f32, cvt_f32_f16, f32_to_fp16, fp16_to_f32, AneSurface};

/// Errors from ANE driver operations.
#[derive(Debug)]
#[non_exhaustive]
pub enum AneError {
    /// IOSurface allocation failed (size too large or system out of memory).
    SurfaceCreationFailed(String),
    /// Required ObjC class not found in private frameworks.
    ClassNotFound(&'static str),
    /// MIL model descriptor creation rejected by framework.
    DescriptorCreationFailed,
    /// ANE model object allocation failed.
    ModelCreationFailed,
    /// MIL → bytecode compilation failed (invalid MIL or unsupported ops).
    CompilationFailed(String),
    /// Bytecode upload to ANE SRAM failed.
    LoadFailed(String),
    /// Hardware execution failed (shape mismatch, ANE busy, or driver error).
    EvalFailed(String),
    /// SRAM release failed.
    UnloadFailed(String),
    /// Filesystem error (temp directory, weight file I/O).
    Io(std::io::Error),
}

impl std::fmt::Display for AneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AneError::SurfaceCreationFailed(msg) => {
                write!(f, "IOSurface creation failed: {}", msg)
            }
            AneError::ClassNotFound(name) => write!(f, "ObjC class not found: {}", name),
            AneError::DescriptorCreationFailed => write!(f, "Model descriptor creation failed"),
            AneError::ModelCreationFailed => write!(f, "Model creation failed"),
            AneError::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            AneError::LoadFailed(msg) => write!(f, "Load failed: {}", msg),
            AneError::EvalFailed(msg) => write!(f, "Evaluation failed: {}", msg),
            AneError::UnloadFailed(msg) => write!(f, "Unload failed: {}", msg),
            AneError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for AneError {}

impl From<std::io::Error> for AneError {
    fn from(e: std::io::Error) -> Self {
        AneError::Io(e)
    }
}
