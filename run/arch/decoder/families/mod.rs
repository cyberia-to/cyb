//! Model families.
//!
//! Each transformer family (Llama, Qwen, Mistral, Gemma 1/2/3, Gemma 4, …)
//! diverges from the LlamaStyle baseline along a small number of concrete
//! axes: normalisation formula, embedding scaling, attention scaling,
//! V-norm, per-layer dim switching, etc. Rather than scatter
//! `model_type.starts_with("gemma")` branches through the runtime, each
//! family gets its own file here that returns a `FamilyProfile`.
//!
//! Adding a new family:
//!   1. Drop a new file into `families/` with `pub fn profile(...) -> FamilyProfile`.
//!   2. Add one match arm to `for_model_type` below.
//!   3. Done — the runtime reads the profile through `config.family.*`.
//!
//! Adding a new variant axis (e.g. `shared_qk_weight`):
//!   1. Add a field to `FamilyProfile`.
//!   2. Set it in each family's `profile()`.
//!   3. Read `config.family.shared_qk_weight` at the one site that cares.
//!
//! Spec: specs/arch.md §LlamaStyle / §LlamaStyle+.

mod baseline;
mod gemma;
mod gemma4;

/// How Q·K^T is scaled before softmax.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttnScale {
    /// Standard transformer: 1 / sqrt(layer_head_dim). Llama, Qwen, Mistral, Phi.
    PerHeadDim,
    /// Fixed divisor independent of head_dim. Gemma 2/3 use
    /// `query_pre_attn_scalar` (default 256) regardless of per-layer head_dim.
    FixedDivisor(usize),
    /// No extra scaling — Q and K are pre-normalised by q_norm / k_norm so
    /// their dot product is already bounded (Gemma 4).
    Unity,
}

/// Per-family deviations from the LlamaStyle baseline.
///
/// Populated once at config parse time by `for_model_type`. All runtime
/// code reads fields here — no string matching on `model_type` anywhere
/// outside the family files.
#[derive(Clone, Debug)]
pub struct FamilyProfile {
    /// Human-readable name for logs / `mr status` output.
    pub name: &'static str,
    /// RMSNorm applies `(1 + w) * x / rms` instead of `w * x / rms`.
    /// Gemma 1/2/3 store norm weights as offsets from 1; Gemma 4 and
    /// Llama/Qwen/Mistral use standard `w * x / rms`. Baked in at weight
    /// load so the runtime stays on one `Op::RmsNorm` codepath.
    pub rmsnorm_plus_one: bool,
    /// Multiply input embeddings by sqrt(hidden_size) on lookup.
    /// Every Gemma version; Llama / Qwen do not.
    pub scaled_embeddings: bool,
    /// Apply RMSNorm-without-scale (pure rms divide, no learned weight)
    /// to V per head before the KV-cache write. Gemma 4 unique.
    pub v_norm_per_head: bool,
    /// How attention scores are scaled before softmax.
    pub attn_scale: AttnScale,
}

impl FamilyProfile {
    /// Dispatch on the `.model`'s `model_type` string. `query_pre_attn_scalar`
    /// is the config override (consulted only by families that use
    /// `AttnScale::FixedDivisor`).
    pub fn for_model_type(model_type: &str, query_pre_attn_scalar: Option<usize>) -> Self {
        match model_type {
            "gemma4" | "gemma4_text" => gemma4::profile(),
            "gemma" | "gemma2" | "gemma3" | "gemma3_text" => gemma::profile(query_pre_attn_scalar),
            _ => baseline::profile(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llama_baseline() {
        let p = FamilyProfile::for_model_type("llama", None);
        assert!(!p.rmsnorm_plus_one);
        assert!(!p.scaled_embeddings);
        assert!(!p.v_norm_per_head);
        assert!(matches!(p.attn_scale, AttnScale::PerHeadDim));
    }

    #[test]
    fn qwen3_matches_llama_baseline() {
        let p = FamilyProfile::for_model_type("qwen3", None);
        assert!(!p.rmsnorm_plus_one);
        assert!(!p.scaled_embeddings);
        assert!(matches!(p.attn_scale, AttnScale::PerHeadDim));
    }

    #[test]
    fn gemma3_flips_norm_and_embed_scale() {
        let p = FamilyProfile::for_model_type("gemma3", None);
        assert!(p.rmsnorm_plus_one);
        assert!(p.scaled_embeddings);
        assert!(!p.v_norm_per_head);
        assert!(matches!(p.attn_scale, AttnScale::FixedDivisor(256)));
    }

    #[test]
    fn gemma3_respects_query_pre_attn_override() {
        let p = FamilyProfile::for_model_type("gemma3", Some(128));
        assert!(matches!(p.attn_scale, AttnScale::FixedDivisor(128)));
    }

    #[test]
    fn gemma4_unity_attn_scale_and_v_norm() {
        let p = FamilyProfile::for_model_type("gemma4", None);
        assert!(!p.rmsnorm_plus_one);
        assert!(p.scaled_embeddings);
        assert!(p.v_norm_per_head);
        assert!(matches!(p.attn_scale, AttnScale::Unity));
    }
}
