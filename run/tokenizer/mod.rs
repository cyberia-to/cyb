//! Tokenizer — BPE + byte-level + chat templates.
//!
//! Spec: specs/tokenizer.md

mod bpe;
mod byte_level;
mod chat;
mod loader;

pub use bpe::Bpe;
pub use chat::{apply_chat_template, ChatMessage};
pub use loader::build_tokenizer;

/// High-level tokenizer API: owns BPE + optional chat template.
pub struct Tokenizer {
    pub bpe: Bpe,
    pub eos_token_ids: Vec<u32>,
    pub chat_format: Option<String>,
    pub chat_template: Option<String>,
}

impl Tokenizer {
    pub fn encode(&self, text: &str) -> Vec<u32> {
        self.bpe.encode(text)
    }

    pub fn decode(&self, ids: &[u32], skip_special: bool) -> String {
        self.bpe.decode(ids, skip_special)
    }

    pub fn apply_chat_template(
        &self,
        messages: &[ChatMessage],
        add_generation_prompt: bool,
    ) -> String {
        apply_chat_template(
            self.chat_format.as_deref(),
            self.chat_template.as_deref(),
            messages,
            add_generation_prompt,
        )
    }

    pub fn is_eos(&self, id: u32) -> bool {
        self.eos_token_ids.contains(&id)
    }
}
