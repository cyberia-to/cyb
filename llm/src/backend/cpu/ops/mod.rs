//! CPU operations for transformer training/inference
//!
//! All operations work on f32 channel-first layout: `[dim, seq]`
//! where `x[d * seq + t]` is dimension `d` at position `t`.

pub mod activation;
pub mod adam;
pub mod attention;
pub mod embed;
pub mod loss;
pub mod rmsnorm;
pub mod rope;
pub mod sample;
