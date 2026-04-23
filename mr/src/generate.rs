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

/// Suppress (or amplify) logits for recently generated tokens before sampling.
///
/// `penalty > 1.0` makes recent tokens *less* likely — the usual case for
/// reducing n-gram loops. `penalty < 1.0` amplifies them. `penalty == 1.0`
/// is a no-op.
///
/// Sign-aware: positive logits are divided, negative logits are multiplied,
/// so the penalty always pushes the score toward zero (following HF's
/// `transformers` convention).
pub fn apply_repetition_penalty(logits: &mut [f32], recent_tokens: &[u32], penalty: f32) {
    if penalty == 1.0 {
        return;
    }
    for &token in recent_tokens {
        let idx = token as usize;
        if idx < logits.len() {
            if logits[idx] > 0.0 {
                logits[idx] /= penalty;
            } else {
                logits[idx] *= penalty;
            }
        }
    }
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
    let debug_tokens = std::env::var("MR_DEBUG_TOKENS").is_ok();
    let mut generated = Vec::with_capacity(max_tokens);
    for step in 0..max_tokens {
        let next = sample(&logits, sample_cfg);
        if debug_tokens {
            // Top-5 logits + sampled id, for diagnosing empty-output failures.
            let mut idx: Vec<usize> = (0..logits.len()).collect();
            idx.sort_unstable_by(|&a, &b| logits[b].partial_cmp(&logits[a]).unwrap());
            let top5: Vec<String> = idx
                .iter()
                .take(5)
                .map(|&i| format!("{i}={:.2}", logits[i]))
                .collect();
            eprintln!(
                "step {step:3}: sampled={next} \"{}\" eos?={} top5={}",
                tok.decode(&[next], false).replace('\n', "\\n"),
                tok.is_eos(next),
                top5.join(" "),
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repetition_penalty_no_op_at_one() {
        let mut logits = [1.0, 2.0, -0.5];
        let before = logits;
        apply_repetition_penalty(&mut logits, &[0, 1, 2], 1.0);
        assert_eq!(logits, before);
    }

    #[test]
    fn repetition_penalty_divides_positive_multiplies_negative() {
        let mut logits = [2.0f32, -1.0, 3.0, -4.0];
        apply_repetition_penalty(&mut logits, &[0, 1], 2.0);
        assert!((logits[0] - 1.0).abs() < 1e-6, "positive → divided by 2");
        assert!((logits[1] - (-2.0)).abs() < 1e-6, "negative → multiplied by 2");
        // Indices not in recent list untouched
        assert_eq!(logits[2], 3.0);
        assert_eq!(logits[3], -4.0);
    }

    #[test]
    fn repetition_penalty_skips_out_of_range() {
        let mut logits = [1.0f32, 2.0];
        apply_repetition_penalty(&mut logits, &[0, 5, 10], 2.0); // 5, 10 out of range
        assert_eq!(logits[0], 0.5);
        assert_eq!(logits[1], 2.0); // untouched
    }
}
