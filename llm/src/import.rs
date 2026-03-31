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

    // 1. Convert JSON configs → TOML (or create manually for non-HF models)
    convert_config_json(model_dir, &mut result);
    create_manual_configs(model_dir, &mut result);
    ensure_model_type(model_dir);
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

    // model_type — always first, used for runtime dispatch
    if let Some(v) = json.get("model_type").and_then(|v| v.as_str()) {
        toml.push_str(&format!("model_type = \"{v}\"\n"));
    }

    // architecture — full HF class name
    if let Some(v) = json.get("architectures").and_then(|a| a.as_array())
        .and_then(|a| a.first()).and_then(|v| v.as_str())
    {
        toml.push_str(&format!("architecture = \"{v}\"\n"));
    }

    // For VLM models, arch params are nested under text_config
    let params = json.get("text_config").unwrap_or(&json);

    // Core integer params
    let fields = [
        ("hidden_size", "hidden_size"),
        ("num_attention_heads", "num_attention_heads"),
        ("num_key_value_heads", "num_key_value_heads"),
        ("num_hidden_layers", "num_hidden_layers"),
        ("intermediate_size", "intermediate_size"),
        ("vocab_size", "vocab_size"),
        ("max_position_embeddings", "max_position_embeddings"),
        ("head_dim", "head_dim"),
        ("num_experts", "num_experts"),
        ("num_experts_per_tok", "num_experts_per_tok"),
        ("num_labels", "num_labels"),
    ];
    for (json_key, toml_key) in fields {
        if let Some(v) = params.get(json_key).and_then(|v| v.as_u64()) {
            toml.push_str(&format!("{toml_key} = {v}\n"));
        }
    }

    let float_fields = [
        ("rope_theta", "rope_theta"),
        ("rms_norm_eps", "rms_norm_eps"),
        ("layer_norm_eps", "layer_norm_eps"),
    ];
    for (json_key, toml_key) in float_fields {
        if let Some(v) = params.get(json_key).and_then(|v| v.as_f64()) {
            toml.push_str(&format!("{toml_key} = {v}\n"));
        }
    }

    let bool_fields = [
        ("tie_word_embeddings", "tie_word_embeddings"),
    ];
    for (json_key, toml_key) in bool_fields {
        if let Some(v) = params.get(json_key).and_then(|v| v.as_bool()) {
            toml.push_str(&format!("{toml_key} = {v}\n"));
        }
    }

    // Detect vision projector component
    if dir.join("mmproj-model-f16.gguf").exists() {
        toml.push_str("\n[components.vision]\nweights = \"mmproj-model-f16.gguf\"\nrole = \"vision-encoder\"\n");
    }

    if let Err(e) = std::fs::write(&dst, &toml) {
        result.errors.push(format!("write config.toml: {e}"));
        return;
    }
    result.configs_converted += 1;
}

/// Ensure config.toml has model_type field (patch existing files that lack it)
fn ensure_model_type(dir: &Path) {
    let path = dir.join("config.toml");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return,
    };
    if content.contains("model_type") {
        return;
    }
    // Infer model_type from architecture field
    let model_type = if let Some(arch_line) = content.lines().find(|l| l.starts_with("architecture")) {
        let arch = arch_line.split('"').nth(1).unwrap_or("");
        match arch {
            a if a.contains("Qwen3_5") => "qwen3_5_vl",
            a if a.contains("Qwen3") => "qwen3",
            a if a.contains("Qwen2_5_VL") || a.contains("Qwen2VL") => "qwen2_vl",
            a if a.contains("Qwen2") => "qwen2",
            a if a.contains("Llama") => "llama",
            a if a.contains("Phi3") => "phi3",
            a if a.contains("BitNet") => "bitnet",
            a if a.contains("MiMo") => "mimo",
            a if a.contains("Roberta") => "roberta",
            a if a.contains("Deberta") => "deberta-v2",
            a if a.contains("ModernBert") => "modernbert",
            a if a.contains("EuroBert") => "eurobert",
            a if a.contains("Moondream") || a.contains("moondream") => "moondream",
            _ => return, // can't infer, skip
        }
    } else {
        return;
    };

    let new_content = format!("model_type = \"{model_type}\"\n{content}");
    let _ = std::fs::write(&path, new_content);
}

/// Create config.toml for models without config.json (media, non-HF)
fn create_manual_configs(dir: &Path, result: &mut ImportResult) {
    let dst = dir.join("config.toml");
    if dst.exists() {
        return;
    }

    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let toml = match name {
        "glotlid" => r#"model_type = "fasttext"
num_languages = 2102
embedding_dim = 256
"#,
        "whisper-small" => r#"model_type = "whisper"
architecture = "WhisperForConditionalGeneration"
hidden_size = 768
num_attention_heads = 12
num_hidden_layers = 12
vocab_size = 51865
num_mels = 80
"#,
        "yolo11n" => r#"model_type = "yolo"
architecture = "YOLOv11"
variant = "nano"
num_classes = 80
input_size = 640
"#,
        "beats" => r#"model_type = "beats"
architecture = "BEATsAudioClassifier"
hidden_size = 768
num_attention_heads = 12
num_hidden_layers = 12
num_labels = 527
input_sample_rate = 16000
"#,
        "piper-tts" => {
            // Scan for voice ONNX files
            let mut toml = String::from("model_type = \"vits\"\narchitecture = \"VitsModel\"\nsample_rate = 22050\n\n");
            if let Ok(scan) = scan_piper_voices(dir) {
                toml.push_str(&scan);
            }
            return write_config(&dst, &toml, result);
        }
        "xtts-v2" => r#"model_type = "xtts"
architecture = "XttsModel"
gpt_layers = 30
gpt_n_model_channels = 1024
gpt_n_heads = 16
num_audio_tokens = 1026
d_vector_dim = 512
output_sample_rate = 24000
languages = ["en", "es", "fr", "de", "it", "pt", "pl", "tr", "ru", "nl", "cs", "ar", "zh-cn", "hu", "ko", "ja", "hi"]

[components.gpt]
weights = "model.pth"
role = "autoregressive-decoder"

[components.dvae]
weights = "dvae.pth"
role = "discrete-vae"

[components.mel_stats]
weights = "mel_stats.pth"
role = "mel-statistics"

[components.speakers]
weights = "speakers_xtts.pth"
role = "speaker-embeddings"
"#,
        "wan22-video" => {
            // Find VAE safetensors
            let vae_file = find_vae_file(dir);
            let vae_path = vae_file.as_deref().unwrap_or("VAE/Wan2.2_VAE.safetensors");
            let toml = format!(
                "model_type = \"wan\"\narchitecture = \"Wan2.2-TI2V\"\ntask = \"text-image-to-video\"\n\n\
                 [components.transformer]\nweights = \"weights.gguf\"\nrole = \"diffusion-transformer\"\n\n\
                 [components.vae]\nweights = \"{vae_path}\"\nrole = \"vae-decoder\"\n"
            );
            return write_config(&dst, &toml, result);
        }
        _ => return,
    };

    write_config(&dst, toml, result);
}

fn write_config(dst: &Path, content: &str, result: &mut ImportResult) {
    if let Err(e) = std::fs::write(dst, content) {
        result.errors.push(format!("write config.toml: {e}"));
    } else {
        result.configs_converted += 1;
    }
}

fn scan_piper_voices(dir: &Path) -> std::io::Result<String> {
    let mut voices = String::new();
    fn visit(dir: &Path, base: &Path, voices: &mut String) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, base, voices);
                } else if path.extension().map(|e| e == "onnx").unwrap_or(false) {
                    let rel = path.strip_prefix(base).unwrap_or(&path);
                    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
                    // Extract language from name (e.g. "en_US-amy-low" → "en")
                    let lang = name.split('-').next().and_then(|s| s.split('_').next()).unwrap_or("unknown");
                    voices.push_str(&format!(
                        "[[voices]]\nname = \"{name}\"\nlanguage = \"{lang}\"\nweights = \"{}\"\n\n",
                        rel.display()
                    ));
                }
            }
        }
    }
    visit(dir, dir, &mut voices);
    Ok(voices)
}

fn find_vae_file(dir: &Path) -> Option<String> {
    let vae_dir = dir.join("VAE");
    if !vae_dir.exists() {
        return None;
    }
    if let Ok(entries) = std::fs::read_dir(&vae_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_str().unwrap_or("");
            if name.ends_with(".safetensors") {
                return Some(format!("VAE/{name}"));
            }
        }
    }
    None
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
    // GGUF without safetensors → rename largest to weights.gguf, delete ONNX dupes
    else if !gguf_files.is_empty() && !has_safetensors {
        // Find the largest GGUF (main model weights)
        let mut largest = &gguf_files[0];
        let mut largest_size = 0u64;
        for gf in &gguf_files {
            let sz = gf.metadata().map(|m| m.len()).unwrap_or(0);
            if sz > largest_size {
                largest_size = sz;
                largest = gf;
            }
        }
        let src = largest.path();
        let dst = dir.join("weights.gguf");
        if src != dst && !dst.exists() {
            if let Err(e) = std::fs::rename(&src, &dst) {
                result.errors.push(format!("rename weights: {e}"));
            }
        }
        delete_onnx_dirs(dir, &entries, result);
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
    // GGUF exists alongside ONNX (multiple GGUF or other combos) → keep GGUF, delete ONNX
    else if has_gguf {
        if gguf_files.len() == 1 {
            let src = gguf_files[0].path();
            let dst = dir.join("weights.gguf");
            if src != dst && !dst.exists() {
                let _ = std::fs::rename(&src, &dst);
            }
        }
        delete_onnx_dirs(dir, &entries, result);
        result.weights_format = "gguf".into();
    }
    // Pure ONNX models (no safetensors, no GGUF)
    else {
        let has_onnx = entries.iter().any(|e| {
            e.path().extension().map(|x| x == "onnx").unwrap_or(false)
                || (e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                    && e.file_name().to_str().unwrap_or("").starts_with("onnx"))
        });
        if has_onnx {
            result.weights_format = "onnx".into();
        }
        // PT/PTH → rename single .pt to weights.pt
        let pt_files: Vec<_> = entries.iter().filter(|e| {
            let ext = e.path().extension().map(|x| x.to_str().unwrap_or("").to_string()).unwrap_or_default();
            (ext == "pt" || ext == "pth") && e.metadata().map(|m| m.is_file()).unwrap_or(false)
        }).collect();
        if pt_files.len() == 1 && !has_onnx {
            let src = pt_files[0].path();
            let ext = src.extension().unwrap_or_default().to_str().unwrap_or("pt");
            let dst = dir.join(format!("weights.{ext}"));
            if src != dst && !dst.exists() {
                let _ = std::fs::rename(&src, &dst);
            }
            result.weights_format = "pytorch".into();
        } else if !pt_files.is_empty() && !has_onnx {
            result.weights_format = "pytorch".into();
        }
        // .bin (fasttext, GGML) → rename to weights.bin
        let bin_files: Vec<_> = entries.iter().filter(|e| {
            let n = e.file_name();
            let n = n.to_str().unwrap_or("");
            e.metadata().map(|m| m.is_file()).unwrap_or(false)
                && n.ends_with(".bin")
                && (n == "model.bin" || n.starts_with("ggml"))
        }).collect();
        if bin_files.len() == 1 && pt_files.is_empty() && !has_onnx {
            let src = bin_files[0].path();
            let dst = dir.join("weights.bin");
            if src != dst && !dst.exists() {
                let _ = std::fs::rename(&src, &dst);
            }
            result.weights_format = "bin".into();
        }
    }
}

/// Delete ONNX directories when a higher-priority format (GGUF/safetensors) exists
fn delete_onnx_dirs(dir: &Path, entries: &[std::fs::DirEntry], result: &mut ImportResult) {
    for entry in entries {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let name = entry.file_name();
            let name = name.to_str().unwrap_or("");
            if name.starts_with("onnx") {
                let size = dir_size(&entry.path());
                if std::fs::remove_dir_all(entry.path()).is_ok() {
                    result.files_deleted += 1;
                    result.downloaded_bytes += size;
                }
            }
        }
    }
}

// ── Junk cleanup ─────────────────────────────────────────────────

/// Files and dirs to delete from a canonical model directory
const JUNK_FILES: &[&str] = &[
    ".gitattributes",
    ".DS_Store",
    "LICENSE",
    "LICENSE.md",
    "USE_POLICY.md",
    "NOTICE",
    "README.md",
    "flax_model.msgpack",
    "tf_model.h5",
    "rust_model.ot",
    // Redundant with tokenizer.json
    "merges.txt",
    "vocab.json",
    "added_tokens.json",
    "special_tokens_map.json",
    // Redundant with chat.toml
    "chat_template.jinja",
    "chat_template.json",
    // Redundant with config.toml
    "preprocessor_config.json",
    "video_preprocessor_config.json",
    "processor_config.json",
    // Misc
    "requirements.txt",
    "versions.txt",
];

const JUNK_DIRS: &[&str] = &[
    ".huggingface",
    ".cache",
    "__pycache__",
    ".git",
    "runs",
];

const JUNK_PATTERNS: &[&str] = &[
    "pytorch_model",       // pytorch_model.bin, pytorch_model-*.bin
    "TestPassed",          // abliteration test artifacts
];

/// File extensions that are always junk in a model directory
const JUNK_EXTENSIONS: &[&str] = &[
    "py",    // handler.py, modeling_*.py, configuration_*.py, etc.
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
            if entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                if std::fs::remove_file(entry.path()).is_ok() {
                    result.files_deleted += 1;
                }
            }
            continue;
        }

        // Extension match (.py files etc.)
        if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
            if JUNK_EXTENSIONS.contains(&ext) {
                if entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                    if std::fs::remove_file(entry.path()).is_ok() {
                        result.files_deleted += 1;
                    }
                    continue;
                }
            }
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

    // Delete original JSON configs: either TOML replacement exists,
    // or the JSON was empty/stub (no useful data extracted)
    let json_toml_pairs = [
        ("config.json", "config.toml"),
        ("tokenizer_config.json", "chat.toml"),
        ("generation_config.json", "sampling.toml"),
    ];
    for (json_name, toml_name) in json_toml_pairs {
        let json_path = dir.join(json_name);
        let toml_path = dir.join(toml_name);
        if json_path.exists() {
            // Delete if TOML exists, OR if this is generation_config without
            // useful params (just _from_model_config + transformers_version)
            let should_delete = toml_path.exists() || {
                json_name == "generation_config.json" && {
                    std::fs::read_to_string(&json_path)
                        .map(|s| !s.contains("temperature") && !s.contains("top_p"))
                        .unwrap_or(false)
                }
            };
            if should_delete {
                if std::fs::remove_file(&json_path).is_ok() {
                    result.files_deleted += 1;
                }
            }
        }
    }

    // Delete duplicate onnx/ dir when onnx_q8/ exists (keep only quantized)
    let has_onnx_q8 = dir.join("onnx_q8").exists();
    let has_onnx = dir.join("onnx").exists();
    if has_onnx && has_onnx_q8 {
        if std::fs::remove_dir_all(dir.join("onnx")).is_ok() {
            result.files_deleted += 1;
        }
    }

    // Clean junk inside onnx/ and onnx_q8/ dirs — keep only .onnx + .onnx_data
    for onnx_dir_name in &["onnx", "onnx_q8"] {
        let onnx_dir = dir.join(onnx_dir_name);
        if !onnx_dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&onnx_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_str().unwrap_or("");
                let is_weight = name.ends_with(".onnx")
                    || name.ends_with(".onnx_data");
                if !is_weight && entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                    if std::fs::remove_file(entry.path()).is_ok() {
                        result.files_deleted += 1;
                    }
                }
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
