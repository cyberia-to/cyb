//! Gemma 1 / 2 / 3.
//!
//! Shared quirks relative to LlamaStyle baseline:
//!   - RMSNorm uses `(1 + w) * x / rms` — weights stored as offsets from 1.
//!     Baked in at load so the runtime keeps one `Op::RmsNorm` codepath.
//!   - Input embeddings multiplied by sqrt(hidden_size) on lookup.
//!   - Attention scores scaled by 1/sqrt(query_pre_attn_scalar) (default 256),
//!     independent of per-layer head_dim. HF Gemma3TextConfig default.
//!
//! Gemma 3 also adds: sliding-window attention, final-logit softcapping,
//! GELU (instead of SiLU), post-attn / post-FFN norms — those are carried
//! as plain `LlamaConfig` fields (`sliding_window`, `final_logit_softcapping`,
//! `hidden_activation`, `post_attn_norm` tensor presence) because they are
//! configurable per model, not per family.
//!
//! Spec: reference/runtime/arch.md §LlamaStyle+.

use super::{AttnScale, FamilyProfile};

pub fn profile(query_pre_attn_scalar: Option<usize>) -> FamilyProfile {
    FamilyProfile {
        name: "Gemma 1/2/3",
        rmsnorm_plus_one: true,
        scaled_embeddings: true,
        v_norm_per_head: false,
        attn_scale: AttnScale::FixedDivisor(query_pre_attn_scalar.unwrap_or(256)),
    }
}
