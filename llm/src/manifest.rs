//! Model manifest — source of truth for cyb-llm runtime.
//!
//! Three MVP target models covering the product range.
//! Manifest drives status, fetch, and serve commands.

use std::path::PathBuf;

/// Where local models live
pub fn models_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join("llm")
}

/// Model role in the soma architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Router,     // always loaded, classifies queries
    Specialist, // on-demand, domain-specific
    General,    // on-demand, broad capability
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Router => write!(f, "router"),
            Self::Specialist => write!(f, "specialist"),
            Self::General => write!(f, "general"),
        }
    }
}

/// A model in the manifest
#[derive(Debug, Clone)]
pub struct ModelSpec {
    /// Name on disk: ~/llm/{name}.model
    pub name: &'static str,
    /// HuggingFace repo (base model for tokenizer)
    pub hf_repo: &'static str,
    /// Ollama tag for benchmark comparison (None = not in ollama)
    pub ollama_tag: Option<&'static str>,
    /// Role in soma routing
    pub role: Role,
    /// Expected RAM in MB (Q4)
    pub ram_mb: u32,
    /// Short description
    pub notes: &'static str,
}

/// MVP manifest — 3 target models
pub const MANIFEST: &[ModelSpec] = &[
    ModelSpec {
        name: "qwen3-0.6b-abl",
        hf_repo: "Qwen/Qwen3-0.6B",
        ollama_tag: Some("qwen3:0.6b"),
        role: Role::Router,
        ram_mb: 350,
        notes: "always on, classifies queries, 300+ tok/s target",
    },
    ModelSpec {
        name: "qwen2.5-coder-14b-abl",
        hf_repo: "Qwen/Qwen2.5-Coder-14B",
        ollama_tag: Some("qwen2.5-coder:14b"),
        role: Role::Specialist,
        ram_mb: 9000,
        notes: "code generation, Q4, 25+ tok/s target",
    },
    ModelSpec {
        name: "gemma-4-31b",
        hf_repo: "google/gemma-4-31B",
        ollama_tag: None, // not yet in ollama
        role: Role::General,
        ram_mb: 18000,
        notes: "multimodal, 60 layers, 30+ tok/s target",
    },
];

/// Format byte count as human-readable size
pub fn format_size(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        format!("{:.1}G", bytes as f64 / (1u64 << 30) as f64)
    } else if bytes >= 1 << 20 {
        format!("{}M", bytes >> 20)
    } else if bytes >= 1 << 10 {
        format!("{}K", bytes >> 10)
    } else {
        format!("{}B", bytes)
    }
}
