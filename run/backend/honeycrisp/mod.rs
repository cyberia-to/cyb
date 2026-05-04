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
    /// Logical byte length of valid data inside `buffer` (may be ≤ buffer.size()
    /// if the buffer was over-allocated).
    bytes: usize,
}

/// Send-marked wrapper for Buffer. Metal buffers are thread-safe per Apple
/// docs; Rust marks the underlying raw pointer as !Send conservatively.
struct PooledBuf(aruminium::Buffer);
unsafe impl Send for PooledBuf {}

/// Tiny LIFO pool of Metal buffers keyed by a power-of-two size class.
/// Avoids `newBufferWithLength` syscalls in the hot path.
struct BufferPool {
    by_class: std::sync::Mutex<std::collections::HashMap<usize, Vec<PooledBuf>>>,
}

impl BufferPool {
    fn new() -> Self {
        Self { by_class: std::sync::Mutex::new(std::collections::HashMap::new()) }
    }
    fn class_for(size: usize) -> usize {
        // Round up to next power of two, min 4 KB.
        let s = size.max(4096);
        s.next_power_of_two()
    }
    fn pop(&self, size: usize) -> Option<aruminium::Buffer> {
        let cls = Self::class_for(size);
        self.by_class.lock().ok()?.get_mut(&cls)?.pop().map(|p| p.0)
    }
    fn push(&self, buf: aruminium::Buffer) {
        let cls = Self::class_for(buf.size());
        if let Ok(mut map) = self.by_class.lock() {
            map.entry(cls).or_insert_with(Vec::new).push(PooledBuf(buf));
        }
    }
}

/// Owned-or-borrowed Metal buffer reference for kernel dispatch.
enum BufRef<'a> {
    Owned(aruminium::Buffer),
    Borrowed(&'a aruminium::Buffer),
}

impl<'a> BufRef<'a> {
    fn as_buffer(&self) -> &aruminium::Buffer {
        match self {
            BufRef::Owned(b) => b,
            BufRef::Borrowed(b) => b,
        }
    }
}

unsafe impl Send for HcBuffer {}
unsafe impl Sync for HcBuffer {}

impl BackendData for HcBuffer {
    fn backend_name(&self) -> &'static str { "honeycrisp" }
    fn as_any(&self) -> &dyn Any { self }
    fn try_as_host_bytes(&self) -> Option<&[u8]> {
        if !self.buffer.is_shared() { return None; }
        // Shared storage: contents pointer is CPU-readable. Only valid AFTER
        // the dispatch wait for the command buffer that wrote it. Our backend
        // always waits inside batch_raw before returning, so this is safe.
        Some(&self.buffer.as_bytes()[..self.bytes])
    }
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
    pipe_q4: HcPipeline,
    pipe_q4k: HcPipeline,
    pipe_q6k: HcPipeline,
    pipe_q8: HcPipeline,
    pipe_add: HcPipeline,
    pipe_silu_mul: HcPipeline,
    pipe_rope: HcPipeline,
    /// Recyclable scratch buffer pool — avoid `newBufferWithLength` per call
    /// inside fused chains.
    scratch: BufferPool,
}

impl HoneycrispBackend {
    pub fn new() -> Result<Self, BackendError> {
        let device = Arc::new(HoneycrispDevice::new()?);
        let pipe_matmul = HcPipeline(device.pipeline(kernels::matmul::MSL)?);
        let pipe_rmsnorm = HcPipeline(device.pipeline(kernels::rmsnorm::MSL)?);
        let pipe_silu = HcPipeline(device.pipeline(kernels::silu::MSL)?);
        let pipe_q4k = HcPipeline(device.pipeline(kernels::q4k_matmul::MSL)?);
        let pipe_q6k = HcPipeline(device.pipeline(kernels::q6k_matmul::MSL)?);
        let pipe_q8 = HcPipeline(device.pipeline(kernels::q8_matmul::MSL)?);
        let pipe_q4 = HcPipeline(device.pipeline(kernels::q4_matmul::MSL)?);
        let pipe_add = HcPipeline(device.pipeline(kernels::elementwise::ADD_MSL)?);
        let pipe_silu_mul = HcPipeline(device.pipeline(kernels::elementwise::SILU_MUL_MSL)?);
        let pipe_rope = HcPipeline(device.pipeline(kernels::rope::MSL)?);
        Ok(Self {
            device,
            cpu: CpuBackend::new(),
            pipe_matmul,
            pipe_rmsnorm,
            pipe_silu,
            pipe_q4,
            pipe_q4k,
            pipe_q6k,
            pipe_q8,
            pipe_add,
            pipe_silu_mul,
            pipe_rope,
            scratch: BufferPool::new(),
        })
    }

    /// Get a scratch buffer of at least `size` bytes — pulled from the pool
    /// or freshly allocated. Returned to the pool by `release_scratch`.
    fn take_scratch(&self, size: usize) -> Result<aruminium::Buffer, BackendError> {
        if let Some(b) = self.scratch.pop(size) {
            return Ok(b);
        }
        self.device.alloc(BufferPool::class_for(size))
    }
    fn release_scratch(&self, buf: aruminium::Buffer) {
        self.scratch.push(buf);
    }

    /// Owned-or-borrowed Metal buffer reference. Returns Borrowed for tensors
    /// already GPU-resident (zero copy) and Owned for host tensors (one upload).
    fn buf_ref<'a>(&self, t: &'a Tensor) -> Result<BufRef<'a>, BackendError> {
        match &t.data {
            TensorData::Host(bytes) => {
                let buf = self
                    .device
                    .gpu
                    .buffer_with_data(bytes.as_slice())
                    .map_err(|e| BackendError::Internal(format!("buffer_with_data: {e}")))?;
                Ok(BufRef::Owned(buf))
            }
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp: tensor on a different backend".into())
                })?;
                Ok(BufRef::Borrowed(&h.buffer))
            }
        }
    }

    /// Legacy upload helper: always returns an owned buffer (used where the
    /// dispatch path requires an owned buffer to extend its lifetime).
    /// Prefer `buf_ref` for new code.
    fn upload_tensor(&self, t: &Tensor) -> Result<aruminium::Buffer, BackendError> {
        match &t.data {
            TensorData::Host(bytes) => self
                .device
                .gpu
                .buffer_with_data(bytes.as_slice())
                .map_err(|e| BackendError::Internal(format!("buffer_with_data: {e}"))),
            TensorData::Backend(b) => {
                if let Some(h) = b.as_any().downcast_ref::<HcBuffer>() {
                    let bytes = &h.buffer.as_bytes()[..h.bytes];
                    self.device
                        .gpu
                        .buffer_with_data(bytes)
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

    /// Wrap a GPU output buffer as a Backend Tensor. Caller must guarantee the
    /// buffer has been written by a completed dispatch (waited inside batch_raw).
    fn wrap_output(&self, buf: aruminium::Buffer, shape: Vec<usize>, dtype: DType) -> Tensor {
        let bytes = dtype.bytes_for(crate::core::tensor::numel(&shape));
        let handle = HcBuffer { buffer: buf, bytes };
        Tensor { shape, dtype, data: TensorData::Backend(Arc::new(handle)) }
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
            // Only ops large enough to amortize per-dispatch wait cost.
            // RmsNorm uses parallel reduction over D; even tiny ones beat CPU
            // upload-roundtrip for f32. Add/Silu are too cheap and lose to CPU.
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
                let x_buf = self.buf_ref(x)?;
                let w_buf = self.buf_ref(w)?;
                let out_buf = kernels::matmul::dispatch(
                    &self.device, &self.pipe_matmul.0,
                    x_buf.as_buffer(), w_buf.as_buffer(), batch, n, k,
                )?;
                let mut out_shape = x.shape.clone();
                *out_shape.last_mut().unwrap() = n as usize;
                Ok(vec![self.wrap_output(out_buf, out_shape, DType::F32)])
            }
            Op::RmsNorm { eps } => {
                let x = inputs[0];
                let g = inputs[1];
                let d = g.shape[0] as u32;
                let batch: u32 = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
                let x_buf = self.buf_ref(x)?;
                let g_buf = self.buf_ref(g)?;
                let out_buf = kernels::rmsnorm::dispatch(
                    &self.device, &self.pipe_rmsnorm.0,
                    x_buf.as_buffer(), g_buf.as_buffer(), batch, d, *eps,
                )?;
                Ok(vec![self.wrap_output(out_buf, x.shape.clone(), DType::F32)])
            }
            Op::Silu => {
                let x = inputs[0];
                let n = x.numel() as u32;
                let x_buf = self.buf_ref(x)?;
                let out_buf = kernels::silu::dispatch(&self.device, &self.pipe_silu.0, x_buf.as_buffer(), n)?;
                Ok(vec![self.wrap_output(out_buf, x.shape.clone(), DType::F32)])
            }
            Op::Add => {
                if inputs.len() != 2 {
                    return Err(BackendError::InvalidInput {
                        op: "Add",
                        reason: format!("expected 2 inputs, got {}", inputs.len()),
                    });
                }
                let a = inputs[0];
                let b = inputs[1];
                // Output shape = max(a, b) per dim. We support: same-shape, or
                // b broadcast along leading dims (e.g. bias [D] added to [B, D]).
                let n = a.numel().max(b.numel()) as u32;
                let a_len = a.numel() as u32;
                let b_len = b.numel() as u32;
                let a_buf = self.buf_ref(a)?;
                let b_buf = self.buf_ref(b)?;
                let out_buf = kernels::elementwise::dispatch_add(
                    &self.device, &self.pipe_add.0,
                    a_buf.as_buffer(), b_buf.as_buffer(), n, a_len, b_len,
                )?;
                let out_shape = if a.numel() >= b.numel() { a.shape.clone() } else { b.shape.clone() };
                Ok(vec![self.wrap_output(out_buf, out_shape, DType::F32)])
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
        let n_bytes = bytes.len();
        let buffer = self
            .device
            .gpu
            .buffer_with_data(bytes)
            .map_err(|e| BackendError::Internal(format!("buffer_with_data: {e}")))?;
        let handle = HcBuffer { buffer, bytes: n_bytes };
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

        // QuantKind selects the GPU dispatch path; everything else falls back to CPU.
        enum QuantKind { Q4K, Q6K, Q8, Q4 }
        let kind = match w.dtype {
            DType::Q4_K if k % 256 == 0 => QuantKind::Q4K,
            DType::Q6_K if k % 256 == 0 => QuantKind::Q6K,
            DType::Q8   if k % kernels::q8_matmul::BLOCK_SIZE == 0 => QuantKind::Q8,
            DType::Q4   if k % kernels::q4_matmul::BLOCK_SIZE == 0 => QuantKind::Q4,
            _ => return self.cpu_quant_matmul(x, w),
        };
        let n_blocks: usize = match kind {
            QuantKind::Q4K | QuantKind::Q6K => k / 256,
            QuantKind::Q8 => k / kernels::q8_matmul::BLOCK_SIZE,
            QuantKind::Q4 => k / kernels::q4_matmul::BLOCK_SIZE,
        };

        let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
        let x_buf = self.buf_ref(x)?;
        let w_buf = self.buf_ref(w)?;

        let out_buf = match kind {
            QuantKind::Q4K => kernels::q4k_matmul::dispatch(
                &self.device, &self.pipe_q4k.0,
                x_buf.as_buffer(), w_buf.as_buffer(), batch, n as u32, n_blocks as u32,
            )?,
            QuantKind::Q6K => kernels::q6k_matmul::dispatch(
                &self.device, &self.pipe_q6k.0,
                x_buf.as_buffer(), w_buf.as_buffer(), batch, n as u32, n_blocks as u32,
            )?,
            QuantKind::Q8 => kernels::q8_matmul::dispatch(
                &self.device, &self.pipe_q8.0,
                x_buf.as_buffer(), w_buf.as_buffer(), batch, n as u32, n_blocks as u32,
            )?,
            QuantKind::Q4 => kernels::q4_matmul::dispatch(
                &self.device, &self.pipe_q4.0,
                x_buf.as_buffer(), w_buf.as_buffer(), batch, n as u32, n_blocks as u32,
            )?,
        };

        let mut out_shape = x.shape.clone();
        *out_shape.last_mut().unwrap() = n;
        Ok(self.wrap_output(out_buf, out_shape, DType::F32))
    }

    /// Batched: encode all `ws` matmuls into ONE command buffer, single wait.
    /// Saves the fixed per-dispatch submit + wait overhead (~50–100 µs each).
    fn quant_matmul_multi(
        &self,
        x: &Tensor,
        ws: &[&Tensor],
    ) -> Result<Vec<Tensor>, BackendError> {
        if ws.is_empty() { return Ok(Vec::new()); }

        // Homogeneous Q8/Q4 batch with same K. Otherwise per-call routing.
        let k = ws[0].shape[1];
        let kind0 = ws[0].dtype;
        let supported = ws.iter().all(|w| {
            w.dtype == kind0
                && w.shape[1] == k
                && (matches!(w.dtype, DType::Q8) && w.shape[1] % kernels::q8_matmul::BLOCK_SIZE == 0
                    || matches!(w.dtype, DType::Q4) && w.shape[1] % kernels::q4_matmul::BLOCK_SIZE == 0)
        });
        if !supported {
            return ws.iter().map(|w| self.quant_matmul(x, w)).collect();
        }
        let all_backend = ws.iter().all(|w| matches!(w.data, TensorData::Backend(_)));
        if !all_backend {
            return ws.iter().map(|w| self.quant_matmul(x, w)).collect();
        }

        let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
        let block_size = match kind0 {
            DType::Q8 => kernels::q8_matmul::BLOCK_SIZE,
            DType::Q4 => kernels::q4_matmul::BLOCK_SIZE,
            _ => unreachable!(),
        };
        let pipe = match kind0 {
            DType::Q8 => &self.pipe_q8.0,
            DType::Q4 => &self.pipe_q4.0,
            _ => unreachable!(),
        };
        let simds_per_group = match kind0 {
            DType::Q8 => kernels::q8_matmul::SIMDS_PER_GROUP,
            DType::Q4 => kernels::q4_matmul::SIMDS_PER_GROUP,
            _ => unreachable!(),
        };
        let n_blocks = (k / block_size) as u32;

        // Upload x once (or borrow if already on GPU).
        let x_buf = self.buf_ref(x)?;

        // Borrow weight buffers in place — no copy.
        let mut weight_refs: Vec<&aruminium::Buffer> = Vec::with_capacity(ws.len());
        let mut ns: Vec<usize> = Vec::with_capacity(ws.len());
        for w in ws {
            let h = match &w.data {
                TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("quant_matmul_multi: wrong backend tensor".into())
                })?,
                _ => unreachable!("all_backend guard above"),
            };
            weight_refs.push(&h.buffer);
            ns.push(w.shape[0]);
        }
        let mut out_bufs: Vec<aruminium::Buffer> = Vec::with_capacity(ws.len());
        for &n in &ns {
            out_bufs.push(self.device.alloc((batch as usize * n * 4).max(4))?);
        }

        // ONE command buffer, encode all ws dispatches, ONE wait.
        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    for i in 0..ws.len() {
                        let n = ns[i] as u32;
                        let total_rows = batch * n;
                        let groups_x = (total_rows + simds_per_group - 1) / simds_per_group;
                        let threads_per_group = simds_per_group * 32;

                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch, n_rows: n, n_blocks, pad: 0 };

                        enc.bind(pipe);
                        enc.bind_buffer(x_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(weight_refs[i], 0, 1);
                        enc.bind_buffer(&out_bufs[i], 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8,
                            std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    }
                });
            });
        }

        // Wrap outputs as backend tensors (shared memory — readable after wait).
        let mut results = Vec::with_capacity(ws.len());
        for (out_buf, n) in out_bufs.into_iter().zip(ns.into_iter()) {
            let mut out_shape = x.shape.clone();
            *out_shape.last_mut().unwrap() = n;
            results.push(self.wrap_output(out_buf, out_shape, DType::F32));
        }
        Ok(results)
    }

    // silu_mul: GPU only when called inside a fused chain (no individual override).
    // Standalone call still uses CPU through trait default for shape preservation.

    /// Fused: input_norm → q/k/v matmul → qk_norm Q/K — ONE command buffer.
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
        // Eligibility: homogeneous Q8 or Q4 weights, all GPU-resident.
        let kind0 = q_proj_w.dtype;
        let same_kind = k_proj_w.dtype == kind0 && v_proj_w.dtype == kind0;
        let valid_kind = matches!(kind0, DType::Q8 | DType::Q4);
        let on_gpu = matches!(q_proj_w.data, TensorData::Backend(_))
            && matches!(k_proj_w.data, TensorData::Backend(_))
            && matches!(v_proj_w.data, TensorData::Backend(_));
        let block_size = match kind0 {
            DType::Q8 => kernels::q8_matmul::BLOCK_SIZE,
            DType::Q4 => kernels::q4_matmul::BLOCK_SIZE,
            _ => 0,
        };
        let aligned = block_size > 0 && q_proj_w.shape[1] % block_size == 0;
        if !same_kind || !valid_kind || !on_gpu || !aligned
            || hidden.dtype != DType::F32 || input_norm_gamma.dtype != DType::F32
            || q_norm_gamma.dtype != DType::F32 || k_norm_gamma.dtype != DType::F32
        {
            return Backend::fused_norm_qkv_qknorm(
                self, hidden, input_norm_gamma,
                q_proj_w, k_proj_w, v_proj_w,
                q_norm_gamma, k_norm_gamma,
                eps, num_q_heads, num_k_heads, head_dim,
            );
        }
        let pipe = match kind0 {
            DType::Q8 => &self.pipe_q8.0,
            DType::Q4 => &self.pipe_q4.0,
            _ => unreachable!(),
        };
        let simds_per_group = match kind0 {
            DType::Q8 => kernels::q8_matmul::SIMDS_PER_GROUP,
            DType::Q4 => kernels::q4_matmul::SIMDS_PER_GROUP,
            _ => unreachable!(),
        };

        let batch = hidden.shape[..hidden.shape.len() - 1].iter().product::<usize>() as u32;
        let d = input_norm_gamma.shape[0] as u32;
        let n_blocks_d = (d as usize / block_size) as u32;
        let q_n = q_proj_w.shape[0] as u32;
        let k_n = k_proj_w.shape[0] as u32;
        let v_n = v_proj_w.shape[0] as u32;

        let h_buf = self.buf_ref(hidden)?;
        let g_buf = self.buf_ref(input_norm_gamma)?;
        let qn_buf = self.buf_ref(q_norm_gamma)?;
        let kn_buf = self.buf_ref(k_norm_gamma)?;
        let q_w_h = match &q_proj_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let k_w_h = match &k_proj_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let v_w_h = match &v_proj_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };

        let normed_buf = self.take_scratch((batch as usize * d as usize * 4).max(4))?;
        // Scratch q/k for the matmul output, then re-used as input to qk_norm.
        // v output stays as final tensor.
        let q_buf = self.take_scratch((batch as usize * q_n as usize * 4).max(4))?;
        let k_buf = self.take_scratch((batch as usize * k_n as usize * 4).max(4))?;
        let q_norm_out = self.device.alloc((batch as usize * q_n as usize * 4).max(4))?;
        let k_norm_out = self.device.alloc((batch as usize * k_n as usize * 4).max(4))?;
        let v_out = self.device.alloc((batch as usize * v_n as usize * 4).max(4))?;

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    // 1) Input RmsNorm
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = P { batch, d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(h_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(g_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&normed_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((batch as usize, 1, 1), (256, 1, 1));
                    }
                    let dispatch_q = |enc: &aruminium::Batch,
                                      x_buf: &aruminium::Buffer,
                                      w_buf: &aruminium::Buffer,
                                      out: &aruminium::Buffer,
                                      n_rows: u32,
                                      n_blocks: u32| {
                        let total_rows = batch * n_rows;
                        let groups_x = (total_rows + simds_per_group - 1) / simds_per_group;
                        let threads_per_group = simds_per_group * 32;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch, n_rows, n_blocks, pad: 0 };
                        enc.bind(pipe);
                        enc.bind_buffer(x_buf, 0, 0);
                        enc.bind_buffer(w_buf, 0, 1);
                        enc.bind_buffer(out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8,
                            std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    };
                    // 2-4) Q/K/V matmul
                    dispatch_q(enc, &normed_buf, &q_w_h.buffer, &q_buf, q_n, n_blocks_d);
                    dispatch_q(enc, &normed_buf, &k_w_h.buffer, &k_buf, k_n, n_blocks_d);
                    dispatch_q(enc, &normed_buf, &v_w_h.buffer, &v_out, v_n, n_blocks_d);
                    // 5-6) QK norm (per-head)
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        // Q: batch=num_q_heads, d=head_dim
                        let p_q = P { batch: num_q_heads as u32, d: head_dim as u32, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(&q_buf, 0, 0);
                        enc.bind_buffer(qn_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&q_norm_out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p_q as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((num_q_heads, 1, 1), (256, 1, 1));

                        // K: batch=num_k_heads, d=head_dim
                        let p_k = P { batch: num_k_heads as u32, d: head_dim as u32, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(&k_buf, 0, 0);
                        enc.bind_buffer(kn_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&k_norm_out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p_k as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((num_k_heads, 1, 1), (256, 1, 1));
                    }
                });
            });
        }

        self.release_scratch(normed_buf);
        self.release_scratch(q_buf);
        self.release_scratch(k_buf);

        let q_t = self.wrap_output(q_norm_out, vec![1, num_q_heads * head_dim], DType::F32);
        let k_t = self.wrap_output(k_norm_out, vec![1, num_k_heads * head_dim], DType::F32);
        let v_t = self.wrap_output(v_out, vec![1, v_n as usize], DType::F32);
        Ok((q_t, k_t, v_t))
    }

    /// Fused FFN: norm + gate + up + silu_mul + down — ONE command buffer.
    /// Saves ~2 waits per layer (was: norm+gate_up batch, then silu CPU, then down).
    fn fused_norm_swiglu_down(
        &self,
        hidden: &Tensor,
        post_norm_gamma: &Tensor,
        gate_w: &Tensor,
        up_w: &Tensor,
        down_w: &Tensor,
        eps: f32,
    ) -> Result<Tensor, BackendError> {
        // Eligibility check — falls back to default chain if anything off.
        let weights_q8 = matches!(gate_w.dtype, DType::Q8)
            && matches!(up_w.dtype, DType::Q8)
            && matches!(down_w.dtype, DType::Q8);
        let weights_on_gpu = matches!(gate_w.data, TensorData::Backend(_))
            && matches!(up_w.data, TensorData::Backend(_))
            && matches!(down_w.data, TensorData::Backend(_));
        let k_match = gate_w.shape[1] == up_w.shape[1]
            && gate_w.shape[0] == up_w.shape[0]
            && down_w.shape[1] == gate_w.shape[0]
            && gate_w.shape[1] == hidden.shape[hidden.shape.len() - 1];
        let aligned = gate_w.shape[1] % kernels::q8_matmul::BLOCK_SIZE == 0
            && down_w.shape[1] % kernels::q8_matmul::BLOCK_SIZE == 0;
        if !weights_q8 || !weights_on_gpu || !k_match || !aligned
            || hidden.dtype != DType::F32 || post_norm_gamma.dtype != DType::F32
        {
            return Backend::fused_norm_swiglu_down(
                self, hidden, post_norm_gamma, gate_w, up_w, down_w, eps,
            );
        }

        let batch = hidden.shape[..hidden.shape.len() - 1].iter().product::<usize>() as u32;
        let d = post_norm_gamma.shape[0] as u32;          // hidden dim
        let inter = gate_w.shape[0] as u32;               // intermediate dim
        let down_n = down_w.shape[0] as u32;              // == hidden dim
        let n_blocks_d = (d as usize / kernels::q8_matmul::BLOCK_SIZE) as u32;
        let n_blocks_inter = (inter as usize / kernels::q8_matmul::BLOCK_SIZE) as u32;

        let h_buf = self.buf_ref(hidden)?;
        let g_buf = self.buf_ref(post_norm_gamma)?;
        let gate_w_h = match &gate_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let up_w_h = match &up_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let down_w_h = match &down_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };

        let normed_size = (batch as usize * d as usize * 4).max(4);
        let inter_size = (batch as usize * inter as usize * 4).max(4);
        let out_size = (batch as usize * down_n as usize * 4).max(4);
        let normed_buf = self.take_scratch(normed_size)?;
        let gate_buf = self.take_scratch(inter_size)?;
        let up_buf = self.take_scratch(inter_size)?;
        let mid_buf = self.take_scratch(inter_size)?;
        let out_buf = self.device.alloc(out_size)?;

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    // 1) Post-RMS norm
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = P { batch, d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(h_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(g_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&normed_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((batch as usize, 1, 1), (256, 1, 1));
                    }
                    // helper to dispatch a q8 matmul reading from `x_buf` into `out`
                    let dispatch_q8 = |enc: &aruminium::Batch,
                                       x_buf: &aruminium::Buffer,
                                       w_buf: &aruminium::Buffer,
                                       out: &aruminium::Buffer,
                                       n_rows: u32,
                                       n_blocks: u32| {
                        let total_rows = batch * n_rows;
                        let groups_x = (total_rows + kernels::q8_matmul::SIMDS_PER_GROUP - 1)
                            / kernels::q8_matmul::SIMDS_PER_GROUP;
                        let threads_per_group = kernels::q8_matmul::SIMDS_PER_GROUP * 32;

                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch, n_rows, n_blocks, pad: 0 };
                        enc.bind(&self.pipe_q8.0);
                        enc.bind_buffer(x_buf, 0, 0);
                        enc.bind_buffer(w_buf, 0, 1);
                        enc.bind_buffer(out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8,
                            std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    };
                    // 2) gate = normed @ gate_w
                    dispatch_q8(enc, &normed_buf, &gate_w_h.buffer, &gate_buf, inter, n_blocks_d);
                    // 3) up = normed @ up_w
                    dispatch_q8(enc, &normed_buf, &up_w_h.buffer, &up_buf, inter, n_blocks_d);
                    // 4) mid = silu(gate) * up
                    {
                        let n = batch * inter;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { n: u32, pad0: u32, pad1: u32, pad2: u32 }
                        let p = P { n, pad0: 0, pad1: 0, pad2: 0 };
                        enc.bind(&self.pipe_silu_mul.0);
                        enc.bind_buffer(&gate_buf, 0, 0);
                        enc.bind_buffer(&up_buf, 0, 1);
                        enc.bind_buffer(&mid_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((((n as usize) + 63) / 64, 1, 1), (64, 1, 1));
                    }
                    // 5) out = mid @ down_w
                    dispatch_q8(enc, &mid_buf, &down_w_h.buffer, &out_buf, down_n, n_blocks_inter);
                });
            });
        }

        // Return scratch buffers to the pool.
        self.release_scratch(normed_buf);
        self.release_scratch(gate_buf);
        self.release_scratch(up_buf);
        self.release_scratch(mid_buf);

        let mut out_shape = hidden.shape.clone();
        *out_shape.last_mut().unwrap() = down_n as usize;
        Ok(self.wrap_output(out_buf, out_shape, DType::F32))
    }

    /// Batched RmsNorm — multiple independent (x, gamma) pairs in one command
    /// buffer with a single wait.
    fn rms_norm_multi(
        &self,
        pairs: &[(&Tensor, &Tensor)],
        eps: f32,
    ) -> Result<Vec<Tensor>, BackendError> {
        if pairs.is_empty() { return Ok(Vec::new()); }
        // All inputs must be f32 (host or backend). If any quantized, fall back.
        let supported = pairs.iter().all(|(x, g)| x.dtype == DType::F32 && g.dtype == DType::F32);
        if !supported {
            return Backend::rms_norm_multi(self, pairs, eps);
        }

        // Resolve buf refs and allocate outputs.
        struct Item<'a> {
            x: BufRef<'a>,
            g: BufRef<'a>,
            out: aruminium::Buffer,
            shape: Vec<usize>,
            batch: u32,
            d: u32,
        }
        let mut items: Vec<Item> = Vec::with_capacity(pairs.len());
        for (x, g) in pairs {
            let d = g.shape[0] as u32;
            let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
            let n_bytes = (batch as usize * d as usize * 4).max(4);
            let out = self.device.alloc(n_bytes)?;
            items.push(Item {
                x: self.buf_ref(x)?,
                g: self.buf_ref(g)?,
                out,
                shape: x.shape.clone(),
                batch,
                d,
            });
        }

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    for it in &items {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = P { batch: it.batch, d: it.d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(it.x.as_buffer(), 0, 0);
                        enc.bind_buffer(it.g.as_buffer(), 0, 1);
                        enc.bind_buffer(&it.out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((it.batch as usize, 1, 1), (256, 1, 1));
                    }
                });
            });
        }

        let mut results = Vec::with_capacity(items.len());
        for it in items {
            results.push(self.wrap_output(it.out, it.shape, DType::F32));
        }
        Ok(results)
    }

    /// Fused (RmsNorm + N quant matmul) — encodes the whole chain into ONE
    /// command buffer with ONE wait. Saves the per-op submit overhead that
    /// dominates small-matmul timings.
    fn fused_norm_quant_matmul_multi(
        &self,
        x: &Tensor,
        gamma: &Tensor,
        eps: f32,
        ws: &[&Tensor],
    ) -> Result<Vec<Tensor>, BackendError> {
        if ws.is_empty() { return Ok(Vec::new()); }
        let k = ws[0].shape[1];
        let kind0 = ws[0].dtype;
        let supported = ws.iter().all(|w| {
            w.dtype == kind0
                && w.shape[1] == k
                && (matches!(w.dtype, DType::Q8) && w.shape[1] % kernels::q8_matmul::BLOCK_SIZE == 0
                    || matches!(w.dtype, DType::Q4) && w.shape[1] % kernels::q4_matmul::BLOCK_SIZE == 0)
        });
        let weights_on_gpu = ws.iter().all(|w| matches!(w.data, TensorData::Backend(_)));
        if !supported || !weights_on_gpu || x.dtype != DType::F32 || gamma.dtype != DType::F32 {
            return Backend::fused_norm_quant_matmul_multi(self, x, gamma, eps, ws);
        }

        let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
        let d = gamma.shape[0] as u32;
        if k as u32 != d {
            return Err(BackendError::ShapeMismatch {
                op: "fused_norm_quant_matmul_multi",
                expected: vec![d as usize],
                got: vec![k],
            });
        }

        let block_size = match kind0 {
            DType::Q8 => kernels::q8_matmul::BLOCK_SIZE,
            DType::Q4 => kernels::q4_matmul::BLOCK_SIZE,
            _ => unreachable!(),
        };
        let pipe = match kind0 {
            DType::Q8 => &self.pipe_q8.0,
            DType::Q4 => &self.pipe_q4.0,
            _ => unreachable!(),
        };
        let simds_per_group = match kind0 {
            DType::Q8 => kernels::q8_matmul::SIMDS_PER_GROUP,
            DType::Q4 => kernels::q4_matmul::SIMDS_PER_GROUP,
            _ => unreachable!(),
        };
        let n_blocks = (k / block_size) as u32;
        let x_buf = self.buf_ref(x)?;
        let g_buf = self.buf_ref(gamma)?;

        // Allocate normed (intermediate, pooled) and one output per matmul.
        let normed_buf = self.take_scratch((batch as usize * d as usize * 4).max(4))?;
        let mut weight_refs: Vec<&aruminium::Buffer> = Vec::with_capacity(ws.len());
        let mut ns: Vec<usize> = Vec::with_capacity(ws.len());
        for w in ws {
            let h = match &w.data {
                TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("fused: wrong backend tensor".into())
                })?,
                _ => unreachable!(),
            };
            weight_refs.push(&h.buffer);
            ns.push(w.shape[0]);
        }
        let mut out_bufs: Vec<aruminium::Buffer> = Vec::with_capacity(ws.len());
        for &n in &ns {
            out_bufs.push(self.device.alloc((batch as usize * n * 4).max(4))?);
        }

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    // 1) RmsNorm: x → normed_buf
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct NormParams { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = NormParams { batch, d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(x_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(g_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&normed_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const NormParams as *const u8,
                            std::mem::size_of::<NormParams>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((batch as usize, 1, 1), (256, 1, 1));
                    }
                    // 2) N quant matmuls reading from normed_buf
                    for (i, w_buf) in weight_refs.iter().enumerate() {
                        let n = ns[i] as u32;
                        let total_rows = batch * n;
                        let groups_x = (total_rows + simds_per_group - 1) / simds_per_group;
                        let threads_per_group = simds_per_group * 32;

                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch, n_rows: n, n_blocks, pad: 0 };

                        enc.bind(pipe);
                        enc.bind_buffer(&normed_buf, 0, 0);
                        enc.bind_buffer(w_buf, 0, 1);
                        enc.bind_buffer(&out_bufs[i], 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8,
                            std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    }
                });
            });
        }

        // Recycle the normed scratch.
        self.release_scratch(normed_buf);

        let mut results = Vec::with_capacity(ws.len());
        for (out_buf, n) in out_bufs.into_iter().zip(ns.into_iter()) {
            let mut out_shape = x.shape.clone();
            *out_shape.last_mut().unwrap() = n;
            results.push(self.wrap_output(out_buf, out_shape, DType::F32));
        }
        Ok(results)
    }
}
