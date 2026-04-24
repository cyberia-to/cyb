//! CPU reference library — every op in pure Rust f32.
//!
//! This is the correctness authority. GPU backends are verified against
//! these outputs. The library is also the fallback inside wgpu+rs for
//! ops the GPU cannot dispatch.
//!
//! Spec: specs/ops.md

use crate::backend::{Backend, BackendError, BackendKind};
use crate::core::dtype::DType;
use crate::core::op::Op;
use crate::core::tensor::{Tensor, TensorData};
use std::sync::Arc;

mod matmul;
mod rmsnorm;
mod rope;
mod softmax;
mod activation;
pub mod quant;
pub mod quant_matmul;

pub use matmul::matmul_f32;
pub use rmsnorm::rms_norm_f32;
pub use rope::rope_f32;
pub use softmax::softmax_f32;
pub use activation::{silu_f32, gelu_erf_f32, gelu_tanh_f32, swiglu_f32};
pub use quant::dequantize_to_f32;
pub use quant_matmul::matmul_quant_f32;

/// CPU reference backend — implements every op correctly in f32.
pub struct CpuBackend;

impl CpuBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for CpuBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Cpu
    }

    fn supports(&self, _op: &Op, _inputs: &[&Tensor]) -> bool {
        // CPU supports every op by design (v1 scope).
        // If an op is not yet implemented, execute() returns UnsupportedOp.
        true
    }

    fn execute(&self, op: &Op, inputs: &[&Tensor]) -> Result<Vec<Tensor>, BackendError> {
        match op {
            Op::RmsNorm { eps } => {
                let [x, g] = require(inputs, 2, op)?;
                let out = rms_norm_f32(x, g, *eps)?;
                Ok(vec![out])
            }
            Op::Matmul => {
                let [x, w] = require(inputs, 2, op)?;
                let out = matmul_f32(x, w)?;
                Ok(vec![out])
            }
            Op::Rope { head_dim, rope_dim, base } => {
                // x, pos_ids
                let [x, pos] = require(inputs, 2, op)?;
                let out = rope_f32(x, pos, *head_dim as usize, *rope_dim as usize, *base)?;
                Ok(vec![out])
            }
            Op::Softmax { dim } => {
                let [x] = require(inputs, 1, op)?;
                let out = softmax_f32(x, *dim)?;
                Ok(vec![out])
            }
            Op::Silu => {
                let [x] = require(inputs, 1, op)?;
                let out = silu_f32(x)?;
                Ok(vec![out])
            }
            Op::Gelu { approximate } => {
                let [x] = require(inputs, 1, op)?;
                let out = if *approximate { gelu_tanh_f32(x)? } else { gelu_erf_f32(x)? };
                Ok(vec![out])
            }
            Op::Add => {
                let [a, b] = require(inputs, 2, op)?;
                let out = binary_broadcast(a, b, |x, y| x + y)?;
                Ok(vec![out])
            }
            Op::Mul => {
                let [a, b] = require(inputs, 2, op)?;
                let out = binary_broadcast(a, b, |x, y| x * y)?;
                Ok(vec![out])
            }
            Op::Sub => {
                let [a, b] = require(inputs, 2, op)?;
                let out = binary_broadcast(a, b, |x, y| x - y)?;
                Ok(vec![out])
            }
            Op::SwiGlu => {
                // inputs: x, W_gate, W_up, W_down
                let [x, wg, wu, wd] = require(inputs, 4, op)?;
                let out = swiglu_f32(x, wg, wu, wd)?;
                Ok(vec![out])
            }
            Op::Reshape { shape } => {
                let [x] = require(inputs, 1, op)?;
                let x_numel = x.numel();
                let mut out_shape: Vec<usize> = Vec::with_capacity(shape.len());
                let mut infer_idx: Option<usize> = None;
                let mut product = 1usize;
                for (i, &d) in shape.iter().enumerate() {
                    if d == -1 {
                        if infer_idx.is_some() {
                            return Err(BackendError::InvalidInput {
                                op: "Reshape",
                                reason: "at most one -1 dim allowed".into(),
                            });
                        }
                        infer_idx = Some(i);
                        out_shape.push(0);
                    } else {
                        out_shape.push(d as usize);
                        product *= d as usize;
                    }
                }
                if let Some(idx) = infer_idx {
                    if product == 0 || x_numel % product != 0 {
                        return Err(BackendError::InvalidInput {
                            op: "Reshape",
                            reason: format!("cannot infer dim: {x_numel} / {product} has remainder"),
                        });
                    }
                    out_shape[idx] = x_numel / product;
                } else if out_shape.iter().product::<usize>() != x_numel {
                    return Err(BackendError::InvalidInput {
                        op: "Reshape",
                        reason: format!(
                            "element count mismatch: shape {:?} = {} ≠ input {}",
                            out_shape,
                            out_shape.iter().product::<usize>(),
                            x_numel
                        ),
                    });
                }
                Ok(vec![Tensor { shape: out_shape, dtype: x.dtype, data: x.data.clone() }])
            }
            Op::TokenEmbed => {
                // inputs[0]: token ids as f32 [seq] (values are token IDs cast to f32)
                // inputs[1]: embed weight f32 [vocab_size, hidden]
                let [ids, w] = require(inputs, 2, op)?;
                if w.rank() != 2 {
                    return Err(BackendError::InvalidInput {
                        op: "TokenEmbed",
                        reason: format!("weight must be rank 2, got {:?}", w.shape),
                    });
                }
                let hidden = w.shape[1];
                let id_data = ids.as_f32();
                let w_data = w.as_f32();
                let seq = id_data.len();
                let mut out = vec![0f32; seq * hidden];
                for (s, &tok_f) in id_data.iter().enumerate() {
                    let tok = tok_f as usize;
                    let src = &w_data[tok * hidden..(tok + 1) * hidden];
                    out[s * hidden..(s + 1) * hidden].copy_from_slice(src);
                }
                Ok(vec![Tensor::from_f32(vec![seq, hidden], out)])
            }
            Op::Sdpa { num_heads, kv_heads, head_dim, .. } => {
                // Q: [num_heads, head_dim]
                // K: [total_seq, kv_heads * head_dim]
                // V: [total_seq, kv_heads * head_dim]
                // Output: [num_heads, head_dim]
                let [q, k, v] = require(inputs, 3, op)?;
                let nh = *num_heads as usize;
                let kvh = *kv_heads as usize;
                let hd = *head_dim as usize;
                let groups = if kvh == 0 { 1 } else { nh / kvh };
                let kv_flat = kvh * hd;
                let total_seq = if k.shape.len() >= 1 { k.shape[0] } else { 1 };
                let scale = (hd as f32).sqrt().recip();

                let q_data = q.as_f32();
                let k_data = k.as_f32();
                let v_data = v.as_f32();
                let mut out = vec![0f32; nh * hd];

                for h in 0..nh {
                    let kv_head = h / groups;
                    let q_h = &q_data[h * hd..(h + 1) * hd];

                    let mut scores: Vec<f32> = (0..total_seq)
                        .map(|t| {
                            let k_off = t * kv_flat + kv_head * hd;
                            let k_slice = &k_data[k_off..k_off + hd];
                            q_h.iter().zip(k_slice).map(|(a, b)| a * b).sum::<f32>() * scale
                        })
                        .collect();

                    let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let mut sum = 0f32;
                    for s in scores.iter_mut() { *s = (*s - max).exp(); sum += *s; }
                    if sum > 0.0 { for s in scores.iter_mut() { *s /= sum; } }

                    let out_h = &mut out[h * hd..(h + 1) * hd];
                    for t in 0..total_seq {
                        let v_off = t * kv_flat + kv_head * hd;
                        let v_slice = &v_data[v_off..v_off + hd];
                        for d in 0..hd { out_h[d] += scores[t] * v_slice[d]; }
                    }
                }

                Ok(vec![Tensor::from_f32(vec![nh, hd], out)])
            }
            other => Err(BackendError::UnsupportedOp {
                backend: "cpu",
                op: other.name(),
                input_dtype: inputs.first().map(|t| t.dtype).unwrap_or(DType::F32),
            }),
        }
    }

    fn upload(
        &self,
        bytes: &[u8],
        shape: Vec<usize>,
        dtype: DType,
    ) -> Result<Tensor, BackendError> {
        // CPU "upload" = zero-copy host Tensor construction.
        Ok(Tensor {
            shape,
            dtype,
            data: TensorData::Host(Arc::new(bytes.to_vec())),
        })
    }

    fn download_f32(&self, t: &Tensor) -> Result<Vec<f32>, BackendError> {
        if t.dtype == DType::F32 {
            return Ok(t.to_f32_vec());
        }
        let bytes = t.as_host_bytes().ok_or_else(|| {
            BackendError::Internal("cpu download_f32: backend-resident tensor".into())
        })?;
        Ok(quant::dequantize_to_f32(bytes, t.dtype))
    }
}

/// Helper: require exactly N inputs, return fixed-size array.
fn require<'a, const N: usize>(
    inputs: &[&'a Tensor],
    n: usize,
    op: &Op,
) -> Result<[&'a Tensor; N], BackendError> {
    assert_eq!(N, n, "require<N>: n must match N");
    if inputs.len() != N {
        return Err(BackendError::InvalidInput {
            op: op.name(),
            reason: format!("expected {N} inputs, got {}", inputs.len()),
        });
    }
    let arr: [&'a Tensor; N] = std::array::from_fn(|i| inputs[i]);
    Ok(arr)
}

/// Element-wise binary op with NumPy broadcasting rules.
fn binary_broadcast(a: &Tensor, b: &Tensor, f: impl Fn(f32, f32) -> f32) -> Result<Tensor, BackendError> {
    let (out_shape, a_strides, b_strides) = broadcast_shapes(&a.shape, &b.shape)
        .ok_or_else(|| BackendError::ShapeMismatch {
            op: "binary_broadcast",
            expected: a.shape.clone(),
            got: b.shape.clone(),
        })?;
    let n: usize = out_shape.iter().product();
    let a_data = a.as_f32();
    let b_data = b.as_f32();
    let mut out = vec![0f32; n];
    for idx in 0..n {
        let ai = strided_index(idx, &out_shape, &a_strides);
        let bi = strided_index(idx, &out_shape, &b_strides);
        out[idx] = f(a_data[ai], b_data[bi]);
    }
    Ok(Tensor::from_f32(out_shape, out))
}

/// Compute broadcast output shape and per-input strides (0 where a dim broadcasts).
fn broadcast_shapes(a: &[usize], b: &[usize]) -> Option<(Vec<usize>, Vec<usize>, Vec<usize>)> {
    let rank = a.len().max(b.len());
    let a_pad: Vec<usize> = std::iter::repeat(1).take(rank - a.len()).chain(a.iter().copied()).collect();
    let b_pad: Vec<usize> = std::iter::repeat(1).take(rank - b.len()).chain(b.iter().copied()).collect();
    let mut out_shape = Vec::with_capacity(rank);
    for i in 0..rank {
        if a_pad[i] == b_pad[i] {
            out_shape.push(a_pad[i]);
        } else if a_pad[i] == 1 {
            out_shape.push(b_pad[i]);
        } else if b_pad[i] == 1 {
            out_shape.push(a_pad[i]);
        } else {
            return None;
        }
    }
    // Strides for broadcasting: 0 where input dim == 1, else normal stride.
    let real_strides = |shape: &[usize]| -> Vec<usize> {
        let mut s = vec![0usize; rank];
        let mut stride = 1usize;
        for i in (0..rank).rev() {
            let dim = if i < rank - shape.len() { 1 } else { shape[i - (rank - shape.len())] };
            s[i] = if dim == 1 { 0 } else { stride };
            stride *= dim;
        }
        s
    };
    Some((out_shape, real_strides(a), real_strides(b)))
}

/// Map a flat output index to the input's strided address.
fn strided_index(flat: usize, out_shape: &[usize], strides: &[usize]) -> usize {
    let mut rem = flat;
    let mut addr = 0usize;
    for i in 0..out_shape.len() {
        let div: usize = out_shape[i + 1..].iter().product();
        let coord = rem / div;
        rem %= div;
        addr += coord * strides[i];
    }
    addr
}
