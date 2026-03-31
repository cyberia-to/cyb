//! Import pipeline: download HF repo → canonical soma model layout.
//!
//! Canonical layout per model:
//!   config.toml        — architecture params
//!   vocab.toml         — tokenizer metadata (type, vocab_size)
//!   chat.toml          — chat template + special tokens
//!   sampling.toml      — inference defaults
//!   tokenizer.json     — kept for tokenizers crate (runtime needs it)
//!   weights.*          — single weights file
//!
//! JSON configs are converted to TOML. Duplicates, junk, and training
//! artifacts are deleted. Weight files renamed to weights.*.

use std::path::{Path, PathBuf};

/// Result of an import operation
#[derive(Default)]
pub struct ImportResult {
    pub downloaded_bytes: u64,
    pub final_bytes: u64,
    pub files_deleted: usize,
    pub configs_converted: usize,
    pub weights_format: String,
    pub errors: Vec<String>,
}

/// Run the full import pipeline on an already-downloaded model directory.
/// Does NOT download — call after fetch or on a local dir.
pub fn canonicalize(model_dir: &Path) -> ImportResult {
    let mut result = ImportResult::default();

    // 1. Convert JSON configs → TOML
    convert_config_json(model_dir, &mut result);
    convert_tokenizer_config(model_dir, &mut result);
    convert_generation_config(model_dir, &mut result);
    create_vocab_toml(model_dir, &mut result);

    // 2. Select and rename weights
    select_weights(model_dir, &mut result);

    // 3. Clean junk
    clean_junk(model_dir, &mut result);

    // 4. Compute final size
    result.final_bytes = dir_size(model_dir);

    result
}

// ── Config converters ────────────────────────────────────────────

fn convert_config_json(dir: &Path, result: &mut ImportResult) {
    let src = dir.join("config.json");
    let dst = dir.join("config.toml");
    if dst.exists() || !src.exists() {
        return;
    }

    let json_str = match std::fs::read_to_string(&src) {
        Ok(s) => s,
        Err(e) => {
            result.errors.push(format!("read config.json: {e}"));
            return;
        }
    };
    let json: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            result.errors.push(format!("parse config.json: {e}"));
            return;
        }
    };

    let mut toml = String::new();

    // Architecture
    if let Some(v) = json.get("architectures").and_then(|a| a.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()) {
        toml.push_str(&format!("architecture = \"{v}\"\n"));
    } else if let Some(v) = json.get("model_type").and_then(|v| v.as_str()) {
        toml.push_str(&format!("model_type = \"{v}\"\n"));
    }

    // Core params — emit each if present
    let fields = [
        ("hidden_size", "hidden_size"),
        ("num_attention_heads", "num_attention_heads"),
        ("num_key_value_heads", "num_key_value_heads"),
        ("num_hidden_layers", "num_hidden_layers"),
        ("intermediate_size", "intermediate_size"),
        ("vocab_size", "vocab_size"),
        ("max_position_embeddings", "max_position_embeddings"),
        ("num_experts", "num_experts"),
        ("num_experts_per_tok", "num_experts_per_tok"),
    ];
    for (json_key, toml_key) in fields {
        if let Some(v) = json.get(json_key).and_then(|v| v.as_u64()) {
            toml.push_str(&format!("{toml_key} = {v}\n"));
        }
    }

    let float_fields = [
        ("rope_theta", "rope_theta"),
        ("rms_norm_eps", "rms_norm_eps"),
        ("layer_norm_eps", "layer_norm_eps"),
    ];
    for (json_key, toml_key) in float_fields {
        if let Some(v) = json.get(json_key).and_then(|v| v.as_f64()) {
            toml.push_str(&format!("{toml_key} = {v}\n"));
        }
    }

    let bool_fields = [
        ("tie_word_embeddings", "tie_word_embeddings"),
    ];
    for (json_key, toml_key) in bool_fields {
        if let Some(v) = json.get(json_key).and_then(|v| v.as_bool()) {
            toml.push_str(&format!("{toml_key} = {v}\n"));
        }
    }

    if let Err(e) = std::fs::write(&dst, &toml) {
        result.errors.push(format!("write config.toml: {e}"));
        return;
    }
    result.configs_converted += 1;
}

fn convert_tokenizer_config(dir: &Path, result: &mut ImportResult) {
    let dst = dir.join("chat.toml");
    if dst.exists() {
        return;
    }

    // Try tokenizer_config.json first, fallback to chat_template in config
    let src = dir.join("tokenizer_config.json");
    if !src.exists() {
        return;
    }

    let json_str = match std::fs::read_to_string(&src) {
        Ok(s) => s,
        Err(_) => return,
    };
    let json: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut toml = String::new();

    // Chat template
    if let Some(template) = json.get("chat_template").and_then(|v| v.as_str()) {
        // Use triple-quoted string for multiline templates
        toml.push_str(&format!("template = \"\"\"\n{template}\n\"\"\"\n"));
    }

    // Special tokens
    for (json_key, toml_key) in [
        ("bos_token", "bos_token"),
        ("eos_token", "eos_token"),
        ("pad_token", "pad_token"),
        ("unk_token", "unk_token"),
    ] {
        let val = json.get(json_key);
        if let Some(s) = val.and_then(|v| v.as_str()) {
            toml.push_str(&format!("{toml_key} = \"{s}\"\n"));
        } else if let Some(s) = val.and_then(|v| v.get("content")).and_then(|v| v.as_str()) {
            // Some tokenizers nest: {"content": "<|...|>", "single_word": false, ...}
            toml.push_str(&format!("{toml_key} = \"{s}\"\n"));
        }
    }

    if toml.is_empty() {
        return;
    }

    if let Err(e) = std::fs::write(&dst, &toml) {
        result.errors.push(format!("write chat.toml: {e}"));
        return;
    }
    result.configs_converted += 1;
}

fn convert_generation_config(dir: &Path, result: &mut ImportResult) {
    let dst = dir.join("sampling.toml");
    if dst.exists() {
        return;
    }

    let src = dir.join("generation_config.json");
    if !src.exists() {
        return;
    }

    let json_str = match std::fs::read_to_string(&src) {
        Ok(s) => s,
        Err(_) => return,
    };
    let json: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut toml = String::new();

    // Float params
    for (json_key, toml_key) in [
        ("temperature", "temperature"),
        ("top_p", "top_p"),
        ("repetition_penalty", "repetition_penalty"),
    ] {
        if let Some(v) = json.get(json_key).and_then(|v| v.as_f64()) {
            toml.push_str(&format!("{toml_key} = {v}\n"));
        }
    }

    // Int params
    for (json_key, toml_key) in [
        ("top_k", "top_k"),
        ("max_new_tokens", "max_tokens"),
        ("max_length", "max_length"),
    ] {
        if let Some(v) = json.get(json_key).and_then(|v| v.as_u64()) {
            toml.push_str(&format!("{toml_key} = {v}\n"));
        }
    }

    // EOS token IDs → stop tokens
    if let Some(eos) = json.get("eos_token_id") {
        if let Some(arr) = eos.as_array() {
            let ids: Vec<String> = arr.iter().filter_map(|v| v.as_u64().map(|n| n.to_string())).collect();
            if !ids.is_empty() {
                toml.push_str(&format!("eos_token_ids = [{}]\n", ids.join(", ")));
            }
        } else if let Some(id) = eos.as_u64() {
            toml.push_str(&format!("eos_token_ids = [{}]\n", id));
        }
    }

    if toml.is_empty() {
        return;
    }

    if let Err(e) = std::fs::write(&dst, &toml) {
        result.errors.push(format!("write sampling.toml: {e}"));
        return;
    }
    result.configs_converted += 1;
}

fn create_vocab_toml(dir: &Path, result: &mut ImportResult) {
    let dst = dir.join("vocab.toml");
    if dst.exists() {
        return;
    }

    // Read tokenizer.json for metadata (we keep the .json for the runtime)
    let src = dir.join("tokenizer.json");
    if !src.exists() {
        // No tokenizer (e.g. BEATs, YOLO) — skip
        return;
    }

    let json_str = match std::fs::read_to_string(&src) {
        Ok(s) => s,
        Err(_) => return,
    };
    let json: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut toml = String::new();

    // Detect tokenizer type
    let model_type = json
        .get("model")
        .and_then(|m| m.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");
    toml.push_str(&format!("type = \"{}\"\n", model_type.to_lowercase()));

    // Vocab size from model.vocab
    if let Some(vocab) = json.get("model").and_then(|m| m.get("vocab")).and_then(|v| v.as_object()) {
        toml.push_str(&format!("vocab_size = {}\n", vocab.len()));
    }

    // Merges count
    if let Some(merges) = json.get("model").and_then(|m| m.get("merges")).and_then(|v| v.as_array()) {
        toml.push_str(&format!("merges_count = {}\n", merges.len()));
    }

    toml.push_str("# full vocabulary in tokenizer.json (kept for runtime)\n");

    if let Err(e) = std::fs::write(&dst, &toml) {
        result.errors.push(format!("write vocab.toml: {e}"));
        return;
    }
    result.configs_converted += 1;
}

// ── Weight selection ─────────────────────────────────────────────

fn select_weights(dir: &Path, result: &mut ImportResult) {
    // Priority: safetensors > gguf > onnx > pt > bin
    // Find what we have
    let entries: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .collect();

    let has_safetensors = entries.iter().any(|e| {
        let n = e.file_name();
        let n = n.to_str().unwrap_or("");
        n.ends_with(".safetensors") && !n.contains("index")
    });
    let has_gguf = entries.iter().any(|e| {
        e.path().extension().map(|x| x == "gguf").unwrap_or(false)
    });

    // Find the single best weights file and rename to weights.*
    // For multi-shard safetensors, skip rename (keep as-is per spec)
    let safetensors_files: Vec<_> = entries.iter().filter(|e| {
        let n = e.file_name();
        let n = n.to_str().unwrap_or("");
        n.ends_with(".safetensors") && !n.contains("index")
    }).collect();

    let gguf_files: Vec<_> = entries.iter().filter(|e| {
        e.path().extension().map(|x| x == "gguf").unwrap_or(false)
    }).collect();

    // Single safetensors → rename to weights.safetensors
    if safetensors_files.len() == 1 && !has_gguf {
        let src = safetensors_files[0].path();
        let dst = dir.join("weights.safetensors");
        if src != dst && !dst.exists() {
            if let Err(e) = std::fs::rename(&src, &dst) {
                result.errors.push(format!("rename weights: {e}"));
            }
        }
        result.weights_format = "safetensors".into();
    }
    // Single GGUF → rename to weights.gguf
    else if gguf_files.len() == 1 && !has_safetensors {
        let src = gguf_files[0].path();
        let dst = dir.join("weights.gguf");
        if src != dst && !dst.exists() {
            if let Err(e) = std::fs::rename(&src, &dst) {
                result.errors.push(format!("rename weights: {e}"));
            }
        }
        result.weights_format = "gguf".into();
    }
    // GGUF exists alongside safetensors → GGUF is quantized, prefer it
    else if gguf_files.len() == 1 && has_safetensors {
        // Rename GGUF
        let src = gguf_files[0].path();
        let dst = dir.join("weights.gguf");
        if src != dst && !dst.exists() {
            let _ = std::fs::rename(&src, &dst);
        }
        // Delete safetensors (they're the bloated originals)
        for sf in &safetensors_files {
            let size = sf.metadata().map(|m| m.len()).unwrap_or(0);
            if std::fs::remove_file(sf.path()).is_ok() {
                result.files_deleted += 1;
                result.downloaded_bytes += size;
            }
        }
        // Also delete safetensors index
        for e in &entries {
            let n = e.file_name();
            if n.to_str().unwrap_or("").contains(".safetensors.index.json") {
                let _ = std::fs::remove_file(e.path());
                result.files_deleted += 1;
            }
        }
        result.weights_format = "gguf".into();
    }
    // Multi-shard safetensors without GGUF — keep as-is
    else if safetensors_files.len() > 1 && !has_gguf {
        result.weights_format = format!("safetensors ({}x shards)", safetensors_files.len());
    }
    // ONNX models
    else {
        let has_onnx = entries.iter().any(|e| {
            e.path().extension().map(|x| x == "onnx").unwrap_or(false)
        });
        if has_onnx {
            result.weights_format = "onnx".into();
        }
        // PT models
        let has_pt = entries.iter().any(|e| {
            let ext = e.path().extension().map(|x| x.to_str().unwrap_or("").to_string()).unwrap_or_default();
            ext == "pt" || ext == "pth"
        });
        if has_pt && !has_onnx {
            result.weights_format = "pytorch".into();
        }
        // .bin (fasttext etc)
        let has_bin = entries.iter().any(|e| {
            let n = e.file_name();
            let n = n.to_str().unwrap_or("");
            (n == "model.bin" || n.starts_with("ggml")) && n.ends_with(".bin")
        });
        if has_bin && !has_pt && !has_onnx {
            result.weights_format = "bin".into();
        }
    }
}

// ── Junk cleanup ─────────────────────────────────────────────────

/// Files and dirs to delete from a canonical model directory
const JUNK_FILES: &[&str] = &[
    ".gitattributes",
    "LICENSE",
    "LICENSE.md",
    "USE_POLICY.md",
    "NOTICE",
    "README.md",
    "flax_model.msgpack",
    "tf_model.h5",
    "rust_model.ot",
];

const JUNK_DIRS: &[&str] = &[
    ".huggingface",
    ".cache",
    "__pycache__",
    ".git",
    "runs",
];

const JUNK_PATTERNS: &[&str] = &[
    "pytorch_model",     // pytorch_model.bin, pytorch_model-*.bin
    "TestPassed",        // abliteration test artifacts
    "added_tokens.json", // redundant with tokenizer.json
    "special_tokens_map.json", // redundant with chat.toml
];

fn clean_junk(dir: &Path, result: &mut ImportResult) {
    // Delete junk files
    let entries: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .collect();

    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_str().unwrap_or("");

        // Exact match junk files
        if JUNK_FILES.contains(&name_str) {
            if std::fs::remove_file(entry.path()).is_ok() {
                result.files_deleted += 1;
            }
            continue;
        }

        // Pattern match
        if JUNK_PATTERNS.iter().any(|p| name_str.contains(p)) {
            let meta = entry.metadata();
            if meta.map(|m| m.is_file()).unwrap_or(false) {
                if std::fs::remove_file(entry.path()).is_ok() {
                    result.files_deleted += 1;
                }
            }
            continue;
        }

        // Junk directories
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if JUNK_DIRS.contains(&name_str) {
                if std::fs::remove_dir_all(entry.path()).is_ok() {
                    result.files_deleted += 1;
                }
            }
        }
    }

    // Delete original JSON configs now that TOML versions exist
    let json_toml_pairs = [
        ("config.json", "config.toml"),
        ("tokenizer_config.json", "chat.toml"),
        ("generation_config.json", "sampling.toml"),
    ];
    for (json_name, toml_name) in json_toml_pairs {
        let json_path = dir.join(json_name);
        let toml_path = dir.join(toml_name);
        if json_path.exists() && toml_path.exists() {
            if std::fs::remove_file(&json_path).is_ok() {
                result.files_deleted += 1;
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────

/// Public wrapper for dir_size
pub fn dir_size_pub(path: &Path) -> u64 {
    dir_size(path)
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else {
                total += p.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}
