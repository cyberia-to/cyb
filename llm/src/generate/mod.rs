//! Text generation — autoregressive loop with tokenizer

pub mod sampler;

use std::path::Path;
use std::sync::Arc;

use crate::backend::wgpu::model::{NativeModel, argmax, sample_top_p};
use crate::backend::wgpu::pipelines::Pipelines;

/// Apply chat template if the model is instruction-tuned.
/// Reads [tokenizer] from config TOML to detect instruction models.
pub fn apply_chat_template(prompt: &str, config_toml: &str) -> String {
    // Instruction-tuned models with ChatML: eos_token = "<|im_end|>"
    let has_chatml = config_toml.lines().any(|l| {
        let t = l.trim();
        t.starts_with("eos_token") && t.contains("<|im_end|>")
    });
    if has_chatml {
        let formatted = format!(
            "<|im_start|>user\n{prompt}<|im_end|>\n<|im_start|>assistant\n"
        );
        log::info!("Chat template applied (ChatML format)");
        return formatted;
    }
    // No chat template — base model, use raw prompt
    prompt.to_string()
}

/// Detect EOS tokens from config TOML [tokenizer] section + tokenizer vocab
pub fn detect_eos_tokens(tokenizer: &tokenizers::Tokenizer, config_toml: &str) -> Vec<u32> {
    let mut eos = Vec::new();

    // 1. Read from config TOML [tokenizer] section
    // eos_id = 151645 or eos_token_ids = [151645, 151643]
    for line in config_toml.lines() {
        let t = line.trim();
        if t.starts_with("eos_id") && !t.starts_with("eos_id ") || t.starts_with("eos_id =") {
            if let Some(val) = t.split('=').nth(1) {
                if let Ok(id) = val.trim().parse::<u32>() {
                    if !eos.contains(&id) { eos.push(id); }
                }
            }
        }
        if t.starts_with("eos_token_ids") {
            // Parse array like [151645, 151643]
            if let Some(arr) = t.split('=').nth(1) {
                let arr = arr.trim().trim_start_matches('[').trim_end_matches(']');
                for part in arr.split(',') {
                    if let Ok(id) = part.trim().parse::<u32>() {
                        if !eos.contains(&id) { eos.push(id); }
                    }
                }
            }
        }
    }

    // 2. Fallback: common hardcoded IDs
    for &id in &[2u32, 50256] {
        if !eos.contains(&id) {
            eos.push(id);
        }
    }

    // 3. Lookup special tokens in tokenizer
    for special in &["<|endoftext|>", "</s>", "<|end_of_text|>", "<|eot_id|>", "<|im_end|>"] {
        if let Some(id) = tokenizer.token_to_id(special) {
            if !eos.contains(&id) {
                eos.push(id);
            }
        }
    }

    log::info!("EOS tokens: {:?}", eos);
    eos
}

/// Stats from the last generate() call
#[derive(Default, Clone)]
pub struct GenerateStats {
    pub decode_tokens: u32,
    pub decode_ms: u64,
    pub total_ms: u64,
}

/// Text generator — model + tokenizer + generation loop
pub struct TextGenerator {
    model: NativeModel,
    tokenizer: tokenizers::Tokenizer,
    eos_tokens: Vec<u32>,
    stats: GenerateStats,
}

impl TextGenerator {
    /// Create a text generator from .model file — tokenizer built from embedded vocab.
    pub fn new(
        model_path: &Path,
        pipelines: Arc<Pipelines>,
    ) -> Result<Self, String> {
        let (model, vocab_str) = NativeModel::load(model_path, pipelines)?;

        let tokenizer = crate::cyb_format::build_tokenizer_from_vocab(&vocab_str)?;

        // Read config for EOS detection (fast text-only read)
        let config_toml = crate::cyb_format::read_model_config(model_path)
            .map(|(_, cfg)| cfg)
            .unwrap_or_default();
        let eos_tokens = detect_eos_tokens(&tokenizer, &config_toml);

        Ok(Self {
            model,
            tokenizer,
            eos_tokens,
            stats: GenerateStats::default(),
        })
    }

    /// Generate text from a prompt
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String, String> {
        self.stats = GenerateStats::default();
        // Enable GPU argmax for greedy decoding
        if temperature <= 0.0 {
            self.model.greedy_mode = true;
        }

        let encoding = self
            .tokenizer
            .encode(prompt, true)  // add_special_tokens=true (BOS)
            .map_err(|e| format!("Tokenization failed: {e}"))?;
        let mut token_ids: Vec<u32> = encoding.get_ids().to_vec();
        log::info!("Prompt tokens ({} tokens): {:?}", token_ids.len(), token_ids);

        // Print prompt
        print!("{prompt}");
        use std::io::Write;
        std::io::stdout().flush().ok();

        let prefill_start = std::time::Instant::now();

        // Prefill: process prompt tokens
        let mut logits = Vec::new();
        for t in 0..token_ids.len() {
            logits = self.model.forward(&[token_ids[t]]);
        }

        let prefill_ms = prefill_start.elapsed().as_secs_f64() * 1000.0;
        log::info!("Prefill: {} tokens in {:.1}ms ({:.1} ms/tok)", token_ids.len(), prefill_ms, prefill_ms / token_ids.len() as f64);

        let decode_start = std::time::Instant::now();

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
                let elapsed = decode_start.elapsed().as_secs_f64();
                let tps = (step as f64) / elapsed;
                log::debug!("Step {step}: {tps:.1} tok/s");
            }
        }

        let decode_s = decode_start.elapsed().as_secs_f64();
        let total_s = prefill_ms / 1000.0 + decode_s;
        let gen_count = token_ids.len() - encoding.get_ids().len();
        let decode_tps = if decode_s > 0.0 { gen_count as f64 / decode_s } else { 0.0 };
        println!();
        println!("---");
        println!(
            "Prefill: {:.0}ms | Decode: {gen_count} tokens in {decode_s:.2}s ({decode_tps:.1} tok/s) | Total: {total_s:.2}s",
            prefill_ms
        );

        self.stats = GenerateStats {
            decode_tokens: gen_count as u32,
            decode_ms: (decode_s * 1000.0) as u64,
            total_ms: (total_s * 1000.0) as u64,
        };

        Ok(generated)
    }

    /// Generate without printing to stdout — for benchmarks/status
    pub fn generate_quiet(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String, String> {
        self.stats = GenerateStats::default();
        if temperature <= 0.0 {
            self.model.greedy_mode = true;
        }

        let encoding = self.tokenizer.encode(prompt, true)
            .map_err(|e| format!("Tokenization failed: {e}"))?;
        let mut token_ids: Vec<u32> = encoding.get_ids().to_vec();

        let mut logits = Vec::new();
        for t in 0..token_ids.len() {
            logits = self.model.forward(&[token_ids[t]]);
        }

        let decode_start = std::time::Instant::now();
        let mut generated = String::new();
        for _step in 0..max_tokens {
            let next_token = if temperature <= 0.0 { argmax(&logits) } else { sample_top_p(&logits, temperature, 0.9) };
            if self.eos_tokens.contains(&next_token) { break; }
            token_ids.push(next_token);
            let decoded = self.tokenizer.decode(&[next_token], false).unwrap_or_default();
            generated.push_str(&decoded);
            logits = self.model.forward(&[next_token]);
        }

        let decode_s = decode_start.elapsed().as_secs_f64();
        let gen_count = token_ids.len() - encoding.get_ids().len();
        self.stats = GenerateStats {
            decode_tokens: gen_count as u32,
            decode_ms: (decode_s * 1000.0) as u64,
            total_ms: (decode_s * 1000.0) as u64,
        };
        Ok(generated)
    }

    /// Stats from the last generate() call
    pub fn last_stats(&self) -> &GenerateStats {
        &self.stats
    }

    /// Reset the KV cache for a new conversation
    pub fn reset(&mut self) {
        self.model.reset_kv_cache();
    }
}
