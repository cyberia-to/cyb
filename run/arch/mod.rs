//! Architecture implementations.
//!
//! Each submodule is one compute graph family — the forward-pass logic,
//! weight layout, and config for a class of models.
//!
//! Current:
//!   decoder — causal decoder (Llama, Qwen, Gemma, Phi, Mistral, …)
//!
//! Future:
//!   encoder         — masked encoder (DeBERTa, ModernBERT, Jina)
//!   encoder_decoder — sequence-to-sequence (Whisper, T5)
//!   diffusion       — latent diffusion (Flux, Wan)

pub mod decoder;

pub use decoder::{LlamaConfig, LlamaModel, LayerWeights};
