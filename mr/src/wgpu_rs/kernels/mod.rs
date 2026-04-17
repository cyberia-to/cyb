//! WGSL kernels and their dispatch wrappers.
//!
//! Each kernel is a native GPU implementation of the corresponding op
//! from reference/runtime/ops.md. Output must match CPU reference
//! within ε tolerance (verified by Tier 1 tests).

pub mod matmul;
pub mod rmsnorm;
pub mod rope;
pub mod silu;
pub mod softmax;
