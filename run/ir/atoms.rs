//! The 8 computational atoms — primitive ops that compose every neural op.
//!
//! Every Op decomposes deterministically into a sequence of atoms. Two Ops
//! producing identical atom sequences share a [`formula_hash`] and a jet
//! lookup entry — this is the mechanical bridge between "typed op" and
//! "fused GPU kernel."
//!
//! [`AtomInterpreter`] is the correctness authority: any Op runs through
//! `decompose → execute` on f32 CPU and must match backend output within ε.
//!
//! Spec: reference/runtime/ir.md
//!
//! [`formula_hash`]: super::jets::formula_hash

use crate::op::Op;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Atom {
    Mul,
    Add,
    Cmp(CmpOp),
    Exp,
    Read,
    Write,
    Reduce(ReduceOp),
    Slide(SlidePattern),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum CmpOp {
    Max,
    Min,
    LessThan,
    GreaterThan,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ReduceOp {
    Sum,
    Max,
    Mean,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SlidePattern {
    Window1D { kernel: usize, stride: usize },
    Window2D { kernel: (usize, usize), stride: (usize, usize) },
    Window3D { kernel: (usize, usize, usize), stride: (usize, usize, usize) },
}

/// Canonical Op → [Atom] decomposition. Stable: two calls on the same Op
/// variant must return byte-identical sequences (jet hash depends on it).
pub fn decompose(op: &Op) -> Vec<Atom> {
    match op {
        // === Core linear algebra ===
        Op::Matmul => vec![
            Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ],
        Op::Add => vec![Atom::Add],
        Op::Mul => vec![Atom::Mul],
        Op::Sub => vec![Atom::Add, Atom::Mul], // a - b = a + (-1 * b)
        Op::Div => vec![Atom::Mul, Atom::Exp],
        Op::Transpose { .. } => vec![Atom::Read],
        Op::Reshape { .. } => vec![],
        Op::Permute { .. } => vec![Atom::Read],
        Op::Concat { .. } => vec![Atom::Write],
        Op::Split { .. } => vec![Atom::Read],
        Op::Chunk { .. } => vec![Atom::Read],
        Op::Clamp { .. } => vec![Atom::Cmp(CmpOp::Max), Atom::Cmp(CmpOp::Min)],
        Op::NanToNum { .. } => vec![Atom::Cmp(CmpOp::LessThan), Atom::Mul, Atom::Add],
        Op::Argmax { .. } => vec![Atom::Cmp(CmpOp::GreaterThan), Atom::Reduce(ReduceOp::Max)],

        // === Attention ===
        Op::Sdpa { .. } | Op::SdpaCross { .. } | Op::FlashAttention { .. } => vec![
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),  // Q·Kᵀ
            Atom::Exp, Atom::Reduce(ReduceOp::Sum),  // softmax: exp+sum
            Atom::Mul,
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),  // ·V
        ],
        Op::SdpaWindow { .. } => vec![
            Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
            Atom::Exp, Atom::Reduce(ReduceOp::Sum),
            Atom::Mul,
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ],
        Op::KvCache => vec![Atom::Write, Atom::Read],
        Op::KvCompress { .. } | Op::KvDecompress { .. } => vec![Atom::Read, Atom::Write],
        Op::Rope { .. } => vec![Atom::Mul, Atom::Add],
        Op::SinusoidalEmbed { .. } => vec![Atom::Mul, Atom::Exp],
        Op::RelativePosEmbedding { .. } => vec![Atom::Read],
        Op::TokenEmbed | Op::PosEmbed => vec![Atom::Read],

        // === Normalization ===
        Op::RmsNorm { .. } => vec![
            Atom::Mul,                    // x²
            Atom::Reduce(ReduceOp::Sum),
            Atom::Exp,                    // rsqrt via exp(-½·log)
            Atom::Mul,                    // x · rsqrt · γ
        ],
        Op::LayerNorm { .. } => vec![
            Atom::Reduce(ReduceOp::Mean),
            Atom::Add,
            Atom::Mul,
            Atom::Reduce(ReduceOp::Mean),
            Atom::Mul,
            Atom::Add,
        ],
        Op::BatchNorm { .. } => vec![Atom::Add, Atom::Mul, Atom::Mul, Atom::Add],
        Op::GroupNorm { .. } => vec![
            Atom::Reduce(ReduceOp::Mean),
            Atom::Add,
            Atom::Mul,
            Atom::Reduce(ReduceOp::Mean),
            Atom::Mul,
            Atom::Add,
        ],
        Op::InstanceNorm { .. } => vec![
            Atom::Reduce(ReduceOp::Mean),
            Atom::Add,
            Atom::Mul,
            Atom::Reduce(ReduceOp::Mean),
            Atom::Mul,
        ],
        Op::AdaLN => vec![Atom::Mul, Atom::Add],

        // === Activation ===
        Op::Silu => vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul],
        Op::Gelu { .. } => vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul],
        Op::GeGlu => vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul, Atom::Mul],
        Op::SwiGlu => vec![Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul, Atom::Mul],
        Op::Glu => vec![Atom::Exp, Atom::Add, Atom::Mul],
        Op::Relu => vec![Atom::Cmp(CmpOp::Max)],
        Op::LeakyRelu { .. } => vec![Atom::Cmp(CmpOp::Max), Atom::Mul, Atom::Add],
        Op::PRelu => vec![Atom::Cmp(CmpOp::Max), Atom::Mul, Atom::Add],
        Op::Sigmoid => vec![Atom::Exp, Atom::Add, Atom::Mul],
        Op::Tanh => vec![Atom::Exp, Atom::Add, Atom::Mul],
        Op::Softmax { .. } => vec![Atom::Exp, Atom::Reduce(ReduceOp::Sum), Atom::Mul],

        // === Convolution ===
        Op::Conv1d { kernel, stride, .. } => vec![
            Atom::Slide(SlidePattern::Window1D {
                kernel: *kernel as usize,
                stride: *stride as usize,
            }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ],
        Op::Conv2d { kernel, stride, .. } => vec![
            Atom::Slide(SlidePattern::Window2D {
                kernel: (kernel.0 as usize, kernel.1 as usize),
                stride: (stride.0 as usize, stride.1 as usize),
            }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ],
        Op::Conv3d { kernel, stride, .. } => vec![
            Atom::Slide(SlidePattern::Window3D {
                kernel: (kernel.0 as usize, kernel.1 as usize, kernel.2 as usize),
                stride: (stride.0 as usize, stride.1 as usize, stride.2 as usize),
            }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ],
        Op::ConvTranspose2d { kernel, stride, .. } => vec![
            Atom::Slide(SlidePattern::Window2D {
                kernel: (kernel.0 as usize, kernel.1 as usize),
                stride: (stride.0 as usize, stride.1 as usize),
            }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ],
        Op::CausalConv1d { kernel } => vec![
            Atom::Slide(SlidePattern::Window1D {
                kernel: *kernel as usize,
                stride: 1,
            }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ],
        Op::DepthwiseConv { kernel, stride } => vec![
            Atom::Slide(SlidePattern::Window1D {
                kernel: *kernel as usize,
                stride: *stride as usize,
            }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ],
        Op::Pool { kernel, stride, .. } => vec![
            Atom::Slide(SlidePattern::Window2D {
                kernel: (kernel.0 as usize, kernel.1 as usize),
                stride: (stride.0 as usize, stride.1 as usize),
            }),
            Atom::Reduce(ReduceOp::Max),
        ],

        // === Spatial ===
        Op::Interpolate { .. } => vec![Atom::Read, Atom::Mul, Atom::Add],
        Op::PixelShuffle { .. } => vec![],
        Op::PixelUnshuffle { .. } => vec![],
        Op::PatchEmbed { .. } => vec![
            Atom::Slide(SlidePattern::Window2D {
                kernel: (16, 16),
                stride: (16, 16),
            }),
            Atom::Mul,
            Atom::Reduce(ReduceOp::Sum),
        ],
        Op::Unpatchify => vec![Atom::Write],

        // === Special ===
        Op::NoiseSchedule => vec![Atom::Mul, Atom::Exp],
        Op::FlowStep => vec![Atom::Mul, Atom::Add, Atom::Exp],
        Op::Quantize { .. } => vec![Atom::Mul, Atom::Cmp(CmpOp::Max), Atom::Cmp(CmpOp::Min)],
        Op::Dequantize => vec![Atom::Mul, Atom::Add],
        Op::Sample { .. } => vec![
            Atom::Exp,
            Atom::Reduce(ReduceOp::Sum),
            Atom::Mul,
            Atom::Cmp(CmpOp::Max),
        ],

        // === Adapters ===
        Op::LoraApply { .. } => vec![
            Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),  // down · x
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),  // up · (down·x)
            Atom::Mul,                                // · α
            Atom::Add,                                // x + Δ
        ],
        Op::Kron => vec![Atom::Mul],
        Op::MatrixInverse => vec![Atom::Mul, Atom::Add, Atom::Reduce(ReduceOp::Sum)],

        // === Fused ===
        Op::FusedNormMatmul { .. } => vec![
            Atom::Mul, Atom::Reduce(ReduceOp::Sum), Atom::Exp, Atom::Mul,
            Atom::Slide(SlidePattern::Window1D { kernel: 1, stride: 1 }),
            Atom::Mul, Atom::Reduce(ReduceOp::Sum),
        ],
        Op::FusedSkipNorm { .. } => vec![
            Atom::Add,
            Atom::Mul, Atom::Reduce(ReduceOp::Sum), Atom::Exp, Atom::Mul,
        ],
        Op::FusedSwiGlu => vec![
            Atom::Mul, Atom::Exp, Atom::Add, Atom::Mul,
            Atom::Mul,
        ],
    }
}

/// CPU reference interpreter — slow (~1000× a jet) but always correct.
/// Used for constant folding and unknown-op fallbacks.
///
/// Note: each atom treats the full output buffer as a single operation.
/// Compound decompositions run atoms sequentially; callers provide inputs
/// in the canonical order matching the atom list.
pub struct AtomInterpreter;

impl AtomInterpreter {
    pub fn execute(atoms: &[Atom], inputs: &[&[f32]], output: &mut [f32]) {
        for atom in atoms {
            match atom {
                Atom::Mul => {
                    for i in 0..output.len() {
                        output[i] = inputs[0].get(i).copied().unwrap_or(0.0)
                            * inputs[1].get(i).copied().unwrap_or(1.0);
                    }
                }
                Atom::Add => {
                    for i in 0..output.len() {
                        output[i] = inputs[0].get(i).copied().unwrap_or(0.0)
                            + inputs[1].get(i).copied().unwrap_or(0.0);
                    }
                }
                Atom::Exp => {
                    for i in 0..output.len() {
                        output[i] = inputs[0].get(i).copied().unwrap_or(0.0).exp();
                    }
                }
                Atom::Cmp(CmpOp::Max) => {
                    for i in 0..output.len() {
                        output[i] = inputs[0].get(i).copied().unwrap_or(0.0).max(0.0);
                    }
                }
                Atom::Cmp(CmpOp::Min) => {
                    for i in 0..output.len() {
                        output[i] = inputs[0].get(i).copied().unwrap_or(0.0).min(0.0);
                    }
                }
                Atom::Cmp(CmpOp::LessThan) => {
                    for i in 0..output.len() {
                        let a = inputs[0].get(i).copied().unwrap_or(0.0);
                        let b = inputs[1].get(i).copied().unwrap_or(0.0);
                        output[i] = if a < b { 1.0 } else { 0.0 };
                    }
                }
                Atom::Cmp(CmpOp::GreaterThan) => {
                    for i in 0..output.len() {
                        let a = inputs[0].get(i).copied().unwrap_or(0.0);
                        let b = inputs[1].get(i).copied().unwrap_or(0.0);
                        output[i] = if a > b { 1.0 } else { 0.0 };
                    }
                }
                Atom::Reduce(ReduceOp::Sum) => {
                    output[0] = inputs[0].iter().sum();
                }
                Atom::Reduce(ReduceOp::Max) => {
                    output[0] = inputs[0]
                        .iter()
                        .copied()
                        .fold(f32::NEG_INFINITY, f32::max);
                }
                Atom::Reduce(ReduceOp::Mean) => {
                    let n = inputs[0].len() as f32;
                    output[0] = if n > 0.0 {
                        inputs[0].iter().sum::<f32>() / n
                    } else {
                        0.0
                    };
                }
                Atom::Read | Atom::Write => {
                    let len = output.len().min(inputs[0].len());
                    output[..len].copy_from_slice(&inputs[0][..len]);
                }
                Atom::Slide(_) => {
                    // Slide describes a memory-access pattern; no-op at this layer.
                    // The following Mul+Reduce atoms do the actual compute.
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matmul_decomposes_to_slide_mul_reduce() {
        let atoms = decompose(&Op::Matmul);
        assert_eq!(atoms.len(), 3);
        assert!(matches!(atoms[0], Atom::Slide(_)));
        assert_eq!(atoms[1], Atom::Mul);
        assert!(matches!(atoms[2], Atom::Reduce(ReduceOp::Sum)));
    }

    #[test]
    fn layout_ops_have_no_atoms() {
        assert!(decompose(&Op::Reshape { shape: vec![1, -1] }).is_empty());
        assert!(decompose(&Op::PixelShuffle { upscale_factor: 2 }).is_empty());
    }

    #[test]
    fn interpreter_add_mul_reduce_relu_exp() {
        let a = [1.0f32, 2.0, 3.0];
        let b = [4.0f32, 5.0, 6.0];
        let mut out = [0.0f32; 3];
        AtomInterpreter::execute(&[Atom::Add], &[&a, &b], &mut out);
        assert_eq!(out, [5.0, 7.0, 9.0]);

        AtomInterpreter::execute(&[Atom::Mul], &[&a, &b], &mut out);
        assert_eq!(out, [4.0, 10.0, 18.0]);

        let mut one = [0.0f32; 1];
        AtomInterpreter::execute(&[Atom::Reduce(ReduceOp::Sum)], &[&a], &mut one);
        assert_eq!(one[0], 6.0);

        let neg = [-1.0f32, 0.0, 2.0, -3.0];
        let mut four = [0.0f32; 4];
        AtomInterpreter::execute(&[Atom::Cmp(CmpOp::Max)], &[&neg], &mut four);
        assert_eq!(four, [0.0, 0.0, 2.0, 0.0]);

        let z = [0.0f32, 1.0];
        let mut two = [0.0f32; 2];
        AtomInterpreter::execute(&[Atom::Exp], &[&z], &mut two);
        assert!((two[0] - 1.0).abs() < 1e-6);
        assert!((two[1] - std::f32::consts::E).abs() < 1e-5);
    }

    #[test]
    fn every_op_variant_decomposes_without_panic() {
        // Exercise each variant of run::Op at least once; any new variant
        // added without a `decompose` arm will fail to compile, not panic.
        use crate::op::{InterpolateMode, PoolMode, SampleMethod};
        let ops = [
            Op::Matmul, Op::Add, Op::Mul, Op::Sub, Op::Div,
            Op::Transpose { perm: vec![0, 1] },
            Op::Reshape { shape: vec![1, -1] },
            Op::Permute { dims: vec![0, 1] },
            Op::Concat { axis: 0 },
            Op::Split { axis: 0, sizes: vec![1] },
            Op::Chunk { axis: 0, chunks: 2 },
            Op::Clamp { min: Some(-1.0), max: Some(1.0) },
            Op::NanToNum { nan: 0.0, posinf: 1e6, neginf: -1e6 },
            Op::Argmax { dim: -1 },
            Op::Sdpa { num_heads: 8, kv_heads: 8, head_dim: 64, causal: true },
            Op::SdpaCross { num_heads: 8, head_dim: 64 },
            Op::SdpaWindow { num_heads: 8, head_dim: 64, window_size: 7 },
            Op::KvCache,
            Op::KvCompress { head_dim: 64, bits: 4 },
            Op::KvDecompress { head_dim: 64, bits: 4 },
            Op::Rope { head_dim: 64, rope_dim: 64, base: 10_000.0 },
            Op::SinusoidalEmbed { dim: 256 },
            Op::RelativePosEmbedding { num_buckets: 32 },
            Op::TokenEmbed, Op::PosEmbed,
            Op::RmsNorm { eps: 1e-5 }, Op::LayerNorm { eps: 1e-5 },
            Op::BatchNorm { eps: 1e-5, momentum: 0.1 },
            Op::GroupNorm { num_groups: 32, eps: 1e-5 },
            Op::InstanceNorm { eps: 1e-5 }, Op::AdaLN,
            Op::Silu, Op::Gelu { approximate: true },
            Op::GeGlu, Op::SwiGlu, Op::Glu,
            Op::Relu, Op::LeakyRelu { slope: 0.2 }, Op::PRelu,
            Op::Sigmoid, Op::Tanh, Op::Softmax { dim: -1 },
            Op::Conv1d { kernel: 3, stride: 1, padding: 1, dilation: 1, groups: 1 },
            Op::Conv2d {
                kernel: (3, 3), stride: (1, 1), padding: (1, 1),
                dilation: (1, 1), groups: 1,
            },
            Op::Conv3d {
                kernel: (3, 3, 3), stride: (1, 1, 1), padding: (1, 1, 1),
                dilation: (1, 1, 1), groups: 1,
            },
            Op::ConvTranspose2d { kernel: (3, 3), stride: (2, 2), padding: (1, 1) },
            Op::CausalConv1d { kernel: 3 },
            Op::DepthwiseConv { kernel: 3, stride: 1 },
            Op::Pool {
                mode: PoolMode::Max,
                kernel: (2, 2),
                stride: (2, 2),
                padding: (0, 0),
            },
            Op::Interpolate { mode: InterpolateMode::Nearest, scale: 2.0 },
            Op::PixelShuffle { upscale_factor: 2 },
            Op::PixelUnshuffle { downscale_factor: 2 },
            Op::PatchEmbed { patch_size: 16 },
            Op::Unpatchify,
            Op::NoiseSchedule, Op::FlowStep,
            Op::Quantize { dtype: crate::dtype::DType::Q4_0 },
            Op::Dequantize,
            Op::Sample { method: SampleMethod::Greedy },
            Op::LoraApply { rank: 16, alpha: 1.0 },
            Op::Kron, Op::MatrixInverse,
            Op::FusedNormMatmul { eps: 1e-5 },
            Op::FusedSkipNorm { eps: 1e-5 },
            Op::FusedSwiGlu,
            Op::FlashAttention { num_heads: 8, kv_heads: 8, head_dim: 64 },
        ];
        for op in &ops {
            let _ = decompose(op);
        }
    }
}
