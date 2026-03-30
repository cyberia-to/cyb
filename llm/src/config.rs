//! Model configuration — architecture parameters for transformer models

/// Transformer model configuration.
/// All dimensions needed for MIL generation, weight allocation, and training.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub name: &'static str,
    pub dim: usize,      // embedding/hidden dimension
    pub q_dim: usize,    // query projection dimension (= dim for MHA, > dim for GQA)
    pub kv_dim: usize,   // key/value projection dimension
    pub hidden: usize,   // FFN hidden dimension
    pub heads: usize,    // number of query heads
    pub kv_heads: usize, // number of key/value heads (< heads for GQA)
    pub hd: usize,       // head dimension = q_dim / heads
    pub n_layers: usize, // number of transformer layers
    pub seq: usize,      // sequence length
    pub vocab: usize,    // vocabulary size
}

impl ModelConfig {
    pub fn gqa_ratio(&self) -> usize {
        self.heads / self.kv_heads
    }

    // Weight sizes
    pub fn wq_size(&self) -> usize {
        self.q_dim * self.dim
    }
    pub fn wk_size(&self) -> usize {
        self.kv_dim * self.dim
    }
    pub fn wv_size(&self) -> usize {
        self.kv_dim * self.dim
    }
    pub fn wo_size(&self) -> usize {
        self.dim * self.q_dim
    }
    pub fn w1_size(&self) -> usize {
        self.hidden * self.dim
    }
    pub fn w2_size(&self) -> usize {
        self.dim * self.hidden
    }
    pub fn w3_size(&self) -> usize {
        self.hidden * self.dim
    }

    // ANE kernel spatial dimensions
    pub fn sdpa_fwd_sp(&self) -> usize {
        self.seq + self.q_dim + self.kv_dim + self.kv_dim + self.hd + self.hd
    }
    pub fn qkv_proj_sp(&self) -> usize {
        self.seq + self.q_dim + self.kv_dim + self.kv_dim
    }
    pub fn wo_fwd_sp(&self) -> usize {
        self.seq + self.dim
    }
    pub fn ffn_fused_sp(&self) -> usize {
        2 * self.seq + 3 * self.hidden
    }
    pub fn ffn_bwd_w2t_sp(&self) -> usize {
        self.seq + self.hidden
    }
    pub fn ffn_bwd_w13t_sp(&self) -> usize {
        2 * self.seq + 2 * self.dim
    }
    pub fn wot_bwd_sp(&self) -> usize {
        self.seq + self.q_dim
    }
    pub fn q_bwd_sp(&self) -> usize {
        self.seq + self.dim
    }
    pub fn kv_bwd_sp(&self) -> usize {
        2 * self.seq + 2 * self.dim
    }
    pub fn score_ch(&self) -> usize {
        self.heads * self.seq
    }

    // Total layer params (for checkpoint sizing)
    pub fn layer_params(&self) -> usize {
        self.wq_size()
            + self.wk_size()
            + self.wv_size()
            + self.wo_size()
            + self.w1_size()
            + self.w2_size()
            + self.w3_size()
            + 2 * self.dim
    }
}

pub fn qwen3_06b() -> ModelConfig {
    ModelConfig {
        name: "Qwen3-0.6B",
        dim: 1024,
        q_dim: 2048,
        kv_dim: 1024,
        hidden: 3072,
        heads: 16,
        kv_heads: 8,
        hd: 128,
        n_layers: 28,
        seq: 256,
        vocab: 151936,
    }
}

pub fn stories110m() -> ModelConfig {
    ModelConfig {
        name: "Stories-110M",
        dim: 768,
        q_dim: 768,
        kv_dim: 768,
        hidden: 2048,
        heads: 12,
        kv_heads: 12,
        hd: 64,
        n_layers: 12,
        seq: 256,
        vocab: 32000,
    }
}
