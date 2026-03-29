//! Architecture templates — generate Graph IR for common model architectures
//!
//! For safetensors/GGUF models, the runtime has built-in templates.
//! config comes from config.json (HuggingFace) or GGUF metadata.
//! template + config = concrete graph.

use super::graph::{Graph, TensorMeta, Dim, Residency};
use super::ops::Op;
use super::dtype::DType;

/// Activation function for FFN
#[derive(Clone, Debug)]
pub enum Activation {
    Silu,
    Gelu,
    GeGlu,
}

/// Configuration for transformer models
#[derive(Clone, Debug)]
pub struct TransformerConfig {
    pub hidden_size: usize,
    pub num_heads: usize,
    pub kv_num_heads: usize,
    pub head_dim: usize,
    pub num_layers: usize,
    pub intermediate_size: usize,
    pub vocab_size: usize,
    pub eps: f32,
    pub rope_theta: f32,
    pub max_seq_len: usize,
    pub activation: Activation,
}

impl Default for TransformerConfig {
    fn default() -> Self {
        Self {
            hidden_size: 4096,
            num_heads: 32,
            kv_num_heads: 8,
            head_dim: 128,
            num_layers: 32,
            intermediate_size: 11008,
            vocab_size: 32000,
            eps: 1e-5,
            rope_theta: 10000.0,
            max_seq_len: 4096,
            activation: Activation::Silu,
        }
    }
}

/// Build a transformer decoder graph (LLaMA, Mistral, Qwen, etc.)
///
/// Structure per layer:
///   rmsnorm -> attention(qkv matmul, rope, sdpa, kv_cache, output matmul) ->
///   rmsnorm -> mlp(gate + up matmul, activation, down matmul)
pub fn transformer_decoder(config: &TransformerConfig) -> Graph {
    let mut g = Graph::new();

    let seq_dim = Dim::Dynamic("seq_len".to_string());
    let hidden = config.hidden_size;

    // Token embedding: input_ids -> hidden
    let embed_out = "embed_out".to_string();
    g.add_node(
        Op::TokenEmbed,
        vec!["input_ids".to_string()],
        vec![embed_out.clone()],
    );
    g.add_tensor("input_ids".to_string(), TensorMeta {
        shape: vec![seq_dim.clone()],
        dtype: DType::U8, // token ids
        residency: Residency::Streamed,
    });
    g.add_tensor(embed_out.clone(), TensorMeta {
        shape: vec![seq_dim.clone(), Dim::Fixed(hidden)],
        dtype: DType::F16,
        residency: Residency::Streamed,
    });

    let mut prev_hidden = embed_out;

    for i in 0..config.num_layers {
        let prefix = format!("layer_{i}");

        // Input norm
        let norm1_out = format!("{prefix}.attn_norm_out");
        g.add_node(
            Op::RmsNorm { eps: config.eps },
            vec![prev_hidden.clone(), format!("{prefix}.attn_norm.weight")],
            vec![norm1_out.clone()],
        );

        // QKV matmul
        let qkv_out = format!("{prefix}.qkv_out");
        g.add_node(
            Op::Matmul,
            vec![norm1_out, format!("{prefix}.qkv.weight")],
            vec![qkv_out.clone()],
        );

        // RoPE
        let rope_out = format!("{prefix}.rope_out");
        g.add_node(
            Op::Rope {
                head_dim: config.head_dim as u32,
                base: config.rope_theta,
            },
            vec![qkv_out],
            vec![rope_out.clone()],
        );

        // KV cache
        let kv_out = format!("{prefix}.kv_cached");
        g.add_node(
            Op::KvCache,
            vec![rope_out.clone()],
            vec![kv_out.clone()],
        );
        g.add_tensor(kv_out.clone(), TensorMeta {
            shape: vec![
                Dim::Dynamic("total_seq".to_string()),
                Dim::Fixed(config.kv_num_heads * config.head_dim),
            ],
            dtype: DType::F16,
            residency: Residency::Cached,
        });

        // Scaled dot-product attention
        let attn_out = format!("{prefix}.attn_out");
        g.add_node(
            Op::Sdpa {
                num_heads: config.num_heads as u32,
                kv_heads: config.kv_num_heads as u32,
                head_dim: config.head_dim as u32,
                causal: true,
            },
            vec![rope_out, kv_out],
            vec![attn_out.clone()],
        );

        // Output projection
        let o_proj_out = format!("{prefix}.o_proj_out");
        g.add_node(
            Op::Matmul,
            vec![attn_out, format!("{prefix}.o_proj.weight")],
            vec![o_proj_out.clone()],
        );

        // Residual add
        let residual1 = format!("{prefix}.residual1");
        g.add_node(
            Op::Add,
            vec![prev_hidden.clone(), o_proj_out],
            vec![residual1.clone()],
        );

        // FFN norm
        let norm2_out = format!("{prefix}.ffn_norm_out");
        g.add_node(
            Op::RmsNorm { eps: config.eps },
            vec![residual1.clone(), format!("{prefix}.ffn_norm.weight")],
            vec![norm2_out.clone()],
        );

        // FFN: gate + up projections
        let gate_out = format!("{prefix}.gate_out");
        g.add_node(
            Op::Matmul,
            vec![norm2_out.clone(), format!("{prefix}.gate_proj.weight")],
            vec![gate_out.clone()],
        );

        let up_out = format!("{prefix}.up_out");
        g.add_node(
            Op::Matmul,
            vec![norm2_out, format!("{prefix}.up_proj.weight")],
            vec![up_out.clone()],
        );

        // Activation gate
        let act_out = format!("{prefix}.act_out");
        match config.activation {
            Activation::Silu => {
                // SwiGLU: silu(gate) * up
                let silu_out = format!("{prefix}.silu_out");
                g.add_node(Op::Silu, vec![gate_out], vec![silu_out.clone()]);
                g.add_node(Op::Mul, vec![silu_out, up_out], vec![act_out.clone()]);
            }
            Activation::Gelu => {
                let gelu_out = format!("{prefix}.gelu_out");
                g.add_node(
                    Op::Gelu { approximate: false },
                    vec![gate_out],
                    vec![gelu_out.clone()],
                );
                g.add_node(Op::Mul, vec![gelu_out, up_out], vec![act_out.clone()]);
            }
            Activation::GeGlu => {
                g.add_node(Op::GeGlu, vec![gate_out, up_out], vec![act_out.clone()]);
            }
        }

        // Down projection
        let down_out = format!("{prefix}.down_out");
        g.add_node(
            Op::Matmul,
            vec![act_out, format!("{prefix}.down_proj.weight")],
            vec![down_out.clone()],
        );

        // Residual add
        let residual2 = format!("{prefix}.residual2");
        g.add_node(
            Op::Add,
            vec![residual1, down_out],
            vec![residual2.clone()],
        );

        prev_hidden = residual2;
    }

    // Final norm
    let final_norm = "final_norm_out".to_string();
    g.add_node(
        Op::RmsNorm { eps: config.eps },
        vec![prev_hidden, "model.norm.weight".to_string()],
        vec![final_norm.clone()],
    );

    // LM head
    g.add_node(
        Op::Matmul,
        vec![final_norm, "lm_head.weight".to_string()],
        vec!["logits".to_string()],
    );

    g.add_tensor("logits".to_string(), TensorMeta {
        shape: vec![seq_dim, Dim::Fixed(config.vocab_size)],
        dtype: DType::F32,
        residency: Residency::Streamed,
    });

    log::info!(
        "transformer_decoder template: {} layers, {} nodes",
        config.num_layers,
        g.len()
    );

    g
}

/// Build a transformer encoder graph (BERT, CLIP text, whisper encoder)
///
/// Structure per layer:
///   layernorm -> attention(no KV cache, no causal mask) ->
///   layernorm -> mlp(up, gelu, down)
pub fn transformer_encoder(config: &TransformerConfig) -> Graph {
    let mut g = Graph::new();

    let seq_dim = Dim::Dynamic("seq_len".to_string());
    let hidden = config.hidden_size;

    // Token + position embedding
    let embed_out = "embed_out".to_string();
    g.add_node(
        Op::TokenEmbed,
        vec!["input_ids".to_string()],
        vec!["token_embed".to_string()],
    );
    g.add_node(
        Op::PosEmbed,
        vec!["position_ids".to_string()],
        vec!["pos_embed".to_string()],
    );
    g.add_node(
        Op::Add,
        vec!["token_embed".to_string(), "pos_embed".to_string()],
        vec![embed_out.clone()],
    );
    g.add_tensor(embed_out.clone(), TensorMeta {
        shape: vec![seq_dim.clone(), Dim::Fixed(hidden)],
        dtype: DType::F16,
        residency: Residency::Streamed,
    });

    let mut prev_hidden = embed_out;

    for i in 0..config.num_layers {
        let prefix = format!("layer_{i}");

        // Pre-attention layernorm
        let norm1_out = format!("{prefix}.attn_norm_out");
        g.add_node(
            Op::LayerNorm { eps: config.eps },
            vec![prev_hidden.clone(), format!("{prefix}.attn_norm.weight"), format!("{prefix}.attn_norm.bias")],
            vec![norm1_out.clone()],
        );

        // QKV matmul
        let qkv_out = format!("{prefix}.qkv_out");
        g.add_node(
            Op::Matmul,
            vec![norm1_out, format!("{prefix}.qkv.weight")],
            vec![qkv_out.clone()],
        );

        // Non-causal attention (no KV cache in encoder)
        let attn_out = format!("{prefix}.attn_out");
        g.add_node(
            Op::Sdpa {
                num_heads: config.num_heads as u32,
                kv_heads: config.num_heads as u32, // no GQA in encoder
                head_dim: config.head_dim as u32,
                causal: false,
            },
            vec![qkv_out],
            vec![attn_out.clone()],
        );

        // Output projection
        let o_proj_out = format!("{prefix}.o_proj_out");
        g.add_node(
            Op::Matmul,
            vec![attn_out, format!("{prefix}.o_proj.weight")],
            vec![o_proj_out.clone()],
        );

        // Residual
        let residual1 = format!("{prefix}.residual1");
        g.add_node(
            Op::Add,
            vec![prev_hidden.clone(), o_proj_out],
            vec![residual1.clone()],
        );

        // Pre-FFN layernorm
        let norm2_out = format!("{prefix}.ffn_norm_out");
        g.add_node(
            Op::LayerNorm { eps: config.eps },
            vec![residual1.clone(), format!("{prefix}.ffn_norm.weight"), format!("{prefix}.ffn_norm.bias")],
            vec![norm2_out.clone()],
        );

        // FFN: up -> gelu -> down
        let up_out = format!("{prefix}.up_out");
        g.add_node(
            Op::Matmul,
            vec![norm2_out, format!("{prefix}.up_proj.weight")],
            vec![up_out.clone()],
        );

        let act_out = format!("{prefix}.act_out");
        g.add_node(
            Op::Gelu { approximate: false },
            vec![up_out],
            vec![act_out.clone()],
        );

        let down_out = format!("{prefix}.down_out");
        g.add_node(
            Op::Matmul,
            vec![act_out, format!("{prefix}.down_proj.weight")],
            vec![down_out.clone()],
        );

        // Residual
        let residual2 = format!("{prefix}.residual2");
        g.add_node(
            Op::Add,
            vec![residual1, down_out],
            vec![residual2.clone()],
        );

        prev_hidden = residual2;
    }

    // Final layernorm
    g.add_node(
        Op::LayerNorm { eps: config.eps },
        vec![prev_hidden, "model.norm.weight".to_string(), "model.norm.bias".to_string()],
        vec!["encoder_output".to_string()],
    );

    log::info!(
        "transformer_encoder template: {} layers, {} nodes",
        config.num_layers,
        g.len()
    );

    g
}

/// Build an encoder-decoder graph (whisper, T5)
///
/// Encoder: transformer_encoder
/// Decoder: transformer_decoder with cross-attention inserted after self-attention
pub fn encoder_decoder(
    enc_config: &TransformerConfig,
    dec_config: &TransformerConfig,
) -> Graph {
    let mut g = Graph::new();

    // --- Encoder ---
    let enc = transformer_encoder(enc_config);

    // Copy encoder nodes with "enc." prefix on tensor names
    for node in &enc.nodes {
        let inputs: Vec<String> = node.inputs.iter().map(|s| format!("enc.{s}")).collect();
        let outputs: Vec<String> = node.outputs.iter().map(|s| format!("enc.{s}")).collect();
        g.add_node(node.op.clone(), inputs, outputs);
    }
    for (name, meta) in &enc.tensors {
        g.add_tensor(format!("enc.{name}"), meta.clone());
    }

    let enc_output = "enc.encoder_output".to_string();

    // --- Decoder with cross-attention ---
    let dec_embed_out = "dec.embed_out".to_string();
    g.add_node(
        Op::TokenEmbed,
        vec!["dec.input_ids".to_string()],
        vec![dec_embed_out.clone()],
    );

    let mut prev_hidden = dec_embed_out;

    for i in 0..dec_config.num_layers {
        let prefix = format!("dec.layer_{i}");

        // Self-attention norm
        let norm1_out = format!("{prefix}.attn_norm_out");
        g.add_node(
            Op::RmsNorm { eps: dec_config.eps },
            vec![prev_hidden.clone(), format!("{prefix}.attn_norm.weight")],
            vec![norm1_out.clone()],
        );

        // Self-attention QKV
        let qkv_out = format!("{prefix}.qkv_out");
        g.add_node(
            Op::Matmul,
            vec![norm1_out, format!("{prefix}.qkv.weight")],
            vec![qkv_out.clone()],
        );

        // KV cache for decoder self-attention
        let kv_out = format!("{prefix}.kv_cached");
        g.add_node(Op::KvCache, vec![qkv_out.clone()], vec![kv_out.clone()]);

        // Causal self-attention
        let self_attn_out = format!("{prefix}.self_attn_out");
        g.add_node(
            Op::Sdpa {
                num_heads: dec_config.num_heads as u32,
                kv_heads: dec_config.kv_num_heads as u32,
                head_dim: dec_config.head_dim as u32,
                causal: true,
            },
            vec![qkv_out, kv_out],
            vec![self_attn_out.clone()],
        );

        let self_attn_proj = format!("{prefix}.self_attn_proj");
        g.add_node(
            Op::Matmul,
            vec![self_attn_out, format!("{prefix}.self_o_proj.weight")],
            vec![self_attn_proj.clone()],
        );

        // Residual after self-attention
        let residual1 = format!("{prefix}.residual1");
        g.add_node(
            Op::Add,
            vec![prev_hidden.clone(), self_attn_proj],
            vec![residual1.clone()],
        );

        // Cross-attention norm
        let cross_norm_out = format!("{prefix}.cross_norm_out");
        g.add_node(
            Op::RmsNorm { eps: dec_config.eps },
            vec![residual1.clone(), format!("{prefix}.cross_norm.weight")],
            vec![cross_norm_out.clone()],
        );

        // Cross-attention Q from decoder, KV from encoder
        let cross_q = format!("{prefix}.cross_q");
        g.add_node(
            Op::Matmul,
            vec![cross_norm_out, format!("{prefix}.cross_q.weight")],
            vec![cross_q.clone()],
        );

        let cross_attn_out = format!("{prefix}.cross_attn_out");
        g.add_node(
            Op::SdpaCross {
                num_heads: dec_config.num_heads as u32,
                head_dim: dec_config.head_dim as u32,
            },
            vec![cross_q, enc_output.clone()],
            vec![cross_attn_out.clone()],
        );

        let cross_proj = format!("{prefix}.cross_proj");
        g.add_node(
            Op::Matmul,
            vec![cross_attn_out, format!("{prefix}.cross_o_proj.weight")],
            vec![cross_proj.clone()],
        );

        // Residual after cross-attention
        let residual2 = format!("{prefix}.residual2");
        g.add_node(
            Op::Add,
            vec![residual1, cross_proj],
            vec![residual2.clone()],
        );

        // FFN
        let norm3_out = format!("{prefix}.ffn_norm_out");
        g.add_node(
            Op::RmsNorm { eps: dec_config.eps },
            vec![residual2.clone(), format!("{prefix}.ffn_norm.weight")],
            vec![norm3_out.clone()],
        );

        let gate_out = format!("{prefix}.gate_out");
        g.add_node(
            Op::Matmul,
            vec![norm3_out.clone(), format!("{prefix}.gate_proj.weight")],
            vec![gate_out.clone()],
        );

        let up_out = format!("{prefix}.up_out");
        g.add_node(
            Op::Matmul,
            vec![norm3_out, format!("{prefix}.up_proj.weight")],
            vec![up_out.clone()],
        );

        let silu_out = format!("{prefix}.silu_out");
        g.add_node(Op::Silu, vec![gate_out], vec![silu_out.clone()]);

        let act_out = format!("{prefix}.act_out");
        g.add_node(Op::Mul, vec![silu_out, up_out], vec![act_out.clone()]);

        let down_out = format!("{prefix}.down_out");
        g.add_node(
            Op::Matmul,
            vec![act_out, format!("{prefix}.down_proj.weight")],
            vec![down_out.clone()],
        );

        let residual3 = format!("{prefix}.residual3");
        g.add_node(
            Op::Add,
            vec![residual2, down_out],
            vec![residual3.clone()],
        );

        prev_hidden = residual3;
    }

    // Final norm + lm_head
    let final_norm = "dec.final_norm_out".to_string();
    g.add_node(
        Op::RmsNorm { eps: dec_config.eps },
        vec![prev_hidden, "dec.model.norm.weight".to_string()],
        vec![final_norm.clone()],
    );

    g.add_node(
        Op::Matmul,
        vec![final_norm, "dec.lm_head.weight".to_string()],
        vec!["logits".to_string()],
    );

    log::info!(
        "encoder_decoder template: enc {} layers + dec {} layers, {} nodes",
        enc_config.num_layers,
        dec_config.num_layers,
        g.len()
    );

    g
}
