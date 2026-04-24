//! Tier 4: IR graph executor verification.
//!
//! Builds a graph via `transformer_decoder_for_exec`, runs it through
//! `GraphExecutor`, and checks that its logits match the curated
//! `LlamaModel::forward` path for the same tokens.
//!
//! Skip conditions (soft fail):
//!   - `qwen3-0.6b-abl.model` not present at ~/llm/
//!
//! Spec: specs/ir.md §"Walking the graph"

use run::arch::decoder::{LlamaModel, config::HiddenActivation};
use run::backend::cpu::CpuBackend;
use run::backend::cpu::quant::try_dequantize_to_f32;
use run::core::tensor::Tensor;
use run::format::LoadedModel;
use run::ir::{ExecConfig, GraphExecutor};
use run::ir::templates::{Activation, TransformerConfig, transformer_decoder_for_exec};
use std::collections::HashMap;
use std::path::PathBuf;

fn find_model(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(format!("/Users/mastercyb/llm/{name}.model"));
    p.exists().then_some(p)
}

/// Extract all model weights as dequantized f32 tensors with HF names.
fn extract_weights_f32(lm: &LoadedModel, model: &LlamaModel) -> HashMap<String, Tensor> {
    let config = &model.config;
    let mut weights: HashMap<String, Tensor> = HashMap::new();

    let dequant = |name: &str, shape: Vec<usize>| -> Tensor {
        let meta = lm.tensors.iter().find(|t| t.name == name)
            .unwrap_or_else(|| panic!("tensor not found: {name}"));
        let bytes = lm.tensor_bytes(name)
            .unwrap_or_else(|| panic!("bytes missing: {name}"));
        let f32s = try_dequantize_to_f32(bytes, meta.dtype)
            .unwrap_or_else(|e| panic!("dequant {name}: {e}"));
        Tensor::from_f32(shape, f32s)
    };

    let h = config.hidden_size;
    let vocab = config.vocab_size;

    weights.insert("model.embed_tokens.weight".into(),
        dequant("model.embed_tokens.weight", vec![vocab, h]));

    for i in 0..config.num_hidden_layers {
        let hf = format!("model.layers.{i}");
        let q_dim = config.num_attention_heads * config.layer_head_dim(i);
        let kv_dim = config.layer_kv_heads(i) * config.layer_head_dim(i);
        let inter = config.intermediate_size;
        let hd = config.layer_head_dim(i);

        weights.insert(format!("{hf}.input_layernorm.weight"),
            dequant(&format!("model.layers.{i}.input_layernorm.weight"), vec![h]));
        weights.insert(format!("{hf}.self_attn.q_proj.weight"),
            dequant(&format!("model.layers.{i}.self_attn.q_proj.weight"), vec![q_dim, h]));
        weights.insert(format!("{hf}.self_attn.k_proj.weight"),
            dequant(&format!("model.layers.{i}.self_attn.k_proj.weight"), vec![kv_dim, h]));
        weights.insert(format!("{hf}.self_attn.v_proj.weight"),
            dequant(&format!("model.layers.{i}.self_attn.v_proj.weight"), vec![kv_dim, h]));
        weights.insert(format!("{hf}.self_attn.o_proj.weight"),
            dequant(&format!("model.layers.{i}.self_attn.o_proj.weight"), vec![h, q_dim]));

        if config.has_qk_norm {
            weights.insert(format!("{hf}.self_attn.q_norm.weight"),
                dequant(&format!("model.layers.{i}.self_attn.q_norm.weight"), vec![hd]));
            weights.insert(format!("{hf}.self_attn.k_norm.weight"),
                dequant(&format!("model.layers.{i}.self_attn.k_norm.weight"), vec![hd]));
        }

        weights.insert(format!("{hf}.post_attention_layernorm.weight"),
            dequant(&format!("model.layers.{i}.post_attention_layernorm.weight"), vec![h]));
        weights.insert(format!("{hf}.mlp.gate_proj.weight"),
            dequant(&format!("model.layers.{i}.mlp.gate_proj.weight"), vec![inter, h]));
        weights.insert(format!("{hf}.mlp.up_proj.weight"),
            dequant(&format!("model.layers.{i}.mlp.up_proj.weight"), vec![inter, h]));
        weights.insert(format!("{hf}.mlp.down_proj.weight"),
            dequant(&format!("model.layers.{i}.mlp.down_proj.weight"), vec![h, inter]));
    }

    weights.insert("model.norm.weight".into(),
        dequant("model.norm.weight", vec![h]));

    let lm_head_src = if config.tie_word_embeddings {
        "model.embed_tokens.weight"
    } else {
        "lm_head.weight"
    };
    weights.insert("lm_head.weight".into(),
        dequant(lm_head_src, vec![vocab, h]));

    weights
}

#[test]
fn ir_exec_matches_curated_qwen3_0_6b() {
    let Some(path) = find_model("qwen3-0.6b-abl") else {
        eprintln!("skip: qwen3-0.6b-abl.model not found");
        return;
    };

    let lm = LoadedModel::load(&path).expect("LoadedModel::load");
    let mut curated = LlamaModel::from_loaded(&lm).expect("LlamaModel::from_loaded");
    let config = &curated.config;

    // Build TransformerConfig from the loaded LlamaConfig.
    let activation = match config.hidden_activation {
        HiddenActivation::Silu => Activation::Silu,
        HiddenActivation::GeluTanh => Activation::Gelu,
        HiddenActivation::GeluErf => Activation::Gelu,
    };
    let tc = TransformerConfig {
        hidden_size: config.hidden_size,
        num_heads: config.num_attention_heads,
        kv_num_heads: config.num_key_value_heads,
        head_dim: config.head_dim,
        num_layers: config.num_hidden_layers,
        intermediate_size: config.intermediate_size,
        vocab_size: config.vocab_size,
        eps: config.rms_norm_eps,
        rope_theta: config.rope_theta,
        max_seq_len: config.max_position_embeddings.min(8192),
        activation,
        has_qk_norm: config.has_qk_norm,
    };

    eprintln!(
        "qwen3-0.6b: hidden={} nh={} kvh={} hd={} layers={} qknorm={}",
        tc.hidden_size, tc.num_heads, tc.kv_num_heads, tc.head_dim,
        tc.num_layers, tc.has_qk_norm,
    );

    let weights = extract_weights_f32(&lm, &curated);
    eprintln!("extracted {} weight tensors", weights.len());

    let graph = transformer_decoder_for_exec(&tc);
    eprintln!("graph has {} nodes", graph.len());

    let mut exec = GraphExecutor::prepare(
        graph,
        weights,
        Box::new(CpuBackend::new()),
        ExecConfig::default(),
    )
    .expect("GraphExecutor::prepare");

    let backend = CpuBackend::new();
    let test_tokens: &[u32] = &[1, 2, 3, 100, 999];

    for (step, &tok) in test_tokens.iter().enumerate() {
        // Curated path
        let ref_logits = curated.forward(tok, &backend).expect("curated forward");

        // Graph executor path
        let mut inputs = HashMap::new();
        inputs.insert(
            "input_ids".into(),
            Tensor::from_f32(vec![1], vec![tok as f32]),
        );
        let out = exec.run(inputs).expect("graph executor run");
        let ir_logits = out["logits"].as_f32();

        assert_eq!(
            ir_logits.len(), ref_logits.len(),
            "step {step}: vocab size mismatch"
        );

        // Argmax comparison: IR argmax must be in curated top-5.
        let ir_argmax = ir_logits
            .iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        let ref_top5: Vec<usize> = {
            let mut pairs: Vec<(usize, f32)> = ref_logits.iter().cloned().enumerate().collect();
            pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            pairs[..5].iter().map(|(i, _)| *i).collect()
        };

        // Worst absolute diff (sanity — should be within Q4 error budget ~1.0).
        let worst: f32 = ir_logits.iter().zip(ref_logits.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);

        eprintln!(
            "step {step} tok={tok}: IR argmax={ir_argmax} ref_top5={ref_top5:?} worst_diff={worst:.3}"
        );

        assert!(
            ref_top5.contains(&ir_argmax),
            "step {step} tok={tok}: IR argmax {ir_argmax} not in curated top-5 {ref_top5:?} (worst_diff={worst:.3})"
        );
    }
}
