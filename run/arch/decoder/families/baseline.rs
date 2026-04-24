//! LlamaStyle baseline.
//!
//! Covers: Llama 2 / 3, Mistral, Qwen 2 / 2.5 / 3, Phi 2 / 3 / 4, SmolLM / SmolLM 2,
//! DeepSeek-LLM (dense), StarCoder 2, MiMo, NuExtract, Yi.
//!
//! Variants within this family (attention bias, q/k-norm, tied embeddings,
//! different RoPE theta / head_dim) are captured by tensor presence and
//! plain config fields — no profile-level flags needed.
//!
//! Spec: specs/arch.md §LlamaStyle.

use super::{AttnScale, FamilyProfile};

pub fn profile() -> FamilyProfile {
    FamilyProfile {
        name: "LlamaStyle",
        rmsnorm_plus_one: false,
        scaled_embeddings: false,
        v_norm_per_head: false,
        attn_scale: AttnScale::PerHeadDim,
    }
}
