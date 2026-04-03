#![allow(unused)]
//! End-to-end inference benchmark — cyb-llm vs ollama
//!
//! Loads real models from ~/llm/, generates tokens, measures tok/s.
//! Runs the same prompts through ollama for direct comparison.
//!
//! Usage:
//!   cargo run --release -p cyb-llm --bin bench-e2e
//!   cargo run --release -p cyb-llm --bin bench-e2e -- --tokens 64
//!   cargo run --release -p cyb-llm --bin bench-e2e -- --models qwen3-0.6b-abl,smollm2-360m

use std::panic;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const LLM_DIR: &str = env!("HOME");
const PROMPT: &str = "Explain quantum computing in simple terms.";

struct BenchConfig {
    max_tokens: usize,
    models: Vec<ModelSpec>,
}

struct ModelSpec {
    name: String,
    local_dir: PathBuf,
    ollama_name: Option<String>,
}

struct BenchResult {
    model: String,
    backend: String,
    load_s: f64,
    prefill_ms: f64,
    prompt_tokens: usize,
    gen_tokens: usize,
    gen_s: f64,
    tok_s: f64,
}

fn main() {
    env_logger::init();

    let max_tokens: usize = std::env::args()
        .position(|a| a == "--tokens")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);

    let model_filter: Option<Vec<String>> = std::env::args()
        .position(|a| a == "--models")
        .and_then(|i| std::env::args().nth(i + 1))
        .map(|s| s.split(',').map(|m| m.to_string()).collect());

    let llm_dir = PathBuf::from(format!("{}/llm", LLM_DIR));

    let all_models = vec![
        ModelSpec {
            name: "smollm2-360m".into(),
            local_dir: llm_dir.join("smollm2-360m"),
            ollama_name: Some("smollm2:135m".into()),
        },
        ModelSpec {
            name: "qwen3-0.6b".into(),
            local_dir: llm_dir.join("qwen3-0.6b-abl"),
            ollama_name: Some("qwen3:0.6b".into()),
        },
        ModelSpec {
            name: "qwen2.5-0.5b".into(),
            local_dir: llm_dir.join("qwen2.5-0.5b-abl"),
            ollama_name: Some("qwen2.5:0.5b".into()),
        },
        ModelSpec {
            name: "qwen2.5-coder-1.5b".into(),
            local_dir: llm_dir.join("qwen2.5-coder-1.5b-abl"),
            ollama_name: Some("qwen2.5-coder:1.5b".into()),
        },
        ModelSpec {
            name: "bitnet-2b".into(),
            local_dir: llm_dir.join("bitnet-2b"),
            ollama_name: None,
        },
        ModelSpec {
            name: "nuextract-1.5".into(),
            local_dir: llm_dir.join("nuextract-1.5"),
            ollama_name: None,
        },
        ModelSpec {
            name: "deepseek-r1-8b".into(),
            local_dir: llm_dir.join("deepseek-r1-8b-abl"),
            ollama_name: Some("deepseek-r1:8b".into()),
        },
        ModelSpec {
            name: "qwen2.5-coder-14b".into(),
            local_dir: llm_dir.join("qwen2.5-coder-14b-abl"),
            ollama_name: None,
        },
        ModelSpec {
            name: "qwen3.5-9b".into(),
            local_dir: llm_dir.join("qwen3.5-9b-abl"),
            ollama_name: None,
        },
        ModelSpec {
            name: "llama3.2-1b".into(),
            local_dir: llm_dir.join("llama3.2-1b"),
            ollama_name: Some("llama3.2:1b".into()),
        },
        ModelSpec {
            name: "bostrom".into(),
            local_dir: llm_dir.join("bostrom"),
            ollama_name: Some("bostrom:latest".into()),
        },
    ];

    let models: Vec<ModelSpec> = if let Some(filter) = model_filter {
        all_models
            .into_iter()
            .filter(|m| filter.iter().any(|f| m.name.contains(f)))
            .collect()
    } else {
        all_models
    };

    let config = BenchConfig { max_tokens, models };

    println!("=== cyb-llm End-to-End Inference Benchmark ===");
    println!("Prompt: \"{}\"", PROMPT);
    println!("Max tokens: {}\n", config.max_tokens);

    let mut results: Vec<BenchResult> = Vec::new();

    // ── cyb-llm (wgpu) ──
    println!("── cyb-llm (wgpu backend) ──\n");

    for spec in &config.models {
        let safetensors = spec.local_dir.join("model.safetensors");
        let index_json = spec.local_dir.join("model.safetensors.index.json");
        let tokenizer = spec.local_dir.join("tokenizer.json");

        // Support both single-shard and multi-shard
        if !safetensors.exists() && !index_json.exists() {
            continue; // silently skip non-LLM dirs
        }
        if !tokenizer.exists() {
            continue;
        }

        let name = spec.name.clone();
        let st = safetensors.clone();
        let tok = tokenizer.clone();
        let mt = config.max_tokens;
        match panic::catch_unwind(panic::AssertUnwindSafe(|| bench_cyb(&name, &st, &tok, mt))) {
            Ok(Ok(r)) => {
                println!(
                    "  {:<20} load={:.1}s  prefill={:.0}ms  decode={:.1} tok/s  ({} tokens in {:.1}s)",
                    spec.name, r.load_s, r.prefill_ms, r.tok_s, r.gen_tokens, r.gen_s,
                );
                results.push(r);
            }
            Ok(Err(e)) => println!("  {:<20} ERROR: {}", spec.name, e),
            Err(_) => println!("  {:<20} PANIC (skipped)", spec.name),
        }
    }

    // ── cyb-llm (Metal backend) ──
    #[cfg(target_os = "macos")]
    {
        println!("\n── cyb-llm (Metal backend) ──\n");
        for spec in &config.models {
            let safetensors = spec.local_dir.join("model.safetensors");
            let index_json = spec.local_dir.join("model.safetensors.index.json");
            let tokenizer = spec.local_dir.join("tokenizer.json");
            if (!safetensors.exists() && !index_json.exists()) || !tokenizer.exists() { continue; }

            let name = spec.name.clone();
            let st = safetensors.clone();
            let tok = tokenizer.clone();
            let mt = config.max_tokens;
            match panic::catch_unwind(panic::AssertUnwindSafe(|| bench_metal(&name, &st, &tok, mt))) {
                Ok(Ok(r)) => {
                    println!(
                        "  {:<20} load={:.1}s  decode={:.1} tok/s  ({} tokens in {:.1}s)",
                        spec.name, r.load_s, r.tok_s, r.gen_tokens, r.gen_s,
                    );
                    results.push(r);
                }
                Ok(Err(e)) => println!("  {:<20} ERROR: {}", spec.name, e),
                Err(_) => println!("  {:<20} PANIC (skipped)", spec.name),
            }
        }
    }

    // ── ollama ──
    println!("\n── ollama ──\n");

    for spec in &config.models {
        if let Some(ollama_name) = &spec.ollama_name {
            match bench_ollama(ollama_name, &spec.name, PROMPT, config.max_tokens) {
                Ok(r) => {
                    println!(
                        "  {:<20} decode={:.1} tok/s  ({} tokens in {:.1}s)",
                        spec.name, r.tok_s, r.gen_tokens, r.gen_s,
                    );
                    results.push(r);
                }
                Err(e) => {
                    println!("  {:<20} ERROR: {}", spec.name, e);
                }
            }
        }
    }

    // ── Summary table ──
    println!("\n{:<20} {:>10} {:>10} {:>10} {:>12}", "Model", "Metal", "wgpu", "Ollama", "Gap");
    println!("{}", "─".repeat(66));

    for spec in &config.models {
        let metal = results.iter().find(|r| r.model == spec.name && r.backend == "metal");
        let wgpu = results.iter().find(|r| r.model == spec.name && r.backend == "cyb-llm");
        let oll = results.iter().find(|r| r.model == spec.name && r.backend == "ollama");

        let fmt = |r: Option<&BenchResult>| -> String {
            match r {
                Some(r) => format!("{:.1}", r.tok_s),
                None => "–".to_string(),
            }
        };

        // Gap = best cyb vs ollama
        let best_cyb = [metal, wgpu].iter()
            .filter_map(|r| *r)
            .max_by(|a, b| a.tok_s.partial_cmp(&b.tok_s).unwrap());

        let gap = match (best_cyb, oll) {
            (Some(c), Some(o)) => {
                let pct = ((c.tok_s / o.tok_s) - 1.0) * 100.0;
                if pct >= 0.0 {
                    format!("+{:.0}% ✓", pct)
                } else {
                    format!("{:.0}%", pct)
                }
            }
            _ => "–".to_string(),
        };

        println!(
            "{:<20} {:>10} {:>10} {:>10} {:>12}",
            spec.name, fmt(metal), fmt(wgpu), fmt(oll), gap
        );
    }
}

fn bench_cyb(
    name: &str,
    model_path: &Path,
    _tokenizer_path: &Path,
    max_tokens: usize,
) -> Result<BenchResult, String> {
    // Init wgpu
    let backend = cyb_llm::backend::create_wgpu_backend();
    let pipelines = backend.pipelines;

    // Load model from .model file (tokenizer embedded in vocab section)
    let load_start = Instant::now();
    let mut generator =
        cyb_llm::generate::TextGenerator::new(model_path, pipelines)?;
    let load_s = load_start.elapsed().as_secs_f64();

    // Prompt tokens (approximate)
    let prompt_tokens = PROMPT.split_whitespace().count();

    // Generate (greedy for reproducibility)
    let gen_start = Instant::now();
    let text = generator.generate(PROMPT, max_tokens, 0.0)?;
    let gen_s = gen_start.elapsed().as_secs_f64();

    // Count generated tokens
    let gen_tokens = text.split_whitespace().count().max(1);
    // More accurate: use actual token count from generator
    // For now estimate from timing
    let tok_s = if gen_s > 0.0 {
        max_tokens as f64 / gen_s
    } else {
        0.0
    };

    // Rough prefill estimate (first token latency)
    let prefill_ms = 0.0; // TODO: measure separately

    Ok(BenchResult {
        model: name.to_string(),
        backend: "cyb-llm".to_string(),
        load_s,
        prefill_ms,
        prompt_tokens,
        gen_tokens: max_tokens,
        gen_s,
        tok_s,
    })
}

fn bench_ollama(
    ollama_model: &str,
    display_name: &str,
    prompt: &str,
    max_tokens: usize,
) -> Result<BenchResult, String> {
    // Use ollama API for precise timing
    let output = std::process::Command::new("ollama")
        .args([
            "run", ollama_model,
            "--nowordwrap",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                writeln!(stdin, "{}", prompt).ok();
            }
            child.wait_with_output()
        })
        .map_err(|e| format!("ollama spawn failed: {e}"))?;

    // Parse ollama verbose output or use API
    // Fallback: use the `ollama run --verbose` timing or curl the API
    let api_result = std::process::Command::new("curl")
        .args([
            "-s",
            "http://localhost:11434/api/generate",
            "-d",
            &format!(
                r#"{{"model":"{}","prompt":"{}","stream":false,"options":{{"num_predict":{},"temperature":0}}}}"#,
                ollama_model, prompt, max_tokens
            ),
        ])
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;

    let body = String::from_utf8_lossy(&api_result.stdout);
    // Parse JSON response for timing
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("JSON parse failed: {e}\nbody: {body}"))?;

    let total_ns = json["total_duration"].as_f64().unwrap_or(0.0);
    let eval_count = json["eval_count"].as_u64().unwrap_or(0) as usize;
    let eval_ns = json["eval_duration"].as_f64().unwrap_or(0.0);

    let gen_s = eval_ns / 1e9;
    let tok_s = if gen_s > 0.0 {
        eval_count as f64 / gen_s
    } else {
        0.0
    };

    Ok(BenchResult {
        model: display_name.to_string(),
        backend: "ollama".to_string(),
        load_s: (total_ns - eval_ns) / 1e9,
        prefill_ms: 0.0,
        prompt_tokens: 0,
        gen_tokens: eval_count,
        gen_s,
        tok_s,
    })
}

#[cfg(target_os = "macos")]
fn bench_metal(
    name: &str,
    safetensors: &Path,
    tokenizer_path: &Path,
    max_tokens: usize,
) -> Result<BenchResult, String> {
    use cyb_llm::backend::metal::MetalModel;

    let load_start = std::time::Instant::now();
    let mut model = MetalModel::load_from_safetensors(safetensors)?;
    let load_s = load_start.elapsed().as_secs_f64();

    let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
        .map_err(|e| format!("{e}"))?;
    let encoding = tokenizer.encode(PROMPT, false).map_err(|e| format!("{e}"))?;
    let token_ids = encoding.get_ids();

    // EOS tokens — detect from tokenizer, plus known IDs
    let mut eos: Vec<u32> = vec![
        151643, 151645, // Qwen3
        2,              // Llama </s>
    ];
    for special in &["<|endoftext|>", "</s>", "<|end_of_text|>", "<|eot_id|>"] {
        if let Some(id) = tokenizer.token_to_id(special) {
            if !eos.contains(&id) {
                eos.push(id);
            }
        }
    }

    // Prefill: feed all prompt tokens, keep last predicted token
    let prefill_start = std::time::Instant::now();
    let mut next_token = 0u32;
    for &tid in token_ids {
        next_token = model.forward_decode(tid);
    }
    let prefill_ms = prefill_start.elapsed().as_secs_f64() * 1000.0;

    // Profile first 10 decode steps
    {
        let mut layers_sum = 0.0;
        let mut lm_sum = 0.0;
        let mut argmax_sum = 0.0;
        let profile_steps = 10.min(max_tokens);
        let mut tok = next_token;
        for _ in 0..profile_steps {
            let (nt, l, lm, a) = model.forward_decode_profile(tok, false);
            layers_sum += l;
            lm_sum += lm;
            argmax_sum += a;
            tok = nt;
        }
        let total = layers_sum + lm_sum + argmax_sum;
        println!("    [profile] {profile_steps} steps: layers={:.2}ms ({:.0}%) lm_head={:.2}ms ({:.0}%) argmax={:.2}ms ({:.0}%) total={:.2}ms",
            layers_sum / profile_steps as f64, layers_sum / total * 100.0,
            lm_sum / profile_steps as f64, lm_sum / total * 100.0,
            argmax_sum / profile_steps as f64, argmax_sum / total * 100.0,
            total / profile_steps as f64,
        );
        model.reset_kv_cache();
        // Re-prefill after profile
        for &tid in token_ids {
            next_token = model.forward_decode(tid);
        }
    }

    // Decode
    let decode_start = std::time::Instant::now();
    let mut gen_count = 0;
    for _ in 0..max_tokens {
        if eos.contains(&next_token) {
            println!("    [metal debug] EOS hit: {next_token}");
            break;
        }
        gen_count += 1;
        next_token = model.forward_decode(next_token);
    }
    let gen_s = decode_start.elapsed().as_secs_f64();
    let tok_s = if gen_s > 0.0 { gen_count as f64 / gen_s } else { 0.0 };

    Ok(BenchResult {
        model: name.to_string(),
        backend: "metal".to_string(),
        load_s,
        prefill_ms,
        prompt_tokens: token_ids.len(),
        gen_tokens: gen_count,
        gen_s,
        tok_s,
    })
}
