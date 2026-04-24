//! Primitive types shared by every module.
//!
//! dtype — DType enum (F32, F16, Q4_K, …)
//! op    — Op enum (MatMul, RmsNorm, RoPE, …)
//! tensor — Tensor + Shape

pub mod dtype;
pub mod op;
pub mod tensor;

pub use dtype::DType;
pub use op::{InterpolateMode, Op, PoolMode, SampleMethod};
pub use tensor::{Shape, Tensor};
