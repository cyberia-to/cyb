//! Activation functions and GQA helpers

/// SiLU (Swish) activation: x * sigmoid(x)
pub fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

/// SiLU backward: given dsilu (gradient of SiLU output) and x (original input)
/// d(SiLU)/dx = sigmoid(x) * (1 + x * (1 - sigmoid(x)))
pub fn silu_backward(dsilu: f32, x: f32) -> f32 {
    let sig = 1.0 / (1.0 + (-x).exp());
    dsilu * sig * (1.0 + x * (1.0 - sig))
}

/// GQA tile: expand KV_DIM → Q_DIM by duplicating KV heads
/// input: [kv_dim, seq], output: [q_dim, seq]
pub fn gqa_tile_kv(
    out: &mut [f32],
    input: &[f32],
    kv_heads: usize,
    gqa_ratio: usize,
    hd: usize,
    seq: usize,
) {
    for kv in 0..kv_heads {
        for r in 0..gqa_ratio {
            let q_head = kv * gqa_ratio + r;
            let dst = q_head * hd * seq;
            let src = kv * hd * seq;
            out[dst..dst + hd * seq].copy_from_slice(&input[src..src + hd * seq]);
        }
    }
}

/// GQA reduce: sum Q_DIM → KV_DIM gradients
/// input: [q_dim, seq], output: [kv_dim, seq]
pub fn gqa_reduce_kv(
    out: &mut [f32],
    input: &[f32],
    kv_heads: usize,
    gqa_ratio: usize,
    hd: usize,
    seq: usize,
) {
    out.iter_mut().for_each(|v| *v = 0.0);
    for kv in 0..kv_heads {
        for r in 0..gqa_ratio {
            let q_head = kv * gqa_ratio + r;
            for i in 0..hd * seq {
                out[kv * hd * seq + i] += input[q_head * hd * seq + i];
            }
        }
    }
}

/// Compute logits: x_final[dim] @ embed[vocab, dim]^T → logits[vocab]
pub fn compute_logits(
    logits: &mut [f32],
    x_final: &[f32],
    embed: &[f32],
    vocab: usize,
    dim: usize,
) {
    for v in 0..vocab {
        let mut dot = 0.0f32;
        for d in 0..dim {
            dot += x_final[d] * embed[v * dim + d];
        }
        logits[v] = dot;
    }
}
