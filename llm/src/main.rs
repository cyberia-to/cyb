//! cyb-llm CLI — run LLMs on wgpu

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
    /// Run inference on a model (HuggingFace ID or local path)
    Run {
        /// Model ID (e.g. onnx-community/Qwen3-0.6B-ONNX) or local path to .safetensors/.onnx
        model: String,

        /// Prompt for text generation
        #[arg(short, long, default_value = "Hello, world!")]
        prompt: String,

        /// Maximum tokens to generate
        #[arg(short = 'n', long, default_value_t = 128)]
        max_tokens: usize,

        /// Temperature for sampling (0 = greedy)
        #[arg(short, long, default_value_t = 0.7)]
        temperature: f32,
    },

    /// List cached models
    List,

    /// Show model info for a local file
    Info {
        /// Path to local model file (ONNX, safetensors, GGUF)
        path: String,
    },

    /// Download a model from HuggingFace
    Download {
        /// Model ID on HuggingFace
        model: String,
    },

    /// Compute embeddings from a BERT/encoder model
    Embed {
        /// Path to model file (.safetensors)
        model: String,

        /// Text to encode
        #[arg(short, long, default_value = "Hello, world!")]
        text: String,
    },

    /// Transcribe audio with a Whisper model (audio -> text)
    Transcribe {
        /// Path to GGML whisper model (.bin)
        model: String,

        /// Path to input audio file (16-bit PCM WAV)
        #[arg(short, long)]
        audio: String,
    },

    /// Audit local models against soma manifest
    Audit,

    /// Clean junk files, duplicates, incomplete downloads
    Clean {
        /// Actually delete (default: dry run)
        #[arg(long)]
        execute: bool,
    },

    /// Convert safetensors to quantized GGUF (Q4/Q8/F16)
    Convert {
        /// Model name (from manifest) or "all" for all bloated models
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

    /// Import: canonicalize model directory (JSON→TOML, rename weights, clean junk)
    Import {
        /// Model name or "all"
        name: String,
    },

    /// Pack model into .cyb format (config + weights + optional graph in one file)
    Pack {
        /// Model name or "all"
        name: String,
    },

    /// Extract tensors.toml from .cyb files
    ExtractTensors {
        /// Model name or "all"
        name: String,
    },

    /// Pack all 7 files into .model format (new spec)
    ModelPack {
        /// Model name or "all"
        name: String,
    },

    /// Full status report: formats, sizes, loadability, config completeness
    Status,

    /// Fetch missing or incomplete models from HuggingFace
    Fetch {
        /// Only fetch this model (by name)
        #[arg(short, long)]
        name: Option<String>,

        /// Only fetch models in this tier (tier0, tier1, tier2, media)
        #[arg(short, long)]
        tier: Option<String>,
    },

    /// Start OpenAI-compatible HTTP server
    Serve {
        /// Model to serve (name from manifest or path to .model file)
        #[arg(default_value = "qwen3-0.6b-abl")]
        model: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            model,
            prompt,
            max_tokens,
            temperature,
        } => {
            // Resolve: .model file or directory containing one
            let model_path = std::path::PathBuf::from(&model);
            let model_file = if ext_is(&model_path, "model") && model_path.exists() {
                model_path.clone()
            } else if model_path.is_dir() {
                match std::fs::read_dir(&model_path).into_iter().flatten().flatten()
                    .find(|e| ext_is(&e.path(), "model")).map(|e| e.path()) {
                    Some(p) => p,
                    None => { eprintln!("No .model file in {}", model_path.display()); return; }
                }
            } else {
                eprintln!("Expected .model file or directory. Got: {model}");
                return;
            };

            let use_metal = match cli.backend.as_str() {
                "metal" => true,
                "wgpu" => false,
                _ => cfg!(target_os = "macos"), // auto: Metal on macOS
            };

            println!("Loading: {} (backend: {})", model_file.display(),
                if use_metal { "metal" } else { "wgpu" });

            #[cfg(target_os = "macos")]
            if use_metal {
                // Read config for chat template
                let config_toml = cyb_llm::cyb_format::read_model_config(&model_file)
                    .map(|(_, cfg)| cfg).unwrap_or_default();
                let formatted = cyb_llm::generate::apply_chat_template(&prompt, &config_toml);

                run_metal_model(&model_file, &formatted, max_tokens, temperature);
                return;
            }

            #[cfg(not(target_os = "macos"))]
            if use_metal {
                eprintln!("Metal backend is only available on macOS");
                return;
            }

            let backend = cyb_llm::backend::create_wgpu_backend();
            let load_start = std::time::Instant::now();
            let mut generator = match cyb_llm::generate::TextGenerator::new(
                &model_file, backend.pipelines,
            ) {
                Ok(g) => g,
                Err(e) => { eprintln!("Load failed: {e}"); return; }
            };
            println!("Loaded in {:.1}s", load_start.elapsed().as_secs_f64());

            let config_toml = cyb_llm::cyb_format::read_model_config(&model_file)
                .map(|(_, cfg)| cfg).unwrap_or_default();
            let formatted = cyb_llm::generate::apply_chat_template(&prompt, &config_toml);
            println!("---");
            match generator.generate(&formatted, max_tokens, temperature) {
                Ok(_) => {}
                Err(e) => eprintln!("\nGeneration failed: {e}"),
            }
        }

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

        Commands::Info { path } => {
            let p = std::path::Path::new(&path);
            match cyb_llm::loader::detect_format(p) {
                Ok(fmt) => {
                    println!("Format: {fmt:?}");
                    match cyb_llm::loader::load_model(p) {
                        Ok(graph) => {
                            println!("Nodes: {}", graph.nodes.len());
                            println!("Weights: {}", graph.weights.len());
                            for (name, w) in graph.weights.iter().take(10) {
                                println!("  {}: {:?} {:?} ({}B)", name, w.shape, w.dtype, w.data.len());
                            }
                        }
                        Err(e) => eprintln!("Load error: {e}"),
                    }
                }
                Err(e) => eprintln!("Format detect error: {e}"),
            }
        },

        Commands::Download { model } => {
            println!("Downloading {model}...");
            match cyb_llm::hub::download_model(&model) {
                Ok(path) => println!("Downloaded to: {}", path.display()),
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        Commands::Embed { model, text } => {
            run_embed(&model, &text);
        }

        Commands::Transcribe { model, audio } => {
            run_transcribe(&model, &audio);
        }

        Commands::Audit => {
            run_audit();
        }

        Commands::Status => {
            run_status();
        }

        Commands::Clean { execute } => {
            run_clean(!execute);
        }

        Commands::Fetch { name, tier } => {
            run_fetch(name, tier);
        }

        Commands::Serve { model, port } => {
            eprintln!("serve: coming soon (model={model}, port={port})");
        }

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

/// Run BERT embedding via GraphModel
fn run_embed(model_path: &str, text: &str) {
    use cyb_llm::ir::templates::{bert_encoder, modernbert_encoder, BertConfig};
    use cyb_llm::backend::wgpu::graph_model::{GraphModel, GraphModelConfig, Architecture};

    let path = std::path::Path::new(model_path);
    let model_dir = path.parent().unwrap_or(std::path::Path::new("."));

    // Read config from config.json or config.toml
    let config_json_path = model_dir.join("config.json");
    let config_toml_path = model_dir.join("config.toml");
    let bert_config = if config_json_path.exists() || config_toml_path.exists() {
        let config_json: serde_json::Value = if config_json_path.exists() {
            let s = std::fs::read_to_string(&config_json_path).expect("Cannot read config.json");
            serde_json::from_str(&s).expect("Invalid config.json")
        } else {
            let s = std::fs::read_to_string(&config_toml_path).expect("Cannot read config.toml");
            let tv: toml::Value = toml::from_str(&s).expect("Invalid config.toml");
            cyb_llm::cyb_format::toml_to_json_value(&tv)
        };

        // Auto-detect weight prefix from model_type
        let model_type = config_json["model_type"].as_str().unwrap_or("");
        let weight_prefix = match model_type {
            "roberta" | "xlm-roberta" => "roberta".to_string(),
            "deberta" | "deberta-v2" => "deberta".to_string(),
            "bert" => "bert".to_string(),
            _ => String::new(),  // ModernBERT and others: try no prefix
        };

        BertConfig {
            hidden_size: config_json["hidden_size"].as_u64().unwrap_or(768) as usize,
            num_heads: config_json["num_attention_heads"].as_u64().unwrap_or(12) as usize,
            head_dim: config_json["hidden_size"].as_u64().unwrap_or(768) as usize
                / config_json["num_attention_heads"].as_u64().unwrap_or(12) as usize,
            num_layers: config_json["num_hidden_layers"].as_u64().unwrap_or(12) as usize,
            intermediate_size: config_json["intermediate_size"].as_u64().unwrap_or(3072) as usize,
            vocab_size: config_json["vocab_size"].as_u64().unwrap_or(30522) as usize,
            max_position_embeddings: config_json["max_position_embeddings"].as_u64().unwrap_or(512) as usize,
            type_vocab_size: config_json["type_vocab_size"].as_u64().unwrap_or(2) as usize,
            eps: config_json["layer_norm_eps"].as_f64().unwrap_or(1e-12) as f32,
            weight_prefix,
            num_labels: config_json.get("num_labels").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        }
    } else {
        println!("No config.json found, using BERT-base defaults");
        BertConfig::default()
    };

    println!("BERT config: hidden={}, heads={}, layers={}, vocab={}",
        bert_config.hidden_size, bert_config.num_heads, bert_config.num_layers, bert_config.vocab_size);

    // Load weights
    println!("Loading weights from {}...", path.display());
    let mut graph_with_weights = cyb_llm::loader::load_model(path).expect("Failed to load model");

    // Detect model type from config
    let model_type = {
        let jp = model_dir.join("config.json");
        let tp = model_dir.join("config.toml");
        if jp.exists() {
            let s = std::fs::read_to_string(&jp).unwrap_or_default();
            let j: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
            j["model_type"].as_str().unwrap_or("").to_string()
        } else if tp.exists() {
            let s = std::fs::read_to_string(&tp).unwrap_or_default();
            let tv: toml::Value = toml::from_str(&s).unwrap_or(toml::Value::Table(Default::default()));
            let j = cyb_llm::cyb_format::toml_to_json_value(&tv);
            j["model_type"].as_str().unwrap_or("").to_string()
        } else { String::new() }
    };

    let is_modernbert = model_type == "modernbert";

    // For ModernBERT: split fused QKV and Wi weights into separate Q/K/V and gate/up
    if is_modernbert {
        use cyb_llm::ir::{DType, WeightData};
        let hidden = bert_config.hidden_size;
        let _inter = bert_config.intermediate_size;
        let keys: Vec<String> = graph_with_weights.weights.keys().cloned().collect();
        let qkv_keys: Vec<String> = keys.iter().filter(|k| k.ends_with(".Wqkv.weight") || k.ends_with(".attn.Wqkv.weight")).cloned().collect();
        let wi_keys: Vec<String> = keys.iter().filter(|k| k.ends_with(".mlp.Wi.weight")).cloned().collect();

        for key in qkv_keys {
            if let Some(w) = graph_with_weights.weights.remove(&key) {
                let elem_size = w.dtype.element_size();
                let _row_bytes = hidden * elem_size;
                let prefix = key.strip_suffix(".Wqkv.weight").unwrap().to_string();
                // Dequant to f32 if needed, then split
                let f32_data = cyb_llm::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
                let total_out = f32_data.len() / hidden; // total output neurons
                let per_split = total_out / 3;
                let _f32_row_bytes = hidden * 4; // f32
                for (idx, name) in ["q", "k", "v"].iter().enumerate() {
                    let start = idx * per_split * hidden;
                    let end = start + per_split * hidden;
                    if end <= f32_data.len() {
                        let split_bytes: Vec<u8> = bytemuck::cast_slice(&f32_data[start..end]).to_vec();
                        graph_with_weights.weights.insert(
                            format!("{prefix}.{name}.weight"),
                            WeightData { data: split_bytes, shape: vec![per_split, hidden], dtype: DType::F32, needs_transpose: false },
                        );
                    }
                }
            }
        }
        for key in wi_keys {
            if let Some(w) = graph_with_weights.weights.remove(&key) {
                let prefix = key.strip_suffix(".Wi.weight").unwrap().to_string();
                let f32_data = cyb_llm::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
                let total_out = f32_data.len() / hidden;
                let half_n = total_out / 2;
                let half = half_n * hidden;
                if half * 2 <= f32_data.len() {
                    graph_with_weights.weights.insert(
                        format!("{prefix}.Wi_gate.weight"),
                        WeightData { data: bytemuck::cast_slice(&f32_data[..half]).to_vec(), shape: vec![half_n, hidden], dtype: DType::F32, needs_transpose: false },
                    );
                    graph_with_weights.weights.insert(
                        format!("{prefix}.Wi_up.weight"),
                        WeightData { data: bytemuck::cast_slice(&f32_data[half..half*2]).to_vec(), shape: vec![half_n, hidden], dtype: DType::F32, needs_transpose: false },
                    );
                }
            }
        }
        // ModernBERT: add missing norm weights (ones) and biases (zeros)
        for i in 0..bert_config.num_layers {
            for suffix in &["attn_norm.weight", "mlp_norm.weight"] {
                let name = format!("model.layers.{i}.{suffix}");
                if !graph_with_weights.weights.contains_key(&name) {
                    // Identity LayerNorm: weight = all ones
                    let ones: Vec<u8> = (0..hidden).flat_map(|_| 1.0f32.to_le_bytes()).collect();
                    graph_with_weights.weights.insert(name, WeightData {
                        data: ones,
                        shape: vec![hidden],
                        dtype: DType::F32, needs_transpose: false,
                    });
                }
            }
            for suffix in &["attn_norm.bias", "mlp_norm.bias"] {
                let name = format!("model.layers.{i}.{suffix}");
                if !graph_with_weights.weights.contains_key(&name) {
                    graph_with_weights.weights.insert(name, WeightData {
                        data: vec![0u8; hidden * 4],
                        shape: vec![hidden],
                        dtype: DType::F32, needs_transpose: false,
                    });
                }
            }
        }
        // Embed norm bias
        if !graph_with_weights.weights.contains_key("model.embeddings.norm.bias") {
            graph_with_weights.weights.insert("model.embeddings.norm.bias".to_string(), WeightData {
                data: vec![0u8; hidden * 4],
                shape: vec![hidden],
                dtype: DType::F32, needs_transpose: false,
            });
        }
    }

    // For DeBERTa: rename *_proj.weight → *.weight to match BERT template
    let is_deberta = model_type.starts_with("deberta");
    if is_deberta {
        let rename_keys: Vec<String> = graph_with_weights.weights.keys()
            .filter(|k| k.contains("_proj.weight") || k.contains("_proj.bias"))
            .cloned().collect();
        for key in rename_keys {
            let new_key = key.replace("query_proj", "query")
                .replace("key_proj", "key")
                .replace("value_proj", "value");
            if new_key != key {
                if let Some(w) = graph_with_weights.weights.remove(&key) {
                    graph_with_weights.weights.insert(new_key, w);
                }
            }
        }
    }

    // For DeBERTa: add zero position and token_type embeddings (DeBERTa uses relative position embeddings instead)
    if is_deberta {
        use cyb_llm::ir::{DType, WeightData};
        let hidden = bert_config.hidden_size;
        let max_pos = bert_config.max_position_embeddings;
        let wp = format!("{}.", bert_config.weight_prefix);
        let pos_name = format!("{wp}embeddings.position_embeddings.weight");
        if !graph_with_weights.weights.contains_key(&pos_name) {
            graph_with_weights.weights.insert(pos_name, WeightData {
                data: vec![0u8; max_pos * hidden * 2],  // F16 zeros
                shape: vec![max_pos, hidden],
                dtype: DType::F16, needs_transpose: false,
            });
        }
        let type_name = format!("{wp}embeddings.token_type_embeddings.weight");
        if !graph_with_weights.weights.contains_key(&type_name) {
            graph_with_weights.weights.insert(type_name, WeightData {
                data: vec![0u8; 2 * hidden * 2],  // F16 zeros, 2 types
                shape: vec![2, hidden],
                dtype: DType::F16, needs_transpose: false,
            });
        }
    }

    // EuroBERT (jina): remap weight names to BERT convention and add missing weights
    let is_eurobert = model_type == "eurobert";
    if is_eurobert {
        use cyb_llm::ir::{DType, WeightData};
        let wp = if bert_config.weight_prefix.is_empty() { "".to_string() } else { format!("{}.", bert_config.weight_prefix) };

        // EuroBERT has no position embeddings (uses RoPE) — add zeros
        let pos_name = format!("{wp}embeddings.position_embeddings.weight");
        if !graph_with_weights.weights.contains_key(&pos_name) {
            graph_with_weights.weights.insert(pos_name, WeightData {
                data: vec![0u8; bert_config.max_position_embeddings * bert_config.hidden_size * 4],
                shape: vec![bert_config.max_position_embeddings, bert_config.hidden_size],
                dtype: DType::F32, needs_transpose: false,
            });
        }

        // EuroBERT uses RMSNorm (no bias) but BERT template expects LayerNorm (weight + bias)
        // Add zero biases for all LayerNorm nodes
        for i in 0..bert_config.num_layers {
            let renames = [
                (format!("layers.{i}.input_layernorm.weight"), format!("{wp}encoder.layer.{i}.attention.output.LayerNorm.weight")),
                (format!("layers.{i}.post_attention_layernorm.weight"), format!("{wp}encoder.layer.{i}.output.LayerNorm.weight")),
                (format!("layers.{i}.self_attn.q_proj.weight"), format!("{wp}encoder.layer.{i}.attention.self.query.weight")),
                (format!("layers.{i}.self_attn.k_proj.weight"), format!("{wp}encoder.layer.{i}.attention.self.key.weight")),
                (format!("layers.{i}.self_attn.v_proj.weight"), format!("{wp}encoder.layer.{i}.attention.self.value.weight")),
                (format!("layers.{i}.self_attn.o_proj.weight"), format!("{wp}encoder.layer.{i}.attention.output.dense.weight")),
                (format!("layers.{i}.mlp.gate_proj.weight"), format!("{wp}encoder.layer.{i}.intermediate.dense.weight")),
                (format!("layers.{i}.mlp.down_proj.weight"), format!("{wp}encoder.layer.{i}.output.dense.weight")),
            ];
            for (from, to) in renames {
                if let Some(w) = graph_with_weights.weights.remove(&from) {
                    graph_with_weights.weights.insert(to, w);
                }
            }
            // Add zero biases for LayerNorm and projections
            for suffix in &[
                "attention.output.LayerNorm.bias", "output.LayerNorm.bias",
                "attention.self.query.bias", "attention.self.key.bias",
                "attention.self.value.bias", "attention.output.dense.bias",
                "intermediate.dense.bias", "output.dense.bias",
            ] {
                let name = format!("{wp}encoder.layer.{i}.{suffix}");
                if !graph_with_weights.weights.contains_key(&name) {
                    let dim = if suffix.contains("intermediate") { bert_config.intermediate_size } else { bert_config.hidden_size };
                    graph_with_weights.weights.insert(name, WeightData {
                        data: vec![0u8; dim * 4], shape: vec![dim], dtype: DType::F32, needs_transpose: false,
                    });
                }
            }
        }

        // Rename embeddings
        if let Some(w) = graph_with_weights.weights.remove("embeddings.word_embeddings.weight") {
            graph_with_weights.weights.insert(format!("{wp}embeddings.word_embeddings.weight"), w);
        }
        // Add embedding LayerNorm (zeros for RMSNorm compat)
        for suffix in &["embeddings.LayerNorm.weight", "embeddings.LayerNorm.bias"] {
            let name = format!("{wp}{suffix}");
            if !graph_with_weights.weights.contains_key(&name) {
                let h = bert_config.hidden_size;
                let data = if suffix.contains("bias") { vec![0u8; h * 4] }
                    else { (0..h).flat_map(|_| 1.0f32.to_le_bytes()).collect() };
                graph_with_weights.weights.insert(name, WeightData {
                    data, shape: vec![h], dtype: DType::F32, needs_transpose: false,
                });
            }
        }
        log::info!("EuroBERT: remapped {} layers to BERT convention", bert_config.num_layers);
    }

    // Build graph from template
    let graph = if is_modernbert {
        modernbert_encoder(&bert_config)
    } else {
        bert_encoder(&bert_config)
    };
    println!("Graph template: {} nodes", graph.len());

    // Build tokenizer from .model embedded vocab
    let tokenizer = cyb_llm::cyb_format::build_tokenizer(path)
        .expect("Cannot build tokenizer from .model vocab");

    // Tokenize input — add special tokens (BOS/EOS) only when the model expects them
    // ModernBERT and some encoders don't need special tokens and produce NaN with them
    let add_special = !is_modernbert && model_type != "eurobert";
    let encoding = tokenizer.encode(text, add_special).expect("Tokenization failed");
    let attention_mask = encoding.get_attention_mask();
    let input_ids: Vec<u32> = encoding.get_ids().iter().zip(attention_mask.iter())
        .filter(|&(_, &mask)| mask == 1)
        .map(|(&id, _)| id)
        .collect();
    println!("Input tokens ({} tokens): {:?}", input_ids.len(), input_ids);

    // Initialize GPU
    let backend = cyb_llm::backend::create_wgpu_backend();
    let pipelines = backend.pipelines;

    let gm_config = GraphModelConfig {
        hidden_size: bert_config.hidden_size as u32,
        num_heads: bert_config.num_heads as u32,
        kv_num_heads: bert_config.num_heads as u32,
        head_dim: bert_config.head_dim as u32,
        vocab_size: bert_config.vocab_size as u32,
        num_layers: bert_config.num_layers as u32,
        block_size: 32,
        rope_theta: 0.0,  // BERT doesn't use RoPE
        max_seq_len: bert_config.max_position_embeddings as u32,
        has_qk_norm: false,
    };

    let load_start = std::time::Instant::now();
    let model = GraphModel::new(
        graph,
        &graph_with_weights.weights,
        pipelines,
        Architecture::Encoder,
        gm_config,
    ).expect("Failed to create GraphModel");
    println!("Model loaded in {:.1}s", load_start.elapsed().as_secs_f64());

    // Run embedding — pick output tensor based on model type
    let wp_str = if bert_config.weight_prefix.is_empty() { String::new() } else { format!("{}.", bert_config.weight_prefix) };
    let has_pooler = graph_with_weights.weights.contains_key(&format!("{wp_str}pooler.dense.weight"));
    let (output_name, output_size) = if bert_config.num_labels > 0 {
        println!("Classifier: {} labels", bert_config.num_labels);
        ("logits", bert_config.num_labels)
    } else if is_modernbert {
        let out = format!("layer_{}.residual2", bert_config.num_layers - 1);
        println!("ModernBERT: using last residual: {}", out);
        (out.leak() as &str, bert_config.hidden_size * input_ids.len())
    } else if has_pooler {
        ("pooler_output", bert_config.hidden_size)
    } else {
        let last_layer = format!("layer_{}.output_ln", bert_config.num_layers - 1);
        println!("No pooler weights, using last hidden state: {}", last_layer);
        (last_layer.leak() as &str, bert_config.hidden_size * input_ids.len())
    };

    let embed_start = std::time::Instant::now();
    let output_raw = model.encode(&input_ids, output_name, output_size)
        .expect("Encoding failed");
    let elapsed = embed_start.elapsed().as_secs_f64();

    if bert_config.num_labels > 0 {
        // Classifier output: logits → softmax → label
        println!("---");
        println!("Classification ({} labels), computed in {:.3}s:", bert_config.num_labels, elapsed);
        // Softmax
        let max_val = output_raw.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exps: Vec<f32> = output_raw.iter().map(|v| (v - max_val).exp()).collect();
        let sum: f32 = exps.iter().sum();
        let probs: Vec<f32> = exps.iter().map(|v| v / sum).collect();
        for (i, prob) in probs.iter().enumerate() {
            println!("  label {}: {:.4} ({:.1}%)", i, output_raw[i], prob * 100.0);
        }
        let predicted = probs.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap().0;
        println!("  → predicted: label {} ({:.1}%)", predicted, probs[predicted] * 100.0);
    } else {
        // Embedding output
        let output: Vec<f32> = output_raw[..bert_config.hidden_size.min(output_raw.len())].to_vec();
        println!("---");
        println!("Pooler output ({} dims), computed in {:.3}s:", output.len(), elapsed);
        let preview: Vec<String> = output.iter().take(10).map(|v| format!("{:.4}", v)).collect();
        println!("  [{}{}]", preview.join(", "),
            if output.len() > 10 { ", ..." } else { "" });
    }
}

/// Run Whisper transcription: audio -> mel -> encoder -> decoder -> text
fn run_transcribe(model_path: &str, audio_path: &str) {
    use cyb_llm::transcribe::Transcriber;

    let path = std::path::Path::new(model_path);
    let audio = std::path::Path::new(audio_path);

    // Validate inputs
    if !model_path.ends_with(".bin") {
        eprintln!("Whisper transcription requires GGML (.bin) model format");
        return;
    }
    if !audio.exists() {
        eprintln!("Audio file not found: {}", audio.display());
        return;
    }

    // Initialize GPU
    let backend = cyb_llm::backend::create_wgpu_backend();
    let pipelines = backend.pipelines;

    // Load model
    println!("Loading Whisper model from {}...", path.display());
    let load_start = std::time::Instant::now();
    let mut transcriber = match Transcriber::new(path, pipelines) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Model load failed: {e}");
            return;
        }
    };
    println!("Model loaded in {:.1}s", load_start.elapsed().as_secs_f64());

    // Transcribe
    println!("Transcribing {}...", audio.display());
    let transcribe_start = std::time::Instant::now();
    match transcriber.transcribe(audio) {
        Ok(text) => {
            let elapsed = transcribe_start.elapsed().as_secs_f64();
            println!("---");
            println!("{}", text);
            println!("---");
            println!("Transcribed in {:.1}s", elapsed);
        }
        Err(e) => {
            eprintln!("Transcription failed: {e}");
        }
    }
}

/// Run inference on Metal backend (macOS) — legacy, uses safetensors directly
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn run_metal(model_path: &std::path::Path, tokenizer_path: &std::path::Path, prompt: &str, max_tokens: usize, _temperature: f32, use_f16: bool) {
    use cyb_llm::backend::metal::MetalModel;

    let load_start = std::time::Instant::now();
    let _ = use_f16; // TODO: precision control
    let mut model = match MetalModel::load(model_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Metal model load failed: {e}");
            return;
        }
    };
    println!("Model loaded in {:.1}s (Metal)", load_start.elapsed().as_secs_f64());

    let tokenizer = match tokenizers::Tokenizer::from_file(tokenizer_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Tokenizer load failed: {e}");
            return;
        }
    };

    // Apply chat template if instruction-tuned (legacy Metal path — no .model config)
    let formatted = prompt.to_string();

    let encoding = match tokenizer.encode(formatted.as_str(), true) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Tokenization failed: {e}");
            return;
        }
    };
    let token_ids = encoding.get_ids();
    eprintln!("Tokens ({}): {:?}", token_ids.len(), &token_ids[..token_ids.len().min(20)]);
    let eos = cyb_llm::generate::detect_eos_tokens(&tokenizer, "");

    // Prefill
    let prefill_start = std::time::Instant::now();
    let mut next_token = 0u32;
    for &tid in token_ids {
        next_token = model.forward_decode(tid);
    }
    let prefill_ms = prefill_start.elapsed().as_secs_f64() * 1000.0;

    // Decode
    println!("---");
    use std::io::Write;
    let decode_start = std::time::Instant::now();
    let mut gen_count = 0usize;
    for _ in 0..max_tokens {
        if eos.contains(&next_token) { break; }
        gen_count += 1;
        let decoded = tokenizer.decode(&[next_token], false).unwrap_or_else(|_| "?".to_string());
        print!("{decoded}");
        std::io::stdout().flush().ok();
        next_token = model.forward_decode(next_token);
    }
    let decode_s = decode_start.elapsed().as_secs_f64();
    let tok_s = if decode_s > 0.0 { gen_count as f64 / decode_s } else { 0.0 };
    println!("\n---");
    println!("Prefill: {:.0}ms | Decode: {} tokens in {:.1}s ({:.1} tok/s)", prefill_ms, gen_count, decode_s, tok_s);
}

/// Run inference on Metal backend using .model file (embedded tokenizer)
#[cfg(target_os = "macos")]
fn run_metal_model(model_path: &std::path::Path, prompt: &str, max_tokens: usize, _temperature: f32) {
    use cyb_llm::backend::metal::MetalModel;

    let load_start = std::time::Instant::now();
    let mut model = match MetalModel::load(model_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Metal model load failed: {e}");
            return;
        }
    };

    let tokenizer = match cyb_llm::cyb_format::build_tokenizer(model_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Tokenizer build failed: {e}");
            return;
        }
    };
    println!("Model loaded in {:.1}s (Metal)", load_start.elapsed().as_secs_f64());

    let encoding = match tokenizer.encode(prompt, true) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Tokenization failed: {e}");
            return;
        }
    };
    let token_ids = encoding.get_ids();

    let config_toml = cyb_llm::cyb_format::read_model_config(model_path)
        .map(|(_, cfg)| cfg).unwrap_or_default();
    let eos = cyb_llm::generate::detect_eos_tokens(&tokenizer, &config_toml);

    // Prefill
    let prefill_start = std::time::Instant::now();
    let mut next_token = 0u32;
    for &tid in token_ids {
        next_token = model.forward_decode(tid);
    }
    let prefill_ms = prefill_start.elapsed().as_secs_f64() * 1000.0;

    // Decode
    println!("---");
    use std::io::Write;
    let decode_start = std::time::Instant::now();
    let mut gen_count = 0usize;
    for _ in 0..max_tokens {
        if eos.contains(&next_token) { break; }
        gen_count += 1;
        let decoded = tokenizer.decode(&[next_token], false).unwrap_or_else(|_| "?".to_string());
        print!("{decoded}");
        std::io::stdout().flush().ok();
        next_token = model.forward_decode(next_token);
    }
    let decode_s = decode_start.elapsed().as_secs_f64();
    let tok_s = if decode_s > 0.0 { gen_count as f64 / decode_s } else { 0.0 };
    println!("\n---");
    println!("Prefill: {:.0}ms | Decode: {} tokens in {:.1}s ({:.1} tok/s)", prefill_ms, gen_count, decode_s, tok_s);
}

// ── Model management commands ────────────────────────────────────

fn run_status() {
    use cyb_llm::manifest::{self, format_size, MANIFEST};

    let base = manifest::models_dir();

    println!();
    println!("  \x1b[1mcyb-llm status\x1b[0m — {}", base.display());
    println!();

    // Column widths: model=26, role=11, type=10, size=6, L=3, ctx=6, load=6, wgpu=8, metal=8, llama=6
    let hdr = format!(
        "  {:<26} {:<11} {:<10} {:>6} {:>3} {:>6} {:>6} {:>8} {:>8} {:>6}",
        "MODEL", "ROLE", "TYPE", "SIZE", "L", "CTX", "LOAD", "WGPU", "METAL", "LLAMA"
    );
    let width = 98;
    println!("{hdr}");
    println!("  {}", "─".repeat(width));

    let mut total_disk = 0u64;
    let mut total_ok = 0usize;

    for spec in MANIFEST {
        let model_path = base.join(format!("{}.model", spec.name));

        if !model_path.exists() {
            println!("  {:<26} {:<11} {:<10} \x1b[31mMISSING\x1b[0m",
                spec.name, spec.role, "—");
            continue;
        }

        let disk_bytes = model_path.metadata().map(|m| m.len()).unwrap_or(0);
        total_disk += disk_bytes;

        // Read config from .model file (fast — reads text only, stops at ~~~weights)
        let start = std::time::Instant::now();
        let (model_type, hidden, layers, ctx, load_ok) = match cyb_llm::cyb_format::read_model_config(&model_path) {
            Ok((_card, config)) => {
                let get = |key: &str| -> String {
                    config.lines()
                        .find(|l| l.starts_with(key))
                        .and_then(|l| l.split('=').nth(1))
                        .map(|v| v.trim().trim_matches('"').to_string())
                        .unwrap_or_default()
                };
                let mt = get("model_type");
                let h = get("hidden_size");
                let l = get("num_hidden_layers");
                let c = { let v = get("max_position_embeddings"); if v.is_empty() { get("context_length") } else { v } };
                (mt, h, l, c, true)
            }
            Err(_) => (String::new(), String::new(), String::new(), String::new(), false),
        };
        let load_ms = start.elapsed().as_millis() as u64;

        let type_str = if model_type.is_empty() { "—" } else { &model_type };
        let layers_str = if layers.is_empty() { "—".to_string() } else { layers.clone() };
        let ctx_str = if ctx.is_empty() {
            "—".to_string()
        } else if let Ok(n) = ctx.parse::<u64>() {
            if n >= 1000 { format!("{}K", n / 1000) } else { ctx }
        } else { ctx };
        let load_str = if load_ms > 0 { format!("{}ms", load_ms) } else { "—".to_string() };

        if load_ok { total_ok += 1; }

        // Bench: tok/s for decoder LLMs (catch panics silently)
        let is_decoder = matches!(model_type.as_str(),
            "qwen2" | "qwen3" | "llama" | "phi3" | "bitnet" | "mimo" | "gemma4");

        let clear_line = "\x1b[2K\r"; // ANSI: clear entire line + carriage return

        // Returns (visible_text, ansi_text) where ansi_text has correct padding
        let bench_safe = |label: &str, f: &dyn Fn() -> (String, String), col_width: usize| -> String {
            eprint!("{clear_line}  bench {label} {}...", spec.name);
            let prev_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            std::panic::set_hook(prev_hook);
            match result {
                Ok((ts, sane)) => {
                    // Pad the visible part (ts + space + sane_char) to col_width
                    // sane is 1 visible char wrapped in ANSI codes
                    let visible_len = ts.len() + 2; // "N.N ✓" = ts + " " + 1 char
                    let pad = if visible_len < col_width { col_width - visible_len } else { 0 };
                    format!("{:>pad$}{ts} {sane}", "", pad = pad)
                }
                Err(_) => {
                    let pad = if col_width > 3 { col_width - 3 } else { 0 };
                    format!("{:>pad$}\x1b[31merr\x1b[0m", "", pad = pad)
                }
            }
        };

        let wgpu_str = if is_decoder {
            let mp = model_path.clone();
            bench_safe("wgpu", &|| quick_bench_model(&mp), 8)
        } else { format!("{:>8}", "—") };

        #[cfg(target_os = "macos")]
        let metal_str: String = if is_decoder {
            let mp = model_path.clone();
            bench_safe("metal", &|| quick_bench_metal(&mp), 8)
        } else { format!("{:>8}", "—") };
        #[cfg(not(target_os = "macos"))]
        let metal_str = format!("{:>8}", "—");

        let llama_str: String = if is_decoder {
            match spec.ollama_tag {
                Some(tag) => {
                    eprint!("{clear_line}  bench ollama {}...", spec.name);
                    let r = bench_ollama(tag);
                    format!("{:>6}", r)
                }
                None => format!("{:>6}", "—"),
            }
        } else { format!("{:>6}", "—") };

        // Clear progress line, print final row
        eprint!("{clear_line}");
        println!("  {:<26} {:<11} {:<10} {:>6} {:>3} {:>6} {:>6} {} {} {}",
            spec.name, spec.role, type_str,
            format_size(disk_bytes), layers_str, ctx_str,
            load_str, wgpu_str, metal_str, llama_str);
    }

    println!("  {}", "─".repeat(width));
    println!();
    println!("  \x1b[1m{}\x1b[0m models  ·  \x1b[32m{}\x1b[0m ready  ·  {} on disk",
        MANIFEST.len(), total_ok, format_size(total_disk));
    println!();
}

fn ext_is(path: &std::path::Path, ext: &str) -> bool {
    path.extension().and_then(|e| e.to_str()).map(|e| e == ext).unwrap_or(false)
}

/// Quick bench: load .model, generate tokens, return (tok/s, sanity) strings.
fn quick_bench_model(model_path: &std::path::Path) -> (String, String) {
    let prev = log::max_level();
    log::set_max_level(log::LevelFilter::Error);

    let backend = cyb_llm::backend::create_wgpu_backend();
    let mut generator = match cyb_llm::generate::TextGenerator::new(
        model_path, backend.pipelines,
    ) {
        Ok(g) => g,
        Err(e) => {
            log::set_max_level(prev);
            log::set_max_level(prev);
            log::warn!("WGPU {}: {}", model_path.file_stem().unwrap_or_default().to_str().unwrap_or(""), e);
            return ("\x1b[31merr\x1b[0m".into(), "—".into());
        },
    };

    let config_toml = cyb_llm::cyb_format::read_model_config(model_path)
        .map(|(_, cfg)| cfg).unwrap_or_default();
    let prompt = cyb_llm::generate::apply_chat_template("What is 2+2? /no_think", &config_toml);
    let result = generator.generate_quiet(&prompt, 64, 0.0);

    log::set_max_level(prev);

    match result {
        Ok(text) => {
            let stats = generator.last_stats();
            let tok_s = if stats.decode_tokens > 0 && stats.decode_ms > 10 {
                let v = stats.decode_tokens as f64 / (stats.decode_ms as f64 / 1000.0);
                format!("{:.1}", v)
            } else {
                "0".into()
            };

            let text = text.trim().to_lowercase();
            let check_text = if let Some(pos) = text.find("</think>") {
                &text[pos+8..]
            } else { &text };
            let sane = if check_text.contains('4') || check_text.contains("four") {
                "\x1b[32m✓\x1b[0m"
            } else if text.len() < 2 {
                "\x1b[31m✗\x1b[0m"
            } else {
                "\x1b[33m?\x1b[0m"
            };

            (tok_s, sane.into())
        }
        Err(_) => ("\x1b[31merr\x1b[0m".into(), "—".into()),
    }
}

/// Quick bench Metal: load .model, generate tokens, return (tok/s, sane) strings.
#[cfg(target_os = "macos")]
fn quick_bench_metal(model_path: &std::path::Path) -> (String, String) {
    use cyb_llm::backend::metal::MetalModel;

    let prev = log::max_level();
    log::set_max_level(log::LevelFilter::Error);

    let mut model = match MetalModel::load(model_path) {
        Ok(m) => m,
        Err(e) => {
            log::set_max_level(prev);
            log::warn!("Metal {}: {}", model_path.file_stem().unwrap_or_default().to_str().unwrap_or(""), e);
            return ("\x1b[31merr\x1b[0m".into(), "—".into());
        }
    };

    let tokenizer = match cyb_llm::cyb_format::build_tokenizer(model_path) {
        Ok(t) => t,
        Err(_) => { log::set_max_level(prev); return ("—".into(), "—".into()); }
    };

    let config_toml = cyb_llm::cyb_format::read_model_config(model_path)
        .map(|(_, cfg)| cfg).unwrap_or_default();
    let prompt = cyb_llm::generate::apply_chat_template("What is 2+2? /no_think", &config_toml);
    let encoding = match tokenizer.encode(prompt.as_str(), true) {
        Ok(e) => e,
        Err(_) => { log::set_max_level(prev); return ("err".into(), "—".into()); }
    };
    let token_ids = encoding.get_ids();

    // Prefill
    let mut next_token = 0u32;
    for &tid in token_ids {
        next_token = model.forward_decode(tid);
    }

    // Decode 64 tokens, collect text
    let decode_start = std::time::Instant::now();
    let mut count = 0u32;
    let mut generated_ids = Vec::new();
    let eos = cyb_llm::generate::detect_eos_tokens(&tokenizer, &config_toml);
    for _ in 0..64 {
        if eos.contains(&next_token) { break; }
        count += 1;
        generated_ids.push(next_token);
        next_token = model.forward_decode(next_token);
    }
    let elapsed = decode_start.elapsed().as_secs_f64();

    log::set_max_level(prev);
    let tok_s = if count > 0 && elapsed > 0.01 {
        format!("{:.0}", count as f64 / elapsed)
    } else { "0".into() };

    let text = tokenizer.decode(&generated_ids, true).unwrap_or_default().to_lowercase();
    let check = if let Some(pos) = text.find("</think>") { &text[pos+8..] } else { &text };
    let sane = if check.contains('4') || check.contains("four") {
        "\x1b[32m✓\x1b[0m"
    } else if text.len() < 2 {
        "\x1b[31m✗\x1b[0m"
    } else {
        "\x1b[33m?\x1b[0m"
    };

    (tok_s, sane.into())
}

/// Bench ollama: call API, return tok/s string
fn bench_ollama(tag: &str) -> String {
    use std::io::Read;

    let body = format!(
        r#"{{"model":"{}","prompt":"What is 2+2? /no_think","stream":false,"options":{{"num_predict":64,"temperature":0}}}}"#,
        tag
    );

    let result = std::process::Command::new("curl")
        .args(["-s", "--max-time", "30", "http://localhost:11434/api/generate", "-d", &body])
        .output();

    match result {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            // Parse eval_count and eval_duration from JSON
            let eval_count = extract_json_u64(&text, "eval_count");
            let eval_dur_ns = extract_json_u64(&text, "eval_duration");
            if eval_count > 0 && eval_dur_ns > 0 {
                let tok_s = eval_count as f64 / (eval_dur_ns as f64 / 1e9);
                format!("{:.0}", tok_s)
            } else {
                "—".into()
            }
        }
        _ => "—".into(),
    }
}

fn extract_json_u64(json: &str, key: &str) -> u64 {
    let pattern = format!("\"{}\":", key);
    json.find(&pattern).and_then(|pos| {
        let after = &json[pos + pattern.len()..];
        let num_str: String = after.chars().skip_while(|c| c.is_whitespace()).take_while(|c| c.is_ascii_digit()).collect();
        num_str.parse().ok()
    }).unwrap_or(0)
}

#[allow(dead_code)]
fn scan_subdirs_size(dir: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                total += scan_subdirs_size(&entry.path());
                if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                    for se in sub_entries.flatten() {
                        if let Ok(m) = se.metadata() {
                            if m.is_file() { total += m.len(); }
                        }
                    }
                }
            }
        }
    }
    total
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
if isinstance(vocab, dict):
    for t, i in sorted(vocab.items(), key=lambda x: x[1]):
        lines.append(f'{{i}} = "{{esc(t)}}"')
for at in added:
    tid = at.get('id', -1)
    content = at.get('content', '')
    if tid >= 0 and content:
        lines.append(f'{{tid}} = "{{esc(content)}}"')
else:
    for i, item in enumerate(vocab):
        t = item[0] if isinstance(item, list) else str(item)
        lines.append(f'{{i}} = "{{esc(t)}}"')
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
