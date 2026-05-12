//! Manifest of models this runtime targets.
//!
//! Single source of truth for what `mr status` tests.

pub struct Entry {
    pub name: &'static str,
    pub family: &'static str,
    pub role: &'static str,
    pub notes: &'static str,
    /// Ollama tag for the LLAMA reference column (None = not in ollama)
    pub ollama_tag: Option<&'static str>,
    /// Which quality check to run in `mr status`
    pub eval: EvalKind,
}

#[derive(Copy, Clone)]
pub enum EvalKind {
    /// Chat: "What is 2+2?" — answer must contain "4"
    Math,
    /// Code completion (no-chat): "def fibonacci(n):\n" — answer must look like code
    Code,
}

pub const MANIFEST: &[Entry] = &[
    Entry {
        name: "qwen3-0.6b-abl",
        family: "LlamaStyle",
        role: "router",
        notes: "always-on classifier, qwen3 with qk_norm",
        ollama_tag: Some("qwen3:0.6b"),
        eval: EvalKind::Math,
    },
    Entry {
        name: "qwen2.5-coder-1.5b-abl",
        family: "LlamaStyle",
        role: "code",
        notes: "small code model, qwen2 with attn_bias",
        ollama_tag: Some("qwen2.5-coder:1.5b"),
        eval: EvalKind::Code,
    },
    Entry {
        name: "qwen2.5-coder-14b-abl",
        family: "LlamaStyle",
        role: "code",
        notes: "large code model, needs fused Q4_K matmul to load",
        ollama_tag: Some("qwen2.5-coder:14b"),
        eval: EvalKind::Code,
    },
    Entry {
        name: "gemma-4-31b",
        family: "LlamaStyle+",
        role: "general",
        notes: "Gemma 3/4 extensions required (softcapping, sliding window, K=V)",
        ollama_tag: None,
        eval: EvalKind::Math,
    },
];

pub fn models_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("llm")
}
