//! Causal decoder architecture.
//!
//! One compute graph covers all modern decoder-only transformers:
//! Llama, Mistral, Qwen 2/2.5/3, Phi 2/3/4, Gemma 1/2/3/4, SmolLM,
//! DeepSeek-dense, StarCoder, MiMo, NuExtract.
//!
//! Per-family behavioral differences (norm formula, embedding scale,
//! attention scale, v-norm) are captured in `families/` — no string
//! matching on model_type outside that folder.
//!
//! Spec: specs/arch.md §decoder

pub mod config;
pub mod families;
mod forward;
mod weights;

pub use config::LlamaConfig;
pub use forward::LlamaModel;
pub use weights::LayerWeights;
