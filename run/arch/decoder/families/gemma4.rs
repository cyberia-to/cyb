//! Gemma 4.
//!
//! Departures from Gemma 3:
//!   - Standard RMSNorm (`w * x / rms`). The `(1 + w)` offset is dropped —
//!     see `Gemma4RMSNorm.forward` in HF transformers.
//!   - Attention score scaling = 1.0 (no sqrt divisor). Q and K are
//!     pre-normalised by `q_norm` / `k_norm` with learned weights, so their
//!     dot product is already bounded.
//!   - V projection additionally goes through RMSNorm without a learned
//!     scale (per-head pure rms divide) before the KV cache write.
//!   - Per-layer attention dimension switching (sliding vs full-attention
//!     layers use different `head_dim` / `num_key_value_heads`), per-kind
//!     RoPE (`rope_theta_full` + `partial_rotary_factor`), K=V shared
//!     projection for full-attention layers, per-layer scalar on the
//!     residual output. These live in `LlamaConfig` as direct fields since
//!     they vary per model checkpoint, not per family.
//!
//! Retained from Gemma 1/2/3:
//!   - Embedding scale × sqrt(hidden_size).
//!
//! Spec: specs/arch.md §LlamaStyle+.

use super::{AttnScale, FamilyProfile};

pub fn profile() -> FamilyProfile {
    FamilyProfile {
        name: "Gemma 4",
        rmsnorm_plus_one: false,
        scaled_embeddings: true,
        v_norm_per_head: true,
        attn_scale: AttnScale::Unity,
    }
}
