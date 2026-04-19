//! Q5_K — 5-bit K-quant, 256-value superblocks.
//!
//! NOT YET IMPLEMENTED. Callers should use `try_dequantize_to_f32`
//! which surfaces `BackendError::UnsupportedDtype` instead of panicking.

pub const BLOCK_SIZE: usize = 256;
pub const BLOCK_BYTES: usize = 176;

/// Legacy panicking entry point — retained for tests that intentionally
/// probe error paths. Production code paths route through
/// `try_dequantize_to_f32` which returns `BackendError::UnsupportedDtype`.
pub fn dequantize(_bytes: &[u8]) -> Vec<f32> {
    panic!("Q5_K dequant not implemented — use try_dequantize_to_f32 for the graceful error path");
}
