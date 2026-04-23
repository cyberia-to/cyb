//! Op enum — one variant per primitive operation.
//!
//! Spec: reference/runtime/ops.md, reference/runtime/ir.md

use serde::{Deserialize, Serialize};

/// One operation in a graph. Variants correspond to math in ops.md.
///
/// Each variant carries its non-tensor parameters. Tensor inputs/outputs
/// are tracked separately by the graph Node ([ir.md]).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Op {
    // ==== Linear algebra ====
    Matmul,
    Add,
    Mul,
    Sub,
    Div,
    Transpose { perm: Vec<usize> },
    Reshape { shape: Vec<i64> },
    Permute { dims: Vec<usize> },
    Concat { axis: usize },
    Split { axis: usize, sizes: Vec<usize> },
    Chunk { axis: usize, chunks: usize },
    Clamp { min: Option<f32>, max: Option<f32> },
    NanToNum { nan: f32, posinf: f32, neginf: f32 },
    Argmax { dim: i32 },

    // ==== Attention ====
    Sdpa { num_heads: u32, kv_heads: u32, head_dim: u32, causal: bool },
    SdpaCross { num_heads: u32, head_dim: u32 },
    SdpaWindow { num_heads: u32, head_dim: u32, window_size: u32 },
    KvCache,
    KvCompress { head_dim: u32, bits: u32 },
    KvDecompress { head_dim: u32, bits: u32 },
    /// Rotary positional embedding (NeoX pairing).
    /// `rope_dim` rotated dims (≤ head_dim); the rest pass through unchanged.
    /// Default `rope_dim == head_dim` is the standard full-rotary case;
    /// Gemma-4 full-attention layers use `rope_dim = head_dim / 4`
    /// (`partial_rotary_factor = 0.25`).
    Rope { head_dim: u32, rope_dim: u32, base: f32 },
    SinusoidalEmbed { dim: u32 },
    RelativePosEmbedding { num_buckets: u32 },
    TokenEmbed,
    PosEmbed,

    // ==== Normalization ====
    RmsNorm { eps: f32 },
    LayerNorm { eps: f32 },
    BatchNorm { eps: f32, momentum: f32 },
    GroupNorm { num_groups: u32, eps: f32 },
    InstanceNorm { eps: f32 },
    AdaLN,

    // ==== Activation ====
    Silu,
    Gelu { approximate: bool },
    GeGlu,
    SwiGlu,
    Glu,
    Relu,
    LeakyRelu { slope: f32 },
    PRelu,
    Sigmoid,
    Tanh,
    Softmax { dim: i32 },

    // ==== Convolution ====
    Conv1d { kernel: u32, stride: u32, padding: u32, dilation: u32, groups: u32 },
    Conv2d {
        kernel: (u32, u32),
        stride: (u32, u32),
        padding: (u32, u32),
        dilation: (u32, u32),
        groups: u32,
    },
    Conv3d {
        kernel: (u32, u32, u32),
        stride: (u32, u32, u32),
        padding: (u32, u32, u32),
        dilation: (u32, u32, u32),
        groups: u32,
    },
    ConvTranspose2d {
        kernel: (u32, u32),
        stride: (u32, u32),
        padding: (u32, u32),
    },
    CausalConv1d { kernel: u32 },
    DepthwiseConv { kernel: u32, stride: u32 },
    Pool {
        mode: PoolMode,
        kernel: (u32, u32),
        stride: (u32, u32),
        padding: (u32, u32),
    },

    // ==== Spatial ====
    Interpolate { mode: InterpolateMode, scale: f32 },
    PixelShuffle { upscale_factor: u32 },
    PixelUnshuffle { downscale_factor: u32 },
    PatchEmbed { patch_size: u32 },
    Unpatchify,

    // ==== Diffusion / flow ====
    NoiseSchedule,
    FlowStep,

    // ==== Quantization ====
    Quantize { dtype: crate::dtype::DType },
    Dequantize,

    // ==== Sampling ====
    Sample { method: SampleMethod },

    // ==== Adapters ====
    LoraApply { rank: u32, alpha: f32 },
    Kron,
    MatrixInverse,

    // ==== Fused (optimization) ====
    FusedNormMatmul { eps: f32 },
    FusedSkipNorm { eps: f32 },
    FusedSwiGlu,
    FlashAttention { num_heads: u32, kv_heads: u32, head_dim: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolMode {
    Max,
    Avg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterpolateMode {
    Nearest,
    Bilinear,
    Area,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SampleMethod {
    Greedy,
    Temperature { temp: f32 },
    TopK { k: u32, temp: f32 },
    TopP { p: f32, temp: f32 },
    MinP { p: f32, temp: f32 },
}

impl Op {
    /// Human-readable name. Stable for logging and error messages.
    pub fn name(&self) -> &'static str {
        match self {
            Op::Matmul => "Matmul",
            Op::Add => "Add",
            Op::Mul => "Mul",
            Op::Sub => "Sub",
            Op::Div => "Div",
            Op::Transpose { .. } => "Transpose",
            Op::Reshape { .. } => "Reshape",
            Op::Permute { .. } => "Permute",
            Op::Concat { .. } => "Concat",
            Op::Split { .. } => "Split",
            Op::Chunk { .. } => "Chunk",
            Op::Clamp { .. } => "Clamp",
            Op::NanToNum { .. } => "NanToNum",
            Op::Argmax { .. } => "Argmax",
            Op::Sdpa { .. } => "Sdpa",
            Op::SdpaCross { .. } => "SdpaCross",
            Op::SdpaWindow { .. } => "SdpaWindow",
            Op::KvCache => "KvCache",
            Op::KvCompress { .. } => "KvCompress",
            Op::KvDecompress { .. } => "KvDecompress",
            Op::Rope { .. } => "Rope",
            Op::SinusoidalEmbed { .. } => "SinusoidalEmbed",
            Op::RelativePosEmbedding { .. } => "RelativePosEmbedding",
            Op::TokenEmbed => "TokenEmbed",
            Op::PosEmbed => "PosEmbed",
            Op::RmsNorm { .. } => "RmsNorm",
            Op::LayerNorm { .. } => "LayerNorm",
            Op::BatchNorm { .. } => "BatchNorm",
            Op::GroupNorm { .. } => "GroupNorm",
            Op::InstanceNorm { .. } => "InstanceNorm",
            Op::AdaLN => "AdaLN",
            Op::Silu => "Silu",
            Op::Gelu { .. } => "Gelu",
            Op::GeGlu => "GeGlu",
            Op::SwiGlu => "SwiGlu",
            Op::Glu => "Glu",
            Op::Relu => "Relu",
            Op::LeakyRelu { .. } => "LeakyRelu",
            Op::PRelu => "PRelu",
            Op::Sigmoid => "Sigmoid",
            Op::Tanh => "Tanh",
            Op::Softmax { .. } => "Softmax",
            Op::Conv1d { .. } => "Conv1d",
            Op::Conv2d { .. } => "Conv2d",
            Op::Conv3d { .. } => "Conv3d",
            Op::ConvTranspose2d { .. } => "ConvTranspose2d",
            Op::CausalConv1d { .. } => "CausalConv1d",
            Op::DepthwiseConv { .. } => "DepthwiseConv",
            Op::Pool { .. } => "Pool",
            Op::Interpolate { .. } => "Interpolate",
            Op::PixelShuffle { .. } => "PixelShuffle",
            Op::PixelUnshuffle { .. } => "PixelUnshuffle",
            Op::PatchEmbed { .. } => "PatchEmbed",
            Op::Unpatchify => "Unpatchify",
            Op::NoiseSchedule => "NoiseSchedule",
            Op::FlowStep => "FlowStep",
            Op::Quantize { .. } => "Quantize",
            Op::Dequantize => "Dequantize",
            Op::Sample { .. } => "Sample",
            Op::LoraApply { .. } => "LoraApply",
            Op::Kron => "Kron",
            Op::MatrixInverse => "MatrixInverse",
            Op::FusedNormMatmul { .. } => "FusedNormMatmul",
            Op::FusedSkipNorm { .. } => "FusedSkipNorm",
            Op::FusedSwiGlu => "FusedSwiGlu",
            Op::FlashAttention { .. } => "FlashAttention",
        }
    }

    /// True if this op carries state across forward calls (e.g. KV cache).
    pub fn is_stateful(&self) -> bool {
        matches!(self, Op::KvCache)
    }

    /// True if this op is a pure view / layout change — no compute, output
    /// data aliases input data. Graph passes can elide these at no cost.
    pub fn is_layout_only(&self) -> bool {
        matches!(
            self,
            Op::Reshape { .. }
                | Op::Transpose { .. }
                | Op::Permute { .. }
                | Op::Split { .. }
                | Op::Chunk { .. }
                | Op::PixelShuffle { .. }
                | Op::PixelUnshuffle { .. }
                | Op::Unpatchify
        )
    }
}
