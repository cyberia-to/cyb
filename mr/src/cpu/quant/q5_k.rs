//! Q5_K — 5-bit K-quant with 256-value superblocks.
//!
//! Block layout (176 bytes):
//!   f16 d / f16 dmin / u8 scales[12] / u8 qh[32] / u8 qs[128]
//!
//! Spec: reference/runtime/quant.md
//!
//! NOT YET IMPLEMENTED. Returns an error via panic() — fail-loud rather
//! than return wrong values. Add full implementation when the first
//! model in our manifest uses Q5_K.

pub const BLOCK_SIZE: usize = 256;
pub const BLOCK_BYTES: usize = 176;

pub fn dequantize(_bytes: &[u8]) -> Vec<f32> {
    panic!("Q5_K dequant not implemented yet — see reference/runtime/quant.md");
}
