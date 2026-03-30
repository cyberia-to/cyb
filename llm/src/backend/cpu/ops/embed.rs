//! Embedding lookup and backward

/// Gather embeddings: x[d, t] = embed[tok, d] (transposed into channel-first)
pub fn lookup(x: &mut [f32], embed: &[f32], tokens: &[u16], dim: usize, seq: usize) {
    for t in 0..seq {
        let tok = tokens[t] as usize;
        for d in 0..dim {
            x[d * seq + t] = embed[tok * dim + d];
        }
    }
}

/// Single-token embed (for decode): x[d] = embed[token, d]
pub fn lookup_single(x: &mut [f32], embed: &[f32], token: usize, dim: usize) {
    x[..dim].copy_from_slice(&embed[token * dim..(token + 1) * dim]);
}

/// Scatter gradients back to embedding matrix
pub fn backward(d_embed: &mut [f32], dx: &[f32], tokens: &[u16], dim: usize, seq: usize) {
    for t in 0..seq {
        let tok = tokens[t] as usize;
        for d in 0..dim {
            d_embed[tok * dim + d] += dx[d * seq + t];
        }
    }
}

/// Vocab compaction map: only optimize tokens that appear in training data
pub struct VocabMap {
    pub compact_vocab: usize,
    pub full_to_compact: Vec<i32>, // [VOCAB] → compact id (-1 if unused)
    pub compact_to_full: Vec<usize>, // [compact_vocab] → full vocab id
}

impl VocabMap {
    /// Build from training data token stream
    pub fn build(data: &[u16], full_vocab: usize) -> Self {
        let mut f2c = vec![-1i32; full_vocab];
        for &tok in data {
            f2c[tok as usize] = 0;
        }
        let mut cid = 0i32;
        for v in 0..full_vocab {
            if f2c[v] == 0 {
                f2c[v] = cid;
                cid += 1;
            } else {
                f2c[v] = -1;
            }
        }
        let compact_vocab = cid as usize;
        let mut c2f = vec![0usize; compact_vocab];
        for v in 0..full_vocab {
            if f2c[v] >= 0 {
                c2f[f2c[v] as usize] = v;
            }
        }
        VocabMap {
            compact_vocab,
            full_to_compact: f2c,
            compact_to_full: c2f,
        }
    }

    /// Extract compact embedding from full
    pub fn compact_embed(&self, full_embed: &[f32], dim: usize) -> Vec<f32> {
        let mut ce = vec![0.0f32; self.compact_vocab * dim];
        for c in 0..self.compact_vocab {
            let fv = self.compact_to_full[c];
            ce[c * dim..(c + 1) * dim].copy_from_slice(&full_embed[fv * dim..(fv + 1) * dim]);
        }
        ce
    }

    /// Scatter compact gradients back to full
    pub fn scatter_grads(&self, full_grad: &mut [f32], compact_grad: &[f32], dim: usize) {
        for c in 0..self.compact_vocab {
            let fv = self.compact_to_full[c];
            for d in 0..dim {
                full_grad[fv * dim + d] += compact_grad[c * dim + d];
            }
        }
    }

    /// Copy compact weights back to full embedding
    pub fn update_full(&self, full_embed: &mut [f32], compact_embed: &[f32], dim: usize) {
        for c in 0..self.compact_vocab {
            let fv = self.compact_to_full[c];
            full_embed[fv * dim..(fv + 1) * dim]
                .copy_from_slice(&compact_embed[c * dim..(c + 1) * dim]);
        }
    }
}
