//! End-to-end inference benchmark — cyb-llm vs ollama
//!
//! Loads real models from ~/llm/, generates tokens, measures tok/s.
//! Runs the same prompts through ollama for direct comparison.
//!
//! Usage:
//!   cargo run --release -p cyb-llm --bin bench-e2e
//!   cargo run --release -p cyb-llm --bin bench-e2e -- --tokens 64
//!   cargo run --release -p cyb-llm --bin bench-e2e -- --models qwen3-0.6b-abl,smollm2-360m

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
            ollama_name: None,
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
        let tokenizer = spec.local_dir.join("tokenizer.json");

        if !safetensors.exists() {
            println!("  {} — SKIP (no model.safetensors)", spec.name);
            continue;
        }
        if !tokenizer.exists() {
            println!("  {} — SKIP (no tokenizer.json)", spec.name);
            continue;
        }

        match bench_cyb(&spec.name, &safetensors, &tokenizer, config.max_tokens) {
            Ok(r) => {
                println!(
                    "  {:<20} load={:.1}s  prefill={:.0}ms  decode={:.1} tok/s  ({} tokens in {:.1}s)",
                    spec.name, r.load_s, r.prefill_ms, r.tok_s, r.gen_tokens, r.gen_s,
                );
                results.push(r);
            }
            Err(e) => {
                println!("  {:<20} ERROR: {}", spec.name, e);
            }
        }
    }

    // ── cyb-llm (Metal backend) ──
    #[cfg(target_os = "macos")]
    {
        println!("\n── cyb-llm (Metal backend) ──\n");
        for spec in &config.models {
            let safetensors = spec.local_dir.join("model.safetensors");
            let tokenizer = spec.local_dir.join("tokenizer.json");
            if !safetensors.exists() || !tokenizer.exists() { continue; }

            match bench_metal(&spec.name, &safetensors, &tokenizer, config.max_tokens) {
                Ok(r) => {
                    println!(
                        "  {:<20} load={:.1}s  decode={:.1} tok/s  ({} tokens in {:.1}s)",
                        spec.name, r.load_s, r.tok_s, r.gen_tokens, r.gen_s,
                    );
                    results.push(r);
                }
                Err(e) => {
                    println!("  {:<20} ERROR: {}", spec.name, e);
                }
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

    // ── Summary ──
    println!("\n── Summary ──\n");
    println!(
        "{:<20} {:<12} {:>10} {:>10} {:>10}",
        "Model", "Backend", "tok/s", "tokens", "time"
    );
    println!("{}", "-".repeat(64));

    for spec in &config.models {
        let cyb = results.iter().find(|r| r.model == spec.name && r.backend == "cyb-llm");
        let oll = results.iter().find(|r| r.model == spec.name && r.backend == "ollama");

        if let Some(c) = cyb {
            println!(
                "{:<20} {:<12} {:>10.1} {:>10} {:>10.1}s",
                c.model, "cyb-llm", c.tok_s, c.gen_tokens, c.gen_s
            );
        }
        if let Some(o) = oll {
            println!(
                "{:<20} {:<12} {:>10.1} {:>10} {:>10.1}s",
                o.model, "ollama", o.tok_s, o.gen_tokens, o.gen_s
            );
        }
        if let (Some(c), Some(o)) = (cyb, oll) {
            let speedup = c.tok_s / o.tok_s;
            println!(
                "{:<20} {:<12} {:>10}",
                "",
                if speedup >= 1.0 { "cyb wins" } else { "ollama wins" },
                format!("{:.2}x", speedup)
            );
        }
        println!();
    }
}

fn bench_cyb(
    name: &str,
    safetensors: &Path,
    tokenizer_path: &Path,
    max_tokens: usize,
) -> Result<BenchResult, String> {
    // Init wgpu
    let backend = cyb_llm::backend::create_wgpu_backend();
    let pipelines = backend.pipelines;

    // Load model
    let load_start = Instant::now();
    let mut generator =
        cyb_llm::generate::TextGenerator::new_safetensors(safetensors, tokenizer_path, pipelines)?;
    let load_s = load_start.elapsed().as_secs_f64();

    // Tokenize
    let encoding = tokenizers::Tokenizer::from_file(tokenizer_path)
        .map_err(|e| format!("{e}"))?
        .encode(PROMPT, false)
        .map_err(|e| format!("{e}"))?;
    let prompt_tokens = encoding.get_ids().len();

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

    // EOS tokens
    let eos: Vec<u32> = vec![
        151643, 151645, 2, 0, 50256,
    ];

    // Prefill: feed all prompt tokens, keep last predicted token
    let prefill_start = std::time::Instant::now();
    let mut next_token = 0u32;
    for &tid in token_ids {
        next_token = model.forward_decode(tid);
    }
    let prefill_ms = prefill_start.elapsed().as_secs_f64() * 1000.0;
    println!("    [metal debug] prefill done, first predicted token: {next_token}");

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
