//! Metal backend — MSL jets dispatched via aluminum
//!
//! Layer 2: knows ops (matmul, attention, norm), knows nothing about models.
//! Uses aluminum (layer 1) for device, buffer, pipeline, dispatch.
//!
//! Performance records (M1 Pro 16-core, GPU timestamps):
//!   matmul_f16:          3,708 GFLOPS sustained (87.9% of MMA ceiling)
//!   matmul_q4:           3,204 GFLOPS (prefill)
//!   matvec_q4 batch=8:     714 GFLOPS, 83 tok/s (2.4× llama.cpp)
//!   matvec_ternary b=8:    906 GOPS, 105 tok/s (3× llama.cpp)
//!
//! See RECORD.md for full optimization history (25 ideas tested).

pub mod kernels {
    // ── Matmul (prefill) ──
    pub const MATMUL_F16: &str = include_str!("kernels/matmul_f16.metal");
    pub const MATMUL_Q4: &str = include_str!("kernels/matmul_q4.metal");

    // ── Matvec single (decode batch=1) ──
    pub const MATVEC_Q4: &str = include_str!("kernels/matvec_q4.metal");
    pub const MATVEC_TERNARY: &str = include_str!("kernels/matvec_ternary.metal");
    pub const MATVEC_Q4K: &str = include_str!("kernels/matvec_q4k.metal");

    // ── Matvec batched (decode batch>1, dequant-once-dot-many) ──
    pub const MATVEC_Q4_BATCH: &str = include_str!("kernels/matvec_q4_batch.metal");
    pub const MATVEC_TERNARY_BATCH: &str = include_str!("kernels/matvec_ternary_batch.metal");
}
