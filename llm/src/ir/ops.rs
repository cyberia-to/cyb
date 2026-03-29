//! Operation types for the Graph IR

/// Operations that can appear in the computation graph
#[derive(Clone, Debug)]
pub enum Op {
    // ---- Linear algebra ----
    Matmul,
    Add,
    Mul,
    Sub,
    Div,

    // ---- Attention ----
    Attention {
        num_heads: u32,
        kv_heads: u32,
        head_dim: u32,
    },
    Rope {
        head_dim: u32,
    },
    KvCacheAppend,

    // ---- Normalization ----
    RmsNorm {
        eps: f32,
    },
    LayerNorm {
        eps: f32,
    },

    // ---- Activation ----
    Silu,
    Gelu,
    Relu,
    Sigmoid,
    Softmax,
    Tanh,

    // ---- Fused ops ----
    SiluMul,
    FusedNormMatmul {
        eps: f32,
    },
    FusedSkipNorm {
        eps: f32,
    },

    // ---- Tensor manipulation ----
    Reshape {
        shape: Vec<i64>,
    },
    Transpose {
        perm: Vec<usize>,
    },
    Concat {
        axis: usize,
    },
    Embed,

    // ---- Quantization ----
    Dequantize,

    // ---- Sampling ----
    Argmax,
}

impl Op {
    /// Human-readable name for logging
    pub fn name(&self) -> &'static str {
        match self {
            Op::Matmul => "Matmul",
            Op::Add => "Add",
            Op::Mul => "Mul",
            Op::Sub => "Sub",
            Op::Div => "Div",
            Op::Attention { .. } => "Attention",
            Op::Rope { .. } => "Rope",
            Op::KvCacheAppend => "KvCacheAppend",
            Op::RmsNorm { .. } => "RmsNorm",
            Op::LayerNorm { .. } => "LayerNorm",
            Op::Silu => "Silu",
            Op::Gelu => "Gelu",
            Op::Relu => "Relu",
            Op::Sigmoid => "Sigmoid",
            Op::Softmax => "Softmax",
            Op::Tanh => "Tanh",
            Op::SiluMul => "SiluMul",
            Op::FusedNormMatmul { .. } => "FusedNormMatmul",
            Op::FusedSkipNorm { .. } => "FusedSkipNorm",
            Op::Reshape { .. } => "Reshape",
            Op::Transpose { .. } => "Transpose",
            Op::Concat { .. } => "Concat",
            Op::Embed => "Embed",
            Op::Dequantize => "Dequantize",
            Op::Argmax => "Argmax",
        }
    }
}
