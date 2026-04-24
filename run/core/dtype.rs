//! Tensor element data types.
//!
//! Spec: reference/runtime/tensor.md, reference/runtime/quant.md

use serde::{Deserialize, Serialize};

/// Element data type.
///
/// Floating-point types are IEEE 754 (F32/F16) or Brain-float (BF16).
/// Quantized types have exact byte layouts in quant.md.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum DType {
    F32,
    F16,
    BF16,
    I8,
    U8,
    Bool,
    Q8_0,
    Q4_0,
    Q2_K,
    Q3_K,
    Q4_K,
    Q5_K,
    Q6_K,
    Ternary,
}

impl DType {
    /// Block size in elements (1 for scalar types).
    pub fn block_size(self) -> usize {
        match self {
            DType::F32 | DType::F16 | DType::BF16 | DType::I8 | DType::U8 | DType::Bool => 1,
            DType::Q4_0 | DType::Q8_0 => 32,
            DType::Q2_K | DType::Q3_K | DType::Q4_K | DType::Q5_K | DType::Q6_K => 256,
            DType::Ternary => 4, // 4 values per byte
        }
    }

    /// Bytes per block (for scalar types: bytes per element).
    pub fn block_bytes(self) -> usize {
        match self {
            DType::F32 => 4,
            DType::F16 | DType::BF16 => 2,
            DType::I8 | DType::U8 | DType::Bool => 1,
            DType::Q4_0 => 18,
            DType::Q8_0 => 34,
            DType::Q2_K => 84,
            DType::Q3_K => 110,
            DType::Q4_K => 144,
            DType::Q5_K => 176,
            DType::Q6_K => 210,
            DType::Ternary => 1,
        }
    }

    /// Total bytes required for `n` values stored in this dtype.
    ///
    /// Panics if `n` is not a multiple of `block_size()` for quantized types.
    pub fn bytes_for(self, n: usize) -> usize {
        let bs = self.block_size();
        assert!(
            n % bs == 0,
            "DType {self:?} requires block-aligned count, got {n}"
        );
        (n / bs) * self.block_bytes()
    }

    /// Canonical encoding string used in `.model` tensor index.
    pub fn as_str(self) -> &'static str {
        match self {
            DType::F32 => "u32",
            DType::F16 => "u16",
            DType::BF16 => "bf16",
            DType::I8 => "i8",
            DType::U8 => "u8",
            DType::Bool => "bool",
            DType::Q8_0 => "q8",
            DType::Q4_0 => "q4",
            DType::Q2_K => "q2k",
            DType::Q3_K => "q3k",
            DType::Q4_K => "q4k",
            DType::Q5_K => "q5k",
            DType::Q6_K => "q6k",
            DType::Ternary => "ternary",
        }
    }

    /// Parse from canonical encoding string.
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "u32" | "f32" => DType::F32,
            "u16" | "f16" => DType::F16,
            "bf16" => DType::BF16,
            "i8" => DType::I8,
            "u8" => DType::U8,
            "bool" => DType::Bool,
            "q8" | "q8_0" => DType::Q8_0,
            "q4" | "q4_0" => DType::Q4_0,
            "q2k" | "q2_k" => DType::Q2_K,
            "q3k" | "q3_k" => DType::Q3_K,
            "q4k" | "q4_k" => DType::Q4_K,
            "q5k" | "q5_k" => DType::Q5_K,
            "q6k" | "q6_k" => DType::Q6_K,
            "ternary" => DType::Ternary,
            _ => return None,
        })
    }

    /// True if this is a floating-point scalar type.
    pub fn is_float(self) -> bool {
        matches!(self, DType::F32 | DType::F16 | DType::BF16)
    }

    /// True if this is a block-quantized type (needs dequant to use).
    pub fn is_quantized(self) -> bool {
        matches!(
            self,
            DType::Q4_0
                | DType::Q8_0
                | DType::Q2_K
                | DType::Q3_K
                | DType::Q4_K
                | DType::Q5_K
                | DType::Q6_K
                | DType::Ternary
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_sizes_match_spec() {
        assert_eq!(DType::F32.block_bytes(), 4);
        assert_eq!(DType::F16.block_bytes(), 2);
        assert_eq!(DType::Q4_0.block_size(), 32);
        assert_eq!(DType::Q4_0.block_bytes(), 18);
        assert_eq!(DType::Q4_K.block_size(), 256);
        assert_eq!(DType::Q4_K.block_bytes(), 144);
        assert_eq!(DType::Q6_K.block_bytes(), 210);
    }

    #[test]
    fn bytes_for_roundtrip() {
        assert_eq!(DType::F32.bytes_for(1024), 4096);
        assert_eq!(DType::Q4_K.bytes_for(256), 144);
        assert_eq!(DType::Q4_K.bytes_for(1024), 576); // 4 blocks
    }

    #[test]
    #[should_panic(expected = "block-aligned")]
    fn bytes_for_unaligned_panics() {
        DType::Q4_K.bytes_for(300); // not a multiple of 256
    }

    #[test]
    fn dtype_str_roundtrip() {
        for dt in [
            DType::F32,
            DType::F16,
            DType::Q4_0,
            DType::Q4_K,
            DType::Q6_K,
            DType::Ternary,
        ] {
            assert_eq!(DType::from_str(dt.as_str()), Some(dt));
        }
    }
}
