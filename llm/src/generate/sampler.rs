//! Sampling strategies for text generation

/// Greedy argmax
pub fn argmax(logits: &[f32]) -> u32 {
    logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i as u32)
        .unwrap_or(0)
}

/// Temperature scaling + softmax + top-p (nucleus) sampling
pub fn sample_top_p(logits: &[f32], temperature: f32, top_p: f32) -> u32 {
    if temperature <= 0.0 {
        return argmax(logits);
    }

    let scaled: Vec<f32> = logits.iter().map(|&v| v / temperature).collect();

    // Softmax
    let max_val = scaled.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp_vals: Vec<f32> = scaled.iter().map(|&v| (v - max_val).exp()).collect();
    let sum: f32 = exp_vals.iter().sum();
    let probs: Vec<f32> = exp_vals.iter().map(|&v| v / sum).collect();

    // Sort descending by probability
    let mut indexed: Vec<(usize, f32)> = probs.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Nucleus: keep tokens until cumulative prob >= top_p
    let mut cumulative = 0.0;
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for (idx, prob) in &indexed {
        cumulative += prob;
        candidates.push((*idx, *prob));
        if cumulative >= top_p {
            break;
        }
    }

    // Renormalize and sample
    let total: f32 = candidates.iter().map(|(_, p)| p).sum();
    let r = random_f32();

    let mut acc = 0.0;
    for (idx, prob) in &candidates {
        acc += prob / total;
        if acc >= r {
            return *idx as u32;
        }
    }

    candidates.last().map(|(i, _)| *i as u32).unwrap_or(0)
}

/// Top-k sampling: keep only top K tokens, then sample
pub fn sample_top_k(logits: &[f32], temperature: f32, k: usize) -> u32 {
    if temperature <= 0.0 || k == 1 {
        return argmax(logits);
    }

    let scaled: Vec<f32> = logits.iter().map(|&v| v / temperature).collect();

    let mut indexed: Vec<(usize, f32)> = scaled.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    indexed.truncate(k);

    // Softmax over top-k
    let max_val = indexed.iter().map(|(_, v)| *v).fold(f32::NEG_INFINITY, f32::max);
    let exp_vals: Vec<(usize, f32)> = indexed.iter().map(|(i, v)| (*i, (v - max_val).exp())).collect();
    let sum: f32 = exp_vals.iter().map(|(_, v)| v).sum();

    let r = random_f32();
    let mut acc = 0.0;
    for (idx, val) in &exp_vals {
        acc += val / sum;
        if acc >= r {
            return *idx as u32;
        }
    }

    exp_vals.last().map(|(i, _)| *i as u32).unwrap_or(0)
}

/// Simple RNG using system time (not cryptographic)
fn random_f32() -> f32 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let x = seed.wrapping_mul(1103515245).wrapping_add(12345);
    (x as f32 / u32::MAX as f32).abs()
}
