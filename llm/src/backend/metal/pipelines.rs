//! MetalPipelines: device init, MSL compilation, buffer helpers
//!
//! Compiles all MSL kernels at startup via aruminium.

use aruminium::{Buffer, Dispatch, Gpu, GpuError, Pipeline, Queue};

/// All compiled Metal compute pipelines + device state.
pub struct MetalPipelines {
    pub device: Gpu,
    pub queue: Queue,
    pub dispatcher: Dispatch,

    // Prefill (matmul)
    pub matmul_f16: Pipeline,
    pub matmul_q4: Pipeline,

    // Decode single (matvec batch=1)
    pub matvec_q4: Pipeline,
    pub matvec_ternary: Pipeline,
    pub matvec_q4k: Pipeline,
    pub matvec_q5k: Pipeline,
    pub matvec_q6k: Pipeline,
    pub matvec_q3k: Pipeline,
    pub matvec_q2k: Pipeline,

    // Decode batched (dequant-once-dot-many)
    pub matvec_q4_batch: Pipeline,
    pub matvec_ternary_batch: Pipeline,

    // Optimized / fused kernels
    pub matvec_q4_fast: Pipeline,
    pub fused_rope_qk: Pipeline,
    pub fused_kv_append: Pipeline,
    pub fused_add_norm: Pipeline,
    pub matvec_q4_fast_batch4: Pipeline,
    pub fused_qkv: Pipeline,
    pub fused_gate_up: Pipeline,
    pub fused_silu_down: Pipeline,

    // Transformer ops (all fp16)
    pub embed: Pipeline,
    pub rms_norm: Pipeline,
    pub rope: Pipeline,
    pub add_f16: Pipeline,
    pub silu_mul_f16: Pipeline,
    pub relu2_mul_f16: Pipeline,
    pub attention_decode: Pipeline,
    pub kv_append: Pipeline,
    pub kv_expand: Pipeline,
    pub f16_matvec: Pipeline,
    pub argmax_partial: Pipeline,
    pub argmax_reduce: Pipeline,
}

impl MetalPipelines {
    /// Initialize Metal device and compile all MSL kernels.
    pub fn new() -> Result<Self, GpuError> {
        let device = Gpu::open()?;
        log::info!("Metal: {}", device.name());
        log::info!(
            "  unified={}, max_buf={}GB",
            device.has_unified_memory(),
            device.max_buffer_length() >> 30,
        );

        let queue = device.new_command_queue()?;
        let dispatcher = Dispatch::new(&queue);

        let compile = |src: &str, fname: &str| -> Result<Pipeline, GpuError> {
            let lib = device.compile(src)?;
            let func = lib.function(fname)?;
            device.pipeline(&func)
        };

        use super::kernels;

        let matmul_f16 = compile(kernels::MATMUL_F16, "matmul_f16")?;
        let matmul_q4 = compile(kernels::MATMUL_Q4, "matmul_q4")?;
        let matvec_q4 = compile(kernels::MATVEC_Q4, "matvec_q4")?;
        let matvec_ternary = compile(kernels::MATVEC_TERNARY, "matvec_ternary")?;
        let matvec_q4k = compile(kernels::MATVEC_Q4K, "matvec_q4k")?;
        let matvec_q5k = compile(kernels::MATVEC_Q5K, "matvec_q5k")?;
        let matvec_q6k = compile(kernels::MATVEC_Q6K, "matvec_q6k")?;
        let matvec_q3k = compile(kernels::MATVEC_Q3K, "matvec_q3k")?;
        let matvec_q2k = compile(kernels::MATVEC_Q2K, "matvec_q2k")?;

        let batch_src = |src: &str, b: u32| format!("#define BATCH {b}\n{src}");
        let matvec_q4_batch = compile(&batch_src(kernels::MATVEC_Q4_BATCH, 8), "matvec_q4_batch")?;
        let matvec_ternary_batch =
            compile(&batch_src(kernels::MATVEC_TERNARY_BATCH, 8), "matvec_ternary_batch")?;

        let matvec_q4_fast = compile(kernels::MATVEC_Q4_FAST, "matvec_q4_fast")?;
        let fused_rope_qk = compile(kernels::FUSED_ROPE, "fused_rope_qk")?;
        let fused_kv_append = compile(kernels::FUSED_KV_APPEND, "fused_kv_append")?;
        let fused_add_norm = compile(kernels::FUSED_ADD_NORM, "fused_add_norm_f16")?;
        let matvec_q4_fast_batch4 = compile(&batch_src(kernels::MATVEC_Q4_FAST_BATCH, 4), "matvec_q4_fast_batch")?;
        let fused_qkv = compile(kernels::FUSED_QKV, "fused_qkv_q4")?;
        let fused_gate_up = compile(kernels::FUSED_GATE_UP, "fused_gate_up_q4")?;
        let fused_silu_down = compile(kernels::FUSED_SILU_DOWN, "fused_silu_down_q4")?;

        let embed = compile(kernels::EMBED, "embed_f16")?;
        let rms_norm = compile(kernels::RMS_NORM, "rms_norm_f16")?;
        let rope = compile(kernels::ROPE, "rope_f16")?;
        let add_f16 = compile(kernels::ELEMENTWISE, "add_f16")?;
        let silu_mul_f16 = compile(kernels::ELEMENTWISE, "silu_mul_f16")?;
        let relu2_mul_f16 = compile(kernels::ELEMENTWISE, "relu2_mul_f16")?;
        let attention_decode = compile(kernels::ATTENTION, "attention_decode_f16")?;
        let kv_append = compile(kernels::KV_CACHE, "kv_append_f16")?;
        let kv_expand = compile(kernels::KV_CACHE, "kv_expand_f16")?;
        let f16_matvec = compile(kernels::F16_MATVEC, "f16_matvec")?;
        let argmax_partial = compile(kernels::ARGMAX, "argmax_partial")?;
        let argmax_reduce = compile(kernels::ARGMAX, "argmax_reduce")?;

        log::info!("Metal: all 23 MSL kernels compiled");

        Ok(MetalPipelines {
            device, queue, dispatcher,
            matmul_f16, matmul_q4,
            matvec_q4, matvec_q4_fast,
            fused_rope_qk, fused_kv_append, fused_add_norm,
            matvec_q4_fast_batch4, fused_qkv, fused_gate_up, fused_silu_down,
            matvec_ternary, matvec_q4k, matvec_q5k, matvec_q6k, matvec_q3k, matvec_q2k,
            matvec_q4_batch, matvec_ternary_batch,
            embed, rms_norm, rope, add_f16, silu_mul_f16, relu2_mul_f16,
            attention_decode, kv_append, kv_expand, f16_matvec, argmax_partial, argmax_reduce,
        })
    }

    /// Upload f16 data to a shared GPU buffer.
    pub fn upload_f16(&self, data: &[u16]) -> Result<Buffer, GpuError> {
        let bytes = bytemuck::cast_slice::<u16, u8>(data);
        self.device.buffer_with_data(bytes)
    }

    /// Upload f32 data to a shared GPU buffer.
    pub fn upload_f32(&self, data: &[f32]) -> Result<Buffer, GpuError> {
        let bytes = bytemuck::cast_slice::<f32, u8>(data);
        self.device.buffer_with_data(bytes)
    }

    /// Upload raw bytes to a shared GPU buffer.
    pub fn upload_bytes(&self, data: &[u8]) -> Result<Buffer, GpuError> {
        self.device.buffer_with_data(data)
    }

    /// Allocate an uninitialized shared GPU buffer.
    pub fn alloc(&self, size: usize) -> Result<Buffer, GpuError> {
        self.device.buffer(size)
    }
}
