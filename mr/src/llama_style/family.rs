//! Family profile: the per-variant quirks of a LlamaStyle(+) model
//! collapsed into one struct.
//!
//! Each transformer family (Llama, Qwen, Gemma 1/2/3, Gemma 4, …) diverges
//! from the LlamaStyle baseline in a small number of specific ways —
//! normalisation format, embedding scaling, attention scaling, V-norm,
//! etc. Keeping those as ad-hoc `model_type.starts_with("gemma")` branches
//! sprinkled through the runtime does not scale.
//!
//! `FamilyProfile` captures every variant axis in one struct, populated
//! once at config parse time. The runtime reads its fields; no string
//! matching in the hot path.
//!
//! Spec: reference/runtime/arch.md §LlamaStyle / §LlamaStyle+

/// How Q·K^T is scaled before softmax.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttnScale {
    /// Standard transformer: 1 / sqrt(head_dim). Llama, Qwen, Mistral, Phi.
    PerHeadDim,
    /// Fixed divisor independent of head_dim. Gemma 3 uses
    /// `query_pre_attn_scalar` (default 256) regardless of per-layer head_dim.
    FixedDivisor(usize),
    /// No extra scaling — Q and K are pre-normalised by q_norm / k_norm
    /// so the dot product is already bounded (Gemma 4).
    Unity,
}

/// Per-family deviations from LlamaStyle baseline.
#[derive(Clone, Debug)]
pub struct FamilyProfile {
    /// RMSNorm applies `(1 + w) * x / rms` instead of `w * x / rms`.
    /// Gemma 1 / 2 / 3 store norm weights as offsets from 1; Gemma 4 reverts
    /// to standard `w * x / rms`; Llama/Qwen/Mistral/etc. never use the +1
    /// form. Encoded as a transform applied at weight-load time so the
    /// runtime stays on one RmsNorm codepath.
    pub rmsnorm_plus_one: bool,

    /// Multiply looked-up input embeddings by sqrt(hidden_size).
    /// Every Gemma version does this; Llama/Qwen/etc. do not.
    pub scaled_embeddings: bool,

    /// Apply RMSNorm-without-scale (a pure rms divide, no learned weight)
    /// to V per head before the KV cache write. Gemma 4 unique.
    pub v_norm_per_head: bool,

    /// How attention scores are scaled before softmax.
    pub attn_scale: AttnScale,
}

impl FamilyProfile {
    /// Derive the profile from the `.model`'s `model_type` string plus
    /// optional config overrides. `query_pre_attn_scalar` is only consulted
    /// when the family uses `AttnScale::FixedDivisor` (Gemma 2/3).
    pub fn for_model_type(model_type: &str, query_pre_attn_scalar: Option<usize>) -> Self {
        match model_type {
            // Gemma 4 reverts RMSNorm to the standard formula and relies on
            // q_norm / k_norm to bound Q·K^T — the Unity attention scale.
            "gemma4" | "gemma4_text" => Self {
                rmsnorm_plus_one: false,
                scaled_embeddings: true,
                v_norm_per_head: true,
                attn_scale: AttnScale::Unity,
            },
            // Gemma 1 / 2 / 3: RmsNorm stores `w - 1`; fixed attention divisor.
            "gemma" | "gemma2" | "gemma3" | "gemma3_text" => Self {
                rmsnorm_plus_one: true,
                scaled_embeddings: true,
                v_norm_per_head: false,
                attn_scale: AttnScale::FixedDivisor(query_pre_attn_scalar.unwrap_or(256)),
            },
            // LlamaStyle baseline — Llama, Qwen, Mistral, Phi, SmolLM, etc.
            _ => Self {
                rmsnorm_plus_one: false,
                scaled_embeddings: false,
                v_norm_per_head: false,
                attn_scale: AttnScale::PerHeadDim,
            },
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
