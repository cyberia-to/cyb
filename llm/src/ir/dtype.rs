//! Data types for tensors

/// Tensor element data type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DType {
    F32,
    F16,
    BF16,
    Q8,
    Q4,
    Q4_1,
    Ternary,
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
        }
    }
}

impl std::fmt::Display for DType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
