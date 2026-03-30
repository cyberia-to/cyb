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

        log::info!("Metal: all 7 MSL kernels compiled");

        Ok(MetalPipelines {
            device,
            queue,
            dispatcher,
            matmul_f16,
            matmul_q4,
            matvec_q4,
            matvec_ternary,
            matvec_q4k,
            matvec_q4_batch,
            matvec_ternary_batch,
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
