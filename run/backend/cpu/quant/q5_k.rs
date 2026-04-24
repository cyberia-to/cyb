//! Q5_K — 5-bit K-quant, 256-value superblocks.
//!
//! Not yet implemented. `try_dequantize_to_f32` returns
//! `BackendError::UnsupportedDtype` for Q5_K weights.
//! Spec: specs/quant.md §Q5_K.

pub const BLOCK_SIZE: usize = 256;
pub const BLOCK_BYTES: usize = 176;
