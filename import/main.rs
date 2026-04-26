//! mi — model importer CLI (binary entry point of the `import` crate).
//!
//! HuggingFace directory (safetensors + tokenizer.json + config.json)
//! or GGUF → cyb `.model`. Runtime is in `run/`.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mi")]
#[command(about = "Model importer — HF / GGUF / safetensors → cyb .model")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import a directory containing GGUF + tokenizer.json + config.json
    /// into a `.model` file under `~/llm/`.
    Import {
        /// Path to the source directory.
        dir: String,
    },

    /// List models cached under ~/.cache/huggingface/hub.
    List,

    /// Download a model from HuggingFace (best-effort ONNX/safetensors discovery).
    Download {
        /// HF repo id (e.g. "Qwen/Qwen3-0.6B").
        model: String,
    },
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Import { dir } => run_import(&dir),
        Commands::List => run_list(),
        Commands::Download { model } => run_download(&model),
    }
}

fn run_list() {
    println!("Cached HF models:");
    let cache_dir = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".cache/huggingface/hub"))
        .unwrap_or_default();
    if !cache_dir.exists() {
        println!("  (none)");
        return;
    }
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(s) = entry.file_name().to_str() {
                    if s.starts_with("models--") {
                        println!("  {}", s.replace("models--", "").replace("--", "/"));
                    }
                }
            }
        }
    }
}

fn run_download(model_id: &str) {
    println!("Downloading {model_id}...");
    match import::hf::download_model(model_id) {
        Ok(downloaded) => {
            println!(
                "Artifact ({:?}): {}",
                downloaded.kind,
                downloaded.artifact.display()
            );
            for s in &downloaded.siblings {
                println!("  sibling: {}", s.display());
            }
            if let Some(dir) = downloaded.snapshot_dir() {
                println!("\nSnapshot dir: {}", dir.display());
                println!("Run: mi import {}", dir.display());
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

/// Import: source dir with weights artifact + tokenizer.json + config.json → `.model`.
fn run_import(dir_path: &str) {
    let dir = std::path::Path::new(dir_path);
    if !dir.is_dir() {
        eprintln!("Expected directory with weights + tokenizer.json + config.json");
        std::process::exit(1);
    }

    // Locate the weights artifact. Priority safetensors > GGUF > ONNX,
    // matching the hf.md fetch contract. The loader auto-detects format.
    let entries: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .ok()
        .map(|es| es.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    let ext_eq = |p: &std::path::Path, want: &str| {
        p.extension()
            .and_then(|x| x.to_str())
            .map(|x| x == want)
            .unwrap_or(false)
    };
    let safetensors_files: Vec<_> = entries
        .iter()
        .filter(|p| ext_eq(p, "safetensors"))
        .cloned()
        .collect();
    let gguf_files: Vec<_> = entries
        .iter()
        .filter(|p| ext_eq(p, "gguf"))
        .cloned()
        .collect();
    let onnx_files: Vec<_> = entries
        .iter()
        .filter(|p| ext_eq(p, "onnx"))
        .cloned()
        .collect();

    let gguf_path = if !safetensors_files.is_empty() {
        // Sharded safetensors: pick any shard; the loader follows the index.
        // Single-file: just one entry.
        let path = safetensors_files[0].clone();
        println!("Source: safetensors — {}", path.display());
        path
    } else if gguf_files.len() == 1 {
        let path = gguf_files.into_iter().next().unwrap();
        println!("Source: GGUF — {}", path.display());
        path
    } else if gguf_files.len() > 1 {
        eprintln!(
            "Expected one .gguf in {}, found {}",
            dir.display(),
            gguf_files.len()
        );
        std::process::exit(1);
    } else if !onnx_files.is_empty() {
        let path = onnx_files.into_iter().next().unwrap();
        println!("Source: ONNX — {}", path.display());
        path
    } else {
        eprintln!(
            "No model artifact found in {} (looked for safetensors / gguf / onnx)",
            dir.display()
        );
        std::process::exit(1);
    };

    let t_load = std::time::Instant::now();
    let weights = match import::loader::load_model(&gguf_path) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("GGUF load failed: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "Loaded {} tensors in {:.1}s",
        weights.len(),
        t_load.elapsed().as_secs_f64()
    );

    // config.json → config.toml
    let config_json_path = dir.join("config.json");
    let config_json: serde_json::Value = if config_json_path.exists() {
        let s = std::fs::read_to_string(&config_json_path).expect("read config.json");
        serde_json::from_str(&s).expect("parse config.json")
    } else {
        eprintln!("No config.json found");
        std::process::exit(1);
    };
    let text_config = config_json.get("text_config").unwrap_or(&config_json);
    let model_type = config_json
        .get("model_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let tok_config_path = dir.join("tokenizer_config.json");
    let eos_token_str = if tok_config_path.exists() {
        let tc = std::fs::read_to_string(&tok_config_path).unwrap_or_default();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&tc) {
            v.get("eos_token")
                .and_then(|t| {
                    t.as_str().map(|s| s.to_string()).or_else(|| {
                        t.get("content")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string())
                    })
                })
                .unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let hidden_size = text_config["hidden_size"].as_u64().unwrap_or(0);
    let num_heads = text_config["num_attention_heads"].as_u64().unwrap_or(0);
    let kv_heads = text_config["num_key_value_heads"]
        .as_u64()
        .unwrap_or(num_heads);
    let num_layers = text_config["num_hidden_layers"].as_u64().unwrap_or(0);
    let intermediate_size = text_config["intermediate_size"].as_u64().unwrap_or(0);
    let vocab_size = text_config["vocab_size"].as_u64().unwrap_or(0);
    let head_dim = text_config["head_dim"]
        .as_u64()
        .unwrap_or(hidden_size / num_heads.max(1));
    let max_pos = text_config["max_position_embeddings"]
        .as_u64()
        .unwrap_or(8192);
    // Per-kind RoPE (Gemma-4): rope_parameters.{full_attention, sliding_attention}.
    // Flat rope_theta is the LlamaStyle / Gemma-3 case; the nested form
    // extracts sliding's theta here and full's theta+partial_rotary later.
    let rope_params = text_config.get("rope_parameters");
    let rope_sliding = rope_params.and_then(|p| p.get("sliding_attention"));
    let rope_full = rope_params.and_then(|p| p.get("full_attention"));
    let rope_theta = rope_sliding
        .and_then(|s| s.get("rope_theta"))
        .and_then(|v| v.as_f64())
        .or_else(|| text_config["rope_theta"].as_f64())
        .unwrap_or(10000.0);
    let rope_theta_full = rope_full
        .and_then(|f| f.get("rope_theta"))
        .and_then(|v| v.as_f64());
    let partial_rotary_factor_full = rope_full
        .and_then(|f| f.get("partial_rotary_factor"))
        .and_then(|v| v.as_f64());
    let rms_norm_eps = text_config["rms_norm_eps"].as_f64().unwrap_or(1e-6);
    let tie_word_embeddings = text_config["tie_word_embeddings"]
        .as_bool()
        .or_else(|| config_json["tie_word_embeddings"].as_bool())
        .unwrap_or(true);

    // LlamaStyle+ (Gemma 3/4) optional fields. Each is omitted from the
    // config when absent so plain LlamaStyle models stay clean.
    // Spec: run/specs/format.md §"LlamaStyle+ extra fields"
    let hidden_activation = text_config["hidden_activation"]
        .as_str()
        .map(|s| s.to_string());
    let final_logit_softcapping = text_config["final_logit_softcapping"].as_f64();
    let attention_k_eq_v = text_config["attention_k_eq_v"].as_bool();
    let sliding_window = text_config["sliding_window"].as_u64();
    let global_head_dim = text_config["global_head_dim"].as_u64();
    let num_global_key_value_heads = text_config["num_global_key_value_heads"].as_u64();
    let layer_types: Vec<String> = text_config["layer_types"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    println!(
        "Architecture: {model_type}, hidden={hidden_size}, heads={num_heads}/{kv_heads}, \
         layers={num_layers}, tie_embed={tie_word_embeddings}"
    );
    if !layer_types.is_empty() {
        let n_sliding = layer_types.iter().filter(|t| *t == "sliding_attention").count();
        let n_full = layer_types.iter().filter(|t| *t == "full_attention").count();
        println!(
            "LlamaStyle+: layer_types={n_sliding} sliding / {n_full} full, \
             gh_dim={global_head_dim:?}, n_gkv={num_global_key_value_heads:?}, \
             k_eq_v={attention_k_eq_v:?}, softcap={final_logit_softcapping:?}, \
             act={hidden_activation:?}"
        );
    }

    // Build the optional LlamaStyle+ block. Each field is added on its own
    // line so the spec ordering is preserved and absent fields produce no
    // dangling whitespace.
    let mut llamaplus = String::new();
    if let Some(act) = &hidden_activation {
        llamaplus.push_str(&format!("hidden_activation = \"{act}\"\n"));
    }
    if let Some(cap) = final_logit_softcapping {
        llamaplus.push_str(&format!("final_logit_softcapping = {cap}\n"));
    }
    if let Some(k_eq_v) = attention_k_eq_v {
        llamaplus.push_str(&format!("attention_k_eq_v = {k_eq_v}\n"));
    }
    if let Some(win) = sliding_window {
        llamaplus.push_str(&format!("sliding_window = {win}\n"));
    }
    if let Some(gh) = global_head_dim {
        llamaplus.push_str(&format!("global_head_dim = {gh}\n"));
    }
    if let Some(gkv) = num_global_key_value_heads {
        llamaplus.push_str(&format!("num_global_key_value_heads = {gkv}\n"));
    }
    if !layer_types.is_empty() {
        let quoted: Vec<String> = layer_types
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect();
        llamaplus.push_str(&format!("layer_types = [{}]\n", quoted.join(", ")));
    }
    if let Some(rt_full) = rope_theta_full {
        llamaplus.push_str(&format!("rope_theta_full = {rt_full}\n"));
    }
    if let Some(prf) = partial_rotary_factor_full {
        llamaplus.push_str(&format!("partial_rotary_factor_full = {prf}\n"));
    }

    // Canonical config — integers only. eps stored as 1/ε; rope_theta is
    // already integer-valued in source (10000, 500000, 1_000_000 etc.).
    let rms_norm_eps_inv = if rms_norm_eps > 0.0 && rms_norm_eps < 1.0 {
        (1.0 / rms_norm_eps).round() as u64
    } else {
        // Source already provided 1/ε integer form; preserve.
        rms_norm_eps.round() as u64
    };
    let rope_theta_int = rope_theta.round() as u64;
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
rope_theta = {rope_theta_int}
rms_norm_eps = {rms_norm_eps_inv}
tie_word_embeddings = {tie_word_embeddings}
{llamaplus}
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
        params = hidden_size * num_layers * 12,
        eos_token = eos_token_str,
        source = config_json_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or(""),
    );

    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("model");
    let card = format!("# {name}\n\n{model_type}, {num_layers} layers, {hidden_size} hidden.\n");

    // tokenizer.json → vocab.toml (via Python helper — includes added_tokens so
    // chat-template specials like <|im_start|> don't decompose into single
    // chars. See fix committed 57363817.).
    let tokenizer_path = dir.join("tokenizer.json");
    let vocab_toml = if tokenizer_path.exists() {
        println!("Generating vocab.toml from tokenizer.json...");
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
            _ => {
                eprintln!("Failed to generate vocab.toml");
                String::new()
            }
        }
    } else {
        eprintln!("No tokenizer.json found");
        String::new()
    };

    // Log dtype distribution for transparency.
    {
        let mut dtype_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for w in weights.weights.values() {
            *dtype_counts.entry(format!("{:?}", w.dtype)).or_default() += 1;
        }
        let mut counts: Vec<_> = dtype_counts.into_iter().collect();
        counts.sort();
        println!("  dtypes: {:?}", counts);
    }

    println!("Packing {} tensors...", weights.len());
    let mut tensors_lines: Vec<String> = Vec::new();
    let mut weight_data: Vec<u8> = Vec::new();
    let mut offset = 0usize;

    let mut tensor_names: Vec<&String> = weights.weights.keys().collect();
    tensor_names.sort();

    // Existing HF-canonical names in the source. Used to detect K=V layers
    // where v_proj is absent and must be materialised from k_proj.
    let existing_hf: std::collections::HashSet<String> = tensor_names
        .iter()
        .map(|n| import::naming::gguf_to_hf(n))
        .collect();
    // Per spec (import/specs/import.md §"K=V shared projection"): when
    // the layer is K=V at the source (no v_proj tensor), the importer emits
    // a v_proj tensor with the same bytes as k_proj. Runtime stays one
    // codepath. Gemma-4 only sets K=V on full_attention layers.
    let kv_eq_layers: std::collections::HashSet<usize> =
        if attention_k_eq_v == Some(true) {
            layer_types
                .iter()
                .enumerate()
                .filter(|(_, t)| *t == "full_attention")
                .map(|(i, _)| i)
                .collect()
        } else {
            std::collections::HashSet::new()
        };

    let mut counts_by_enc: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for tname in &tensor_names {
        let w = &weights.weights[*tname];
        let hf_name = import::naming::gguf_to_hf(tname);

        // Canonical alignment: every tensor goes source → f32 → canonical encoding.
        // Source dtype is irrelevant to the *output* layout; only the canonical
        // policy (by tensor name) decides what the bytes look like on disk.
        let f32s = import::dequantize_to_f32(&w.data, w.dtype);
        if f32s.is_empty() {
            eprintln!("warn: {tname} dequant returned empty (dtype {:?})", w.dtype);
            continue;
        }

        let encoding = import::naming::canonical_encoding_for(&hf_name);
        let canonical_bytes: Vec<u8> = match encoding {
            "u32" => import::quant::canonical::f32_to_u32(&f32s),
            "u16" => import::quant::canonical::f32_to_u16(&f32s),
            "q4" => {
                if f32s.len() % 32 != 0 {
                    eprintln!(
                        "warn: {hf_name} has {} elements, not a multiple of 32 — falling back to u32",
                        f32s.len()
                    );
                    counts_by_enc.entry("u32_fallback").and_modify(|c| *c += 1).or_insert(1);
                    import::quant::canonical::f32_to_u32(&f32s)
                } else {
                    import::quant::canonical::f32_to_q4(&f32s)
                }
            }
            "q8" => import::quant::canonical::f32_to_q8(&f32s),
            "ternary" => import::quant::canonical::f32_to_ternary(&f32s),
            other => {
                eprintln!("warn: unknown canonical encoding {other} for {hf_name}; using u32");
                import::quant::canonical::f32_to_u32(&f32s)
            }
        };
        // Track the encoding written (after fallback resolution).
        let written_enc = if encoding == "q4" && f32s.len() % 32 != 0 { "u32" } else { encoding };
        counts_by_enc.entry(written_enc).and_modify(|c| *c += 1).or_insert(1);

        let size = canonical_bytes.len();
        let shape_str = w
            .shape
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        tensors_lines.push(format!(
            "[\"{}\"]\nshape    = [{}]\nencoding = \"{}\"\noffset   = {}\nsize     = {}\n",
            hf_name, shape_str, written_enc, offset, size
        ));
        weight_data.extend_from_slice(&canonical_bytes);
        offset += size;
        let data_ref: &[u8] = &canonical_bytes;

        // K=V duplicate emission. If this is a k_proj for a K=V full layer
        // and the source lacks v_proj, emit v_proj with identical bytes.
        if let Some(rest) = hf_name.strip_prefix("model.layers.") {
            if let Some(dot) = rest.find('.') {
                if let Ok(layer_idx) = rest[..dot].parse::<usize>() {
                    let suffix = &rest[dot + 1..];
                    if suffix == "self_attn.k_proj.weight"
                        && kv_eq_layers.contains(&layer_idx)
                    {
                        let v_name = format!(
                            "model.layers.{layer_idx}.self_attn.v_proj.weight"
                        );
                        if !existing_hf.contains(&v_name) {
                            tensors_lines.push(format!(
                                "[\"{}\"]\nshape    = [{}]\nencoding = \"{}\"\noffset   = {}\nsize     = {}\n",
                                v_name, shape_str, written_enc, offset, size
                            ));
                            weight_data.extend_from_slice(data_ref);
                            offset += size;
                        }
                    }
                }
            }
        }
    }
    let tensors_toml = tensors_lines.join("\n");

    println!("Canonical encoding distribution:");
    let mut entries: Vec<_> = counts_by_enc.iter().collect();
    entries.sort_by_key(|(k, _)| *k);
    for (enc, count) in entries {
        println!("  {enc}: {count} tensors");
    }
    println!(
        "  weights: {} bytes ({:.1} GB)",
        weight_data.len(),
        weight_data.len() as f64 / 1e9
    );

    // The optional ~~~graph section is not part of the canonical .model
    // spec (cyb/cyb-model). Re-introduce as a formal extension if useful.

    let output_name = name.strip_suffix("-import").unwrap_or(name);
    let models_dir = import::manifest::models_dir();
    if let Err(e) = std::fs::create_dir_all(&models_dir) {
        eprintln!("Cannot create {}: {e}", models_dir.display());
        std::process::exit(1);
    }
    // Write canonical to <NAME>.canonical.model — keep the original
    // <NAME>.model intact until canonical alignment is verified end-to-end.
    let output_path = models_dir.join(format!("{output_name}.canonical.model"));
    println!("Writing {}...", output_path.display());

    match import::cyb_format::write_model_file(
        &output_path,
        &output_name,
        &card,
        &config_toml,
        "",
        "rs",
        None,
        &tensors_toml,
        &vocab_toml,
        "",
        &weight_data,
    ) {
        Ok(()) => {
            let size = output_path.metadata().map(|m| m.len()).unwrap_or(0);
            println!(
                "OK: {} ({:.1} GB)",
                output_path.display(),
                size as f64 / 1e9
            );
        }
        Err(e) => {
            eprintln!("FAIL: {e}");
            std::process::exit(1);
        }
    }
}
