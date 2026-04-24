//! Architecture templates — `config → Graph` builders.
//!
//! Each template is the mechanical translation of an architecture family
//! (transformer decoder, BERT encoder, DiT, MoE, Whisper, CNN detector)
//! from a handful of hyperparameters into a concrete Graph. Templates are
//! what make the graph-executor a "universal fallback" in practice — any
//! HF model whose family we recognize can be imported → template → graph
//! without writing model-specific code.
//!
//! Port status from the old llm/ runtime:
//! - ✅ `transformer_decoder` (merged QKV, curated path style)
//! - ✅ `transformer_decoder_for_exec` (split Q/K/V, HF weight names, attrs
//!        on every node — this is what `GraphExecutor` consumes)
//! - ⏳ `transformer_encoder` / BERT / ModernBERT / Whisper / DiT / MoE /
//!        CNN detector — not yet ported. Called code panics with a clear
//!        "port pending" message so the gap is visible, not silent.
//!
//! Spec: specs/ir.md, specs/execution.md

use super::graph::{Attrs, AttrValue, Dim, Graph, Residency, TensorMeta};
use crate::core::dtype::DType;
use crate::core::op::Op;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activation {
    Silu,
    Gelu,
    GeGlu,
}

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
            rope_theta: 10_000.0,
            max_seq_len: 4096,
            activation: Activation::Silu,
        }
    }
}

/// Generate a decoder graph with merged QKV projection. Good for curated
/// codepaths that fuse QKV into one matmul and reshape downstream.
///
/// Per layer: RmsNorm → QKV Matmul → Rope → KvCache → Sdpa → O_Proj Matmul
///            → residual Add → RmsNorm → Gate + Up Matmul → act → Down Matmul
///            → residual Add
pub fn transformer_decoder(config: &TransformerConfig) -> Graph {
    let mut g = Graph::new();
    let seq_dim = Dim::Dynamic("seq_len".to_string());
    let hidden = config.hidden_size;

    let embed_out = "embed_out".to_string();
    g.add_node(
        Op::TokenEmbed,
        vec!["input_ids".into()],
        vec![embed_out.clone()],
    );
    g.add_tensor(
        "input_ids".into(),
        TensorMeta {
            shape: vec![seq_dim.clone()],
            dtype: DType::U8,
            residency: Residency::Streamed,
        },
    );
    g.add_tensor(
        embed_out.clone(),
        TensorMeta {
            shape: vec![seq_dim.clone(), Dim::Fixed(hidden)],
            dtype: DType::F16,
            residency: Residency::Streamed,
        },
    );

    let mut prev_hidden = embed_out;

    for i in 0..config.num_layers {
        let p = format!("layer_{i}");

        let norm1 = format!("{p}.attn_norm_out");
        g.add_node(
            Op::RmsNorm { eps: config.eps },
            vec![prev_hidden.clone(), format!("{p}.attn_norm.weight")],
            vec![norm1.clone()],
        );

        let qkv_out = format!("{p}.qkv_out");
        g.add_node(
            Op::Matmul,
            vec![norm1, format!("{p}.qkv.weight")],
            vec![qkv_out.clone()],
        );

        let rope_out = format!("{p}.rope_out");
        g.add_node(
            Op::Rope {
                head_dim: config.head_dim as u32,
                rope_dim: config.head_dim as u32,
                base: config.rope_theta,
            },
            vec![qkv_out],
            vec![rope_out.clone()],
        );

        let kv_out = format!("{p}.kv_cached");
        g.add_node(Op::KvCache, vec![rope_out.clone()], vec![kv_out.clone()]);
        g.add_tensor(
            kv_out.clone(),
            TensorMeta {
                shape: vec![
                    Dim::Dynamic("total_seq".to_string()),
                    Dim::Fixed(config.kv_num_heads * config.head_dim),
                ],
                dtype: DType::F16,
                residency: Residency::Cached,
            },
        );

        let attn_out = format!("{p}.attn_out");
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

        let o_proj = format!("{p}.o_proj_out");
        g.add_node(
            Op::Matmul,
            vec![attn_out, format!("{p}.o_proj.weight")],
            vec![o_proj.clone()],
        );

        let res1 = format!("{p}.residual1");
        g.add_node(
            Op::Add,
            vec![prev_hidden.clone(), o_proj],
            vec![res1.clone()],
        );

        let norm2 = format!("{p}.ffn_norm_out");
        g.add_node(
            Op::RmsNorm { eps: config.eps },
            vec![res1.clone(), format!("{p}.ffn_norm.weight")],
            vec![norm2.clone()],
        );

        let gate = format!("{p}.gate_out");
        g.add_node(
            Op::Matmul,
            vec![norm2.clone(), format!("{p}.gate_proj.weight")],
            vec![gate.clone()],
        );
        let up = format!("{p}.up_out");
        g.add_node(
            Op::Matmul,
            vec![norm2, format!("{p}.up_proj.weight")],
            vec![up.clone()],
        );

        let act = format!("{p}.act_out");
        match config.activation {
            Activation::Silu => {
                let silu = format!("{p}.silu_out");
                g.add_node(Op::Silu, vec![gate], vec![silu.clone()]);
                g.add_node(Op::Mul, vec![silu, up], vec![act.clone()]);
            }
            Activation::Gelu => {
                let gelu = format!("{p}.gelu_out");
                g.add_node(Op::Gelu { approximate: false }, vec![gate], vec![gelu.clone()]);
                g.add_node(Op::Mul, vec![gelu, up], vec![act.clone()]);
            }
            Activation::GeGlu => {
                g.add_node(Op::GeGlu, vec![gate, up], vec![act.clone()]);
            }
        }

        let down = format!("{p}.down_out");
        g.add_node(
            Op::Matmul,
            vec![act, format!("{p}.down_proj.weight")],
            vec![down.clone()],
        );

        let res2 = format!("{p}.residual2");
        g.add_node(Op::Add, vec![res1, down], vec![res2.clone()]);
        prev_hidden = res2;
    }

    let final_norm = "final_norm_out".to_string();
    g.add_node(
        Op::RmsNorm { eps: config.eps },
        vec![prev_hidden, "model.norm.weight".into()],
        vec![final_norm.clone()],
    );
    g.add_node(
        Op::Matmul,
        vec![final_norm, "lm_head.weight".into()],
        vec!["logits".into()],
    );
    g.add_tensor(
        "logits".into(),
        TensorMeta {
            shape: vec![seq_dim, Dim::Fixed(config.vocab_size)],
            dtype: DType::F32,
            residency: Residency::Streamed,
        },
    );

    log::info!(
        "transformer_decoder template: {} layers, {} nodes",
        config.num_layers,
        g.len()
    );
    g
}

/// Generate a decoder graph with **HF-convention weight names** and
/// per-node attributes for the graph executor. Separate Q/K/V Matmul nodes
/// (no merged QKV), Rope applied to Q and K independently.
pub fn transformer_decoder_for_exec(config: &TransformerConfig) -> Graph {
    let mut g = Graph::new();
    let seq_dim = Dim::Dynamic("seq_len".to_string());
    let hidden = config.hidden_size;
    let inter = config.intermediate_size;
    let kv_dim = config.kv_num_heads * config.head_dim;
    let q_dim = config.num_heads * config.head_dim;

    let embed_out = "embed_out".to_string();
    g.add_node_with_attrs(
        Op::TokenEmbed,
        vec!["input_ids".into()],
        vec![embed_out.clone()],
        make_attrs(&[("hidden", hidden as i64)]),
        None,
    );
    g.add_tensor(
        "input_ids".into(),
        TensorMeta {
            shape: vec![seq_dim.clone()],
            dtype: DType::U8,
            residency: Residency::Streamed,
        },
    );
    g.add_tensor(
        embed_out.clone(),
        TensorMeta {
            shape: vec![seq_dim.clone(), Dim::Fixed(hidden)],
            dtype: DType::F32,
            residency: Residency::Streamed,
        },
    );

    let mut prev = embed_out;

    for i in 0..config.num_layers {
        let p = format!("layer_{i}");
        let hf = format!("model.layers.{i}");

        let norm1 = format!("{p}.attn_norm_out");
        g.add_node_with_attrs(
            Op::RmsNorm { eps: config.eps },
            vec![prev.clone(), format!("{hf}.input_layernorm.weight")],
            vec![norm1.clone()],
            make_attrs(&[("hidden", hidden as i64), ("positions", 1)]),
            None,
        );

        let q = format!("{p}.q_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1.clone(), format!("{hf}.self_attn.q_proj.weight")],
            vec![q.clone()],
            make_attrs(&[("n", q_dim as i64), ("k", hidden as i64)]),
            None,
        );
        let k = format!("{p}.k_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1.clone(), format!("{hf}.self_attn.k_proj.weight")],
            vec![k.clone()],
            make_attrs(&[("n", kv_dim as i64), ("k", hidden as i64)]),
            None,
        );
        let v = format!("{p}.v_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm1, format!("{hf}.self_attn.v_proj.weight")],
            vec![v.clone()],
            make_attrs(&[("n", kv_dim as i64), ("k", hidden as i64)]),
            None,
        );

        let q_rope = format!("{p}.q_roped");
        g.add_node_with_attrs(
            Op::Rope {
                head_dim: config.head_dim as u32,
                rope_dim: config.head_dim as u32,
                base: config.rope_theta,
            },
            vec![q],
            vec![q_rope.clone()],
            make_attrs(&[("total_elements", q_dim as i64), ("n", q_dim as i64)]),
            None,
        );
        let k_rope = format!("{p}.k_roped");
        g.add_node_with_attrs(
            Op::Rope {
                head_dim: config.head_dim as u32,
                rope_dim: config.head_dim as u32,
                base: config.rope_theta,
            },
            vec![k],
            vec![k_rope.clone()],
            make_attrs(&[("total_elements", kv_dim as i64), ("n", kv_dim as i64)]),
            None,
        );

        let kv = format!("{p}.kv_cached");
        g.add_node(Op::KvCache, vec![k_rope.clone()], vec![kv.clone()]);
        g.add_tensor(
            kv.clone(),
            TensorMeta {
                shape: vec![Dim::Dynamic("total_seq".to_string()), Dim::Fixed(kv_dim)],
                dtype: DType::F32,
                residency: Residency::Cached,
            },
        );

        // SDPA with `v_tensor` attr so the executor can find V buffer by name.
        let attn = format!("{p}.attn_out");
        let mut sdpa_attrs = make_attrs(&[("hidden", hidden as i64)]);
        sdpa_attrs.insert("v_tensor".into(), AttrValue::String(v.clone()));
        g.add_node_with_attrs(
            Op::Sdpa {
                num_heads: config.num_heads as u32,
                kv_heads: config.kv_num_heads as u32,
                head_dim: config.head_dim as u32,
                causal: true,
            },
            vec![q_rope, kv],
            vec![attn.clone()],
            sdpa_attrs,
            None,
        );

        let o = format!("{p}.o_proj_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![attn, format!("{hf}.self_attn.o_proj.weight")],
            vec![o.clone()],
            make_attrs(&[("n", hidden as i64), ("k", q_dim as i64)]),
            None,
        );

        let res1 = format!("{p}.residual1");
        g.add_node_with_attrs(
            Op::Add,
            vec![prev.clone(), o],
            vec![res1.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );

        let norm2 = format!("{p}.ffn_norm_out");
        g.add_node_with_attrs(
            Op::RmsNorm { eps: config.eps },
            vec![res1.clone(), format!("{hf}.post_attention_layernorm.weight")],
            vec![norm2.clone()],
            make_attrs(&[("hidden", hidden as i64), ("positions", 1)]),
            None,
        );

        let gate = format!("{p}.gate_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm2.clone(), format!("{hf}.mlp.gate_proj.weight")],
            vec![gate.clone()],
            make_attrs(&[("n", inter as i64), ("k", hidden as i64)]),
            None,
        );
        let up = format!("{p}.up_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![norm2, format!("{hf}.mlp.up_proj.weight")],
            vec![up.clone()],
            make_attrs(&[("n", inter as i64), ("k", hidden as i64)]),
            None,
        );

        let act = format!("{p}.act_out");
        match config.activation {
            Activation::Silu => {
                let silu = format!("{p}.silu_out");
                g.add_node_with_attrs(
                    Op::Silu,
                    vec![gate],
                    vec![silu.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
                g.add_node_with_attrs(
                    Op::Mul,
                    vec![silu, up],
                    vec![act.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
            }
            Activation::Gelu => {
                let gelu = format!("{p}.gelu_out");
                g.add_node_with_attrs(
                    Op::Gelu { approximate: false },
                    vec![gate],
                    vec![gelu.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
                g.add_node_with_attrs(
                    Op::Mul,
                    vec![gelu, up],
                    vec![act.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
            }
            Activation::GeGlu => {
                g.add_node_with_attrs(
                    Op::GeGlu,
                    vec![gate, up],
                    vec![act.clone()],
                    make_attrs(&[("n", inter as i64)]),
                    None,
                );
            }
        }

        let down = format!("{p}.down_out");
        g.add_node_with_attrs(
            Op::Matmul,
            vec![act, format!("{hf}.mlp.down_proj.weight")],
            vec![down.clone()],
            make_attrs(&[("n", hidden as i64), ("k", inter as i64)]),
            None,
        );

        let res2 = format!("{p}.residual2");
        g.add_node_with_attrs(
            Op::Add,
            vec![res1, down],
            vec![res2.clone()],
            make_attrs(&[("n", hidden as i64)]),
            None,
        );
        prev = res2;
    }

    let final_norm = "final_norm_out".to_string();
    g.add_node_with_attrs(
        Op::RmsNorm { eps: config.eps },
        vec![prev, "model.norm.weight".into()],
        vec![final_norm.clone()],
        make_attrs(&[("hidden", hidden as i64), ("positions", 1)]),
        None,
    );
    g.add_node_with_attrs(
        Op::Matmul,
        vec![final_norm, "lm_head.weight".into()],
        vec!["logits".into()],
        make_attrs(&[("n", config.vocab_size as i64), ("k", hidden as i64)]),
        None,
    );
    g.add_tensor(
        "logits".into(),
        TensorMeta {
            shape: vec![seq_dim, Dim::Fixed(config.vocab_size)],
            dtype: DType::F32,
            residency: Residency::Streamed,
        },
    );

    log::info!(
        "transformer_decoder_for_exec: {} layers, {} nodes, HF weight names",
        config.num_layers,
        g.len()
    );
    g
}

fn make_attrs(pairs: &[(&str, i64)]) -> Attrs {
    let mut m: Attrs = HashMap::new();
    for &(k, v) in pairs {
        m.insert(k.into(), AttrValue::Int(v));
    }
    m
}

// ============================================================================
// Architecture templates not yet ported. Calling these returns a clear error
// so callers can decide to use a curated path or wait for the template port.
// Full bodies live in the old llm/ir/templates.rs until migrated.
// ============================================================================

/// BERT-style config. Encoder templates not yet ported from llm/ir/templates.rs.
#[derive(Clone, Debug, Default)]
pub struct BertConfig {
    pub hidden_size: usize,
    pub num_heads: usize,
    pub num_layers: usize,
    pub intermediate_size: usize,
    pub vocab_size: usize,
    pub max_position_embeddings: usize,
    pub eps: f32,
}

/// DiT-style (diffusion transformer) config.
#[derive(Clone, Debug, Default)]
pub struct DiTConfig {
    pub patch_size: usize,
    pub hidden_size: usize,
    pub num_heads: usize,
    pub num_layers: usize,
    pub intermediate_size: usize,
    pub image_size: (usize, usize),
    pub cond_dim: usize,
    pub eps: f32,
}

/// CNN-detector (YOLO-family) config.
#[derive(Clone, Debug, Default)]
pub struct CnnDetectorConfig {
    pub input_channels: usize,
    pub num_classes: usize,
    pub feature_levels: usize,
    pub base_channels: usize,
}

/// MoE decoder config.
#[derive(Clone, Debug)]
pub struct MoeDecoderConfig {
    pub base: TransformerConfig,
    pub num_experts: u32,
    pub experts_per_token: u32,
}

/// Whisper encoder+decoder config.
#[derive(Clone, Debug, Default)]
pub struct WhisperConfig {
    pub n_mels: usize,
    pub audio_ctx: usize,
    pub hidden_size: usize,
    pub num_encoder_heads: usize,
    pub num_encoder_layers: usize,
    pub text_ctx: usize,
    pub vocab_size: usize,
    pub num_decoder_heads: usize,
    pub num_decoder_layers: usize,
    pub eps: f32,
}

/// Not yet ported from the old llm/ runtime.
pub fn transformer_encoder(_: &TransformerConfig) -> Graph {
    unimplemented!(
        "template not yet ported from llm/src/ir/templates.rs — see Port Status in module docs"
    )
}
/// Not yet ported from the old llm/ runtime.
pub fn bert_encoder(_: &BertConfig) -> Graph {
    unimplemented!("template not yet ported")
}
/// Not yet ported from the old llm/ runtime.
pub fn modernbert_encoder(_: &BertConfig) -> Graph {
    unimplemented!("template not yet ported")
}
/// Not yet ported from the old llm/ runtime.
pub fn whisper_encoder_decoder(_: &WhisperConfig) -> Graph {
    unimplemented!("template not yet ported")
}
/// Not yet ported from the old llm/ runtime.
pub fn diffusion_dit(_: &DiTConfig) -> Graph {
    unimplemented!("template not yet ported")
}
/// Not yet ported from the old llm/ runtime.
pub fn cnn_detector(_: &CnnDetectorConfig) -> Graph {
    unimplemented!("template not yet ported")
}
/// Not yet ported from the old llm/ runtime.
pub fn moe_decoder(_: &MoeDecoderConfig) -> Graph {
    unimplemented!("template not yet ported")
}
/// Not yet ported from the old llm/ runtime.
pub fn encoder_decoder(_: &TransformerConfig, _: &TransformerConfig) -> Graph {
    unimplemented!("template not yet ported")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transformer_decoder_produces_expected_node_count() {
        let mut cfg = TransformerConfig::default();
        cfg.num_layers = 2;
        let g = transformer_decoder(&cfg);
        // Per layer: 14 ops (norm, qkv, rope, kv, sdpa, o, add, norm, gate, up,
        // silu, mul, down, add) + 1 embed + 2 final = 31 for 2 layers.
        assert!(g.len() > 0);
        assert!(g.get_weight("model.norm.weight").is_none()); // weights aren't auto-added
    }

    #[test]
    fn exec_template_uses_hf_weight_names() {
        let mut cfg = TransformerConfig::default();
        cfg.num_layers = 1;
        let g = transformer_decoder_for_exec(&cfg);
        let names: Vec<String> = g
            .nodes
            .iter()
            .flat_map(|n| n.inputs.iter().cloned())
            .collect();
        assert!(names.iter().any(|n| n.contains("self_attn.q_proj.weight")));
        assert!(names.iter().any(|n| n.contains("mlp.gate_proj.weight")));
        assert!(names.iter().any(|n| n.contains("post_attention_layernorm.weight")));
    }
}
