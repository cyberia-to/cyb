//! MetalPipelines: device init, MSL compilation, buffer helpers
//!
//! Compiles all 7 MSL kernels at startup. Provides ComputeDispatcher for hot-path
//! inference and buffer upload helpers for weight staging.

use aruminium::{
    ComputeDispatcher, MtlBuffer, MtlCommandQueue, MtlComputePipeline, MtlDevice,
};

/// All compiled Metal compute pipelines + device state.
pub struct MetalPipelines {
    pub device: MtlDevice,
    pub queue: MtlCommandQueue,
    pub dispatcher: ComputeDispatcher,

    // Prefill (matmul)
    pub matmul_f16: MtlComputePipeline,
    pub matmul_q4: MtlComputePipeline,

    // Decode single (matvec batch=1)
    pub matvec_q4: MtlComputePipeline,
    pub matvec_ternary: MtlComputePipeline,
    pub matvec_q4k: MtlComputePipeline,

    // Decode batched (dequant-once-dot-many)
    pub matvec_q4_batch: MtlComputePipeline,
    pub matvec_ternary_batch: MtlComputePipeline,

    // Optimized / fused kernels
    pub matvec_q4_fast: MtlComputePipeline,
    pub fused_rope_qk: MtlComputePipeline,
    pub fused_kv_append: MtlComputePipeline,
    pub fused_add_norm: MtlComputePipeline,
    pub matvec_q4_fast_batch4: MtlComputePipeline,
    pub fused_qkv: MtlComputePipeline,
    pub fused_gate_up: MtlComputePipeline,

    // Transformer ops (all fp16)
    pub embed: MtlComputePipeline,
    pub rms_norm: MtlComputePipeline,
    pub rope: MtlComputePipeline,
    pub add_f16: MtlComputePipeline,
    pub silu_mul_f16: MtlComputePipeline,
    pub attention_decode: MtlComputePipeline,
    pub kv_append: MtlComputePipeline,
    pub kv_expand: MtlComputePipeline,
    pub f16_matvec: MtlComputePipeline,
    pub argmax: MtlComputePipeline,
}

impl MetalPipelines {
    /// Initialize Metal device and compile all MSL kernels.
    pub fn new() -> Result<Self, aruminium::MetalError> {
        let device = MtlDevice::system_default()?;
        log::info!("Metal: {}", device.name());
        log::info!(
            "  unified={}, max_buf={}GB, max_threads={:?}",
            device.has_unified_memory(),
            device.max_buffer_length() >> 30,
            device.max_threads_per_threadgroup(),
        );

        let queue = device.new_command_queue()?;
        let dispatcher = ComputeDispatcher::new(&queue);

        // Compile all kernels
        let compile = |src: &str, fname: &str| -> Result<MtlComputePipeline, aruminium::MetalError> {
            let lib = device.new_library_with_source(src)?;
            let func = lib.get_function(fname)?;
            device.new_compute_pipeline(&func)
        };

        use super::kernels;

        let matmul_f16 = compile(kernels::MATMUL_F16, "matmul_f16")?;
        let matmul_q4 = compile(kernels::MATMUL_Q4, "matmul_q4")?;
        let matvec_q4 = compile(kernels::MATVEC_Q4, "matvec_q4")?;
        let matvec_ternary = compile(kernels::MATVEC_TERNARY, "matvec_ternary")?;
        let matvec_q4k = compile(kernels::MATVEC_Q4K, "matvec_q4k")?;

        // Batch kernels: prepend #define BATCH 8
        let batch_src = |src: &str| format!("#define BATCH 8\n{}", src);
        let matvec_q4_batch = compile(&batch_src(kernels::MATVEC_Q4_BATCH), "matvec_q4_batch")?;
        let matvec_ternary_batch =
            compile(&batch_src(kernels::MATVEC_TERNARY_BATCH), "matvec_ternary_batch")?;

        // Optimized / fused kernels
        let matvec_q4_fast = compile(kernels::MATVEC_Q4_FAST, "matvec_q4_fast")?;
        let fused_rope_qk = compile(kernels::FUSED_ROPE, "fused_rope_qk")?;
        let fused_kv_append = compile(kernels::FUSED_KV_APPEND, "fused_kv_append")?;
        let fused_add_norm = compile(kernels::FUSED_ADD_NORM, "fused_add_norm_f16")?;
        let batch_src = |src: &str, b: u32| format!("#define BATCH {b}\n{src}");
        let matvec_q4_fast_batch4 = compile(&batch_src(kernels::MATVEC_Q4_FAST_BATCH, 4), "matvec_q4_fast_batch")?;
        let fused_qkv = compile(kernels::FUSED_QKV, "fused_qkv_q4")?;
        let fused_gate_up = compile(kernels::FUSED_GATE_UP, "fused_gate_up_q4")?;

        // Transformer ops
        let embed = compile(kernels::EMBED, "embed_f16")?;
        let rms_norm = compile(kernels::RMS_NORM, "rms_norm_f16")?;
        let rope = compile(kernels::ROPE, "rope_f16")?;
        let add_f16 = compile(kernels::ELEMENTWISE, "add_f16")?;
        let silu_mul_f16 = compile(kernels::ELEMENTWISE, "silu_mul_f16")?;
        let attention_decode = compile(kernels::ATTENTION, "attention_decode_f16")?;
        let kv_append = compile(kernels::KV_CACHE, "kv_append_f16")?;
        let kv_expand = compile(kernels::KV_CACHE, "kv_expand_f16")?;
        let f16_matvec = compile(kernels::F16_MATVEC, "f16_matvec")?;
        let argmax = compile(kernels::ARGMAX, "argmax_f16")?;

        log::info!("Metal: all 18 MSL kernels compiled");

        Ok(MetalPipelines {
            device,
            queue,
            dispatcher,
            matmul_f16,
            matmul_q4,
            matvec_q4,
            matvec_q4_fast,
            fused_rope_qk,
            fused_kv_append,
            fused_add_norm,
            matvec_q4_fast_batch4,
            fused_qkv,
            fused_gate_up,
            matvec_ternary,
            matvec_q4k,
            matvec_q4_batch,
            matvec_ternary_batch,
            embed,
            rms_norm,
            rope,
            add_f16,
            silu_mul_f16,
            attention_decode,
            kv_append,
            kv_expand,
            f16_matvec,
            argmax,
        })
    }

    /// Upload f16 data to a shared GPU buffer.
    pub fn upload_f16(&self, data: &[u16]) -> Result<MtlBuffer, aruminium::MetalError> {
        let bytes = bytemuck::cast_slice::<u16, u8>(data);
        self.device.new_buffer_with_data(bytes)
    }

    /// Upload f32 data to a shared GPU buffer.
    pub fn upload_f32(&self, data: &[f32]) -> Result<MtlBuffer, aruminium::MetalError> {
        let bytes = bytemuck::cast_slice::<f32, u8>(data);
        self.device.new_buffer_with_data(bytes)
    }

    /// Upload raw bytes to a shared GPU buffer.
    pub fn upload_bytes(&self, data: &[u8]) -> Result<MtlBuffer, aruminium::MetalError> {
        self.device.new_buffer_with_data(data)
    }

    /// Allocate an uninitialized shared GPU buffer.
    pub fn alloc(&self, size: usize) -> Result<MtlBuffer, aruminium::MetalError> {
        self.device.new_buffer(size)
    }
}
