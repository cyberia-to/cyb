//! Text generation — autoregressive loop with tokenizer

pub mod sampler;

use std::path::Path;
use std::sync::Arc;

use crate::backend::wgpu::model::{NativeModel, argmax, sample_top_p};
use crate::backend::wgpu::pipelines::Pipelines;

/// Apply chat template if the model is instruction-tuned.
/// Returns the formatted prompt or the original if no template applies.
pub fn apply_chat_template(prompt: &str, model_dir: &Path) -> String {
    // Read tokenizer_config.json for chat_template
    let tc_path = model_dir.join("tokenizer_config.json");
    if let Ok(data) = std::fs::read_to_string(&tc_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if json.get("chat_template").and_then(|v| v.as_str()).is_some() {
                // Model has a chat template — apply ChatML format
                // (covers Qwen2/2.5, SmolLM2-Instruct, Llama3, BitNet)
                let formatted = format!(
                    "<|im_start|>user\n{prompt}<|im_end|>\n<|im_start|>assistant\n"
                );
                log::info!("Chat template applied (ChatML format)");
                return formatted;
            }
        }
    }
    // No chat template — base model, use raw prompt
    prompt.to_string()
}

/// Detect EOS tokens from tokenizer + generation_config.json
pub fn detect_eos_tokens(tokenizer: &tokenizers::Tokenizer, model_dir: &Path) -> Vec<u32> {
    let mut eos = Vec::new();

    // 1. Read generation_config.json (authoritative source)
    let gen_config_path = model_dir.join("generation_config.json");
    if let Ok(data) = std::fs::read_to_string(&gen_config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(eos_id) = json.get("eos_token_id") {
                match eos_id {
                    serde_json::Value::Number(n) => {
                        if let Some(id) = n.as_u64() {
                            eos.push(id as u32);
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        for v in arr {
                            if let Some(id) = v.as_u64() {
                                eos.push(id as u32);
                            }
                        }
                    }
                    _ => {}
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

/// Text generator — model + tokenizer + generation loop
pub struct TextGenerator {
    model: NativeModel,
    tokenizer: tokenizers::Tokenizer,
    /// EOS token IDs (model-specific)
    eos_tokens: Vec<u32>,
}

impl TextGenerator {
    /// Create a text generator from .cyb file — the canonical constructor.
    pub fn new(
        cyb_path: &Path,
        tokenizer_path: &Path,
        pipelines: Arc<Pipelines>,
    ) -> Result<Self, String> {
        let model = NativeModel::load(cyb_path, pipelines)?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Tokenizer load failed: {e}"))?;

        let model_dir = cyb_path.parent().unwrap_or(Path::new("."));
        let eos_tokens = detect_eos_tokens(&tokenizer, model_dir);

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

        let model_dir = model_path.parent().unwrap_or(Path::new("."));
        let eos_tokens = detect_eos_tokens(&tokenizer, model_dir);

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

        let model_dir = model_path.parent().unwrap_or(Path::new("."));
        let eos_tokens = detect_eos_tokens(&tokenizer, model_dir);

        Ok(Self {
            model,
            tokenizer,
            eos_tokens,
        })
    }

    /// Create a new text generator from .cyb model file
    pub fn new_cyb(
        cyb_path: &Path,
        tokenizer_path: &Path,
        pipelines: Arc<Pipelines>,
    ) -> Result<Self, String> {
        let model = NativeModel::load_from_cyb(cyb_path, pipelines)?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Tokenizer load failed: {e}"))?;

        let model_dir = cyb_path.parent().unwrap_or(Path::new("."));
        let eos_tokens = detect_eos_tokens(&tokenizer, model_dir);

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

        Ok(generated)
    }

    /// Reset the KV cache for a new conversation
    pub fn reset(&mut self) {
        self.model.reset_kv_cache();
    }
}
