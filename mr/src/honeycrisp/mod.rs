//! honeycrisp — Apple Silicon turbo backend via aruminium.
//!
//! Stack: Metal (GPU) + ANE + AMX + NEON + unimem zero-copy.
//! Currently implements Metal compute kernels for hot f32 ops;
//! ANE/AMX integration is future.
//!
//! Spec: reference/runtime/architecture.md#honeycrisp

#![cfg(target_os = "macos")]

use crate::backend::{Backend, BackendError, BackendKind};
use crate::cpu::CpuBackend;
use crate::dtype::DType;
use crate::op::Op;
use crate::tensor::{BackendData, Tensor, TensorData};
use std::any::Any;
use std::sync::Arc;

mod device;
mod kernels;

use device::HoneycrispDevice;

struct HcBuffer {
    buffer: aruminium::Buffer,
}

// SAFETY: same reasoning as HoneycrispDevice — Metal buffers are
// thread-safe; Rust's conservative pointer bounds don't reflect that.
unsafe impl Send for HcBuffer {}
unsafe impl Sync for HcBuffer {}

impl BackendData for HcBuffer {
    fn backend_name(&self) -> &'static str {
        "honeycrisp"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct HoneycrispBackend {
    device: Arc<HoneycrispDevice>,
    cpu: CpuBackend,
}

impl HoneycrispBackend {
    pub fn new() -> Result<Self, BackendError> {
        let device = Arc::new(HoneycrispDevice::new()?);
        Ok(Self {
            device,
            cpu: CpuBackend::new(),
        })
    }

    fn upload_tensor(&self, t: &Tensor) -> Result<aruminium::Buffer, BackendError> {
        match &t.data {
            TensorData::Host(bytes) => self
                .device
                .gpu
                .buffer_with_data(bytes.as_slice())
                .map_err(|e| BackendError::Internal(format!("buffer_with_data: {e}"))),
            TensorData::Backend(b) => {
                if let Some(h) = b.as_any().downcast_ref::<HcBuffer>() {
                    // Clone the Metal buffer reference (aruminium Buffer is Arc-like).
                    // For now, upload again to avoid borrowing complexity.
                    let data = h.buffer.read(|bytes| bytes.to_vec());
                    self.device
                        .gpu
                        .buffer_with_data(&data)
                        .map_err(|e| BackendError::Internal(format!("re-upload: {e}")))
                } else {
                    Err(BackendError::Internal(
                        "honeycrisp: tensor on a different backend".into(),
                    ))
                }
            }
        }
    }

    fn read_f32(&self, buf: &aruminium::Buffer, n: usize) -> Vec<f32> {
        buf.read(|bytes| {
            let needed = n * 4;
            bytemuck::cast_slice::<u8, f32>(&bytes[..needed]).to_vec()
        })
    }
}

impl Backend for HoneycrispBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Honeycrisp
    }

    fn supports(&self, op: &Op, inputs: &[&Tensor]) -> bool {
        match op {
            Op::Matmul | Op::RmsNorm { .. } | Op::Silu
                if inputs.iter().all(|t| t.dtype == DType::F32) =>
            {
                true
            }
            _ => false,
        }
    }

    fn execute(&self, op: &Op, inputs: &[&Tensor]) -> Result<Vec<Tensor>, BackendError> {
        if !self.supports(op, inputs) {
            return self.cpu.execute(op, inputs);
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
                if w.rank() != 2 || x.shape.last() != Some(&w.shape[1]) {
                    return Err(BackendError::ShapeMismatch {
                        op: "Matmul",
                        expected: vec![0, w.shape[1]],
                        got: x.shape.clone(),
                    });
                }
                let batch: u32 = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
                let n = w.shape[0] as u32;
                let k = w.shape[1] as u32;

                let x_buf = self.upload_tensor(x)?;
                let w_buf = self.upload_tensor(w)?;
                let out_buf = kernels::matmul::dispatch(&self.device, &x_buf, &w_buf, batch, n, k)?;
                let out_f32 = self.read_f32(&out_buf, (batch * n) as usize);

                let mut out_shape = x.shape.clone();
                *out_shape.last_mut().unwrap() = n as usize;
                Ok(vec![Tensor::from_f32(out_shape, out_f32)])
            }
            Op::RmsNorm { eps } => {
                let x = inputs[0];
                let g = inputs[1];
                let d = g.shape[0] as u32;
                let batch: u32 = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
                let x_buf = self.upload_tensor(x)?;
                let g_buf = self.upload_tensor(g)?;
                let out_buf =
                    kernels::rmsnorm::dispatch(&self.device, &x_buf, &g_buf, batch, d, *eps)?;
                let out_f32 = self.read_f32(&out_buf, (batch * d) as usize);
                Ok(vec![Tensor::from_f32(x.shape.clone(), out_f32)])
            }
            Op::Silu => {
                let x = inputs[0];
                let n = x.numel() as u32;
                let x_buf = self.upload_tensor(x)?;
                let out_buf = kernels::silu::dispatch(&self.device, &x_buf, n)?;
                let out_f32 = self.read_f32(&out_buf, n as usize);
                Ok(vec![Tensor::from_f32(x.shape.clone(), out_f32)])
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
        // Keep on host; GPU buffers created lazily per op for now.
        // Future: persistent GPU buffers for weights via unimem zero-copy.
        self.cpu.upload(bytes, shape, dtype)
    }

    fn download_f32(&self, t: &Tensor) -> Result<Vec<f32>, BackendError> {
        match &t.data {
            TensorData::Host(_) => self.cpu.download_f32(t),
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp: unknown tensor".into())
                })?;
                Ok(self.read_f32(&h.buffer, t.numel()))
            }
        }
    }
}
