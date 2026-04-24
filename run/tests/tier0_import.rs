//! Tier 0: import round-trip on real .model files.
//!
//! For each real model we have locally: load, verify structure, check
//! a sample tensor dequants to sane values.
//!
//! Spec: specs/test.md#tier-0-import-round-trip

use run::backend::cpu::dequantize_to_f32;
use run::format::LoadedModel;
use std::path::Path;

/// Paths tried in order — skip test if none exist.
fn find_model(name: &str) -> Option<std::path::PathBuf> {
    let candidates = [
        format!("/Users/mastercyb/llm/{name}.model"),
    ];
    candidates
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.exists())
}

#[test]
fn qwen3_0_6b_loads_and_has_core_tensors() {
    let Some(path) = find_model("qwen3-0.6b-abl") else {
        eprintln!("skip: qwen3-0.6b-abl.model not found");
        return;
    };
    let lm = LoadedModel::load(&path).expect("load");

    // Basic invariants.
    assert!(!lm.file.config.is_empty(), "config section empty");
    assert!(!lm.file.vocab_toml.is_empty(), "vocab section empty");
    assert!(!lm.tensors.is_empty(), "no tensors parsed");

    // LlamaStyle needs these tensors.
    let required = [
        "model.embed_tokens.weight",
        "model.norm.weight",
        "model.layers.0.input_layernorm.weight",
        "model.layers.0.self_attn.q_proj.weight",
        "model.layers.0.self_attn.k_proj.weight",
        "model.layers.0.self_attn.v_proj.weight",
        "model.layers.0.self_attn.o_proj.weight",
        "model.layers.0.post_attention_layernorm.weight",
        "model.layers.0.mlp.gate_proj.weight",
        "model.layers.0.mlp.up_proj.weight",
        "model.layers.0.mlp.down_proj.weight",
    ];
    for name in required {
        let found = lm.tensors.iter().any(|t| t.name == name);
        assert!(found, "missing required tensor: {name}");
    }

    // Embed weights dequant should produce finite values.
    let embed = lm
        .tensors
        .iter()
        .find(|t| t.name == "model.embed_tokens.weight")
        .unwrap();
    eprintln!(
        "embed: dtype={:?}, shape={:?}, size={}",
        embed.dtype, embed.shape, embed.size
    );
    let bytes = lm
        .tensor_bytes("model.embed_tokens.weight")
        .expect("embed bytes");
    assert_eq!(bytes.len() as u64, embed.size, "embed byte count mismatch");

    let vals = dequantize_to_f32(bytes, embed.dtype);
    let expected_count: usize = embed.shape.iter().product();
    assert_eq!(vals.len(), expected_count, "dequant count mismatch");

    // Sanity: no NaN/Inf, reasonable magnitude.
    let finite = vals.iter().filter(|v| v.is_finite()).count();
    assert_eq!(finite, vals.len(), "some embed values not finite");

    let max_abs = vals.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
    assert!(
        max_abs > 1e-3 && max_abs < 10.0,
        "embed magnitude suspicious: max_abs={max_abs}"
    );
}
