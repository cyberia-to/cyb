//! LlamaStyle curated codepath.
//!
//! Covers: Llama, Mistral, Qwen2/2.5/3, Phi, Gemma 1/2, SmolLM, DeepSeek-dense,
//! StarCoder, MiMo, NuExtract (config-driven).
//!
//! Spec: reference/runtime/arch.md#llamastyle

pub mod config;
pub mod family;
mod forward;
mod weights;

pub use config::LlamaConfig;
pub use family::{AttnScale, FamilyProfile};
pub use forward::LlamaModel;
pub use weights::LayerWeights;
