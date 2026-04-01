//! Operation types for the Graph IR — ~48 jets from spec
//!
//! Each op is a recognized composition of atoms that maps to a fused GPU kernel.
//! Grouped by category as defined in the llm spec.

use super::dtype::DType;

/// Pooling mode for pool ops
#[derive(Clone, Debug, PartialEq)]
pub enum PoolMode {
    Max,
    Avg,
}

/// Interpolation mode for spatial upsampling
#[derive(Clone, Debug, PartialEq)]
pub enum InterpolateMode {
    Nearest,
    Bilinear,
    Area,
}

/// Sampling method for text generation
#[derive(Clone, Debug, PartialEq)]
pub enum SampleMethod {
    Greedy,
    TopP(f32),
    TopK(u32),
    TopKTopP(u32, f32),
}

/// Operations that can appear in the computation graph.
///
/// ~48 jets: fused GPU kernels replacing recognized compositions of atoms.
/// Validated against ComfyUI source — covers SD, SDXL, Flux, SD3, Wan2.2,
/// ESRGAN, whisper, YOLO, TTS, BitNet.
#[derive(Clone, Debug)]
pub enum Op {
    // ==== Core linear algebra ====
    /// Matrix multiply — 60% of all compute. Variants: f16, q8, q4, ternary (BitNet)
    Matmul,
    /// Elementwise add (supports broadcasting for adaLN modulation)
    Add,
    /// Elementwise multiply (supports broadcasting)
    Mul,
    /// Elementwise subtract
    Sub,
    /// Elementwise divide
    Div,
    /// Transpose with permutation
    Transpose { perm: Vec<usize> },
    /// Reshape tensor to new shape (-1 for inferred dim)
    Reshape { shape: Vec<i64> },
    /// Permute dimensions
    Permute { dims: Vec<usize> },
    /// Concatenate along axis
    Concat { axis: usize },
    /// Split along axis into chunks of given sizes
    Split { axis: usize, sizes: Vec<usize> },
    /// Split into equal chunks along axis
    Chunk { axis: usize, chunks: usize },
    /// Clamp values to [min, max] — numerical stability for fp16/fp8
    Clamp { min: Option<f32>, max: Option<f32> },
    /// Replace NaN/Inf — numerical stability
    NanToNum { nan: f32, posinf: f32, neginf: f32 },

    // ==== Attention ====
    /// Scaled dot-product attention + flash attention path
    Sdpa { num_heads: u32, kv_heads: u32, head_dim: u32, causal: bool },
    /// Cross-attention (whisper decoder, diffusion, IP-adapter)
    SdpaCross { num_heads: u32, head_dim: u32 },
    /// Shifted window attention (SwinIR)
    SdpaWindow { num_heads: u32, head_dim: u32, window_size: u32 },
    /// KV cache — append/lookup, memory lifecycle
    KvCache,
    /// TurboQuant KV compress: PolarQuant rotation + angle quantization (values),
    /// QJL sign-bit projection (keys). Runs after K/V projection, before cache store.
    KvCompress { head_dim: u32, bits: u32 },
    /// TurboQuant KV decompress: angle dequant + inverse rotation (values),
    /// QJL inner product estimation (keys). Runs at attention score computation.
    KvDecompress { head_dim: u32, bits: u32 },
    /// Rotary position embedding (1D for LLMs, multi-axis for DiT/video)
    Rope { head_dim: u32, base: f32 },
    /// Sinusoidal timestep embedding for diffusion models
    SinusoidalEmbed { dim: u32 },
    /// Learned relative position bias table (T5, SwinIR)
    RelativePosEmbedding { num_buckets: u32 },

    // ==== Normalization ====
    /// RMS normalization — llama, qwen, mistral, T5, Flux QKNorm
    RmsNorm { eps: f32 },
    /// Layer normalization — BERT, DeBERTa, whisper, CLIP, SwinIR
    LayerNorm { eps: f32 },
    /// Batch normalization — YOLO
    BatchNorm { eps: f32, momentum: f32 },
    /// Group normalization — diffusion UNet, VAE (groups=32)
    GroupNorm { num_groups: u32, eps: f32 },
    /// Instance normalization — ACE Step audio
    InstanceNorm { eps: f32 },
    /// Adaptive layer norm: shift + x * (1 + scale) — all DiT-family
    AdaLN,

    // ==== Activation ====
    /// SiLU — llama, qwen, UNet ResBlocks
    Silu,
    /// GELU — BERT, GPT, SwinIR. Variants: standard, tanh-approximate
    Gelu { approximate: bool },
    /// Gated GELU feedforward: gelu(gate) * x
    GeGlu,
    /// Gated SiLU feedforward: w2(silu(w1(x)) * w3(x))
    SwiGlu,
    /// Gated linear unit with sigmoid (Stable Audio Conformer)
    Glu,
    /// ReLU — YOLO, classic CNNs
    Relu,
    /// Leaky ReLU — ESRGAN, upscalers (slope=0.2)
    LeakyRelu { slope: f32 },
    /// Parametric ReLU (some upscaler variants)
    PRelu,
    /// Sigmoid activation
    Sigmoid,
    /// Tanh activation
    Tanh,
    /// Softmax along dimension
    Softmax { dim: i32 },

    // ==== Convolution ====
    /// 1D convolution — TTS, audio models
    Conv1d { kernel: u32, stride: u32, padding: u32, groups: u32 },
    /// 2D convolution — YOLO, UNet, VAE, ControlNet, upscalers
    Conv2d { kernel: (u32, u32), stride: (u32, u32), padding: (u32, u32), groups: u32 },
    /// 3D convolution — video models (SVD, Wan2.2, HunyuanVideo VAE)
    Conv3d { kernel: (u32, u32, u32), stride: (u32, u32, u32), padding: (u32, u32, u32), groups: u32 },
    /// Transposed 2D convolution — learned upsampling
    ConvTranspose2d { kernel: (u32, u32), stride: (u32, u32), padding: (u32, u32) },
    /// Causal 1D convolution with replicate padding (Wan video)
    CausalConv1d { kernel: u32 },
    /// Depthwise convolution — groups=dim, audio Conformer
    DepthwiseConv { kernel: u32, stride: u32 },
    /// Pooling — max, avg (2d)
    Pool { mode: PoolMode, kernel: (u32, u32) },

    // ==== Spatial ops ====
    /// Nearest/bilinear/area upsampling — UNet, VAE, resolution matching
    Interpolate { mode: InterpolateMode, scale: f32 },
    /// Sub-pixel convolution for upscaling: (C*r^2, H, W) -> (C, H*r, W*r)
    PixelShuffle { upscale_factor: u32 },
    /// Inverse pixel shuffle: (C, H, W) -> (C*r^2, H/r, W/r)
    PixelUnshuffle { downscale_factor: u32 },
    /// Patch embedding — Conv with kernel=stride=patch_size. All DiT models
    PatchEmbed { patch_size: u32 },
    /// Reconstruct spatial tensor from patch sequence
    Unpatchify,

    // ==== Embedding ====
    /// Token embedding — lookup table
    TokenEmbed,
    /// Position embedding — learned or sinusoidal
    PosEmbed,

    // ==== Special ====
    /// Diffusion noise schedule (cosine, linear, flow-matching sigma)
    NoiseSchedule,
    /// Normalizing flow step (VITS/TTS)
    FlowStep,
    /// Runtime quantization to Q4/Q8
    Quantize { dtype: DType },
    /// Runtime dequantization
    Dequantize,
    /// Sampling — top-p, top-k, temperature, grammar-constrained
    Sample { method: SampleMethod },

    // ==== Adapter ops (LoRA runtime) ====
    /// LoRA: up @ down with alpha/rank scaling, applied lazily
    LoraApply { rank: u32, alpha: f32 },
    /// Kronecker product for LoKr adapter
    Kron,
    /// Matrix inverse — Cayley transform for OFT adapter: R = (I+Q)(I-Q)^-1
    MatrixInverse,

    // ==== Fused ops (recognized compositions) ====
    /// Fused RmsNorm + Matmul — single kernel
    FusedNormMatmul { eps: f32 },
    /// Fused Add + RmsNorm — skip connection + norm
    FusedSkipNorm { eps: f32 },
    /// Fused gate + up + SiLU + mul
    FusedSwiGlu,
    /// Flash attention — fused SDPA with tiling
    FlashAttention { num_heads: u32, kv_heads: u32, head_dim: u32 },

    // ==== Legacy / compatibility ====
    /// Argmax (for greedy decoding before Sample was added)
    Argmax,
}

impl Op {
    /// Human-readable name for logging
    pub fn name(&self) -> &'static str {
        match self {
            // Core linear algebra
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

            // Attention
            Op::Sdpa { .. } => "Sdpa",
            Op::SdpaCross { .. } => "SdpaCross",
            Op::SdpaWindow { .. } => "SdpaWindow",
            Op::KvCache => "KvCache",
            Op::Rope { .. } => "Rope",
            Op::SinusoidalEmbed { .. } => "SinusoidalEmbed",
            Op::RelativePosEmbedding { .. } => "RelativePosEmbedding",

            // Normalization
            Op::RmsNorm { .. } => "RmsNorm",
            Op::LayerNorm { .. } => "LayerNorm",
            Op::BatchNorm { .. } => "BatchNorm",
            Op::GroupNorm { .. } => "GroupNorm",
            Op::InstanceNorm { .. } => "InstanceNorm",
            Op::AdaLN => "AdaLN",

            // Activation
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

            // Convolution
            Op::Conv1d { .. } => "Conv1d",
            Op::Conv2d { .. } => "Conv2d",
            Op::Conv3d { .. } => "Conv3d",
            Op::ConvTranspose2d { .. } => "ConvTranspose2d",
            Op::CausalConv1d { .. } => "CausalConv1d",
            Op::DepthwiseConv { .. } => "DepthwiseConv",
            Op::Pool { .. } => "Pool",

            // Spatial
            Op::Interpolate { .. } => "Interpolate",
            Op::PixelShuffle { .. } => "PixelShuffle",
            Op::PixelUnshuffle { .. } => "PixelUnshuffle",
            Op::PatchEmbed { .. } => "PatchEmbed",
            Op::Unpatchify => "Unpatchify",

            // Embedding
            Op::TokenEmbed => "TokenEmbed",
            Op::PosEmbed => "PosEmbed",

            // Special
            Op::NoiseSchedule => "NoiseSchedule",
            Op::FlowStep => "FlowStep",
            Op::Quantize { .. } => "Quantize",
            Op::Dequantize => "Dequantize",
            Op::Sample { .. } => "Sample",

            // Adapter
            Op::LoraApply { .. } => "LoraApply",
            Op::Kron => "Kron",
            Op::MatrixInverse => "MatrixInverse",

            // Fused
            Op::FusedNormMatmul { .. } => "FusedNormMatmul",
            Op::FusedSkipNorm { .. } => "FusedSkipNorm",
            Op::FusedSwiGlu => "FusedSwiGlu",
            Op::FlashAttention { .. } => "FlashAttention",

            // Legacy
            Op::Argmax => "Argmax",
            Op::KvCompress { .. } => "KvCompress",
            Op::KvDecompress { .. } => "KvDecompress",
        }
    }

    /// Whether this op carries state between inference calls
    pub fn is_stateful(&self) -> bool {
        matches!(self, Op::KvCache)
    }

    /// Whether this op is a pure memory layout change (zero compute)
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
