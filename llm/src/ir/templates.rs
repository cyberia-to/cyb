//! Architecture templates — generate Graph IR for common model architectures
//!
//! For safetensors/GGUF models, the runtime has built-in templates.
//! config comes from config.json (HuggingFace) or GGUF metadata.
//! template + config = concrete graph.

use super::graph::{Graph, TensorMeta, Dim, Residency};
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
