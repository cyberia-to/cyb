use crate::graph::value::Value;

/// Sample a token index from logits with temperature
pub fn sample_top_p(
    logits: &Value,
    temperature: f32,
    top_p: f32,
) -> usize {
    let logits_vec: Vec<f32> = logits.to_vec_f32();

    if temperature <= 0.0 {
        // Greedy — return argmax
        return logits_vec
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    // Apply temperature
    let scaled: Vec<f32> = logits_vec.iter().map(|&v| v / temperature).collect();

    // Softmax
    let max_val = scaled.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp_vals: Vec<f32> = scaled.iter().map(|&v| (v - max_val).exp()).collect();
    let sum: f32 = exp_vals.iter().sum();
    let probs: Vec<f32> = exp_vals.iter().map(|&v| v / sum).collect();

    // Sort by probability (descending) for top-p
    let mut indexed: Vec<(usize, f32)> = probs.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Nucleus sampling (top-p)
    let mut cumulative = 0.0;
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for (idx, prob) in &indexed {
        cumulative += prob;
        candidates.push((*idx, *prob));
        if cumulative >= top_p {
            break;
        }
    }

    // Renormalize
    let total: f32 = candidates.iter().map(|(_, p)| p).sum();
    let candidates: Vec<(usize, f32)> = candidates
        .into_iter()
        .map(|(i, p)| (i, p / total))
        .collect();

    // Sample using simple RNG (deterministic for now)
    let r: f32 = simple_random();
    let mut acc = 0.0;
    for (idx, prob) in &candidates {
        acc += prob;
        if acc >= r {
            return *idx;
        }
    }

    candidates.last().map(|(i, _)| *i).unwrap_or(0)
}

/// Simple pseudo-random number generator (replace with proper RNG later)
fn simple_random() -> f32 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    // LCG
    let x = seed.wrapping_mul(1103515245).wrapping_add(12345);
    (x as f32 / u32::MAX as f32).abs()
}
