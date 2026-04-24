//! wgpu+rs backend — portable GPU with CPU fallback.
//!
//! Design:
//! - Device + queue initialized once via [`Device::new`].
//! - Compute pipelines built lazily per shader, cached.
//! - Every op has a native WGSL kernel; ops not yet implemented fall
//!   back to the CPU reference.
//! - Tensors on GPU wrap `wgpu::Buffer`. Upload copies host bytes to
//!   a STORAGE buffer; download reads back to f32.

use crate::backend::{Backend, BackendError, BackendKind};
use crate::backend::cpu::CpuBackend;
use crate::dtype::DType;
use crate::op::Op;
use crate::tensor::{BackendData, Tensor, TensorData};
use std::any::Any;
use std::sync::Arc;

pub mod alloc;
mod device;
mod kernels;

pub use alloc::{dispatch_2d, FrameAllocator};
use device::Device;

/// GPU-resident buffer handle.
struct GpuBuffer {
    buffer: wgpu::Buffer,
    device: Arc<Device>,
}

impl BackendData for GpuBuffer {
    fn backend_name(&self) -> &'static str {
        "wgpu+rs"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct WgpuRsBackend {
    device: Arc<Device>,
    cpu: CpuBackend,
}

impl WgpuRsBackend {
    pub fn new() -> Result<Self, BackendError> {
        let device = Arc::new(Device::new()?);
        Ok(Self {
            device,
            cpu: CpuBackend::new(),
        })
    }

    fn to_gpu(&self, t: &Tensor) -> Result<wgpu::Buffer, BackendError> {
        let bytes = match &t.data {
            TensorData::Host(b) => b.as_slice(),
            TensorData::Backend(b) => {
                // Already on a backend — check if ours.
                if let Some(g) = b.as_any().downcast_ref::<GpuBuffer>() {
                    return Ok(g.buffer.clone());
                }
                return Err(BackendError::Internal(
                    "wgpu+rs: tensor on a different backend".into(),
                ));
            }
        };
        Ok(self.device.upload_bytes(bytes))
    }

    fn from_gpu_f32(&self, buffer: &wgpu::Buffer, shape: Vec<usize>) -> Tensor {
        let n: usize = shape.iter().product();
        let f32s = self.device.read_f32(buffer, n);
        Tensor::from_f32(shape, f32s)
    }

    fn download_tensor(&self, t: &Tensor) -> Result<Tensor, BackendError> {
        match &t.data {
            TensorData::Host(_) => Ok(t.clone()),
            TensorData::Backend(b) => {
                let g = b.as_any().downcast_ref::<GpuBuffer>().ok_or_else(|| {
                    BackendError::Internal("wgpu+rs: unknown backend tensor".into())
                })?;
                // Download F32 from GPU.
                if t.dtype != DType::F32 {
                    return Err(BackendError::Internal(
                        "wgpu+rs: download non-F32 GPU tensor not yet supported".into(),
                    ));
                }
                let f32s = self.device.read_f32(&g.buffer, t.numel());
                Ok(Tensor::from_f32(t.shape.clone(), f32s))
            }
        }
    }
}

/// Materialize backend-resident inputs to host tensors.
fn materialize_inputs<F>(inputs: &[&Tensor], mut f: F) -> Result<Vec<Tensor>, BackendError>
where
    F: FnMut(&Tensor) -> Result<Tensor, BackendError>,
{
    inputs.iter().map(|t| f(t)).collect()
}

impl Backend for WgpuRsBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::WgpuRs
    }

    fn supports(&self, op: &Op, inputs: &[&Tensor]) -> bool {
        // Currently: native kernels for hot F32 ops.
        match op {
            Op::Matmul | Op::RmsNorm { .. } | Op::Softmax { .. } | Op::Silu
                if inputs.iter().all(|t| t.dtype == DType::F32) =>
            {
                true
            }
            Op::Rope { .. } if inputs.iter().all(|t| t.dtype == DType::F32) => true,
            _ => false, // fall back to CPU for other ops and non-f32 dtypes
        }
    }

    fn execute(&self, op: &Op, inputs: &[&Tensor]) -> Result<Vec<Tensor>, BackendError> {
        if !self.supports(op, inputs) {
            // Materialize any backend-resident inputs to host, then run on CPU.
            let materialized = materialize_inputs(inputs, |t| self.download_tensor(t))?;
            let refs: Vec<&Tensor> = materialized.iter().collect();
            return self.cpu.execute(op, &refs);
        }

        match op {
            Op::Matmul => {
                if inputs.len() != 2 {
                    return Err(BackendError::InvalidInput {
                        op: "Matmul",
                        reason: format!("expected 2 inputs, got {}", inputs.len()),
                    });
                }
                let x = inputs[0];
                let w = inputs[1];
                if w.rank() != 2 {
                    return Err(BackendError::ShapeMismatch {
                        op: "Matmul",
                        expected: vec![0, 0],
                        got: w.shape.clone(),
                    });
                }
                if x.shape.last() != Some(&w.shape[1]) {
                    return Err(BackendError::ShapeMismatch {
                        op: "Matmul",
                        expected: vec![0, w.shape[1]],
                        got: x.shape.clone(),
                    });
                }
                let x_buf = self.to_gpu(x)?;
                let w_buf = self.to_gpu(w)?;
                let n = w.shape[0] as u32;
                let k = w.shape[1] as u32;
                let batch: usize = x.shape[..x.shape.len() - 1].iter().product();
                let out_buf = kernels::matmul::dispatch(
                    &self.device,
                    &x_buf,
                    &w_buf,
                    batch as u32,
                    n,
                    k,
                );
                let mut out_shape = x.shape.clone();
                *out_shape.last_mut().unwrap() = n as usize;
                Ok(vec![self.from_gpu_f32(&out_buf, out_shape)])
            }
            Op::RmsNorm { eps } => {
                let x = inputs[0];
                let g = inputs[1];
                let d = g.shape[0] as u32;
                let batch: u32 = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
                let x_buf = self.to_gpu(x)?;
                let g_buf = self.to_gpu(g)?;
                let out_buf = kernels::rmsnorm::dispatch(&self.device, &x_buf, &g_buf, batch, d, *eps);
                Ok(vec![self.from_gpu_f32(&out_buf, x.shape.clone())])
            }
            Op::Rope { head_dim, rope_dim, base } => {
                if *rope_dim == 0 || *rope_dim % 2 != 0 || *rope_dim > *head_dim {
                    return Err(BackendError::InvalidInput {
                        op: "Rope",
                        reason: format!(
                            "rope_dim must be even and ≤ head_dim, got {rope_dim} / {head_dim}"
                        ),
                    });
                }
                if inputs.len() != 2 {
                    return Err(BackendError::InvalidInput {
                        op: "Rope",
                        reason: format!("expected 2 inputs, got {}", inputs.len()),
                    });
                }
                // Partial rotary not yet implemented in wgpu kernel — fall back
                // to cpu reference (correctness > speed). Track in Phase 0.4 of
                // .claude/plans/cyb-mvp.md.
                if *rope_dim != *head_dim {
                    return self.cpu.execute(op, inputs);
                }
                let x = inputs[0];
                let pos = inputs[1];
                let x_buf = self.to_gpu(x)?;
                let pos_buf = self.to_gpu(pos)?;
                let total = x.numel() as u32;
                let out_buf = kernels::rope::dispatch(
                    &self.device,
                    &x_buf,
                    &pos_buf,
                    total,
                    *head_dim,
                    pos.numel() as u32,
                    *base,
                );
                Ok(vec![self.from_gpu_f32(&out_buf, x.shape.clone())])
            }
            Op::Softmax { dim } => {
                let rank = inputs[0].rank() as i32;
                let norm_dim = if *dim < 0 { rank + dim } else { *dim };
                if norm_dim != rank - 1 {
                    return self.cpu.execute(op, inputs);
                }
                let x = inputs[0];
                let d = *x.shape.last().unwrap() as u32;
                let batch: u32 = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
                let x_buf = self.to_gpu(x)?;
                let out_buf = kernels::softmax::dispatch(&self.device, &x_buf, batch, d);
                Ok(vec![self.from_gpu_f32(&out_buf, x.shape.clone())])
            }
            Op::Silu => {
                let x = inputs[0];
                let n = x.numel() as u32;
                let x_buf = self.to_gpu(x)?;
                let out_buf = kernels::silu::dispatch(&self.device, &x_buf, n);
                Ok(vec![self.from_gpu_f32(&out_buf, x.shape.clone())])
            }
            _ => self.cpu.execute(op, inputs),
        }
    }

    fn upload(
        &self,
        bytes: &[u8],
        shape: Vec<usize>,
        dtype: DType,
    ) -> Result<Tensor, BackendError> {
        // Persistent GPU buffer. Use this for weights to avoid re-upload
        // on every forward call.
        let buf = self.device.upload_bytes(bytes);
        let handle = GpuBuffer {
            buffer: buf,
            device: self.device.clone(),
        };
        Ok(Tensor {
            shape,
            dtype,
            data: TensorData::Backend(Arc::new(handle)),
        })
    }

    fn download_f32(&self, t: &Tensor) -> Result<Vec<f32>, BackendError> {
        match &t.data {
            TensorData::Host(_) => self.cpu.download_f32(t),
            TensorData::Backend(b) => {
                let g = b.as_any().downcast_ref::<GpuBuffer>().ok_or_else(|| {
                    BackendError::Internal("wgpu+rs: download unknown backend tensor".into())
                })?;
                Ok(self.device.read_f32(&g.buffer, t.numel()))
            }
        }
    }
}
