//! Data types for tensors

/// Tensor element data type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum DType {
    F32,
    F16,
    BF16,
    Q8,
    Q4,
    Q4_1,
    Ternary,
    // K-quant types (super blocks of 256 elements)
    Q2_K,
    Q3_K,
    Q4_K,
    Q5_K,
    Q6_K,
}

impl DType {
    /// Size of one element in bytes (for non-quantized types)
    pub fn element_size(&self) -> usize {
        match self {
            DType::F32 => 4,
            DType::F16 | DType::BF16 => 2,
            DType::Q8 => 1,
            DType::Q4 | DType::Q4_1 => 1, // approximate, actual is sub-byte
            DType::Ternary => 1,
            // K-quant: approximate per-element sizes (actual is per-block)
            DType::Q2_K => 1,
            DType::Q3_K => 1,
            DType::Q4_K => 1,
            DType::Q5_K => 1,
            DType::Q6_K => 1,
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            DType::F32 => "f32",
            DType::F16 => "f16",
            DType::BF16 => "bf16",
            DType::Q8 => "q8",
            DType::Q4 => "q4",
            DType::Q4_1 => "q4_1",
            DType::Ternary => "ternary",
            DType::Q2_K => "q2_k",
            DType::Q3_K => "q3_k",
            DType::Q4_K => "q4_k",
            DType::Q5_K => "q5_k",
            DType::Q6_K => "q6_k",
        }
    }

    /// Whether this is a K-quant type (super blocks of 256 elements)
    pub fn is_k_quant(&self) -> bool {
        matches!(self, DType::Q2_K | DType::Q3_K | DType::Q4_K | DType::Q5_K | DType::Q6_K)
    }
}

impl std::fmt::Display for DType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
