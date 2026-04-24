//! honeycrisp — Apple Silicon turbo backend via aruminium.
//!
//! Stack: Metal (GPU) + ANE + AMX + NEON + unimem zero-copy.
//! Currently implements Metal compute kernels for hot f32 ops and
//! fused Q4_K/Q6_K dequant+matmul. ANE/AMX integration is future.
//!
//! Spec: specs/architecture.md#honeycrisp

#![cfg(target_os = "macos")]

use crate::backend::{Backend, BackendError, BackendKind};
use crate::backend::cpu::CpuBackend;
use crate::core::dtype::DType;
use crate::core::op::Op;
use crate::core::tensor::{BackendData, Tensor, TensorData};
use std::any::Any;
use std::sync::Arc;

mod device;
mod kernels;

use device::HoneycrispDevice;

struct HcBuffer {
    buffer: aruminium::Buffer,
}

unsafe impl Send for HcBuffer {}
unsafe impl Sync for HcBuffer {}

impl BackendData for HcBuffer {
    fn backend_name(&self) -> &'static str { "honeycrisp" }
    fn as_any(&self) -> &dyn Any { self }
}

/// Wrapper to make aruminium::Pipeline Send+Sync.
/// Metal objects are thread-safe per Apple's documentation.
struct HcPipeline(aruminium::Pipeline);
unsafe impl Send for HcPipeline {}
unsafe impl Sync for HcPipeline {}

pub struct HoneycrispBackend {
    device: Arc<HoneycrispDevice>,
    cpu: CpuBackend,
    pipe_matmul: HcPipeline,
    pipe_rmsnorm: HcPipeline,
    pipe_silu: HcPipeline,
    pipe_q4k: HcPipeline,
    pipe_q6k: HcPipeline,
}

impl HoneycrispBackend {
    pub fn new() -> Result<Self, BackendError> {
        let device = Arc::new(HoneycrispDevice::new()?);
        let pipe_matmul = HcPipeline(device.pipeline(kernels::matmul::MSL)?);
        let pipe_rmsnorm = HcPipeline(device.pipeline(kernels::rmsnorm::MSL)?);
        let pipe_silu = HcPipeline(device.pipeline(kernels::silu::MSL)?);
        let pipe_q4k = HcPipeline(device.pipeline(kernels::q4k_matmul::MSL)?);
        let pipe_q6k = HcPipeline(device.pipeline(kernels::q6k_matmul::MSL)?);
        Ok(Self {
            device,
            cpu: CpuBackend::new(),
            pipe_matmul,
            pipe_rmsnorm,
            pipe_silu,
            pipe_q4k,
            pipe_q6k,
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

    /// CPU fallback for quant_matmul. If w is GPU-resident (pre-uploaded),
    /// reads the raw bytes back from the Metal buffer first.
    fn cpu_quant_matmul(&self, x: &Tensor, w: &Tensor) -> Result<Tensor, BackendError> {
        match &w.data {
            TensorData::Host(_) => self.cpu.quant_matmul(x, w),
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp cpu_quant_matmul: unknown tensor".into())
                })?;
                let bytes: Arc<Vec<u8>> = Arc::new(h.buffer.read(|b| b.to_vec()));
                let w_host = Tensor {
                    shape: w.shape.clone(),
                    dtype: w.dtype,
                    data: TensorData::Host(bytes),
                };
                self.cpu.quant_matmul(x, &w_host)
            }
        }
    }

    fn download_tensor(&self, t: &Tensor) -> Result<Tensor, BackendError> {
        match &t.data {
            TensorData::Host(_) => Ok(t.clone()),
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp: unknown backend tensor".into())
                })?;
                if t.dtype != DType::F32 {
                    return Err(BackendError::Internal(
                        "honeycrisp: non-F32 GPU download not implemented".into(),
                    ));
                }
                Ok(Tensor::from_f32(t.shape.clone(), self.read_f32(&h.buffer, t.numel())))
            }
        }
    }
}

impl Backend for HoneycrispBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Honeycrisp
    }

    fn uploads_quant_weights(&self) -> bool { true }

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
            let materialized: Result<Vec<Tensor>, BackendError> =
                inputs.iter().map(|t| self.download_tensor(t)).collect();
            let materialized = materialized?;
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
                let out_buf = kernels::matmul::dispatch(
                    &self.device, &self.pipe_matmul.0,
                    &x_buf, &w_buf, batch, n, k,
                )?;
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
                let out_buf = kernels::rmsnorm::dispatch(
                    &self.device, &self.pipe_rmsnorm.0,
                    &x_buf, &g_buf, batch, d, *eps,
                )?;
                let out_f32 = self.read_f32(&out_buf, (batch * d) as usize);
                Ok(vec![Tensor::from_f32(x.shape.clone(), out_f32)])
            }
            Op::Silu => {
                let x = inputs[0];
                let n = x.numel() as u32;
                let x_buf = self.upload_tensor(x)?;
                let out_buf = kernels::silu::dispatch(&self.device, &self.pipe_silu.0, &x_buf, n)?;
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
        let buffer = self
            .device
            .gpu
            .buffer_with_data(bytes)
            .map_err(|e| BackendError::Internal(format!("buffer_with_data: {e}")))?;
        let handle = HcBuffer { buffer };
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
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp: unknown tensor".into())
                })?;
                Ok(self.read_f32(&h.buffer, t.numel()))
            }
        }
    }

    fn quant_matmul(&self, x: &Tensor, w: &Tensor) -> Result<Tensor, BackendError> {
        let n = w.shape[0];
        let k = w.shape[1];
        let is_q4k = match w.dtype {
            DType::Q4_K if k % 256 == 0 => true,
            DType::Q6_K if k % 256 == 0 => false,
            _ => return self.cpu_quant_matmul(x, w),
        };
        let n_blocks = k / 256;

        let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
        let x_buf = self.upload_tensor(x)?;

        // Use GPU-resident buffer if pre-uploaded; otherwise upload on demand.
        let out_buf = match &w.data {
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("quant_matmul: wrong backend tensor".into())
                })?;
                if is_q4k {
                    kernels::q4k_matmul::dispatch(
                        &self.device, &self.pipe_q4k.0,
                        &x_buf, &h.buffer, batch, n as u32, n_blocks as u32,
                    )?
                } else {
                    kernels::q6k_matmul::dispatch(
                        &self.device, &self.pipe_q6k.0,
                        &x_buf, &h.buffer, batch, n as u32, n_blocks as u32,
                    )?
                }
            }
            TensorData::Host(bytes) => {
                let w_buf = self.device.gpu.buffer_with_data(bytes.as_slice())
                    .map_err(|e| BackendError::Internal(format!("w upload: {e}")))?;
                if is_q4k {
                    kernels::q4k_matmul::dispatch(
                        &self.device, &self.pipe_q4k.0,
                        &x_buf, &w_buf, batch, n as u32, n_blocks as u32,
                    )?
                } else {
                    kernels::q6k_matmul::dispatch(
                        &self.device, &self.pipe_q6k.0,
                        &x_buf, &w_buf, batch, n as u32, n_blocks as u32,
                    )?
                }
            }
        };

        let out_f32 = self.read_f32(&out_buf, batch as usize * n);
        let mut out_shape = x.shape.clone();
        *out_shape.last_mut().unwrap() = n;
        Ok(Tensor::from_f32(out_shape, out_f32))
    }
}
