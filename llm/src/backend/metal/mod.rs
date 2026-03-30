//! Metal backend — MSL jets dispatched via aruminium
//!
//! Layer 2: knows ops (matmul, attention, norm), knows nothing about models.
//! Uses aruminium (layer 1) for device, buffer, pipeline, dispatch.
//!
//! Performance records (M1 Pro 16-core, GPU timestamps):
//!   matmul_f16:          3,708 GFLOPS sustained (87.9% of MMA ceiling)
//!   matmul_q4:           3,204 GFLOPS (prefill)
//!   matvec_q4 batch=8:     714 GFLOPS, 83 tok/s (2.4x llama.cpp)
//!   matvec_ternary b=8:    906 GOPS, 105 tok/s (3x llama.cpp)
//!
//! See RECORD.md for full optimization history (25 ideas tested).

pub mod kernels {
    // Matmul (prefill)
    pub const MATMUL_F16: &str = include_str!("kernels/matmul_f16.metal");
    pub const MATMUL_Q4: &str = include_str!("kernels/matmul_q4.metal");

    // Matvec single (decode batch=1)
    pub const MATVEC_Q4: &str = include_str!("kernels/matvec_q4.metal");
    pub const MATVEC_TERNARY: &str = include_str!("kernels/matvec_ternary.metal");
    pub const MATVEC_Q4K: &str = include_str!("kernels/matvec_q4k.metal");

    // Matvec batched (decode batch>1, dequant-once-dot-many)
    pub const MATVEC_Q4_BATCH: &str = include_str!("kernels/matvec_q4_batch.metal");
    pub const MATVEC_TERNARY_BATCH: &str = include_str!("kernels/matvec_ternary_batch.metal");

    // Optimized kernels
    pub const MATVEC_Q4_FAST: &str = include_str!("kernels/matvec_q4_fast.metal");
    pub const FUSED_ROPE: &str = include_str!("kernels/fused_rope.metal");
    pub const FUSED_KV_APPEND: &str = include_str!("kernels/fused_kv_append.metal");
    pub const FUSED_ADD_NORM: &str = include_str!("kernels/fused_add_norm.metal");
    pub const MATVEC_Q4_FAST_BATCH: &str = include_str!("kernels/matvec_q4_fast_batch.metal");

    // Transformer ops (all fp16)
    pub const EMBED: &str = include_str!("kernels/embed.metal");
    pub const RMS_NORM: &str = include_str!("kernels/rms_norm.metal");
    pub const ROPE: &str = include_str!("kernels/rope.metal");
    pub const ELEMENTWISE: &str = include_str!("kernels/elementwise.metal");
    pub const ATTENTION: &str = include_str!("kernels/attention.metal");
    pub const KV_CACHE: &str = include_str!("kernels/kv_cache.metal");
    pub const F16_MATVEC: &str = include_str!("kernels/f16_matvec.metal");
    pub const ARGMAX: &str = include_str!("kernels/argmax.metal");
}

pub mod pipelines;
pub mod dispatch;
pub mod model;

pub use pipelines::MetalPipelines;
pub use model::MetalModel;
