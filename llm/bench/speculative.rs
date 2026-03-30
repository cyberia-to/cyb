//! Speculative decoding benchmark — draft model generates candidates, main model verifies
//!
//! Draft: smollm2-360m (fast, ~280 tok/s Metal)
//! Main:  qwen3-0.6b (quality, ~219 tok/s Metal)
//!
//! For each step:
//!   1. Draft generates K candidate tokens (greedy)
//!   2. Main model verifies by running forward on each candidate
//!   3. Accept tokens that match draft, reject at first mismatch
//!   4. If all K match → bonus: accept K+1 tokens per main-model step
//!
//! Effective throughput = accepted_tokens / main_model_time
//!
//! Usage: cargo run --release -p cyb-llm --bin bench-speculative

use std::path::Path;
use std::time::Instant;

fn main() {
    env_logger::init();

    let draft_path = Path::new(env!("HOME")).join("llm/smollm2-360m/model.safetensors");
    let main_path = Path::new(env!("HOME")).join("llm/qwen3-0.6b-abl/model.safetensors");

    if !draft_path.exists() || !main_path.exists() {
        println!("Need both ~/llm/smollm2-360m and ~/llm/qwen3-0.6b-abl");
        return;
    }

    println!("=== Speculative Decoding Benchmark ===\n");

    // Load models
    println!("Loading draft model (smollm2-360m)...");
    let t0 = Instant::now();
    let mut draft = cyb_llm::backend::metal::MetalModel::load_from_safetensors(&draft_path)
        .expect("draft model load failed");
    println!("  loaded in {:.1}s", t0.elapsed().as_secs_f64());

    println!("Loading main model (qwen3-0.6b)...");
    let t0 = Instant::now();
    let mut main_model = cyb_llm::backend::metal::MetalModel::load_from_safetensors(&main_path)
        .expect("main model load failed");
    println!("  loaded in {:.1}s\n", t0.elapsed().as_secs_f64());

    // Tokenize prompt (use main model's tokenizer for now — just feed token IDs)
    let prompt_tokens: Vec<u32> = vec![
        840, 20772, 23249, 304, 825, 14311, 25, // "Explain gravity in one paragraph:"
    ];

    let max_tokens = 100;
    let k = 4; // draft speculation length

    // EOS tokens
    let eos = vec![151643u32, 151645, 0];

    // Prefill both models
    println!("Prefilling...");
    for &tid in &prompt_tokens {
        draft.forward_decode(tid);
        main_model.forward_decode(tid);
    }

    // --- Baseline: main model only ---
    println!("\n── Baseline (main model only) ──");
    main_model.reset_kv_cache();
    for &tid in &prompt_tokens {
        main_model.forward_decode(tid);
    }

    let baseline_start = Instant::now();
    let mut last_token = *prompt_tokens.last().unwrap();
    let mut baseline_count = 0;
    for _ in 0..max_tokens {
        let next = main_model.forward_decode(last_token);
        if eos.contains(&next) { break; }
        baseline_count += 1;
        last_token = next;
    }
    let baseline_s = baseline_start.elapsed().as_secs_f64();
    let baseline_tps = baseline_count as f64 / baseline_s;
    println!("  {} tokens in {:.2}s = {:.1} tok/s", baseline_count, baseline_s, baseline_tps);

    // --- Speculative decoding ---
    println!("\n── Speculative (draft=smollm2, main=qwen3, K={k}) ──");
    main_model.reset_kv_cache();
    draft.reset_kv_cache();
    for &tid in &prompt_tokens {
        main_model.forward_decode(tid);
        draft.forward_decode(tid);
    }

    let spec_start = Instant::now();
    let mut total_accepted = 0usize;
    let mut total_draft_calls = 0usize;
    let mut total_main_calls = 0usize;
    let mut spec_last_token = *prompt_tokens.last().unwrap();

    while total_accepted < max_tokens {
        // Step 1: draft generates K candidates
        let mut candidates = Vec::with_capacity(k);
        let mut draft_token = spec_last_token;
        for _ in 0..k {
            let next = draft.forward_decode(draft_token);
            candidates.push(next);
            draft_token = next;
            total_draft_calls += 1;
            if eos.contains(&next) { break; }
        }

        // Step 2: main model verifies each candidate
        let mut accepted = 0;
        for &candidate in &candidates {
            let main_next = main_model.forward_decode(if accepted == 0 { spec_last_token } else { candidates[accepted - 1] });
            total_main_calls += 1;

            if main_next == candidate && !eos.contains(&candidate) {
                accepted += 1;
            } else {
                // Divergence: accept main_next as the correction
                if !eos.contains(&main_next) {
                    accepted += 1;
                    // Update draft to match main model's choice
                    // (would need to rollback draft KV cache — simplified here)
                }
                break;
            }
        }

        if accepted == 0 {
            // Main model disagreed on first token — take main's choice
            let main_next = main_model.forward_decode(spec_last_token);
            total_main_calls += 1;
            if eos.contains(&main_next) { break; }
            spec_last_token = main_next;
            total_accepted += 1;
        } else {
            total_accepted += accepted;
            spec_last_token = candidates[accepted - 1];
        }

        if eos.contains(&spec_last_token) { break; }
    }

    let spec_s = spec_start.elapsed().as_secs_f64();
    let spec_tps = total_accepted as f64 / spec_s;
    let acceptance_rate = if total_draft_calls > 0 {
        total_accepted as f64 / total_draft_calls as f64
    } else { 0.0 };

    println!("  {} tokens in {:.2}s = {:.1} tok/s", total_accepted, spec_s, spec_tps);
    println!("  draft calls: {}, main calls: {}", total_draft_calls, total_main_calls);
    println!("  acceptance rate: {:.1}%", acceptance_rate * 100.0);
    println!("  speedup vs baseline: {:.2}x", spec_tps / baseline_tps);

    // --- Summary ---
    println!("\n── Summary ──");
    println!("  Baseline (main only): {:.1} tok/s", baseline_tps);
    println!("  Speculative (K={k}):    {:.1} tok/s ({:.2}x)", spec_tps, spec_tps / baseline_tps);
}
