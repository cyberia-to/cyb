//! Autoregressive text generation — prefill + decode + sampling.
//!
//! Spec: reference/runtime/execution.md, reference/runtime/ops.md §8

use crate::backend::Backend;
use crate::llama_style::LlamaModel;
use crate::tokenizer::{ChatMessage, Tokenizer};

#[derive(Clone, Copy, Debug)]
pub struct SampleConfig {
    pub method: SampleKind,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleKind {
    Greedy,
    TopP,
    TopK,
}

impl Default for SampleConfig {
    fn default() -> Self {
        Self {
            method: SampleKind::Greedy,
            temperature: 1.0,
            top_p: 0.95,
            top_k: 40,
        }
    }
}

/// Sample a token id from logits.
pub fn sample(logits: &[f32], config: SampleConfig) -> u32 {
    match config.method {
        SampleKind::Greedy => argmax(logits) as u32,
        SampleKind::TopP => {
            let mut logits = logits.to_vec();
            if config.temperature > 0.0 && config.temperature != 1.0 {
                for l in &mut logits {
                    *l /= config.temperature;
                }
            }
            let probs = softmax(&logits);
            let mut pairs: Vec<(usize, f32)> = probs.into_iter().enumerate().collect();
            pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            // Cumulative sum, keep until >= top_p
            let mut cum = 0f32;
            let mut keep = 0;
            for (i, (_, p)) in pairs.iter().enumerate() {
                cum += p;
                keep = i + 1;
                if cum >= config.top_p {
                    break;
                }
            }
            // Always keep at least the top 1
            let kept = &pairs[..keep.max(1)];
            let total: f32 = kept.iter().map(|(_, p)| *p).sum();
            let r = rand_f32() * total;
            let mut acc = 0f32;
            for (id, p) in kept {
                acc += p;
                if acc >= r {
                    return *id as u32;
                }
            }
            kept[0].0 as u32
        }
        SampleKind::TopK => {
            let mut logits = logits.to_vec();
            if config.temperature > 0.0 && config.temperature != 1.0 {
                for l in &mut logits {
                    *l /= config.temperature;
                }
            }
            let mut pairs: Vec<(usize, f32)> = logits.into_iter().enumerate().collect();
            pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let kept = &pairs[..config.top_k.min(pairs.len())];
            let max = kept.iter().map(|(_, v)| *v).fold(f32::NEG_INFINITY, f32::max);
            let exps: Vec<f32> = kept.iter().map(|(_, v)| (v - max).exp()).collect();
            let total: f32 = exps.iter().sum();
            let r = rand_f32() * total;
            let mut acc = 0f32;
            for (i, e) in exps.iter().enumerate() {
                acc += e;
                if acc >= r {
                    return kept[i].0 as u32;
                }
            }
            kept[0].0 as u32
        }
    }
}

fn argmax(logits: &[f32]) -> usize {
    logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut exps: Vec<f32> = logits.iter().map(|v| (v - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    for e in &mut exps {
        *e /= sum;
    }
    exps
}

/// Non-cryptographic RNG — xorshift64 seeded from system time, one global state.
/// Fine for sampling; deterministic seeding comes via SampleConfig in the future.
fn rand_f32() -> f32 {
    use std::sync::Mutex;
    static STATE: Mutex<u64> = Mutex::new(0);
    let mut s = STATE.lock().unwrap();
    if *s == 0 {
        *s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234567890ABCDEF)
            | 1;
    }
    let mut x = *s;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *s = x;
    (x as f32) / (u64::MAX as f32)
}

/// Generate text autoregressively.
///
/// Returns (generated_text, generated_token_count).
pub fn generate(
    model: &mut LlamaModel,
    tok: &Tokenizer,
    backend: &dyn Backend,
    prompt: &str,
    max_tokens: usize,
    sample_cfg: SampleConfig,
) -> Result<(String, usize), crate::backend::BackendError> {
    model.reset_kv_cache();

    // Tokenize prompt
    let prompt_ids = tok.encode(prompt);
    log::debug!("prompt tokens: {}", prompt_ids.len());

    // Prefill: run forward for each prompt token, keep logits from last.
    let mut logits: Vec<f32> = Vec::new();
    for &tid in &prompt_ids {
        logits = model.forward(tid, backend)?;
    }

    // Decode: sample next token, feed back in, repeat.
    let mut generated = Vec::with_capacity(max_tokens);
    for _ in 0..max_tokens {
        let next = sample(&logits, sample_cfg);
        if tok.is_eos(next) {
            break;
        }
        generated.push(next);
        logits = model.forward(next, backend)?;
    }

    let text = tok.decode(&generated, false);
    Ok((text, generated.len()))
}

/// Build a prompt from a chat message list using the model's template.
pub fn build_chat_prompt(tok: &Tokenizer, messages: &[ChatMessage]) -> String {
    tok.apply_chat_template(messages, true)
}
