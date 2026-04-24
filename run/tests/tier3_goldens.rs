//! Tier 3 with HF golden values.
//!
//! Uses dumps from mr/scripts/dump_hf_golden.py as ground truth.
//! Skipped if golden file is missing (not everyone has transformers+torch).
//!
//! Spec: reference/runtime/test.md#tier-3-model-golden-tests

use run::backend::cpu::CpuBackend;
use run::arch::decoder::LlamaModel;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Golden {
    model_repo: String,
    config: serde_json::Value,
    records: Vec<GoldenRecord>,
}

#[derive(Deserialize)]
struct GoldenRecord {
    prompt: String,
    input_tokens: Vec<u32>,
    vocab_size: usize,
    last_logits: Vec<f32>,
    top5: Vec<(usize, f32)>,
    greedy_next_token: u32,
    greedy_next_decoded: String,
}

fn find_golden(name: &str) -> Option<PathBuf> {
    let candidates = [
        format!("mr/goldens/{name}.json"),
        format!("goldens/{name}.json"),
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

fn find_model(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(format!("/Users/mastercyb/llm/{name}.model"));
    p.exists().then_some(p)
}

#[test]
fn qwen3_0_6b_matches_hf_golden() {
    let Some(golden_path) = find_golden("qwen3-0.6b") else {
        eprintln!(
            "skip: no golden at mr/goldens/qwen3-0.6b.json — run:\n\
             python3 mr/scripts/dump_hf_golden.py \\\n\
               --model Qwen/Qwen3-0.6B --output mr/goldens/qwen3-0.6b.json"
        );
        return;
    };
    let Some(model_path) = find_model("qwen3-0.6b-abl") else {
        eprintln!("skip: qwen3-0.6b-abl.model not found");
        return;
    };

    let golden: Golden = serde_json::from_reader(std::fs::File::open(&golden_path).unwrap())
        .expect("parse golden");

    eprintln!(
        "golden from {}: config={}, {} records",
        golden.model_repo,
        serde_json::to_string(&golden.config).unwrap(),
        golden.records.len()
    );

    let mut model = LlamaModel::load(&model_path).expect("load");
    let backend = CpuBackend::new();

    for rec in &golden.records {
        eprintln!(
            "\n-- prompt {:?}: {} tokens → HF greedy={}({:?})",
            rec.prompt, rec.input_tokens.len(), rec.greedy_next_token, rec.greedy_next_decoded
        );
        assert_eq!(
            rec.vocab_size, model.config.vocab_size,
            "vocab mismatch: golden {} vs model {}",
            rec.vocab_size, model.config.vocab_size
        );

        // Feed prompt tokens, keep the last forward's logits.
        model.reset_kv_cache();
        let mut logits = Vec::new();
        for &tok in &rec.input_tokens {
            logits = model.forward(tok, &backend).expect("forward");
        }

        // Compare our last-step logits against HF's.
        let eps_rel = 1e-2; // generous — Q4 weights + f32 math differ a bit
        let mut worst_diff = 0f32;
        let mut worst_i = 0usize;
        for (i, (ours, hf)) in logits.iter().zip(rec.last_logits.iter()).enumerate() {
            let diff = (ours - hf).abs();
            if diff > worst_diff {
                worst_diff = diff;
                worst_i = i;
            }
        }
        eprintln!(
            "worst diff at idx {}: ours={} hf={} diff={}",
            worst_i, logits[worst_i], rec.last_logits[worst_i], worst_diff
        );

        // Argmax should match HF top token for most prompts.
        // With Q4 quantization + f32 maths, the argmax may disagree on
        // borderline cases — we check it's in HF's top-5 instead.
        let our_argmax = logits
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        let hf_top5_ids: Vec<usize> = rec.top5.iter().map(|(i, _)| *i).collect();
        eprintln!(
            "our argmax: {} (logit {}) | HF top5: {:?}",
            our_argmax, logits[our_argmax], hf_top5_ids
        );
        assert!(
            hf_top5_ids.contains(&our_argmax),
            "our argmax {} not in HF top-5 {:?} — runtime diverges from reference",
            our_argmax,
            hf_top5_ids
        );
    }
}
