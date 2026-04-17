//! Block quantization dequant — CPU reference.
//!
//! Spec: reference/runtime/quant.md

pub mod q4_0;
pub mod q4_k;
pub mod q5_k;
pub mod q6_k;
pub mod q8_0;

use crate::dtype::DType;

/// Dequantize any supported block-quantized format to f32.
///
/// Panics if dtype is not quantized.
pub fn dequantize_to_f32(bytes: &[u8], dtype: DType) -> Vec<f32> {
    match dtype {
        DType::Q4_0 => q4_0::dequantize(bytes),
        DType::Q4_K => q4_k::dequantize(bytes),
        DType::Q5_K => q5_k::dequantize(bytes),
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
        other => panic!("dequantize_to_f32: dtype {other:?} not supported"),
    }
}
