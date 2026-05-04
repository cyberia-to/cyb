//! Execution backends.
//!
//! Trait + implementations: cpu (portable reference), wgpu (GPU),
//! honeycrisp (Apple Silicon turbo). Adding a backend = new submodule here.
//!
//! Spec: specs/execution.md, specs/architecture.md

pub mod cpu;
pub mod wgpu;

#[cfg(target_os = "macos")]
pub mod honeycrisp;

use crate::core::dtype::DType;
use crate::core::op::Op;
use crate::core::tensor::Tensor;
use thiserror::Error;

/// Three backends + cpu reference library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    /// Pure-Rust CPU reference. Always correct, slow.
    /// Not user-facing; used internally by wgpu+rs for op fallback.
    Cpu,
    /// Portable wgpu GPU + CPU fallback.
    WgpuRs,
    /// Apple Silicon turbo (Metal + ANE + AMX + NEON + unimem).
    Honeycrisp,
    /// Future: trident-compiled bytecode on deterministic VM.
    Nox,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendKind::Cpu => "cpu",
            BackendKind::WgpuRs => "wgpu+rs",
            BackendKind::Honeycrisp => "honeycrisp",
            BackendKind::Nox => "nox",
        }
    }
}

/// Errors surfaced by backends. Structured, actionable.
#[derive(Error, Debug)]
pub enum BackendError {
    #[error("backend {backend} does not support op {op} with dtype {input_dtype:?}")]
    UnsupportedOp {
        backend: &'static str,
        op: &'static str,
        input_dtype: DType,
    },

    #[error("shape mismatch in {op}: expected {expected:?}, got {got:?}")]
    ShapeMismatch {
        op: &'static str,
        expected: Vec<usize>,
        got: Vec<usize>,
    },

    #[error("invalid input to {op}: {reason}")]
    InvalidInput { op: &'static str, reason: String },

    #[error("out of memory on {backend}: requested {requested_bytes} bytes")]
    OutOfMemory {
        backend: &'static str,
        requested_bytes: usize,
    },

    #[error("backend {backend} init failed: {reason}")]
    BackendInit {
        backend: &'static str,
        reason: String,
    },

    #[error("dtype {dtype:?} not implemented in {backend} (blocker: {blocker})")]
    UnsupportedDtype {
        backend: &'static str,
        dtype: DType,
        blocker: &'static str,
    },

    #[error("context overflow: requested position {pos}, max {max}")]
    ContextOverflow { pos: usize, max: usize },

    #[error("NaN/Inf detected in {op} output (layer {layer}, position {pos})")]
    NonFiniteOutput {
        op: &'static str,
        layer: usize,
        pos: usize,
    },

    #[error("tensor: {0}")]
    Tensor(#[from] crate::core::tensor::TensorError),

    #[error("internal: {0}")]
    Internal(String),
}

/// What every backend must implement.
///
/// The CPU reference library implements ALL ops in f32.
/// GPU backends implement what they support; missing ops route to CPU.
pub trait Backend: Send + Sync {
    /// Identity for logging and dispatch.
    fn kind(&self) -> BackendKind;

    /// True if this backend has a native implementation of `op` for the
    /// given input tensor shapes/dtypes.
    ///
    /// If false, the dispatcher falls back to CPU reference.
    fn supports(&self, op: &Op, inputs: &[&Tensor]) -> bool;

    /// Execute one op. Inputs are guaranteed to be on this backend's memory
    /// (caller uploads first if needed).
    ///
    /// Returns output tensor(s), or a structured error.
    fn execute(&self, op: &Op, inputs: &[&Tensor]) -> Result<Vec<Tensor>, BackendError>;

    /// Upload host bytes to backend memory.
    fn upload(
        &self,
        bytes: &[u8],
        shape: Vec<usize>,
        dtype: DType,
    ) -> Result<Tensor, BackendError>;

    /// Move a host tensor to backend memory. Default implementation
    /// uploads raw bytes; backends may override for zero-copy paths.
    fn to_backend(&self, t: &Tensor) -> Result<Tensor, BackendError> {
        match &t.data {
            crate::core::tensor::TensorData::Host(bytes) => {
                self.upload(bytes.as_slice(), t.shape.clone(), t.dtype)
            }
            crate::core::tensor::TensorData::Backend(_) => Ok(t.clone()),
        }
    }

    /// Download tensor to host memory as F32.
    /// For quantized inputs, dequantizes to F32 on download.
    fn download_f32(&self, t: &Tensor) -> Result<Vec<f32>, BackendError>;

    /// True if this backend pre-uploads quant weight bytes during to_backend().
    /// Backends that override quant_matmul to use GPU-resident tensors return true.
    /// Default false: quant weight tensors stay host-resident.
    fn uploads_quant_weights(&self) -> bool { false }

    /// Fused dequantize+matmul for quantized weight matrices.
    ///
    /// `x` is f32 [..., K]. `w` is a weight Tensor with dtype Q4_K/Q6_K/etc
    /// and shape [N, K]. After `to_backend()`, `w` may be GPU-resident.
    ///
    /// Default: extracts host bytes from `w` and delegates to CPU kernel.
    /// GPU backends override to run on-device kernels using `w` directly.
    fn quant_matmul(&self, x: &Tensor, w: &Tensor) -> Result<Tensor, BackendError> {
        let bytes = w.as_host_bytes().ok_or_else(|| BackendError::Internal(
            "quant_matmul default: w is not host-resident (backend missing override)".into(),
        ))?;
        let n = w.shape[0];
        let k = w.shape[1];
        crate::backend::cpu::matmul_quant_f32(x, bytes, w.dtype, n, k)
    }

    /// Batched fused dequant+matmul: y_i = x @ w_i^T for each w in `ws`.
    ///
    /// All matmuls share the same `x`. GPU backends should encode every
    /// dispatch into a single command buffer with one wait — saves the
    /// fixed per-op submit overhead. Default: iterate `quant_matmul`.
    fn quant_matmul_multi(
        &self,
        x: &Tensor,
        ws: &[&Tensor],
    ) -> Result<Vec<Tensor>, BackendError> {
        ws.iter().map(|w| self.quant_matmul(x, w)).collect()
    }

    /// Batched RmsNorm: encode several independent norms into one command buffer.
    /// Default: iterate.
    fn rms_norm_multi(
        &self,
        pairs: &[(&Tensor, &Tensor)],
        eps: f32,
    ) -> Result<Vec<Tensor>, BackendError> {
        pairs.iter()
            .map(|(x, g)| self.execute(&crate::core::op::Op::RmsNorm { eps }, &[x, g]).map(|mut v| v.remove(0)))
            .collect()
    }

    /// Fused RmsNorm followed by N quant matmuls against the normalized output.
    /// GPU backends should encode all (norm + N matmuls) into one command buffer
    /// with a single wait. Default: norm op + iterate quant_matmul.
    fn fused_norm_quant_matmul_multi(
        &self,
        x: &Tensor,
        gamma: &Tensor,
        eps: f32,
        ws: &[&Tensor],
    ) -> Result<Vec<Tensor>, BackendError> {
        let normed = self.execute(&crate::core::op::Op::RmsNorm { eps }, &[x, gamma])?
            .remove(0);
        self.quant_matmul_multi(&normed, ws)
    }

    /// Fused: RmsNorm(hidden) → q_proj/k_proj/v_proj → qk_norm(Q)/qk_norm(K).
    /// Returns (q_norm, k_norm, v) in that order — all in ONE GPU command buffer.
    /// Default: separate per-op call chain.
    fn fused_norm_qkv_qknorm(
        &self,
        hidden: &Tensor,
        input_norm_gamma: &Tensor,
        q_proj_w: &Tensor,
        k_proj_w: &Tensor,
        v_proj_w: &Tensor,
        q_norm_gamma: &Tensor,
        k_norm_gamma: &Tensor,
        eps: f32,
        num_q_heads: usize,
        num_k_heads: usize,
        head_dim: usize,
    ) -> Result<(Tensor, Tensor, Tensor), BackendError> {
        let qkv = self.fused_norm_quant_matmul_multi(
            hidden, input_norm_gamma, eps, &[q_proj_w, k_proj_w, v_proj_w],
        )?;
        let mut it = qkv.into_iter();
        let q = it.next().unwrap();
        let k = it.next().unwrap();
        let v = it.next().unwrap();
        // Reshape q/k for per-head norm. For host tensors this is metadata-only.
        let q_reshaped = Tensor { shape: vec![num_q_heads, head_dim], dtype: q.dtype, data: q.data };
        let k_reshaped = Tensor { shape: vec![num_k_heads, head_dim], dtype: k.dtype, data: k.data };
        let normed = self.rms_norm_multi(
            &[(&q_reshaped, q_norm_gamma), (&k_reshaped, k_norm_gamma)],
            eps,
        )?;
        let mut nit = normed.into_iter();
        let q_n = nit.next().unwrap();
        let k_n = nit.next().unwrap();
        // Reshape back
        let q_flat = Tensor { shape: vec![1, num_q_heads * head_dim], dtype: q_n.dtype, data: q_n.data };
        let k_flat = Tensor { shape: vec![1, num_k_heads * head_dim], dtype: k_n.dtype, data: k_n.data };
        Ok((q_flat, k_flat, v))
    }

    /// Fused FFN: post_norm(hidden) → gate, up → silu(gate)*up → down_proj.
    /// Single GPU command buffer, single wait. Default: per-op on CPU.
    fn fused_norm_swiglu_down(
        &self,
        hidden: &Tensor,
        post_norm_gamma: &Tensor,
        gate_w: &Tensor,
        up_w: &Tensor,
        down_w: &Tensor,
        eps: f32,
    ) -> Result<Tensor, BackendError> {
        let gate_up = self.fused_norm_quant_matmul_multi(
            hidden, post_norm_gamma, eps, &[gate_w, up_w],
        )?;
        let mut it = gate_up.into_iter();
        let gate = it.next().unwrap();
        let up = it.next().unwrap();
        let mid = self.silu_mul(&gate, &up)?;
        self.quant_matmul(&mid, down_w)
    }

    /// Fused SiLU(gate) * up. Default: split into two ops on CPU.
    /// GPU backends override with a single kernel — half the memory bandwidth
    /// and a single dispatch instead of two.
    fn silu_mul(&self, gate: &Tensor, up: &Tensor) -> Result<Tensor, BackendError> {
        // Default falls back to host f32 path.
        let g = if let Some(b) = gate.as_host_bytes() {
            bytemuck::cast_slice::<u8, f32>(b).to_vec()
        } else {
            self.download_f32(gate)?
        };
        let u = if let Some(b) = up.as_host_bytes() {
            bytemuck::cast_slice::<u8, f32>(b).to_vec()
        } else {
            self.download_f32(up)?
        };
        let out: Vec<f32> = g.iter().zip(u.iter())
            .map(|(g, u)| (g / (1.0 + (-g).exp())) * u)
            .collect();
        Ok(Tensor::from_f32(gate.shape.clone(), out))
    }
}
