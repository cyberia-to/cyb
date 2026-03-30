//! Token sampling utilities — top-k with temperature, argmax, xorshift PRNG

/// Sample a token: greedy argmax if temp <= 0, otherwise top-k sampling.
pub fn sample_token(logits: &[f32], vocab: usize, temp: f32, topk: usize) -> usize {
    if temp <= 0.0 {
        argmax(logits, vocab)
    } else {
        sample_topk(logits, vocab, topk, temp)
    }
}

/// Return index of the maximum value in `logits[..v]`.
pub fn argmax(logits: &[f32], v: usize) -> usize {
    let mut best = 0;
    for i in 1..v {
        if logits[i] > logits[best] {
            best = i;
        }
    }
    best
}

/// Top-k sampling with temperature scaling and softmax.
pub fn sample_topk(logits: &[f32], v: usize, k: usize, temp: f32) -> usize {
    let k = k.min(v);
    let mut idx: Vec<usize> = (0..k).collect();
    let mut val: Vec<f32> = logits[..k].to_vec();
    for i in k..v {
        let minj = val
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        if logits[i] > val[minj] {
            val[minj] = logits[i];
            idx[minj] = i;
        }
    }
    let max_v = val.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for v in &mut val {
        *v = ((*v - max_v) / temp).exp();
        sum += *v;
    }
    for v in &mut val {
        *v /= sum;
    }
    let r = rand_f32();
    let mut cum = 0.0f32;
    for i in 0..k {
        cum += val[i];
        if r < cum {
            return idx[i];
        }
    }
    idx[k - 1]
}

/// Simple xorshift64 PRNG returning a value in [0, 1).
pub fn rand_f32() -> f32 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0x12345678DEADBEEF);
    let mut s = STATE.load(Ordering::Relaxed);
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    STATE.store(s, Ordering::Relaxed);
    (s & 0xFFFFFF) as f32 / 0xFFFFFF as f32
}
