//! Architecture templates — generate Graph IR for common model architectures
//!
//! For safetensors/GGUF models, the runtime has built-in templates.
//! config comes from config.json (HuggingFace) or GGUF metadata.
//! template + config = concrete graph.

use std::collections::HashMap;

use super::graph::{Graph, TensorMeta, Dim, Residency, AttrValue, Attrs};
use super::ops::{Op, InterpolateMode};
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

/// Build a transformer decoder graph for the Graph IR executor.
///
/// Unlike `transformer_decoder`, this generates a graph with:
/// - **Standard HuggingFace weight names** (`model.layers.{i}.self_attn.q_proj.weight`)
/// - **Separate Q, K, V projections** (not merged QKV)
/// - **Attrs on every node** with hidden_size, num_heads, etc. for the executor
/// - **RoPE applied separately** to Q and K
///
/// This graph can be executed by `GraphExecutor::execute_decode`.
pub fn transformer_decoder_for_exec(config: &TransformerConfig) -> Graph {
    let mut g = Graph::new();

    let seq_dim = Dim::Dynamic("seq_len".to_string());
    let hidden = config.hidden_size;
    let inter = config.intermediate_size;
    let kv_dim = config.kv_num_heads * config.head_dim;
    let q_dim = config.num_heads * config.head_dim;

    // Token embedding: input_ids -> hidden
    let embed_out = "embed_out".to_string();
    g.add_node_with_attrs(
        Op::TokenEmbed,
        vec!["input_ids".to_string()],
        vec![embed_out.clone()],
        make_attrs(&[("hidden", hidden as i64)]),
        None,
    );
    g.add_tensor("input_ids".to_string(), TensorMeta {
        shape: vec![seq_dim.clone()],
        dtype: DType::U8,
        residency: Residency::Streamed,
    });
    g.add_tensor(embed_out.clone(), TensorMeta {
        shape: vec![seq_dim.clone(), Dim::Fixed(hidden)],
        dtype: DType::F32,
        residency: Residency::Streamed,
    });

    let mut prev_hidden = embed_out;

    for i in 0..config.num_layers {
        let prefix = format!("layer_{i}");
        let hf = format!("model.layers.{i}");

        // Input norm
        let norm1_out = format!("{prefix}.attn_norm_out");
        g.add_node_with_attrs(
            Op::RmsNorm { eps: config.eps },
            vec![prev_hidden.clone(), format!("{hf}.input_layernorm.weight")],
            vec![norm1_out.clone()],
            make_attrs(&[("hidden", hidden as i64), ("positions", 1)]),
            None,
        );

        // Q projection
        let q_out = format!("{prefix}.q_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out.clone(), format!("{hf}.self_attn.q_proj.weight")],
            vec![q_out.clone()],
            make_attrs(&[("n", q_dim as i64), ("k", hidden as i64)]),
            None,
        );

        // K projection
        let k_out = format!("{prefix}.k_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out.clone(), format!("{hf}.self_attn.k_proj.weight")],
            vec![k_out.clone()],
            make_attrs(&[("n", kv_dim as i64), ("k", hidden as i64)]),
            None,
        );

        // V projection
        let v_out = format!("{prefix}.v_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out, format!("{hf}.self_attn.v_proj.weight")],
            vec![v_out.clone()],
            make_attrs(&[("n", kv_dim as i64), ("k", hidden as i64)]),
            None,
        );

        // RoPE on Q
        let q_roped = format!("{prefix}.q_roped");
        g.add_node_with_attrs(
            Op::Rope {
                head_dim: config.head_dim as u32,
                base: config.rope_theta,
            },
            vec![q_out],
            vec![q_roped.clone()],
            make_attrs(&[("total_elements", q_dim as i64), ("n", q_dim as i64)]),
            None,
        );

        // RoPE on K
        let k_roped = format!("{prefix}.k_roped");
        g.add_node_with_attrs(
            Op::Rope {
                head_dim: config.head_dim as u32,
                base: config.rope_theta,
            },
            vec![k_out],
            vec![k_roped.clone()],
            make_attrs(&[("total_elements", kv_dim as i64), ("n", kv_dim as i64)]),
            None,
        );

        // KV cache (passthrough in the graph — actual caching is in executor)
        let kv_out = format!("{prefix}.kv_cached");
        g.add_node(
            Op::KvCache,
            vec![k_roped.clone()],
            vec![kv_out.clone()],
        );
        g.add_tensor(kv_out.clone(), TensorMeta {
            shape: vec![
                Dim::Dynamic("total_seq".to_string()),
                Dim::Fixed(kv_dim),
            ],
            dtype: DType::F32,
            residency: Residency::Cached,
        });

        // Scaled dot-product attention
        // v_tensor attr tells the executor where to find the V buffer
        let attn_out = format!("{prefix}.attn_out");
        let mut sdpa_attrs = make_attrs(&[("hidden", hidden as i64)]);
        sdpa_attrs.insert("v_tensor".to_string(), AttrValue::String(v_out.clone()));
        g.add_node_with_attrs(
            Op::Sdpa {
                num_heads: config.num_heads as u32,
                kv_heads: config.kv_num_heads as u32,
                head_dim: config.head_dim as u32,
                causal: true,
            },
            vec![q_roped, kv_out],
            vec![attn_out.clone()],
            sdpa_attrs,
            None,
        );

        // Output projection
        let o_proj_out = format!("{prefix}.o_proj_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn_out, format!("{hf}.self_attn.o_proj.weight")],
            vec![o_proj_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", q_dim as i64)]),
            None,
        );

        // Residual add
        let residual1 = format!("{prefix}.residual1");
        g.add_node_with_attrs(
            Op::Add,
            vec![prev_hidden.clone(), o_proj_out],
            vec![residual1.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        // FFN norm
        let norm2_out = format!("{prefix}.ffn_norm_out");
        g.add_node_with_attrs(
            Op::RmsNorm { eps: config.eps },
            vec![residual1.clone(), format!("{hf}.post_attention_layernorm.weight")],
            vec![norm2_out.clone()],
            make_attrs(&[("hidden", hidden as i64), ("positions", 1)]),
            None,
        );

        // FFN: gate + up projections
        let gate_out = format!("{prefix}.gate_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm2_out.clone(), format!("{hf}.mlp.gate_proj.weight")],
            vec![gate_out.clone()],
            make_attrs(&[("n", inter as i64), ("k", hidden as i64)]),
            None,
        );

        let up_out = format!("{prefix}.up_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm2_out, format!("{hf}.mlp.up_proj.weight")],
            vec![up_out.clone()],
            make_attrs(&[("n", inter as i64), ("k", hidden as i64)]),
            None,
        );

        // Activation gate
        let act_out = format!("{prefix}.act_out");
        match config.activation {
            Activation::Silu => {
                // SwiGLU: silu(gate) * up
                let silu_out = format!("{prefix}.silu_out");
                g.add_node_with_attrs(
                    Op::Silu,
                    vec![gate_out],
                    vec![silu_out.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
                g.add_node_with_attrs(
                    Op::Mul,
                    vec![silu_out, up_out],
                    vec![act_out.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
            }
            Activation::Gelu => {
                let gelu_out = format!("{prefix}.gelu_out");
                g.add_node_with_attrs(
                    Op::Gelu { approximate: false },
                    vec![gate_out],
                    vec![gelu_out.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
                g.add_node_with_attrs(
                    Op::Mul,
                    vec![gelu_out, up_out],
                    vec![act_out.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
            }
            Activation::GeGlu => {
                g.add_node_with_attrs(
                    Op::GeGlu,
                    vec![gate_out, up_out],
                    vec![act_out.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
            }
        }

        // Down projection
        let down_out = format!("{prefix}.down_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![act_out, format!("{hf}.mlp.down_proj.weight")],
            vec![down_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", inter as i64)]),
            None,
        );

        // Residual add
        let residual2 = format!("{prefix}.residual2");
        g.add_node_with_attrs(
            Op::Add,
            vec![residual1, down_out],
            vec![residual2.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        prev_hidden = residual2;
    }

    // Final norm
    let final_norm = "final_norm_out".to_string();
    g.add_node_with_attrs(
        Op::RmsNorm { eps: config.eps },
        vec![prev_hidden, "model.norm.weight".to_string()],
        vec![final_norm.clone()],
        make_attrs(&[("hidden", hidden as i64), ("positions", 1)]),
        None,
    );

    // LM head
    g.add_node_with_attrs(
        Op::Matmul,
        vec![final_norm, "lm_head.weight".to_string()],
        vec!["logits".to_string()],
        make_attrs(&[("n", config.vocab_size as i64), ("k", hidden as i64)]),
        None,
    );

    g.add_tensor("logits".to_string(), TensorMeta {
        shape: vec![seq_dim, Dim::Fixed(config.vocab_size)],
        dtype: DType::F32,
        residency: Residency::Streamed,
    });

    log::info!(
        "transformer_decoder_for_exec template: {} layers, {} nodes, weight names = HF convention",
        config.num_layers,
        g.len()
    );

    g
}

/// Helper: build an Attrs map from a slice of (key, int_value) pairs
fn make_attrs(pairs: &[(&str, i64)]) -> Attrs {
    let mut m: Attrs = HashMap::new();
    for &(k, v) in pairs {
        m.insert(k.to_string(), AttrValue::Int(v));
    }
    m
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
    {
        let mut pos_node = crate::ir::graph::Node {
            id: 0,
            op: Op::PosEmbed,
            inputs: vec!["position_ids".to_string()],
            outputs: vec!["pos_embed".to_string()],
            attrs: HashMap::new(),
            backend_hint: None,
        };
        pos_node.attrs.insert(
            "embed_weight".to_string(),
            AttrValue::String("embeddings.position_embeddings.weight".to_string()),
        );
        g.nodes.push(pos_node);
    }
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

// ========================================================================
// DiT (Diffusion Transformer) template
// ========================================================================

/// Configuration for DiT-family models (Flux, SD3, Wan2.2)
#[derive(Clone, Debug)]
pub struct DiTConfig {
    pub hidden_size: usize,
    pub num_heads: usize,
    pub head_dim: usize,
    pub num_layers: usize,
    pub intermediate_size: usize,
    pub patch_size: usize,
    pub in_channels: usize,
    pub eps: f32,
    pub time_embed_dim: usize,
}

impl Default for DiTConfig {
    fn default() -> Self {
        Self {
            hidden_size: 1152,
            num_heads: 16,
            head_dim: 72,
            num_layers: 28,
            intermediate_size: 4608,
            patch_size: 2,
            in_channels: 4,
            eps: 1e-6,
            time_embed_dim: 256,
        }
    }
}

/// Build a DiT (Diffusion Transformer) graph
///
/// Structure per layer:
///   adaln(x, t_embed) -> attention -> residual ->
///   adaln(x, t_embed) -> mlp(up, gelu, down) -> residual
///
/// Used by: Flux, SD3, Wan2.2, HunyuanDiT
pub fn diffusion_dit(config: &DiTConfig) -> Graph {
    let mut g = Graph::new();

    let seq_dim = Dim::Dynamic("num_patches".to_string());
    let hidden = config.hidden_size;

    // Patch embedding: image -> patch tokens
    let patch_out = "patch_embed_out".to_string();
    g.add_node(
        Op::PatchEmbed { patch_size: config.patch_size as u32 },
        vec!["latent_input".to_string()],
        vec![patch_out.clone()],
    );
    g.add_tensor("latent_input".to_string(), TensorMeta {
        shape: vec![
            Dim::Fixed(config.in_channels),
            Dim::Dynamic("height".to_string()),
            Dim::Dynamic("width".to_string()),
        ],
        dtype: DType::F16,
        residency: Residency::Streamed,
    });
    g.add_tensor(patch_out.clone(), TensorMeta {
        shape: vec![seq_dim.clone(), Dim::Fixed(hidden)],
        dtype: DType::F16,
        residency: Residency::Streamed,
    });

    // Sinusoidal timestep embedding
    let time_embed = "time_embed".to_string();
    g.add_node(
        Op::SinusoidalEmbed { dim: config.time_embed_dim as u32 },
        vec!["timestep".to_string()],
        vec![time_embed.clone()],
    );

    // Project timestep embedding to hidden_size * 6 (for scale, shift, gate per adaln)
    let time_proj = "time_proj".to_string();
    g.add_node(
        Op::Matmul,
        vec![time_embed, "time_embed.proj.weight".to_string()],
        vec![time_proj.clone()],
    );

    let mut prev_hidden = patch_out;

    for i in 0..config.num_layers {
        let prefix = format!("dit_layer_{i}");

        // AdaLN for attention
        let adaln1_out = format!("{prefix}.adaln1_out");
        g.add_node(
            Op::AdaLN,
            vec![
                prev_hidden.clone(),
                time_proj.clone(),
                format!("{prefix}.adaln1.weight"),
                format!("{prefix}.adaln1.bias"),
            ],
            vec![adaln1_out.clone()],
        );

        // Self-attention QKV
        let qkv_out = format!("{prefix}.qkv_out");
        g.add_node(
            Op::Matmul,
            vec![adaln1_out, format!("{prefix}.qkv.weight")],
            vec![qkv_out.clone()],
        );

        // Self-attention (non-causal for diffusion)
        let attn_out = format!("{prefix}.attn_out");
        g.add_node(
            Op::Sdpa {
                num_heads: config.num_heads as u32,
                kv_heads: config.num_heads as u32,
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

        // AdaLN for FFN
        let adaln2_out = format!("{prefix}.adaln2_out");
        g.add_node(
            Op::AdaLN,
            vec![
                residual1.clone(),
                time_proj.clone(),
                format!("{prefix}.adaln2.weight"),
                format!("{prefix}.adaln2.bias"),
            ],
            vec![adaln2_out.clone()],
        );

        // FFN: up -> gelu -> down
        let up_out = format!("{prefix}.up_out");
        g.add_node(
            Op::Matmul,
            vec![adaln2_out, format!("{prefix}.up_proj.weight")],
            vec![up_out.clone()],
        );

        let act_out = format!("{prefix}.act_out");
        g.add_node(
            Op::Gelu { approximate: true },
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

    // Final AdaLN + linear to reconstruct patches
    let final_adaln = "final_adaln_out".to_string();
    g.add_node(
        Op::AdaLN,
        vec![
            prev_hidden,
            time_proj,
            "final_adaln.weight".to_string(),
            "final_adaln.bias".to_string(),
        ],
        vec![final_adaln.clone()],
    );

    let final_proj = "final_proj_out".to_string();
    g.add_node(
        Op::Matmul,
        vec![final_adaln, "final_proj.weight".to_string()],
        vec![final_proj.clone()],
    );

    // Unpatchify: reassemble patches into image
    g.add_node(
        Op::Unpatchify,
        vec![final_proj],
        vec!["noise_pred".to_string()],
    );

    g.add_tensor("noise_pred".to_string(), TensorMeta {
        shape: vec![
            Dim::Fixed(config.in_channels),
            Dim::Dynamic("height".to_string()),
            Dim::Dynamic("width".to_string()),
        ],
        dtype: DType::F16,
        residency: Residency::Streamed,
    });

    log::info!(
        "diffusion_dit template: {} layers, {} nodes",
        config.num_layers,
        g.len()
    );

    g
}

// ========================================================================
// CNN Detector template (YOLO-family)
// ========================================================================

/// Configuration for CNN detection models
#[derive(Clone, Debug)]
pub struct CnnDetectorConfig {
    /// Backbone channel progression (e.g. [64, 128, 256, 512, 1024])
    pub backbone_channels: Vec<usize>,
    /// Number of convolution blocks per stage
    pub blocks_per_stage: Vec<usize>,
    /// FPN output channels
    pub fpn_channels: usize,
    /// Number of detection classes
    pub num_classes: usize,
    /// Number of anchor boxes per position
    pub num_anchors: usize,
    /// Input image channels (3 for RGB)
    pub in_channels: usize,
    /// BatchNorm epsilon
    pub eps: f32,
}

impl Default for CnnDetectorConfig {
    fn default() -> Self {
        Self {
            backbone_channels: vec![64, 128, 256, 512, 1024],
            blocks_per_stage: vec![1, 2, 8, 8, 4],
            fpn_channels: 256,
            num_classes: 80,
            num_anchors: 3,
            in_channels: 3,
            eps: 1e-5,
        }
    }
}

/// Build a CNN detector graph (YOLO-family)
///
/// Structure:
///   Conv backbone (stages with stride-2 downsampling) ->
///   FPN neck (feature pyramid with lateral connections + upsampling) ->
///   Detection head (conv -> class + box predictions per scale)
pub fn cnn_detector(config: &CnnDetectorConfig) -> Graph {
    let mut g = Graph::new();

    let height = Dim::Dynamic("height".to_string());
    let width = Dim::Dynamic("width".to_string());

    // Input image
    g.add_tensor("image".to_string(), TensorMeta {
        shape: vec![Dim::Fixed(config.in_channels), height.clone(), width.clone()],
        dtype: DType::F16,
        residency: Residency::Streamed,
    });

    let mut prev_out = "image".to_string();
    let mut stage_outputs: Vec<String> = Vec::new();

    // ---- Backbone: conv stages ----
    for (stage_idx, &out_channels) in config.backbone_channels.iter().enumerate() {
        let in_ch = if stage_idx == 0 { config.in_channels } else { config.backbone_channels[stage_idx - 1] };
        let prefix = format!("backbone.stage_{stage_idx}");

        // Stride-2 downsample conv at start of each stage (except first may be stride-1)
        let stride = if stage_idx == 0 { 1 } else { 2 };

        let blocks = if stage_idx < config.blocks_per_stage.len() {
            config.blocks_per_stage[stage_idx]
        } else {
            1
        };

        for block_idx in 0..blocks {
            let block_prefix = format!("{prefix}.block_{block_idx}");
            let _block_in_ch = if block_idx == 0 { in_ch } else { out_channels };
            let block_stride = if block_idx == 0 { stride } else { 1 };

            // Conv -> BatchNorm -> LeakyReLU
            let conv_out = format!("{block_prefix}.conv_out");
            g.add_node(
                Op::Conv2d {
                    kernel: (3, 3),
                    stride: (block_stride as u32, block_stride as u32),
                    padding: (1, 1),
                    groups: 1,
                },
                vec![prev_out.clone(), format!("{block_prefix}.conv.weight"), format!("{block_prefix}.conv.bias")],
                vec![conv_out.clone()],
            );

            let bn_out = format!("{block_prefix}.bn_out");
            g.add_node(
                Op::BatchNorm { eps: config.eps, momentum: 0.1 },
                vec![
                    conv_out,
                    format!("{block_prefix}.bn.running_mean"),
                    format!("{block_prefix}.bn.running_var"),
                    format!("{block_prefix}.bn.weight"),
                    format!("{block_prefix}.bn.bias"),
                ],
                vec![bn_out.clone()],
            );

            let act_out = format!("{block_prefix}.act_out");
            g.add_node(
                Op::LeakyRelu { slope: 0.1 },
                vec![bn_out],
                vec![act_out.clone()],
            );

            prev_out = act_out;
        }

        stage_outputs.push(prev_out.clone());
    }

    // ---- FPN Neck: lateral connections + top-down upsampling ----
    let num_fpn_levels = stage_outputs.len().min(3); // Use last 3 backbone stages
    let start = stage_outputs.len() - num_fpn_levels;
    let mut fpn_outputs: Vec<String> = Vec::new();

    // Lateral connections (1x1 conv to reduce channels)
    let mut lateral_outputs: Vec<String> = Vec::new();
    for (i, stage_out) in stage_outputs[start..].iter().enumerate() {
        let lat_out = format!("fpn.lateral_{i}");
        g.add_node(
            Op::Conv2d {
                kernel: (1, 1),
                stride: (1, 1),
                padding: (0, 0),
                groups: 1,
            },
            vec![stage_out.clone(), format!("fpn.lateral_{i}.weight"), format!("fpn.lateral_{i}.bias")],
            vec![lat_out.clone()],
        );
        lateral_outputs.push(lat_out);
    }

    // Top-down pathway: upsample + add
    let last_idx = lateral_outputs.len() - 1;
    let mut prev_fpn = lateral_outputs[last_idx].clone();
    fpn_outputs.push(prev_fpn.clone());

    for i in (0..last_idx).rev() {
        let upsampled = format!("fpn.upsample_{i}");
        g.add_node(
            Op::Interpolate { mode: InterpolateMode::Nearest, scale: 2.0 },
            vec![prev_fpn],
            vec![upsampled.clone()],
        );

        let merged = format!("fpn.merge_{i}");
        g.add_node(
            Op::Add,
            vec![lateral_outputs[i].clone(), upsampled],
            vec![merged.clone()],
        );

        // Smooth conv
        let smooth = format!("fpn.smooth_{i}");
        g.add_node(
            Op::Conv2d {
                kernel: (3, 3),
                stride: (1, 1),
                padding: (1, 1),
                groups: 1,
            },
            vec![merged, format!("fpn.smooth_{i}.weight"), format!("fpn.smooth_{i}.bias")],
            vec![smooth.clone()],
        );

        fpn_outputs.push(smooth.clone());
        prev_fpn = smooth;
    }

    fpn_outputs.reverse(); // smallest to largest

    // ---- Detection Head ----
    let output_per_anchor = 4 + 1 + config.num_classes; // box + objectness + classes
    for (scale_idx, fpn_out) in fpn_outputs.iter().enumerate() {
        let head_prefix = format!("head.scale_{scale_idx}");

        // Head conv
        let head_conv = format!("{head_prefix}.conv_out");
        g.add_node(
            Op::Conv2d {
                kernel: (1, 1),
                stride: (1, 1),
                padding: (0, 0),
                groups: 1,
            },
            vec![
                fpn_out.clone(),
                format!("{head_prefix}.conv.weight"),
                format!("{head_prefix}.conv.bias"),
            ],
            vec![head_conv.clone()],
        );

        g.add_tensor(head_conv, TensorMeta {
            shape: vec![
                Dim::Fixed(config.num_anchors * output_per_anchor),
                Dim::Dynamic(format!("h_{scale_idx}")),
                Dim::Dynamic(format!("w_{scale_idx}")),
            ],
            dtype: DType::F32,
            residency: Residency::Streamed,
        });
    }

    log::info!(
        "cnn_detector template: {} backbone stages, {} FPN levels, {} nodes",
        config.backbone_channels.len(),
        fpn_outputs.len(),
        g.len()
    );

    g
}

// ========================================================================
// MoE (Mixture of Experts) Decoder template
// ========================================================================

/// Configuration for Mixture of Experts decoder
#[derive(Clone, Debug)]
pub struct MoeDecoderConfig {
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
    /// Number of expert MLPs
    pub num_experts: usize,
    /// Number of experts activated per token
    pub top_k: usize,
    pub activation: Activation,
}

impl Default for MoeDecoderConfig {
    fn default() -> Self {
        Self {
            hidden_size: 4096,
            num_heads: 32,
            kv_num_heads: 8,
            head_dim: 128,
            num_layers: 32,
            intermediate_size: 14336,
            vocab_size: 32000,
            eps: 1e-5,
            rope_theta: 10000.0,
            max_seq_len: 32768,
            num_experts: 8,
            top_k: 2,
            activation: Activation::Silu,
        }
    }
}

/// Build a Mixture of Experts decoder graph (Mixtral, DeepSeek-MoE, etc.)
///
/// Structure per layer:
///   rmsnorm -> attention(qkv, rope, sdpa, kv_cache, o_proj) ->
///   rmsnorm -> router(gate) -> top_k expert selection ->
///   expert_i(gate_proj + up_proj, silu_mul, down_proj) for selected experts ->
///   weighted sum of expert outputs
pub fn moe_decoder(config: &MoeDecoderConfig) -> Graph {
    let mut g = Graph::new();

    let seq_dim = Dim::Dynamic("seq_len".to_string());
    let hidden = config.hidden_size;

    // Token embedding
    let embed_out = "embed_out".to_string();
    g.add_node(
        Op::TokenEmbed,
        vec!["input_ids".to_string()],
        vec![embed_out.clone()],
    );
    g.add_tensor("input_ids".to_string(), TensorMeta {
        shape: vec![seq_dim.clone()],
        dtype: DType::U8,
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

        // === Self-attention block (same as standard transformer) ===

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

        // SDPA
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

        // Residual
        let residual1 = format!("{prefix}.residual1");
        g.add_node(
            Op::Add,
            vec![prev_hidden.clone(), o_proj_out],
            vec![residual1.clone()],
        );

        // === MoE FFN block ===

        // FFN norm
        let norm2_out = format!("{prefix}.ffn_norm_out");
        g.add_node(
            Op::RmsNorm { eps: config.eps },
            vec![residual1.clone(), format!("{prefix}.ffn_norm.weight")],
            vec![norm2_out.clone()],
        );

        // Router: compute expert scores via linear gate
        let router_logits = format!("{prefix}.router_logits");
        g.add_node(
            Op::Matmul,
            vec![norm2_out.clone(), format!("{prefix}.router.weight")],
            vec![router_logits.clone()],
        );

        // Softmax over expert scores
        let router_weights = format!("{prefix}.router_weights");
        g.add_node(
            Op::Softmax { dim: -1 },
            vec![router_logits],
            vec![router_weights.clone()],
        );

        // For each expert, compute the FFN output
        // In practice, the runtime selects top_k experts per token.
        // In the graph IR, we represent all experts and the selection logic.
        let mut expert_outputs: Vec<String> = Vec::new();
        for e in 0..config.num_experts {
            let expert_prefix = format!("{prefix}.expert_{e}");

            // Gate projection
            let gate_out = format!("{expert_prefix}.gate_out");
            g.add_node(
                Op::Matmul,
                vec![norm2_out.clone(), format!("{expert_prefix}.gate_proj.weight")],
                vec![gate_out.clone()],
            );

            // Up projection
            let up_out = format!("{expert_prefix}.up_out");
            g.add_node(
                Op::Matmul,
                vec![norm2_out.clone(), format!("{expert_prefix}.up_proj.weight")],
                vec![up_out.clone()],
            );

            // Activation gate
            let act_out = format!("{expert_prefix}.act_out");
            match config.activation {
                Activation::Silu => {
                    let silu_out = format!("{expert_prefix}.silu_out");
                    g.add_node(Op::Silu, vec![gate_out], vec![silu_out.clone()]);
                    g.add_node(Op::Mul, vec![silu_out, up_out], vec![act_out.clone()]);
                }
                Activation::Gelu => {
                    let gelu_out = format!("{expert_prefix}.gelu_out");
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
            let down_out = format!("{expert_prefix}.down_out");
            g.add_node(
                Op::Matmul,
                vec![act_out, format!("{expert_prefix}.down_proj.weight")],
                vec![down_out.clone()],
            );

            // Scale by router weight
            let scaled = format!("{expert_prefix}.scaled");
            g.add_node(
                Op::Mul,
                vec![down_out, router_weights.clone()],
                vec![scaled.clone()],
            );

            expert_outputs.push(scaled);
        }

        // Sum expert outputs (reduce via pairwise addition)
        let mut accum = expert_outputs[0].clone();
        for e in 1..config.num_experts {
            let sum_name = format!("{prefix}.expert_sum_{e}");
            g.add_node(
                Op::Add,
                vec![accum, expert_outputs[e].clone()],
                vec![sum_name.clone()],
            );
            accum = sum_name;
        }

        // Residual
        let residual2 = format!("{prefix}.residual2");
        g.add_node(
            Op::Add,
            vec![residual1, accum],
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
        "moe_decoder template: {} layers, {} experts, top_{}, {} nodes",
        config.num_layers,
        config.num_experts,
        config.top_k,
        g.len()
    );

    g
}

// ========================================================================
// Whisper encoder-decoder template (GGML weight names)
// ========================================================================

/// Configuration for Whisper models
#[derive(Clone, Debug)]
pub struct WhisperConfig {
    pub n_audio_state: usize,
    pub n_audio_head: usize,
    pub n_audio_layer: usize,
    pub n_audio_ctx: usize,
    pub n_text_state: usize,
    pub n_text_head: usize,
    pub n_text_layer: usize,
    pub n_text_ctx: usize,
    pub n_vocab: usize,
    pub n_mels: usize,
    pub eps: f32,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            n_audio_state: 768,
            n_audio_head: 12,
            n_audio_layer: 12,
            n_audio_ctx: 1500,
            n_text_state: 768,
            n_text_head: 12,
            n_text_layer: 12,
            n_text_ctx: 448,
            n_vocab: 51865,
            n_mels: 80,
            eps: 1e-5,
        }
    }
}

/// Build a Whisper encoder-decoder graph with GGML weight names.
///
/// Encoder: conv1 -> conv2 -> positional_embedding + add -> encoder blocks -> ln_post
/// Each encoder block: attn_ln -> self_attention(Q,K,V,out) -> add -> mlp_ln -> mlp(0,2) -> add
///
/// Decoder: token_embedding + positional_embedding -> decoder blocks -> ln -> lm_head
/// Each decoder block: attn_ln -> self_attn -> add -> cross_attn_ln -> cross_attn -> add -> mlp_ln -> mlp -> add
pub fn whisper_encoder_decoder(config: &WhisperConfig) -> Graph {
    let mut g = Graph::new();

    let audio_state = config.n_audio_state;
    let audio_head_dim = audio_state / config.n_audio_head;
    let text_state = config.n_text_state;
    let text_head_dim = text_state / config.n_text_head;

    // ====================================================================
    // Encoder
    // ====================================================================

    // Conv1: [n_mels, 3] -> [audio_state]
    // Input mel spectrogram has length = n_audio_ctx * 2 (conv2 stride=2 halves it)
    let audio_input_len = config.n_audio_ctx * 2;
    let conv1_out = "enc.conv1_out".to_string();
    g.add_node_with_attrs(
        Op::Conv1d { kernel: 3, stride: 1, padding: 1, groups: 1 },
        vec!["audio_input".to_string(), "encoder.conv1.weight".to_string(), "encoder.conv1.bias".to_string()],
        vec![conv1_out.clone()],
        make_attrs(&[("in_channels", config.n_mels as i64), ("out_channels", audio_state as i64), ("in_length", audio_input_len as i64)]),
        None,
    );
    g.add_tensor("audio_input".to_string(), TensorMeta {
        shape: vec![Dim::Fixed(config.n_mels), Dim::Dynamic("audio_len".to_string())],
        dtype: DType::F32,
        residency: Residency::Streamed,
    });

    // GELU after conv1 — output is [audio_state, audio_input_len]
    let conv1_act = "enc.conv1_act".to_string();
    g.add_node_with_attrs(
        Op::Gelu { approximate: false },
        vec![conv1_out],
        vec![conv1_act.clone()],
        make_attrs(&[("n", (audio_state * audio_input_len) as i64)]),
        None,
    );

    // Conv2: stride 2 downsampling — in_length = audio_input_len (conv1 stride=1 preserves length)
    let conv2_out = "enc.conv2_out".to_string();
    g.add_node_with_attrs(
        Op::Conv1d { kernel: 3, stride: 2, padding: 1, groups: 1 },
        vec![conv1_act, "encoder.conv2.weight".to_string(), "encoder.conv2.bias".to_string()],
        vec![conv2_out.clone()],
        make_attrs(&[("in_channels", audio_state as i64), ("out_channels", audio_state as i64), ("in_length", audio_input_len as i64)]),
        None,
    );

    // GELU after conv2
    let conv2_act = "enc.conv2_act".to_string();
    g.add_node_with_attrs(
        Op::Gelu { approximate: false },
        vec![conv2_out],
        vec![conv2_act.clone()],
        make_attrs(&[("n", (audio_state * config.n_audio_ctx) as i64)]),
        None,
    );

    // Add positional embedding
    let pos_add = "enc.pos_add".to_string();
    g.add_node_with_attrs(
        Op::Add,
        vec![conv2_act, "encoder.positional_embedding".to_string()],
        vec![pos_add.clone()],
        make_attrs(&[("n", (audio_state * config.n_audio_ctx) as i64)]),
        None,
    );

    let mut enc_hidden = pos_add;

    for i in 0..config.n_audio_layer {
        let prefix = format!("enc.block_{i}");
        let w = format!("encoder.blocks.{i}");

        // attn_ln
        let norm1_out = format!("{prefix}.attn_ln_out");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![enc_hidden.clone(), format!("{w}.attn_ln.weight"), format!("{w}.attn_ln.bias")],
            vec![norm1_out.clone()],
            make_attrs(&[("hidden", audio_state as i64), ("positions", config.n_audio_ctx as i64)]),
            None,
        );

        // Q projection
        let q_out = format!("{prefix}.q_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out.clone(), format!("{w}.attn.query.weight")],
            vec![q_out.clone()],
            make_attrs(&[("n", audio_state as i64), ("k", audio_state as i64)]),
            None,
        );
        // Q bias
        let q_biased = format!("{prefix}.q_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![q_out, format!("{w}.attn.query.bias")],
            vec![q_biased.clone()],
            make_attrs(&[("n", audio_state as i64)]),
            None,
        );

        // K projection (no bias in whisper)
        let k_out = format!("{prefix}.k_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out.clone(), format!("{w}.attn.key.weight")],
            vec![k_out.clone()],
            make_attrs(&[("n", audio_state as i64), ("k", audio_state as i64)]),
            None,
        );

        // V projection
        let v_out = format!("{prefix}.v_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out, format!("{w}.attn.value.weight")],
            vec![v_out.clone()],
            make_attrs(&[("n", audio_state as i64), ("k", audio_state as i64)]),
            None,
        );
        let v_biased = format!("{prefix}.v_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![v_out, format!("{w}.attn.value.bias")],
            vec![v_biased.clone()],
            make_attrs(&[("n", audio_state as i64)]),
            None,
        );

        // Bidirectional attention (no causal mask, no KV cache for encoder)
        let attn_out = format!("{prefix}.attn_out");
        let mut sdpa_attrs = make_attrs(&[("hidden", audio_state as i64), ("positions", config.n_audio_ctx as i64)]);
        sdpa_attrs.insert("v_tensor".to_string(), AttrValue::String(v_biased.clone()));
        g.add_node_with_attrs(
            Op::Sdpa {
                num_heads: config.n_audio_head as u32,
                kv_heads: config.n_audio_head as u32,
                head_dim: audio_head_dim as u32,
                causal: false,
            },
            vec![q_biased, k_out],
            vec![attn_out.clone()],
            sdpa_attrs,
            None,
        );

        // Output projection
        let o_out = format!("{prefix}.o_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn_out, format!("{w}.attn.out.weight")],
            vec![o_out.clone()],
            make_attrs(&[("n", audio_state as i64), ("k", audio_state as i64)]),
            None,
        );
        let o_biased = format!("{prefix}.o_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![o_out, format!("{w}.attn.out.bias")],
            vec![o_biased.clone()],
            make_attrs(&[("n", audio_state as i64)]),
            None,
        );

        // Residual add
        let residual1 = format!("{prefix}.residual1");
        g.add_node_with_attrs(
            Op::Add,
            vec![enc_hidden.clone(), o_biased],
            vec![residual1.clone()],
            make_attrs(&[("n", audio_state as i64)]),
            None,
        );

        // mlp_ln
        let norm2_out = format!("{prefix}.mlp_ln_out");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![residual1.clone(), format!("{w}.mlp_ln.weight"), format!("{w}.mlp_ln.bias")],
            vec![norm2_out.clone()],
            make_attrs(&[("hidden", audio_state as i64), ("positions", config.n_audio_ctx as i64)]),
            None,
        );

        // MLP: mlp.0 (up) -> gelu -> mlp.2 (down)
        let mlp_up = format!("{prefix}.mlp_up");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm2_out, format!("{w}.mlp.0.weight")],
            vec![mlp_up.clone()],
            make_attrs(&[("n", (audio_state * 4) as i64), ("k", audio_state as i64)]),
            None,
        );
        let mlp_up_biased = format!("{prefix}.mlp_up_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![mlp_up, format!("{w}.mlp.0.bias")],
            vec![mlp_up_biased.clone()],
            make_attrs(&[("n", (audio_state * 4) as i64)]),
            None,
        );

        let mlp_act = format!("{prefix}.mlp_act");
        g.add_node_with_attrs(
            Op::Gelu { approximate: false },
            vec![mlp_up_biased],
            vec![mlp_act.clone()],
            make_attrs(&[("n", (audio_state * 4) as i64)]),
            None,
        );

        let mlp_down = format!("{prefix}.mlp_down");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![mlp_act, format!("{w}.mlp.2.weight")],
            vec![mlp_down.clone()],
            make_attrs(&[("n", audio_state as i64), ("k", (audio_state * 4) as i64)]),
            None,
        );
        let mlp_down_biased = format!("{prefix}.mlp_down_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![mlp_down, format!("{w}.mlp.2.bias")],
            vec![mlp_down_biased.clone()],
            make_attrs(&[("n", audio_state as i64)]),
            None,
        );

        // Residual add
        let residual2 = format!("{prefix}.residual2");
        g.add_node_with_attrs(
            Op::Add,
            vec![residual1, mlp_down_biased],
            vec![residual2.clone()],
            make_attrs(&[("n", audio_state as i64)]),
            None,
        );

        enc_hidden = residual2;
    }

    // ln_post
    let enc_output = "enc.output".to_string();
    g.add_node_with_attrs(
        Op::LayerNorm { eps: config.eps },
        vec![enc_hidden, "encoder.ln_post.weight".to_string(), "encoder.ln_post.bias".to_string()],
        vec![enc_output.clone()],
        make_attrs(&[("hidden", audio_state as i64), ("positions", config.n_audio_ctx as i64)]),
        None,
    );

    // ====================================================================
    // Decoder
    // ====================================================================

    // Token embedding
    let token_embed = "dec.token_embed".to_string();
    let mut dec_embed_attrs = make_attrs(&[("hidden", text_state as i64)]);
    dec_embed_attrs.insert("embed_weight".to_string(), AttrValue::String("decoder.token_embedding.weight".to_string()));
    g.add_node_with_attrs(
        Op::TokenEmbed,
        vec!["decoder_input_ids".to_string()],
        vec![token_embed.clone()],
        dec_embed_attrs,
        None,
    );
    g.add_tensor("decoder_input_ids".to_string(), TensorMeta {
        shape: vec![Dim::Dynamic("dec_seq".to_string())],
        dtype: DType::U8,
        residency: Residency::Streamed,
    });

    // Positional embedding + add
    let dec_pos_add = "dec.pos_add".to_string();
    g.add_node_with_attrs(
        Op::Add,
        vec![token_embed, "decoder.positional_embedding".to_string()],
        vec![dec_pos_add.clone()],
        make_attrs(&[("n", text_state as i64)]),
        None,
    );

    let mut dec_hidden = dec_pos_add;

    for i in 0..config.n_text_layer {
        let prefix = format!("dec.block_{i}");
        let w = format!("decoder.blocks.{i}");

        // Self-attention: attn_ln -> Q,K,V -> causal SDPA -> out -> residual
        let norm1_out = format!("{prefix}.attn_ln_out");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![dec_hidden.clone(), format!("{w}.attn_ln.weight"), format!("{w}.attn_ln.bias")],
            vec![norm1_out.clone()],
            make_attrs(&[("hidden", text_state as i64), ("positions", 1)]),
            None,
        );

        let q_out = format!("{prefix}.q_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out.clone(), format!("{w}.attn.query.weight")],
            vec![q_out.clone()],
            make_attrs(&[("n", text_state as i64), ("k", text_state as i64)]),
            None,
        );
        let q_biased = format!("{prefix}.q_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![q_out, format!("{w}.attn.query.bias")],
            vec![q_biased.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        let k_out = format!("{prefix}.k_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out.clone(), format!("{w}.attn.key.weight")],
            vec![k_out.clone()],
            make_attrs(&[("n", text_state as i64), ("k", text_state as i64)]),
            None,
        );

        let v_out = format!("{prefix}.v_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1_out, format!("{w}.attn.value.weight")],
            vec![v_out.clone()],
            make_attrs(&[("n", text_state as i64), ("k", text_state as i64)]),
            None,
        );
        let v_biased = format!("{prefix}.v_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![v_out, format!("{w}.attn.value.bias")],
            vec![v_biased.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        // KV cache for decoder self-attention
        let kv_out = format!("{prefix}.kv_cached");
        g.add_node(Op::KvCache, vec![k_out.clone()], vec![kv_out.clone()]);
        g.add_tensor(kv_out.clone(), TensorMeta {
            shape: vec![Dim::Dynamic("total_seq".to_string()), Dim::Fixed(text_state)],
            dtype: DType::F32,
            residency: Residency::Cached,
        });

        // Causal self-attention
        let self_attn_out = format!("{prefix}.self_attn_out");
        let mut sdpa_attrs = make_attrs(&[("hidden", text_state as i64)]);
        sdpa_attrs.insert("v_tensor".to_string(), AttrValue::String(v_biased));
        g.add_node_with_attrs(
            Op::Sdpa {
                num_heads: config.n_text_head as u32,
                kv_heads: config.n_text_head as u32,
                head_dim: text_head_dim as u32,
                causal: true,
            },
            vec![q_biased, kv_out],
            vec![self_attn_out.clone()],
            sdpa_attrs,
            None,
        );

        // Output projection
        let o_out = format!("{prefix}.o_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![self_attn_out, format!("{w}.attn.out.weight")],
            vec![o_out.clone()],
            make_attrs(&[("n", text_state as i64), ("k", text_state as i64)]),
            None,
        );
        let o_biased = format!("{prefix}.o_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![o_out, format!("{w}.attn.out.bias")],
            vec![o_biased.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        let residual1 = format!("{prefix}.residual1");
        g.add_node_with_attrs(
            Op::Add,
            vec![dec_hidden.clone(), o_biased],
            vec![residual1.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        // Cross-attention: cross_attn_ln -> Q from decoder, K/V from encoder
        let cross_norm_out = format!("{prefix}.cross_attn_ln_out");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![residual1.clone(), format!("{w}.cross_attn_ln.weight"), format!("{w}.cross_attn_ln.bias")],
            vec![cross_norm_out.clone()],
            make_attrs(&[("hidden", text_state as i64), ("positions", 1)]),
            None,
        );

        let cross_q = format!("{prefix}.cross_q");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![cross_norm_out, format!("{w}.cross_attn.query.weight")],
            vec![cross_q.clone()],
            make_attrs(&[("n", text_state as i64), ("k", text_state as i64)]),
            None,
        );
        let cross_q_biased = format!("{prefix}.cross_q_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![cross_q, format!("{w}.cross_attn.query.bias")],
            vec![cross_q_biased.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        let cross_attn_out = format!("{prefix}.cross_attn_out");
        g.add_node_with_attrs(
            Op::SdpaCross {
                num_heads: config.n_text_head as u32,
                head_dim: text_head_dim as u32,
            },
            vec![cross_q_biased, enc_output.clone()],
            vec![cross_attn_out.clone()],
            make_attrs(&[("hidden", text_state as i64)]),
            None,
        );

        let cross_o = format!("{prefix}.cross_o");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![cross_attn_out, format!("{w}.cross_attn.out.weight")],
            vec![cross_o.clone()],
            make_attrs(&[("n", text_state as i64), ("k", text_state as i64)]),
            None,
        );
        let cross_o_biased = format!("{prefix}.cross_o_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![cross_o, format!("{w}.cross_attn.out.bias")],
            vec![cross_o_biased.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        let residual2 = format!("{prefix}.residual2");
        g.add_node_with_attrs(
            Op::Add,
            vec![residual1, cross_o_biased],
            vec![residual2.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        // MLP: mlp_ln -> mlp.0 -> gelu -> mlp.2 -> residual
        let mlp_norm_out = format!("{prefix}.mlp_ln_out");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![residual2.clone(), format!("{w}.mlp_ln.weight"), format!("{w}.mlp_ln.bias")],
            vec![mlp_norm_out.clone()],
            make_attrs(&[("hidden", text_state as i64), ("positions", 1)]),
            None,
        );

        let mlp_up = format!("{prefix}.mlp_up");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![mlp_norm_out, format!("{w}.mlp.0.weight")],
            vec![mlp_up.clone()],
            make_attrs(&[("n", (text_state * 4) as i64), ("k", text_state as i64)]),
            None,
        );
        let mlp_up_biased = format!("{prefix}.mlp_up_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![mlp_up, format!("{w}.mlp.0.bias")],
            vec![mlp_up_biased.clone()],
            make_attrs(&[("n", (text_state * 4) as i64)]),
            None,
        );

        let mlp_act = format!("{prefix}.mlp_act");
        g.add_node_with_attrs(
            Op::Gelu { approximate: false },
            vec![mlp_up_biased],
            vec![mlp_act.clone()],
            make_attrs(&[("n", (text_state * 4) as i64)]),
            None,
        );

        let mlp_down = format!("{prefix}.mlp_down");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![mlp_act, format!("{w}.mlp.2.weight")],
            vec![mlp_down.clone()],
            make_attrs(&[("n", text_state as i64), ("k", (text_state * 4) as i64)]),
            None,
        );
        let mlp_down_biased = format!("{prefix}.mlp_down_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![mlp_down, format!("{w}.mlp.2.bias")],
            vec![mlp_down_biased.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        let residual3 = format!("{prefix}.residual3");
        g.add_node_with_attrs(
            Op::Add,
            vec![residual2, mlp_down_biased],
            vec![residual3.clone()],
            make_attrs(&[("n", text_state as i64)]),
            None,
        );

        dec_hidden = residual3;
    }

    // Final decoder layer norm
    let dec_norm_out = "dec.norm_out".to_string();
    g.add_node_with_attrs(
        Op::LayerNorm { eps: config.eps },
        vec![dec_hidden, "decoder.ln.weight".to_string(), "decoder.ln.bias".to_string()],
        vec![dec_norm_out.clone()],
        make_attrs(&[("hidden", text_state as i64), ("positions", 1)]),
        None,
    );

    // LM head: tied to token_embedding (transpose)
    g.add_node_with_attrs(
        Op::Matmul,
        vec![dec_norm_out, "decoder.token_embedding.weight".to_string()],
        vec!["logits".to_string()],
        make_attrs(&[("n", config.n_vocab as i64), ("k", text_state as i64)]),
        None,
    );

    g.add_tensor("logits".to_string(), TensorMeta {
        shape: vec![Dim::Dynamic("dec_seq".to_string()), Dim::Fixed(config.n_vocab)],
        dtype: DType::F32,
        residency: Residency::Streamed,
    });

    log::info!(
        "whisper_encoder_decoder template: enc {} layers + dec {} layers, {} nodes, GGML weight names",
        config.n_audio_layer,
        config.n_text_layer,
        g.len()
    );

    g
}

// ========================================================================
// BERT encoder template (safetensors weight names)
// ========================================================================

/// Configuration for BERT models
#[derive(Clone, Debug)]
pub struct BertConfig {
    pub hidden_size: usize,
    pub num_heads: usize,
    pub head_dim: usize,
    pub num_layers: usize,
    pub intermediate_size: usize,
    pub vocab_size: usize,
    pub max_position_embeddings: usize,
    pub type_vocab_size: usize,
    pub eps: f32,
    /// Weight name prefix (e.g., "roberta", "deberta", "" for plain BERT)
    pub weight_prefix: String,
    /// Number of classification labels (0 = no classifier head, >0 = add classifier)
    pub num_labels: usize,
}

impl Default for BertConfig {
    fn default() -> Self {
        Self {
            hidden_size: 768,
            num_heads: 12,
            head_dim: 64,
            num_layers: 12,
            intermediate_size: 3072,
            vocab_size: 30522,
            max_position_embeddings: 512,
            type_vocab_size: 2,
            eps: 1e-12,
            weight_prefix: String::new(),
            num_labels: 0,
        }
    }
}

/// Build a BERT encoder graph with safetensors weight names.
///
/// Structure:
///   word_embeddings + position_embeddings + token_type_embeddings -> LayerNorm ->
///   encoder blocks (attn_ln -> self_attn(Q,K,V,out) -> add -> ln -> mlp(intermediate,output) -> add -> ln) ->
///   pooler
pub fn bert_encoder(config: &BertConfig) -> Graph {
    let mut g = Graph::new();

    let hidden = config.hidden_size;
    let inter = config.intermediate_size;

    // Weight name prefix (e.g., "roberta." or "deberta." or "")
    let wp = if config.weight_prefix.is_empty() {
        String::new()
    } else {
        format!("{}.", config.weight_prefix)
    };

    // Embedding: word + position + token_type
    let word_embed = "word_embed".to_string();
    let mut word_embed_attrs = make_attrs(&[("hidden", hidden as i64)]);
    word_embed_attrs.insert("embed_weight".to_string(),
        AttrValue::String(format!("{wp}embeddings.word_embeddings.weight")));
    g.add_node_with_attrs(
        Op::TokenEmbed,
        vec!["input_ids".to_string()],
        vec![word_embed.clone()],
        word_embed_attrs,
        None,
    );
    g.add_tensor("input_ids".to_string(), TensorMeta {
        shape: vec![Dim::Dynamic("seq_len".to_string())],
        dtype: DType::U8,
        residency: Residency::Streamed,
    });

    // Position embedding lookup
    let pos_embed = "pos_embed".to_string();
    g.add_node_with_attrs(
        Op::PosEmbed,
        vec!["position_ids".to_string()],
        vec![pos_embed.clone()],
        make_attrs(&[("hidden", hidden as i64)]),
        None,
    );

    // Token type embedding (segment IDs)
    let type_embed = "type_embed".to_string();
    let mut type_embed_attrs = make_attrs(&[("hidden", hidden as i64)]);
    type_embed_attrs.insert("embed_weight".to_string(),
        AttrValue::String(format!("{wp}embeddings.token_type_embeddings.weight")));
    g.add_node_with_attrs(
        Op::TokenEmbed,
        vec!["token_type_ids".to_string()],
        vec![type_embed.clone()],
        type_embed_attrs,
        None,
    );

    // Sum embeddings
    let embed_sum1 = "embed_sum1".to_string();
    g.add_node_with_attrs(
        Op::Add,
        vec![word_embed, pos_embed],
        vec![embed_sum1.clone()],
        make_attrs(&[("n", hidden as i64)]),
        None,
    );
    let embed_sum2 = "embed_sum2".to_string();
    g.add_node_with_attrs(
        Op::Add,
        vec![embed_sum1, type_embed],
        vec![embed_sum2.clone()],
        make_attrs(&[("n", hidden as i64)]),
        None,
    );

    // Embedding LayerNorm
    let embed_norm = "embed_norm".to_string();
    g.add_node_with_attrs(
        Op::LayerNorm { eps: config.eps },
        vec![embed_sum2, format!("{wp}embeddings.LayerNorm.weight"), format!("{wp}embeddings.LayerNorm.bias")],
        vec![embed_norm.clone()],
        make_attrs(&[("hidden", hidden as i64)]),
        None,
    );

    let mut prev_hidden = embed_norm;

    for i in 0..config.num_layers {
        let prefix = format!("layer_{i}");
        let w = format!("{wp}encoder.layer.{i}");

        // Self-attention: Q, K, V projections
        let q_out = format!("{prefix}.q_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![prev_hidden.clone(), format!("{w}.attention.self.query.weight")],
            vec![q_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );
        let q_biased = format!("{prefix}.q_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![q_out, format!("{w}.attention.self.query.bias")],
            vec![q_biased.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        let k_out = format!("{prefix}.k_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![prev_hidden.clone(), format!("{w}.attention.self.key.weight")],
            vec![k_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );
        let k_biased = format!("{prefix}.k_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![k_out, format!("{w}.attention.self.key.bias")],
            vec![k_biased.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        let v_out = format!("{prefix}.v_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![prev_hidden.clone(), format!("{w}.attention.self.value.weight")],
            vec![v_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );
        let v_biased = format!("{prefix}.v_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![v_out, format!("{w}.attention.self.value.bias")],
            vec![v_biased.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        // Bidirectional attention (no causal mask, no KV cache)
        let attn_out = format!("{prefix}.attn_out");
        let mut sdpa_attrs = make_attrs(&[("hidden", hidden as i64)]);
        sdpa_attrs.insert("v_tensor".to_string(), AttrValue::String(v_biased.clone()));
        g.add_node_with_attrs(
            Op::Sdpa {
                num_heads: config.num_heads as u32,
                kv_heads: config.num_heads as u32,
                head_dim: config.head_dim as u32,
                causal: false,
            },
            vec![q_biased, k_biased],
            vec![attn_out.clone()],
            sdpa_attrs,
            None,
        );

        // Attention output dense + residual + LayerNorm
        let attn_dense = format!("{prefix}.attn_dense");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn_out, format!("{w}.attention.output.dense.weight")],
            vec![attn_dense.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );
        let attn_dense_biased = format!("{prefix}.attn_dense_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![attn_dense, format!("{w}.attention.output.dense.bias")],
            vec![attn_dense_biased.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        let residual1 = format!("{prefix}.residual1");
        g.add_node_with_attrs(
            Op::Add,
            vec![prev_hidden.clone(), attn_dense_biased],
            vec![residual1.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        let attn_ln = format!("{prefix}.attn_ln");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![residual1, format!("{w}.attention.output.LayerNorm.weight"), format!("{w}.attention.output.LayerNorm.bias")],
            vec![attn_ln.clone()],
            make_attrs(&[("hidden", hidden as i64)]),
            None,
        );

        // Intermediate dense (up projection)
        let inter_out = format!("{prefix}.inter_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn_ln.clone(), format!("{w}.intermediate.dense.weight")],
            vec![inter_out.clone()],
            make_attrs(&[("n", inter as i64), ("k", hidden as i64)]),
            None,
        );
        let inter_biased = format!("{prefix}.inter_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![inter_out, format!("{w}.intermediate.dense.bias")],
            vec![inter_biased.clone()],
            make_attrs(&[("n", inter as i64)]),
            None,
        );

        // GELU
        let inter_act = format!("{prefix}.inter_act");
        g.add_node_with_attrs(
            Op::Gelu { approximate: false },
            vec![inter_biased],
            vec![inter_act.clone()],
            make_attrs(&[("n", inter as i64)]),
            None,
        );

        // Output dense (down projection)
        let output_dense = format!("{prefix}.output_dense");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![inter_act, format!("{w}.output.dense.weight")],
            vec![output_dense.clone()],
            make_attrs(&[("n", hidden as i64), ("k", inter as i64)]),
            None,
        );
        let output_biased = format!("{prefix}.output_biased");
        g.add_node_with_attrs(
            Op::Add,
            vec![output_dense, format!("{w}.output.dense.bias")],
            vec![output_biased.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        // Residual + LayerNorm
        let residual2 = format!("{prefix}.residual2");
        g.add_node_with_attrs(
            Op::Add,
            vec![attn_ln, output_biased],
            vec![residual2.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        let output_ln = format!("{prefix}.output_ln");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![residual2, format!("{w}.output.LayerNorm.weight"), format!("{w}.output.LayerNorm.bias")],
            vec![output_ln.clone()],
            make_attrs(&[("hidden", hidden as i64)]),
            None,
        );

        prev_hidden = output_ln;
    }

    // Pooler: take [CLS] token, dense + tanh
    let pooler_dense = "pooler_dense".to_string();
    g.add_node_with_attrs(
        Op::Matmul,
        vec![prev_hidden.clone(), format!("{wp}pooler.dense.weight")],
        vec![pooler_dense.clone()],
        make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
        None,
    );
    let pooler_biased = "pooler_biased".to_string();
    g.add_node_with_attrs(
        Op::Add,
        vec![pooler_dense, format!("{wp}pooler.dense.bias")],
        vec![pooler_biased.clone()],
        make_attrs(&[("n", hidden as i64)]),
        None,
    );
    let pooler_out = "pooler_output".to_string();
    g.add_node_with_attrs(
        Op::Tanh,
        vec![pooler_biased],
        vec![pooler_out.clone()],
        make_attrs(&[("n", hidden as i64)]),
        None,
    );

    // Final output tensors
    g.add_tensor("encoder_output".to_string(), TensorMeta {
        shape: vec![Dim::Dynamic("seq_len".to_string()), Dim::Fixed(hidden)],
        dtype: DType::F32,
        residency: Residency::Streamed,
    });

    // Classifier head (optional — for sequence classification models)
    if config.num_labels > 0 {
        // classifier.dense: pooler_output → hidden → tanh
        let cls_dense = "cls_dense".to_string();
        g.add_node_with_attrs(
            Op::Matmul,
            vec![pooler_out.clone(), "classifier.dense.weight".to_string()],
            vec![cls_dense.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );
        let cls_dense_biased = "cls_dense_biased".to_string();
        g.add_node_with_attrs(
            Op::Add,
            vec![cls_dense, "classifier.dense.bias".to_string()],
            vec![cls_dense_biased.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );
        let cls_tanh = "cls_tanh".to_string();
        g.add_node_with_attrs(
            Op::Tanh,
            vec![cls_dense_biased],
            vec![cls_tanh.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );
        // classifier.out_proj: hidden → num_labels
        let cls_out = "cls_out".to_string();
        g.add_node_with_attrs(
            Op::Matmul,
            vec![cls_tanh, "classifier.out_proj.weight".to_string()],
            vec![cls_out.clone()],
            make_attrs(&[("n", config.num_labels as i64), ("k", hidden as i64)]),
            None,
        );
        let logits = "logits".to_string();
        g.add_node_with_attrs(
            Op::Add,
            vec![cls_out, "classifier.out_proj.bias".to_string()],
            vec![logits.clone()],
            make_attrs(&[("n", config.num_labels as i64)]),
            None,
        );
        g.add_tensor(logits, TensorMeta {
            shape: vec![Dim::Fixed(config.num_labels)],
            dtype: DType::F32,
            residency: Residency::Streamed,
        });
    }

    g.add_tensor(pooler_out, TensorMeta {
        shape: vec![Dim::Fixed(hidden)],
        dtype: DType::F32,
        residency: Residency::Streamed,
    });

    log::info!(
        "bert_encoder template: {} layers, {} nodes, safetensors weight names",
        config.num_layers,
        g.len()
    );

    g
}

/// ModernBERT encoder graph.
///
/// Differences from standard BERT:
/// - Fused QKV weight (`attn.Wqkv.weight`): split into Q, K, V at graph level
/// - No bias terms in attention
/// - Different MLP: fused Wi (gate+up), then Wo
/// - ALiBi positional encoding (implicit, no position embeddings)
/// - Weight prefix: `model.`
///
/// Weight names expected (after QKV split by caller):
///   model.layers.{i}.attn.q.weight, model.layers.{i}.attn.k.weight, model.layers.{i}.attn.v.weight
///   model.layers.{i}.attn.Wo.weight
///   model.layers.{i}.attn_norm.weight
///   model.layers.{i}.mlp_norm.weight
///   model.layers.{i}.mlp.Wi_gate.weight, model.layers.{i}.mlp.Wi_up.weight
///   model.layers.{i}.mlp.Wo.weight
///   model.embeddings.tok_embeddings.weight
///   model.final_norm.weight (if exists)
pub fn modernbert_encoder(config: &BertConfig) -> Graph {
    let mut g = Graph::new();
    let hidden = config.hidden_size;
    let inter = config.intermediate_size;

    // Token embedding (no position embeddings — ALiBi is implicit)
    let word_embed = "word_embed".to_string();
    let mut embed_attrs = make_attrs(&[("hidden", hidden as i64)]);
    embed_attrs.insert("embed_weight".to_string(),
        AttrValue::String("model.embeddings.tok_embeddings.weight".to_string()));
    g.add_node_with_attrs(
        Op::TokenEmbed,
        vec!["input_ids".to_string()],
        vec![word_embed.clone()],
        embed_attrs,
        None,
    );
    g.add_tensor("input_ids".to_string(), TensorMeta {
        shape: vec![Dim::Dynamic("seq_len".to_string())],
        dtype: DType::U8,
        residency: Residency::Streamed,
    });

    // Embedding norm
    let embed_norm = "embed_norm".to_string();
    g.add_node_with_attrs(
        Op::LayerNorm { eps: config.eps },
        vec![word_embed, "model.embeddings.norm.weight".to_string(), "model.embeddings.norm.bias".to_string()],
        vec![embed_norm.clone()],
        make_attrs(&[("hidden", hidden as i64)]),
        None,
    );

    let mut prev_hidden = embed_norm;

    for i in 0..config.num_layers {
        let prefix = format!("layer_{i}");
        let w = format!("model.layers.{i}");

        // Attention norm
        let attn_norm_out = format!("{prefix}.attn_norm_out");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![prev_hidden.clone(), format!("{w}.attn_norm.weight"), format!("{w}.attn_norm.bias")],
            vec![attn_norm_out.clone()],
            make_attrs(&[("hidden", hidden as i64)]),
            None,
        );

        // Q, K, V projections (split from fused Wqkv by caller)
        let q_out = format!("{prefix}.q_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn_norm_out.clone(), format!("{w}.attn.q.weight")],
            vec![q_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );
        let k_out = format!("{prefix}.k_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn_norm_out.clone(), format!("{w}.attn.k.weight")],
            vec![k_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );
        let v_out = format!("{prefix}.v_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn_norm_out, format!("{w}.attn.v.weight")],
            vec![v_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );

        // Bidirectional attention
        let attn_out = format!("{prefix}.attn_out");
        let mut sdpa_attrs = make_attrs(&[("hidden", hidden as i64)]);
        sdpa_attrs.insert("v_tensor".to_string(), AttrValue::String(v_out.clone()));
        g.add_node_with_attrs(
            Op::Sdpa {
                num_heads: config.num_heads as u32,
                kv_heads: config.num_heads as u32,
                head_dim: config.head_dim as u32,
                causal: false,
            },
            vec![q_out, k_out],
            vec![attn_out.clone()],
            sdpa_attrs,
            None,
        );

        // Output projection
        let o_out = format!("{prefix}.o_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn_out, format!("{w}.attn.Wo.weight")],
            vec![o_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", hidden as i64)]),
            None,
        );

        // Residual
        let residual1 = format!("{prefix}.residual1");
        g.add_node_with_attrs(
            Op::Add,
            vec![prev_hidden.clone(), o_out],
            vec![residual1.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        // MLP norm
        let mlp_norm_out = format!("{prefix}.mlp_norm_out");
        g.add_node_with_attrs(
            Op::LayerNorm { eps: config.eps },
            vec![residual1.clone(), format!("{w}.mlp_norm.weight"), format!("{w}.mlp_norm.bias")],
            vec![mlp_norm_out.clone()],
            make_attrs(&[("hidden", hidden as i64)]),
            None,
        );

        // MLP: gate + up (fused Wi split by caller), then GELU, then down (Wo)
        let gate_out = format!("{prefix}.gate_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![mlp_norm_out.clone(), format!("{w}.mlp.Wi_gate.weight")],
            vec![gate_out.clone()],
            make_attrs(&[("n", inter as i64), ("k", hidden as i64)]),
            None,
        );
        let up_out = format!("{prefix}.up_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![mlp_norm_out, format!("{w}.mlp.Wi_up.weight")],
            vec![up_out.clone()],
            make_attrs(&[("n", inter as i64), ("k", hidden as i64)]),
            None,
        );

        // GeGLU: GELU(gate) * up
        let geglu_out = format!("{prefix}.geglu_out");
        g.add_node_with_attrs(
            Op::GeGlu,
            vec![gate_out, up_out],
            vec![geglu_out.clone()],
            make_attrs(&[("n", inter as i64)]),
            None,
        );

        // Down projection
        let down_out = format!("{prefix}.down_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![geglu_out, format!("{w}.mlp.Wo.weight")],
            vec![down_out.clone()],
            make_attrs(&[("n", hidden as i64), ("k", inter as i64)]),
            None,
        );

        // Residual
        let residual2 = format!("{prefix}.residual2");
        g.add_node_with_attrs(
            Op::Add,
            vec![residual1, down_out],
            vec![residual2.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        prev_hidden = residual2;
    }

    // Final output: use last hidden as encoder_output
    g.add_tensor("encoder_output".to_string(), TensorMeta {
        shape: vec![Dim::Dynamic("seq_len".to_string()), Dim::Fixed(hidden)],
        dtype: DType::F32,
        residency: Residency::Streamed,
    });

    // Alias the last hidden state for output (passthrough reshape)
    let final_out = "final_output".to_string();
    g.add_node_with_attrs(
        Op::Reshape { shape: vec![] },
        vec![prev_hidden],
        vec![final_out],
        make_attrs(&[]),
        None,
    );

    log::info!(
        "modernbert_encoder template: {} layers, {} nodes",
        config.num_layers, g.len()
    );

    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformer_decoder_for_exec_generates_graph() {
        let config = TransformerConfig {
            hidden_size: 256,
            num_heads: 4,
            kv_num_heads: 2,
            head_dim: 64,
            num_layers: 2,
            intermediate_size: 512,
            vocab_size: 1000,
            eps: 1e-5,
            rope_theta: 10000.0,
            max_seq_len: 128,
            activation: Activation::Silu,
        };

        let g = transformer_decoder_for_exec(&config);

        // Should have nodes
        assert!(g.len() > 0, "Graph should have nodes");

        // Check that weight names follow HuggingFace convention
        let all_inputs: Vec<&str> = g.nodes.iter()
            .flat_map(|n| n.inputs.iter().map(|s| s.as_str()))
            .collect();

        assert!(all_inputs.contains(&"model.layers.0.self_attn.q_proj.weight"),
            "Should have HF-style Q weight name");
        assert!(all_inputs.contains(&"model.layers.0.self_attn.k_proj.weight"),
            "Should have HF-style K weight name");
        assert!(all_inputs.contains(&"model.layers.0.self_attn.v_proj.weight"),
            "Should have HF-style V weight name");
        assert!(all_inputs.contains(&"model.layers.0.self_attn.o_proj.weight"),
            "Should have HF-style O weight name");
        assert!(all_inputs.contains(&"model.layers.0.mlp.gate_proj.weight"),
            "Should have HF-style gate weight name");
        assert!(all_inputs.contains(&"model.layers.0.mlp.up_proj.weight"),
            "Should have HF-style up weight name");
        assert!(all_inputs.contains(&"model.layers.0.mlp.down_proj.weight"),
            "Should have HF-style down weight name");
        assert!(all_inputs.contains(&"model.layers.0.input_layernorm.weight"),
            "Should have HF-style input norm weight name");
        assert!(all_inputs.contains(&"model.layers.0.post_attention_layernorm.weight"),
            "Should have HF-style post norm weight name");
        assert!(all_inputs.contains(&"model.norm.weight"),
            "Should have final norm weight name");
        assert!(all_inputs.contains(&"lm_head.weight"),
            "Should have lm_head weight name");

        // Check outputs
        let all_outputs: Vec<&str> = g.nodes.iter()
            .flat_map(|n| n.outputs.iter().map(|s| s.as_str()))
            .collect();
        assert!(all_outputs.contains(&"logits"), "Should produce logits");
    }

    #[test]
    fn test_exec_graph_has_attrs() {
        let config = TransformerConfig {
            hidden_size: 128,
            num_heads: 2,
            kv_num_heads: 2,
            head_dim: 64,
            num_layers: 1,
            intermediate_size: 256,
            vocab_size: 100,
            eps: 1e-5,
            rope_theta: 10000.0,
            max_seq_len: 64,
            activation: Activation::Silu,
        };

        let g = transformer_decoder_for_exec(&config);

        // Find a Matmul node and check it has n/k attrs
        let matmul_node = g.nodes.iter().find(|n| matches!(n.op, Op::Matmul)).unwrap();
        assert!(matmul_node.attrs.contains_key("n"), "Matmul should have 'n' attr");
        assert!(matmul_node.attrs.contains_key("k"), "Matmul should have 'k' attr");

        // Find a RmsNorm node and check it has hidden attr
        let norm_node = g.nodes.iter().find(|n| matches!(n.op, Op::RmsNorm { .. })).unwrap();
        assert!(norm_node.attrs.contains_key("hidden"), "RmsNorm should have 'hidden' attr");

        // Find an Sdpa node and check it has v_tensor attr
        let sdpa_node = g.nodes.iter().find(|n| matches!(n.op, Op::Sdpa { .. })).unwrap();
        assert!(sdpa_node.attrs.contains_key("v_tensor"), "Sdpa should have 'v_tensor' attr");
    }

    #[test]
    fn test_exec_graph_topological_sort() {
        let config = TransformerConfig::default();
        let mut g = transformer_decoder_for_exec(&config);
        let sorted = g.topological_sort();
        assert!(sorted, "Graph should be sortable (no cycles)");
    }

    #[test]
    fn test_original_transformer_decoder_unchanged() {
        // Verify the original template still works
        let config = TransformerConfig {
            hidden_size: 256,
            num_heads: 4,
            kv_num_heads: 2,
            head_dim: 64,
            num_layers: 2,
            intermediate_size: 512,
            vocab_size: 1000,
            eps: 1e-5,
            rope_theta: 10000.0,
            max_seq_len: 128,
            activation: Activation::Silu,
        };

        let g = transformer_decoder(&config);
        assert!(g.len() > 0, "Original template should still work");
    }

    #[test]
    fn test_whisper_encoder_decoder_template() {
        let config = WhisperConfig {
            n_audio_state: 256,
            n_audio_head: 4,
            n_audio_layer: 2,
            n_audio_ctx: 100,
            n_text_state: 256,
            n_text_head: 4,
            n_text_layer: 2,
            n_text_ctx: 50,
            n_vocab: 1000,
            n_mels: 80,
            eps: 1e-5,
        };

        let g = whisper_encoder_decoder(&config);
        assert!(g.len() > 0, "Whisper template should have nodes");

        // Check GGML weight names are present
        let all_inputs: Vec<&str> = g.nodes.iter()
            .flat_map(|n| n.inputs.iter().map(|s| s.as_str()))
            .collect();

        assert!(all_inputs.contains(&"encoder.conv1.weight"), "Should have encoder conv1 weight");
        assert!(all_inputs.contains(&"encoder.conv1.bias"), "Should have encoder conv1 bias");
        assert!(all_inputs.contains(&"encoder.blocks.0.attn_ln.weight"), "Should have encoder block attn_ln");
        assert!(all_inputs.contains(&"encoder.blocks.0.attn.query.weight"), "Should have encoder Q weight");
        assert!(all_inputs.contains(&"encoder.blocks.0.attn.key.weight"), "Should have encoder K weight");
        assert!(all_inputs.contains(&"encoder.blocks.0.mlp.0.weight"), "Should have encoder MLP up weight");
        assert!(all_inputs.contains(&"encoder.blocks.0.mlp.2.weight"), "Should have encoder MLP down weight");
        assert!(all_inputs.contains(&"encoder.ln_post.weight"), "Should have encoder ln_post");
        assert!(all_inputs.contains(&"decoder.blocks.0.attn.query.weight"), "Should have decoder Q weight");
        assert!(all_inputs.contains(&"decoder.blocks.0.cross_attn.query.weight"), "Should have decoder cross-attn Q");
        assert!(all_inputs.contains(&"decoder.ln.weight"), "Should have decoder final LN");
        assert!(all_inputs.contains(&"decoder.token_embedding.weight"), "Should have decoder token embedding");

        let all_outputs: Vec<&str> = g.nodes.iter()
            .flat_map(|n| n.outputs.iter().map(|s| s.as_str()))
            .collect();
        assert!(all_outputs.contains(&"logits"), "Should produce logits");
        assert!(all_outputs.contains(&"enc.output"), "Should produce encoder output");
    }

    #[test]
    fn test_bert_encoder_template() {
        let config = BertConfig {
            hidden_size: 256,
            num_heads: 4,
            head_dim: 64,
            num_layers: 2,
            intermediate_size: 512,
            vocab_size: 1000,
            max_position_embeddings: 128,
            type_vocab_size: 2,
            eps: 1e-12,
            weight_prefix: String::new(),
            num_labels: 0,
        };

        let g = bert_encoder(&config);
        assert!(g.len() > 0, "BERT template should have nodes");

        // Check safetensors weight names
        let all_inputs: Vec<&str> = g.nodes.iter()
            .flat_map(|n| n.inputs.iter().map(|s| s.as_str()))
            .collect();

        assert!(all_inputs.contains(&"embeddings.LayerNorm.weight"), "Should have embedding LN");
        assert!(all_inputs.contains(&"encoder.layer.0.attention.self.query.weight"), "Should have BERT Q weight");
        assert!(all_inputs.contains(&"encoder.layer.0.attention.self.key.weight"), "Should have BERT K weight");
        assert!(all_inputs.contains(&"encoder.layer.0.attention.self.value.weight"), "Should have BERT V weight");
        assert!(all_inputs.contains(&"encoder.layer.0.attention.output.dense.weight"), "Should have BERT attn output");
        assert!(all_inputs.contains(&"encoder.layer.0.attention.output.LayerNorm.weight"), "Should have BERT attn LN");
        assert!(all_inputs.contains(&"encoder.layer.0.intermediate.dense.weight"), "Should have BERT intermediate");
        assert!(all_inputs.contains(&"encoder.layer.0.output.dense.weight"), "Should have BERT output dense");
        assert!(all_inputs.contains(&"encoder.layer.0.output.LayerNorm.weight"), "Should have BERT output LN");
        assert!(all_inputs.contains(&"pooler.dense.weight"), "Should have pooler");

        let all_outputs: Vec<&str> = g.nodes.iter()
            .flat_map(|n| n.outputs.iter().map(|s| s.as_str()))
            .collect();
        assert!(all_outputs.contains(&"pooler_output"), "Should produce pooler_output");

        // All SDPA nodes should be non-causal
        for node in &g.nodes {
            if let Op::Sdpa { causal, .. } = &node.op {
                assert!(!causal, "BERT attention should be non-causal (bidirectional)");
            }
        }
    }
}
