pub mod sampler;

use std::collections::HashMap;
use std::path::Path;

use crate::graph::OnnxExecutor;
use crate::graph::value::Value;

use crate::Backend;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

pub struct TextGenerator {
    executor: OnnxExecutor,
    tokenizer: tokenizers::Tokenizer,
    device: Device,
    /// Number of KV-cache layers (auto-detected from model outputs)
    num_kv_layers: usize,
}

impl TextGenerator {
    pub fn new(
        model_path: &Path,
        tokenizer_path: &Path,
        device: Device,
    ) -> Result<Self, String> {
        let mut executor = OnnxExecutor::from_file(model_path)?;
        executor.load_from_file(model_path, &device)?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Failed to load tokenizer: {e}"))?;

        // Auto-detect number of KV-cache layers from model outputs
        let num_kv_layers = executor.graph_output_names()
            .iter()
            .filter(|n| n.starts_with("present.") && n.ends_with(".key"))
            .count();

        log::info!("Detected {num_kv_layers} KV-cache layers");

        Ok(Self {
            executor,
            tokenizer,
            device,
            num_kv_layers,
        })
    }

    /// Generate text from a prompt
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String, String> {
        let encoding = self.tokenizer.encode(prompt, false)
            .map_err(|e| format!("Tokenization failed: {e}"))?;
        let mut token_ids: Vec<u32> = encoding.get_ids().to_vec();

        log::info!("Prompt tokens: {:?}", token_ids);

        let mut generated_text = String::new();
        let mut kv_cache: HashMap<String, Value> = HashMap::new();
        let use_kv_cache = self.num_kv_layers > 0;

        for step in 0..max_tokens {
            self.executor.clear_intermediates();

            // On step 0 (prefill): send all tokens
            // On step 1+: send only the last token (KV-cache has the rest)
            let input_tokens = if step == 0 || !use_kv_cache {
                &token_ids[..]
            } else {
                &token_ids[token_ids.len() - 1..]
            };

            let seq_len = input_tokens.len();
            let total_seq = token_ids.len(); // includes past tokens
            let input_floats: Vec<f32> = input_tokens.iter().map(|&id| id as f32).collect();

            // Position IDs: for prefill [0,1,...,seq-1], for decode [total_seq-1]
            let pos_start = if step == 0 || !use_kv_cache { 0 } else { total_seq - 1 };
            let pos_ids: Vec<f32> = (0..seq_len).map(|i| (pos_start + i) as f32).collect();

            let mut inputs = HashMap::new();
            inputs.insert("input_ids".to_string(),
                Value::from_data(input_floats, vec![1, seq_len], &self.device));
            inputs.insert("attention_mask".to_string(),
                Value::from_data(vec![1.0f32; total_seq], vec![1, total_seq], &self.device));
            inputs.insert("position_ids".to_string(),
                Value::from_data(pos_ids, vec![1, seq_len], &self.device));

            // Feed KV-cache from previous step
            if use_kv_cache && step > 0 {
                for (name, val) in &kv_cache {
                    inputs.insert(name.clone(), val.clone());
                }
            }

            // Forward pass
            let outputs = self.executor.run(inputs, &self.device)?;

            // Get logits
            let logits = outputs.get("logits")
                .ok_or("No logits in output")?;

            // Get last position's logits
            let logits_shape = logits.shape();
            let last_logits = if logits_shape.len() >= 2 {
                let s = logits_shape[logits_shape.len() - 2]; // seq dim
                let vocab = *logits_shape.last().unwrap();
                logits.clone().narrow(logits_shape.len() - 2, s - 1, 1).reshape(vec![vocab])
            } else {
                logits.clone()
            };

            let next_token = sampler::sample_top_p(&last_logits, temperature, 0.9);

            // Check for EOS
            let eos_tokens = [50256, 128001, 128009, 2, 1];
            if eos_tokens.contains(&(next_token as u32)) {
                break;
            }

            token_ids.push(next_token as u32);

            // Save KV-cache: present.N.key -> past_key_values.N.key
            if use_kv_cache {
                kv_cache.clear();
                for i in 0..self.num_kv_layers {
                    let present_key = format!("present.{i}.key");
                    let present_val = format!("present.{i}.value");
                    let past_key = format!("past_key_values.{i}.key");
                    let past_val = format!("past_key_values.{i}.value");

                    if let Some(k) = outputs.get(&present_key) {
                        kv_cache.insert(past_key, k.clone());
                    }
                    if let Some(v) = outputs.get(&present_val) {
                        kv_cache.insert(past_val, v.clone());
                    }
                }
                log::debug!("KV-cache: {} entries, step {step}", kv_cache.len());
            }

            let decoded = self.tokenizer.decode(&[next_token as u32], false)
                .map_err(|e| format!("Decode failed: {e}"))?;

            print!("{decoded}");
            use std::io::Write;
            std::io::stdout().flush().ok();

            generated_text.push_str(&decoded);
        }

        println!();
        Ok(generated_text)
    }
}
