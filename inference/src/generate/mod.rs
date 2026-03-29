pub mod sampler;

use std::collections::HashMap;
use std::path::Path;

use burn::prelude::*;
use tokenizers::Tokenizer;

use crate::Backend;
use crate::graph::OnnxExecutor;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

pub struct TextGenerator {
    executor: OnnxExecutor,
    tokenizer: Tokenizer,
    device: Device,
}

impl TextGenerator {
    pub fn new(
        model_path: &Path,
        tokenizer_path: &Path,
        device: Device,
    ) -> Result<Self, String> {
        let mut executor = OnnxExecutor::from_file(model_path)?;
        executor.load_from_file(model_path, &device)?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Failed to load tokenizer: {e}"))?;

        Ok(Self {
            executor,
            tokenizer,
            device,
        })
    }

    /// Generate text from a prompt
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String, String> {
        // Encode prompt
        let encoding = self.tokenizer.encode(prompt, false)
            .map_err(|e| format!("Tokenization failed: {e}"))?;
        let mut token_ids: Vec<u32> = encoding.get_ids().to_vec();

        log::info!("Prompt tokens: {:?}", token_ids);

        let mut generated_text = String::new();

        for step in 0..max_tokens {
            // Clear intermediates from previous step (keep weights + dequant cache)
            self.executor.clear_intermediates();

            // Prepare inputs
            let seq_len = token_ids.len();
            let input_floats: Vec<f32> = token_ids.iter().map(|&id| id as f32).collect();

            let input_data = burn::tensor::TensorData::new(input_floats, vec![1, seq_len]);
            let mask_data = burn::tensor::TensorData::new(vec![1.0f32; seq_len], vec![1, seq_len]);

            let mut inputs = HashMap::new();
            inputs.insert(
                "input_ids".to_string(),
                Value::Float2(Tensor::from_data(input_data, &self.device)),
            );
            inputs.insert(
                "attention_mask".to_string(),
                Value::Float2(Tensor::from_data(mask_data, &self.device)),
            );

            // Forward pass
            let outputs = self.executor.run(inputs, &self.device)?;

            // Get logits
            let logits = outputs.get("logits")
                .ok_or("No logits in output")?;

            // Extract last token logits
            let last_logits = match logits {
                Value::Float3(t) => {
                    let [_batch, seq, vocab] = t.dims();
                    // Get logits for last position
                    let last = t.clone().narrow(1, seq - 1, 1);
                    last.reshape([vocab])
                }
                _ => return Err(format!("Unexpected logits shape: {:?}", logits.shape())),
            };

            // Sample next token
            let next_token = sampler::sample_top_p(&last_logits, temperature, 0.9);

            // Check for EOS (common EOS token IDs)
            let eos_tokens = [
                50256,   // GPT-2
                128001,  // Llama 3
                128009,  // Llama 3 end of turn
                2,       // Llama 2
                1,       // some models
            ];
            if eos_tokens.contains(&(next_token as u32)) {
                break;
            }

            token_ids.push(next_token as u32);

            // Decode the new token
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
