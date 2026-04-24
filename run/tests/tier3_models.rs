//! Tier 3: end-to-end model forward on real models.
//!
//! Loads a real .model, runs one forward pass, checks logits sanity.
//! Full golden-value comparison against HF reference comes next
//! (requires Python dump script to generate expected).
//!
//! Spec: reference/runtime/test.md#tier-3-model-golden-tests

use run::arch::decoder::LlamaModel;
use std::path::PathBuf;

fn find_model(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(format!("/Users/mastercyb/llm/{name}.model"));
    p.exists().then_some(p)
}

#[test]
fn qwen3_0_6b_forward_runs() {
    let Some(path) = find_model("qwen3-0.6b-abl") else {
        eprintln!("skip: qwen3-0.6b-abl.model not found");
        return;
    };

    let mut model = LlamaModel::load(&path).expect("load model");
    eprintln!(
        "model_type={}, layers={}, hidden={}, heads={}/{}, head_dim={}, has_qk_norm={}, has_attn_bias={}",
        model.config.model_type,
        model.config.num_hidden_layers,
        model.config.hidden_size,
        model.config.num_attention_heads,
        model.config.num_key_value_heads,
        model.config.head_dim,
        model.config.has_qk_norm,
        model.config.has_attn_bias,
    );

    // Single forward: token 151644 (<|im_start|>), CPU backend.
    let backend = run::backend::cpu::CpuBackend::new();
    let logits = model.forward(151644, &backend).expect("forward");
    assert_eq!(logits.len(), model.config.vocab_size);

    // Sanity: logits are finite, range is plausible for trained model.
    let finite = logits.iter().filter(|v| v.is_finite()).count();
    assert_eq!(finite, logits.len(), "non-finite logits");
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min = logits.iter().cloned().fold(f32::INFINITY, f32::min);
    eprintln!("logits range: [{min:.2}..{max:.2}]");
    assert!(max > min, "degenerate logits");
    assert!(max - min > 1.0, "logits too flat: {max} - {min}");

    // Argmax
    let argmax = logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();
    eprintln!("argmax after <|im_start|>: {argmax} (logit={})", logits[argmax]);
    // Actual value depends on model quality; just ensure it's not 0 (unk) or out of vocab.
    assert!(argmax > 0);
    assert!(argmax < model.config.vocab_size);
}

#[test]
#[ignore] // slow — big model, dequant takes ~30s. Run with --ignored.
fn qwen25_coder_14b_forward_runs() {
    let Some(path) = find_model("qwen2.5-coder-14b-abl") else {
        eprintln!("skip: coder-14b not found");
        return;
    };
    let mut model = LlamaModel::load(&path).expect("load");
    eprintln!(
        "model_type={}, layers={}, hidden={}, heads={}/{}, head_dim={}, qk_norm={}, attn_bias={}",
        model.config.model_type,
        model.config.num_hidden_layers,
        model.config.hidden_size,
        model.config.num_attention_heads,
        model.config.num_key_value_heads,
        model.config.head_dim,
        model.config.has_qk_norm,
        model.config.has_attn_bias,
    );
    let backend = run::backend::cpu::CpuBackend::new();
    let logits = model.forward(151644, &backend).expect("forward");
    assert_eq!(logits.len(), model.config.vocab_size);
    let finite = logits.iter().filter(|v| v.is_finite()).count();
    assert_eq!(finite, logits.len(), "non-finite logits");
    eprintln!(
        "logits range: [{:.2}, {:.2}]",
        logits.iter().cloned().fold(f32::INFINITY, f32::min),
        logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
    );
}

#[test]
fn qwen25_coder_1_5b_forward_runs() {
    let Some(path) = find_model("qwen2.5-coder-1.5b-abl") else {
        eprintln!("skip: qwen2.5-coder-1.5b-abl.model not found");
        return;
    };
    let mut model = LlamaModel::load(&path).expect("load");
    eprintln!(
        "model_type={}, layers={}, hidden={}, heads={}/{}, head_dim={}, qk_norm={}, attn_bias={}",
        model.config.model_type,
        model.config.num_hidden_layers,
        model.config.hidden_size,
        model.config.num_attention_heads,
        model.config.num_key_value_heads,
        model.config.head_dim,
        model.config.has_qk_norm,
        model.config.has_attn_bias,
    );

    let backend = run::backend::cpu::CpuBackend::new();
    let logits = model.forward(151644, &backend).expect("forward");
    assert_eq!(logits.len(), model.config.vocab_size);
    let finite = logits.iter().filter(|v| v.is_finite()).count();
    assert_eq!(finite, logits.len(), "non-finite logits");
}
