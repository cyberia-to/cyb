//! Block quantization dequant — CPU reference.
//!
//! Spec: specs/quant.md

pub mod q4_0;
pub mod q4_k;
pub mod q5_k;
pub mod q6_k;
pub mod q8_0;

use crate::backend::BackendError;
use crate::core::dtype::DType;

/// Fallible dequant — returns structured error for unsupported dtypes.
pub fn try_dequantize_to_f32(bytes: &[u8], dtype: DType) -> Result<Vec<f32>, BackendError> {
    Ok(match dtype {
        DType::Q4_0 => q4_0::dequantize(bytes),
        DType::Q4_K => q4_k::dequantize(bytes),
        DType::Q5_K => {
            return Err(BackendError::UnsupportedDtype {
                backend: "cpu",
                dtype: DType::Q5_K,
                blocker: "Q5_K dequant not implemented; see specs/quant.md",
            });
        }
        DType::Q6_K => q6_k::dequantize(bytes),
        DType::Q8_0 => q8_0::dequantize(bytes),
        DType::F32 => bytemuck::cast_slice(bytes).to_vec(),
        DType::F16 => bytemuck::cast_slice::<u8, u16>(bytes)
            .iter()
            .map(|&bits| half::f16::from_bits(bits).to_f32())
            .collect(),
        DType::BF16 => bytemuck::cast_slice::<u8, u16>(bytes)
            .iter()
            .map(|&bits| half::bf16::from_bits(bits).to_f32())
            .collect(),
        other => {
            return Err(BackendError::UnsupportedDtype {
                backend: "cpu",
                dtype: other,
                blocker: "not a supported weight dtype",
            });
        }
    })
}

/// Legacy panicking wrapper — retained for tests. New code should call
/// `try_dequantize_to_f32`.
pub fn dequantize_to_f32(bytes: &[u8], dtype: DType) -> Vec<f32> {
    try_dequantize_to_f32(bytes, dtype).expect("dequantize_to_f32: unsupported dtype")
}
