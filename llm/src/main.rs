//! cyb-llm CLI — HF model → .model importer.
//!
//! The inference runtime moved to `mr/` (see reference/runtime/). This
//! crate is the importer: reads HF directories (safetensors / GGUF / ONNX),
//! normalizes them, and writes .model files that mr loads. Subcommands
//! for run/embed/transcribe are gone — use `mr run`, `mr status`, etc.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cyb-llm")]
#[command(about = "Pure Rust LLM runtime — wgpu + Metal")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Backend: auto, metal, wgpu (default: auto — Metal on macOS, wgpu elsewhere)
    #[arg(long, default_value = "auto", global = true)]
    backend: String,

    /// Precision: q4 (quantized, fast), f16 (full precision, accurate)
    #[arg(long, default_value = "q4", global = true)]
    precision: String,
}

#[derive(Subcommand)]
enum Commands {
    /// List models cached under ~/.cache/huggingface/hub
    List,

    /// Download a model from HuggingFace
    Download {
        /// Model ID on HuggingFace
        model: String,
    },

    /// Audit local models against the soma manifest
    Audit,

    /// Clean junk files, duplicates, incomplete downloads
    Clean {
        /// Actually delete (default: dry run)
        #[arg(long)]
        execute: bool,
    },

    /// Convert safetensors to quantized GGUF (Q4 / Q8 / F16)
    Convert {
        /// Model name (from manifest) or "all"
        name: String,
        /// Override quant level: q4, q8, f16 (default: from manifest)
        #[arg(short, long)]
        quant: Option<String>,
        /// Actually convert (default: dry run)
        #[arg(long)]
        execute: bool,
        /// Delete original safetensors after successful conversion
        #[arg(long)]
        cleanup: bool,
    },

    /// Import: GGUF + tokenizer.json + config.json → .model file
    Import {
        /// Path to directory containing the source files
        name: String,
    },

    /// Pack into legacy .cyb format (predecessor of .model)
    Pack { name: String },

    /// Extract tensors.toml from a packed model for inspection
    ExtractTensors { name: String },

    /// Pack all 7 sections into a .model file (new spec)
    ModelPack { name: String },

    /// Fetch missing or incomplete models from HuggingFace
    Fetch {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        tier: Option<String>,
    },
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            println!("Cached models:");
            let cache_dir = std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".cache/huggingface/hub"))
                .unwrap_or_default();
            if cache_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                    for entry in entries.flatten() {
                        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            let name = entry.file_name();
                            if let Some(s) = name.to_str() {
                                if s.starts_with("models--") {
                                    println!(
                                        "  {}",
                                        s.replace("models--", "").replace("--", "/")
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                println!("  (none)");
            }
        }

        Commands::Download { model } => {
            println!("Downloading {model}...");
            match cyb_llm::hub::download_model(&model) {
                Ok(path) => println!("Downloaded to: {}", path.display()),
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        Commands::Audit => run_audit(),
        Commands::Clean { execute } => run_clean(!execute),
        Commands::Fetch { name, tier } => run_fetch(name, tier),

        Commands::Import { name } => {
            run_import(&name);
        }

        Commands::Convert {
            name,
            quant,
            execute,
            cleanup,
        } => {
            run_convert(&name, quant.as_deref(), execute, cleanup);
        }

        Commands::Pack { name } => {
            run_pack(&name);
        }

        Commands::ExtractTensors { name } => {
            run_extract_tensors(&name);
        }

        Commands::ModelPack { name } => {
            run_model_pack(&name);
        }
    }
}

fn run_audit() { eprintln!("audit: not implemented (MVP)"); }
fn run_clean(_dry_run: bool) { eprintln!("clean: not implemented (MVP)"); }
fn run_fetch(_name: Option<String>, _tier: Option<String>) { eprintln!("fetch: not implemented (MVP)"); }
/// Import: GGUF + tokenizer.json + config.json → .model
/// Usage: cyb-llm import ~/llm/gemma-4-31b-import
fn run_import(dir_path: &str) {
    use cyb_llm::cyb_format;
    use cyb_llm::loader;

    let dir = std::path::Path::new(dir_path);
    if !dir.is_dir() {
        eprintln!("Expected directory with GGUF + tokenizer.json + config.json");
        return;
    }

    // Find GGUF file
    let gguf_path = match std::fs::read_dir(dir).ok()
        .and_then(|entries| entries.flatten()
            .find(|e| e.path().extension().map(|x| x == "gguf").unwrap_or(false))
            .map(|e| e.path())) {
        Some(p) => p,
        None => { eprintln!("No .gguf file found in {}", dir.display()); return; }
    };
    println!("GGUF: {}", gguf_path.display());

    // Load GGUF
    let load_start = std::time::Instant::now();
    let graph = match loader::load_model(&gguf_path) {
        Ok(g) => g,
        Err(e) => { eprintln!("GGUF load failed: {e}"); return; }
    };
    println!("Loaded {} tensors in {:.1}s", graph.weights.len(), load_start.elapsed().as_secs_f64());

    // Read config.json
    let config_json_path = dir.join("config.json");
    let config_json: serde_json::Value = if config_json_path.exists() {
        let s = std::fs::read_to_string(&config_json_path).expect("read config.json");
        serde_json::from_str(&s).expect("parse config.json")
    } else {
        eprintln!("No config.json found"); return;
    };

    // Extract architecture params
    let text_config = config_json.get("text_config").unwrap_or(&config_json);
    let model_type = config_json.get("model_type").and_then(|v| v.as_str()).unwrap_or("unknown");

    // Detect EOS token from tokenizer_config.json (for chat template detection)
    let tok_config_path = dir.join("tokenizer_config.json");
    let eos_token_str = if tok_config_path.exists() {
        let tc = std::fs::read_to_string(&tok_config_path).unwrap_or_default();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&tc) {
            v.get("eos_token").and_then(|t| {
                t.as_str().map(|s| s.to_string())
                    .or_else(|| t.get("content").and_then(|c| c.as_str()).map(|s| s.to_string()))
            }).unwrap_or_default()
        } else { String::new() }
    } else { String::new() };
    let hidden_size = text_config["hidden_size"].as_u64().unwrap_or(0);
    let num_heads = text_config["num_attention_heads"].as_u64().unwrap_or(0);
    let kv_heads = text_config["num_key_value_heads"].as_u64().unwrap_or(num_heads);
    let num_layers = text_config["num_hidden_layers"].as_u64().unwrap_or(0);
    let intermediate_size = text_config["intermediate_size"].as_u64().unwrap_or(0);
    let vocab_size = text_config["vocab_size"].as_u64().unwrap_or(0);
    let head_dim = text_config["head_dim"].as_u64().unwrap_or(hidden_size / num_heads);
    let max_pos = text_config["max_position_embeddings"].as_u64().unwrap_or(8192);
    let rope_theta = text_config["rope_theta"].as_f64().unwrap_or(10000.0);
    let rms_norm_eps = text_config["rms_norm_eps"].as_f64().unwrap_or(1e-6);
    let tie_word_embeddings = text_config["tie_word_embeddings"].as_bool()
        .or_else(|| config_json["tie_word_embeddings"].as_bool())
        .unwrap_or(true);

    println!("Architecture: {model_type}, hidden={hidden_size}, heads={num_heads}/{kv_heads}, layers={num_layers}, tie_embed={tie_word_embeddings}");

    // Generate config.toml
    let config_toml = format!(
        r#"model_type = "{model_type}"
parameters = {params}

[architecture]
hidden_size = {hidden_size}
num_attention_heads = {num_heads}
num_key_value_heads = {kv_heads}
head_dim = {head_dim}
num_hidden_layers = {num_layers}
intermediate_size = {intermediate_size}
vocab_size = {vocab_size}
max_position_embeddings = {max_pos}
rope_theta = {rope_theta}
rms_norm_eps = {rms_norm_eps}
tie_word_embeddings = {tie_word_embeddings}

[tokenizer]
type = "bpe"
eos_token = "{eos_token}"

[sampling]
temperature = 700
top_p = 900
scale = 1000

[lineage]
source = "{source}"
"#,
        params = hidden_size * num_layers * 12, // rough estimate
        eos_token = eos_token_str,
        source = config_json_path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or(""),
    );

    // Generate card.md
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("model");
    let card = format!("# {name}\n\n{model_type}, {num_layers} layers, {hidden_size} hidden.\n");

    // Generate vocab.toml from tokenizer.json
    let tokenizer_path = dir.join("tokenizer.json");
    let vocab_toml = if tokenizer_path.exists() {
        println!("Generating vocab.toml from tokenizer.json...");
        // Use Python fix-vocab approach inline
        match std::process::Command::new("python3")
            .arg("-c")
            .arg(format!(r#"
import json, sys
def esc(s):
    r = []
    for c in s:
        if c == '\\': r.append('\\\\')
        elif c == '"': r.append('\\"')
        elif c == '\n': r.append('\\n')
        elif c == '\t': r.append('\\t')
        elif c == '\r': r.append('\\r')
        elif ord(c) < 0x20: r.append(f'\\u{{ord(c):04X}}')
        else: r.append(c)
    return ''.join(r)
with open('{}') as f: tok = json.load(f)
m = tok.get('model', {{}})
vocab = m.get('vocab', {{}})
merges = m.get('merges', [])
added = tok.get('added_tokens', [])
lines = ['[tokens]']
seen_ids = set()
if isinstance(vocab, dict):
    for t, i in sorted(vocab.items(), key=lambda x: x[1]):
        lines.append(f'{{i}} = "{{esc(t)}}"')
        seen_ids.add(i)
else:
    for i, item in enumerate(vocab):
        t = item[0] if isinstance(item, list) else str(item)
        lines.append(f'{{i}} = "{{esc(t)}}"')
        seen_ids.add(i)
# HF special tokens (e.g. <|im_start|>=151644, <|im_end|>=151645) live in
# added_tokens — they have model-valid IDs but aren't in the base vocab.
# Without them, chat-templated prompts decompose into single-char BPE.
for at in added:
    tid = at.get('id', -1)
    content = at.get('content', '')
    if tid >= 0 and content and tid not in seen_ids:
        lines.append(f'{{tid}} = "{{esc(content)}}"')
        seen_ids.add(tid)
if merges:
    lines.append('')
    lines.append('[merges]')
    for i, mg in enumerate(merges):
        if isinstance(mg, list) and len(mg) == 2:
            a, b = mg
        elif isinstance(mg, str):
            parts = mg.split(' ', 1)
            if len(parts) != 2: continue
            a, b = parts
        else: continue
        lines.append(f'{{i}} = ["{{esc(a)}}", "{{esc(b)}}"]')
lines.append('')
print('\n'.join(lines))
"#, tokenizer_path.display()))
            .output()
        {
            Ok(out) if out.status.success() => {
                let v = String::from_utf8_lossy(&out.stdout).to_string();
                println!("  vocab: {} lines", v.lines().count());
                v
            }
            _ => { eprintln!("Failed to generate vocab.toml"); String::new() }
        }
    } else {
        eprintln!("No tokenizer.json found");
        String::new()
    };

    // Generate tensors.toml and pack binary weights
    // Print dtype distribution
    {
        let mut dtype_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for w in graph.weights.values() {
            *dtype_counts.entry(format!("{:?}", w.dtype)).or_default() += 1;
        }
        let mut counts: Vec<_> = dtype_counts.into_iter().collect();
        counts.sort();
        println!("  dtypes: {:?}", counts);
    }

    println!("Packing {} tensors...", graph.weights.len());
    let mut tensors_lines = Vec::new();
    let mut weight_data = Vec::new();
    let mut offset = 0usize;

    // Conversion counters
    let mut q4_0_to_q4k = 0usize;
    let mut q4_1_to_q4k = 0usize;
    let mut bf16_to_f16 = 0usize;

    // Sort tensors by name for deterministic output
    let mut tensor_names: Vec<&String> = graph.weights.keys().collect();
    tensor_names.sort();

    for tname in &tensor_names {
        let w = &graph.weights[*tname];

        // Normalize obsolete formats to canonical set during import:
        //   Q4_0 → Q4_K, Q4_1 → Q4_K, BF16 → F16
        // Canonical set: q4k, q6k, q5k, q3k, q2k, q8, u16, u32
        let (encoding, converted_data): (&str, Option<Vec<u8>>) = match w.dtype {
            cyb_llm::ir::DType::F32 => ("u32", None),
            cyb_llm::ir::DType::F16 => ("u16", None),
            cyb_llm::ir::DType::BF16 => {
                // BF16 → f32 → f16
                bf16_to_f16 += 1;
                let f32s = cyb_llm::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
                let f16_bytes: Vec<u8> = f32s.iter().flat_map(|&v| {
                    half::f16::from_f32(v).to_le_bytes()
                }).collect();
                ("u16", Some(f16_bytes))
            }
            cyb_llm::ir::DType::Q4 => {
                // Q4_0 → f32 → Q4_K
                q4_0_to_q4k += 1;
                let f32s = cyb_llm::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
                let n = w.shape.first().copied().unwrap_or(1);
                let k = if w.shape.len() >= 2 { w.shape[1] } else { f32s.len() / n };
                let q4k_data = cyb_llm::import::quantize_f32_to_q4k(&f32s, n, k);
                ("q4k", Some(q4k_data))
            }
            cyb_llm::ir::DType::Q4_1 => {
                // Q4_1 → f32 → Q4_K
                q4_1_to_q4k += 1;
                let f32s = cyb_llm::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
                let n = w.shape.first().copied().unwrap_or(1);
                let k = if w.shape.len() >= 2 { w.shape[1] } else { f32s.len() / n };
                let q4k_data = cyb_llm::import::quantize_f32_to_q4k(&f32s, n, k);
                ("q4k", Some(q4k_data))
            }
            cyb_llm::ir::DType::Q8 => ("q8", None),
            cyb_llm::ir::DType::Ternary | cyb_llm::ir::DType::U8 => ("ternary", None),
            cyb_llm::ir::DType::Q4_K => ("q4k", None),
            cyb_llm::ir::DType::Q6_K => ("q6k", None),
            cyb_llm::ir::DType::Q2_K => ("q2k", None),
            cyb_llm::ir::DType::Q3_K => ("q3k", None),
            cyb_llm::ir::DType::Q5_K => ("q5k", None),
            _ => ("u32", None),
        };
        let data_ref = converted_data.as_deref().unwrap_or(&w.data);
        let size = data_ref.len();
        let shape_str = w.shape.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ");

        let hf_name = cyb_llm::import::gguf_to_hf(tname);

        tensors_lines.push(format!(
            "[\"{}\"]\nshape    = [{}]\nencoding = \"{}\"\noffset   = {}\nsize     = {}\n",
            hf_name, shape_str, encoding, offset, size
        ));

        weight_data.extend_from_slice(data_ref);
        offset += size;
    }
    let tensors_toml = tensors_lines.join("\n");

    // Print conversion summary
    if q4_0_to_q4k > 0 || q4_1_to_q4k > 0 || bf16_to_f16 > 0 {
        println!("Converted: {} Q4_0 → Q4_K, {} Q4_1 → Q4_K, {} BF16 → F16",
            q4_0_to_q4k, q4_1_to_q4k, bf16_to_f16);
    }
    println!("  weights: {} bytes ({:.1} GB)", weight_data.len(), weight_data.len() as f64 / 1e9);

    // Write .model file
    let output_name = name.replace("-import", "");
    let output_path = cyb_llm::manifest::models_dir().join(format!("{output_name}.model"));
    println!("Writing {}...", output_path.display());

    match cyb_format::write_model_file(
        &output_path, &output_name,
        &card, &config_toml, "", "rs",
        &tensors_toml, &vocab_toml, "",
        &weight_data,
    ) {
        Ok(()) => {
            let size = output_path.metadata().map(|m| m.len()).unwrap_or(0);
            println!("OK: {} ({:.1} GB)", output_path.display(), size as f64 / 1e9);
        }
        Err(e) => eprintln!("FAIL: {e}"),
    }
}
fn run_convert(_name: &str, _quant: Option<&str>, _execute: bool, _cleanup: bool) { eprintln!("convert: not implemented (MVP)"); }
fn run_pack(_name: &str) { eprintln!("pack: not implemented (MVP)"); }
fn run_extract_tensors(_name: &str) { eprintln!("extract-tensors: not implemented (MVP)"); }
fn run_model_pack(_name: &str) { eprintln!("model-pack: not implemented (MVP)"); }

// Legacy audit/clean/fetch/import/convert/pack commands removed.
// Will be reimplemented for .model format in MVP Phase 1.
