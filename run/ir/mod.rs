//! Graph IR — fallback execution tier of the hybrid runtime.
//!
//! The hybrid strategy (specs/architecture.md) has three tiers:
//!   1. **Curated** — hand-written Rust forward paths per architecture
//!      family (see `arch/decoder/`).
//!   2. **Graph** — generic IR walked by [`GraphExecutor`]; handles models
//!      without a curated codepath.
//!   3. **Nox** — future trident-compiled bytecode VM.
//!
//! This module is tier 2. It is *not* dead code; it is the fallback that
//! makes the runtime "work for anything you can express as ops from
//! `specs/ops.md`."
//!
//! Submodules:
//! - [`graph`] — `Graph`, `Node`, `TensorMeta`, `WeightData`, `memory_plan`
//! - [`atoms`] — 8-atom basis + `decompose()` + `AtomInterpreter` (CPU scalar
//!               reference, also used for constant folding)
//! - [`jets`] — formula-hash → named fused-kernel registry
//! - [`fusion`] — load-time optimization passes (DCE, const-fold, structural
//!                fusers like `norm+matmul`, `skip+norm`, `swiglu`)
//! - [`templates`] — architecture-family graph builders (`transformer_decoder`
//!                   and variants; BERT/DiT/Whisper/MoE port pending)
//! - [`executor`] — graph walker (public API stub; full impl lands with the
//!                   GPU kernel shelf in Phase C)

pub mod atoms;
pub mod executor;
pub mod fusion;
pub mod graph;
pub mod jets;
pub mod templates;

pub use atoms::{decompose, Atom, AtomInterpreter, CmpOp, ReduceOp, SlidePattern};
pub use executor::{ExecConfig, ExecStats, GraphExecutor};
pub use fusion::{
    constant_fold, detect_fusions, fuse_norm_matmul, fuse_skip_norm, fuse_swiglu, optimize,
    FusionHint,
};
pub use graph::{
    AttrValue, Attrs, BackendHint, BufferLifetime, Dim, Graph, MemoryPlan, Node, Residency, Shape,
    TensorId, TensorMeta, WeightData,
};
pub use jets::{formula_hash, FormulaHash, Jet, JetRegistry};
pub use templates::{
    bert_encoder, cnn_detector, diffusion_dit, encoder_decoder, modernbert_encoder, moe_decoder,
    transformer_decoder, transformer_decoder_for_exec, transformer_encoder,
    whisper_encoder_decoder, Activation, BertConfig, CnnDetectorConfig, DiTConfig,
    MoeDecoderConfig, TransformerConfig, WhisperConfig,
};
