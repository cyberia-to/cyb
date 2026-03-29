//! Text generation — autoregressive loop with tokenizer

pub mod sampler;

use std::path::Path;
use std::sync::Arc;

use crate::backend::wgpu_backend::model::{NativeModel, argmax, sample_top_p};
use crate::backend::wgpu_backend::pipelines::Pipelines;

/// Text generator — model + tokenizer + generation loop
pub struct TextGenerator {
    model: NativeModel,
    tokenizer: tokenizers::Tokenizer,
    /// EOS token IDs (model-specific)
    eos_tokens: Vec<u32>,
}

impl TextGenerator {
    /// Create a new text generator from ONNX model + tokenizer paths
    pub fn new(
        model_path: &Path,
        tokenizer_path: &Path,
        pipelines: Arc<Pipelines>,
    ) -> Result<Self, String> {
        let model = NativeModel::load_from_onnx(model_path, pipelines)?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Tokenizer load failed: {e}"))?;

        // Common EOS tokens (Qwen3, Llama, GPT-2)
        let eos_tokens = vec![
            151643, // Qwen3 <|endoftext|>
            151645, // Qwen3 <|im_end|>
            2,      // Llama </s>
            50256,  // GPT-2 <|endoftext|>
        ];

        Ok(Self {
            model,
            tokenizer,
            eos_tokens,
        })
    }

    /// Create a new text generator from GGUF model + tokenizer paths
    pub fn new_gguf(
        model_path: &Path,
        tokenizer_path: &Path,
        pipelines: Arc<Pipelines>,
    ) -> Result<Self, String> {
        let model = NativeModel::load_from_gguf(model_path, pipelines)?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Tokenizer load failed: {e}"))?;

        // EOS tokens for common models
        let mut eos_tokens = vec![
            2,      // Llama </s>
            0,      // SmolLM <|endoftext|> (id 0)
            50256,  // GPT-2 <|endoftext|>
        ];
        if let Some(id) = tokenizer.token_to_id("<|endoftext|>") {
            if !eos_tokens.contains(&id) {
                eos_tokens.push(id);
            }
        }
        if let Some(id) = tokenizer.token_to_id("</s>") {
            if !eos_tokens.contains(&id) {
                eos_tokens.push(id);
            }
        }
        if let Some(id) = tokenizer.token_to_id("<|end_of_text|>") {
            if !eos_tokens.contains(&id) {
                eos_tokens.push(id);
            }
        }

        Ok(Self {
            model,
            tokenizer,
            eos_tokens,
        })
    }

    /// Create a new text generator from safetensors model + tokenizer paths
    pub fn new_safetensors(
        model_path: &Path,
        tokenizer_path: &Path,
        pipelines: Arc<Pipelines>,
    ) -> Result<Self, String> {
        let model = NativeModel::load_from_safetensors(model_path, pipelines)?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Tokenizer load failed: {e}"))?;

        // EOS tokens for common models
        let mut eos_tokens = vec![
            2,      // Llama </s>
            0,      // SmolLM <|endoftext|> (id 0)
            50256,  // GPT-2 <|endoftext|>
        ];
        // Try to detect EOS from tokenizer special tokens
        if let Some(id) = tokenizer.token_to_id("<|endoftext|>") {
            if !eos_tokens.contains(&id) {
                eos_tokens.push(id);
            }
        }
        if let Some(id) = tokenizer.token_to_id("</s>") {
            if !eos_tokens.contains(&id) {
                eos_tokens.push(id);
            }
        }

        Ok(Self {
            model,
            tokenizer,
            eos_tokens,
        })
    }

    /// Generate text from a prompt
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String, String> {
        // Enable GPU argmax for greedy decoding
        if temperature <= 0.0 {
            self.model.greedy_mode = true;
        }

        let encoding = self
            .tokenizer
            .encode(prompt, false)
            .map_err(|e| format!("Tokenization failed: {e}"))?;
        let mut token_ids: Vec<u32> = encoding.get_ids().to_vec();
        log::info!("Prompt tokens ({} tokens): {:?}", token_ids.len(), token_ids);

        // Print prompt
        print!("{prompt}");
        use std::io::Write;
        std::io::stdout().flush().ok();

        let gen_start = std::time::Instant::now();

        // Prefill: process prompt tokens
        let mut logits = Vec::new();
        for t in 0..token_ids.len() {
            logits = self.model.forward(&[token_ids[t]]);
        }

        // Autoregressive generation
        let mut generated = String::new();
        for step in 0..max_tokens {
            let next_token = if temperature <= 0.0 {
                argmax(&logits)
            } else {
                sample_top_p(&logits, temperature, 0.9)
            };

            if self.eos_tokens.contains(&next_token) {
                break;
            }

            token_ids.push(next_token);

            let decoded = self
                .tokenizer
                .decode(&[next_token], false)
                .unwrap_or_else(|_| "?".to_string());
            print!("{decoded}");
            std::io::stdout().flush().ok();
            generated.push_str(&decoded);

            logits = self.model.forward(&[next_token]);

            if step > 0 && step % 20 == 0 {
                let elapsed = gen_start.elapsed().as_secs_f64();
                let tps = (step as f64) / elapsed;
                log::debug!("Step {step}: {tps:.1} tok/s");
            }
        }

        let elapsed = gen_start.elapsed().as_secs_f64();
        let gen_count = token_ids.len() - encoding.get_ids().len();
        println!();
        println!("---");
        println!(
            "Generated {gen_count} tokens in {elapsed:.1}s ({:.1} tok/s)",
            gen_count as f64 / elapsed
        );

        Ok(generated)
    }

    /// Reset the KV cache for a new conversation
    pub fn reset(&mut self) {
        self.model.reset_kv_cache();
    }
}
