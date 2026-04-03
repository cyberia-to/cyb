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

            println!("Loading: {}", model_file.display());

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

    // Build tokenizer: from .model vocab or fallback to tokenizer.json
    let tokenizer = if ext_is(path, "model") {
        let (_card, _config) = cyb_llm::cyb_format::read_model_config(path)
            .expect("Cannot read .model");
        let mf = cyb_llm::cyb_format::read_model_file(path)
            .expect("Cannot read .model file");
        cyb_llm::cyb_format::build_tokenizer_from_vocab(&mf.vocab)
            .expect("Cannot build tokenizer from vocab")
    } else {
        let tok_path = model_dir.join("tokenizer.json");
        tokenizers::Tokenizer::from_file(&tok_path)
            .expect("No tokenizer.json found — use .model format")
    };

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
    let mut model = match if use_f16 {
        MetalModel::load_from_safetensors_f16(model_path)
    } else {
        MetalModel::load_from_safetensors(model_path)
    } {
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

// ── Model management commands ────────────────────────────────────

fn run_status() {
    use cyb_llm::manifest::{self, format_size, MANIFEST};

    let base = manifest::models_dir();

    println!();
    println!("  \x1b[1mcyb-llm status\x1b[0m — {}", base.display());
    println!();

    println!("  {:<26} {:>5} {:<10} {:>5} {:>3} {:>5} {:>5} {:>5} {:>3}",
        "MODEL", "TIER", "TYPE", "SIZE", "L", "CTX", "LOAD", "T/S", "GEN");
    println!("  {}", "─".repeat(76));

    let mut total_disk = 0u64;
    let mut total_ok = 0usize;

    for spec in MANIFEST {
        let model_path = base.join(format!("{}.model", spec.name));

        if !model_path.exists() {
            println!("  {:<28} {:<5} {:>8} \x1b[31mMISSING\x1b[0m",
                spec.name, spec.tier, "—");
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

        // Bench: tok/s + quality for decoder LLMs
        let is_decoder = matches!(model_type.as_str(),
            "qwen2" | "qwen3" | "llama" | "phi3" | "bitnet" | "mimo");
        let (tok_s_str, quality_str) = if is_decoder {
            eprint!("  bench {}...\r", spec.name);
            quick_bench_model(&model_path)
        } else {
            ("—".into(), "—".into())
        };

        println!("  {:<28} {:<5} {:<8} {:>6} {:>3} {:>6} {:>5} {:>6} {:>4}",
            spec.name, spec.tier, type_str,
            format_size(disk_bytes), layers_str, ctx_str,
            load_str, tok_s_str, quality_str);
    }

    println!("  {}", "─".repeat(80));
    println!();
    println!("  \x1b[1m{}\x1b[0m models  ·  \x1b[32m{}\x1b[0m ready  ·  {} on disk",
        MANIFEST.len(), total_ok, format_size(total_disk));
    println!();
}

fn ext_is(path: &std::path::Path, ext: &str) -> bool {
    path.extension().and_then(|e| e.to_str()).map(|e| e == ext).unwrap_or(false)
}

/// Quick bench: load .model, generate a few tokens, measure tok/s + check sanity.
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
            return ("fail".into(), format!("\x1b[31m{}\x1b[0m", &e[..e.len().min(40)]));
        },
    };

    let config_toml = cyb_llm::cyb_format::read_model_config(model_path)
        .map(|(_, cfg)| cfg).unwrap_or_default();
    let prompt = cyb_llm::generate::apply_chat_template("What is 2+2?", &config_toml);
    let result = generator.generate_quiet(&prompt, 16, 0.0);

    log::set_max_level(prev);

    match result {
        Ok(text) => {
            let text = text.trim().to_string();
            let stats = generator.last_stats();

            let tok_s = if stats.decode_tokens > 0 && stats.decode_ms > 10 {
                stats.decode_tokens as f64 / (stats.decode_ms as f64 / 1000.0)
            } else { 0.0 };

            let has_answer = text.contains('4') || text.to_lowercase().contains("four");
            let is_garbage = text.len() < 2 || text.chars().filter(|c| c.is_alphanumeric()).count() < 3;

            let quality = if is_garbage { "\x1b[31mbad\x1b[0m" }
                else if has_answer { "\x1b[32mok\x1b[0m" }
                else { "\x1b[33m??\x1b[0m" };

            (format!("{:.1}", tok_s), quality.into())
        }
        Err(_) => ("err".into(), "\x1b[31mfail\x1b[0m".into()),
    }
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

fn run_audit() {
    use cyb_llm::manifest::{self, format_size, Tier};

    let statuses = manifest::inspect_all();
    let unknown = manifest::find_unknown();

    let mut total_current: u64 = 0;
    let mut total_expected: f64 = 0.0;
    let mut issue_count = 0;

    println!(
        "{:<28} {:>6} {:>6} {:>6} {:>6} {}",
        "Model", "Now", "Target", "Tier", "Quant", "Status"
    );
    println!("{}", "─".repeat(90));

    for s in &statuses {
        total_expected += s.spec.expected_gb as f64;

        if !s.exists {
            println!(
                "{:<28} {:>6} {:>5.1}G {:>6} {:>6}  MISSING",
                s.spec.name, "—", s.spec.expected_gb, s.spec.tier, s.spec.target_quant
            );
            issue_count += 1;
            continue;
        }

        total_current += s.size_bytes;
        let size_gb = s.size_bytes as f64 / 1e9;
        let issues = s.issues();
        let status = if issues.is_empty() {
            "ok".to_string()
        } else {
            issue_count += issues.len();
            issues.join(", ")
        };

        // Detect current format
        let _fmt = if s.has_onnx && s.has_safetensors {
            "st+onnx"
        } else if s.has_onnx {
            "onnx"
        } else if s.has_safetensors {
            "st"
        } else if s.has_gguf {
            "gguf"
        } else if s.has_ggml {
            "ggml"
        } else if s.has_pytorch {
            "pt"
        } else {
            "?"
        };

        println!(
            "{:<28} {:>5.1}G {:>5.1}G {:>6} {:>6}  {}",
            s.spec.name, size_gb, s.spec.expected_gb, s.spec.tier, s.spec.target_quant, status
        );
    }

    // Unknown directories
    for (name, size) in &unknown {
        total_current += size;
        println!(
            "{:<28} {:>6} {:>6} {:>6} {:>6}  UNKNOWN (not in soma spec)",
            name,
            format_size(*size),
            "—",
            "?",
            "?"
        );
        issue_count += 1;
    }

    println!("{}", "─".repeat(90));
    println!(
        "{:<28} {:>5.1}G {:>5.1}G",
        "TOTAL",
        total_current as f64 / 1e9,
        total_expected
    );
    let savings = total_current as f64 / 1e9 - total_expected;
    if savings > 0.5 {
        println!("\nPotential savings: {:.1}G", savings);
    }

    // Summary by tier
    println!();
    for tier in [Tier::Tier0, Tier::Tier1, Tier::Tier2, Tier::Media] {
        let tier_models: Vec<_> = statuses.iter().filter(|s| s.spec.tier == tier).collect();
        let tier_size: u64 = tier_models.iter().map(|s| s.size_bytes).sum();
        let tier_ok = tier_models.iter().filter(|s| s.is_ok()).count();
        let tier_total = tier_models.len();
        println!(
            "  {}: {}/{} ok, {}",
            tier,
            tier_ok,
            tier_total,
            format_size(tier_size)
        );
    }

    if issue_count > 0 {
        println!("\n{issue_count} issue(s) found. Run `cyb-llm clean` to fix.");
    } else {
        println!("\nAll models ok.");
    }
}

fn run_clean(dry_run: bool) {
    use cyb_llm::manifest::{self, format_size};

    let statuses = manifest::inspect_all();
    let mut saved: u64 = 0;
    let base = manifest::models_dir();

    for s in &statuses {
        if !s.exists {
            continue;
        }
        let dir = base.join(s.spec.name);

        // Delete files matching delete_patterns
        for pattern in s.spec.delete_patterns {
            for entry in glob_dir(&dir, pattern) {
                if entry.is_file() {
                    let fsize = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    if dry_run {
                        println!(
                            "  WOULD DELETE {}/{} ({})",
                            s.spec.name,
                            entry.file_name().unwrap_or_default().to_string_lossy(),
                            format_size(fsize)
                        );
                    } else {
                        println!(
                            "  DELETE {}/{} ({})",
                            s.spec.name,
                            entry.file_name().unwrap_or_default().to_string_lossy(),
                            format_size(fsize)
                        );
                        let _ = std::fs::remove_file(&entry);
                    }
                    saved += fsize;
                }
            }
        }

        // Remove junk dirs
        for junk in &[".huggingface", ".cache", "__pycache__"] {
            let junk_path = dir.join(junk);
            if junk_path.exists() {
                let size = dir_size_recursive(&junk_path);
                if dry_run {
                    println!(
                        "  WOULD DELETE {}/{}/ ({})",
                        s.spec.name,
                        junk,
                        format_size(size)
                    );
                } else {
                    println!(
                        "  DELETE {}/{}/ ({})",
                        s.spec.name,
                        junk,
                        format_size(size)
                    );
                    let _ = std::fs::remove_dir_all(&junk_path);
                }
                saved += size;
            }
        }

        // Remove junk files
        for junk in &[
            ".gitattributes",
            "README.md",
            "LICENSE",
            "LICENSE.md",
            "USE_POLICY.md",
            "NOTICE",
        ] {
            let junk_path = dir.join(junk);
            if junk_path.is_file() {
                let size = junk_path.metadata().map(|m| m.len()).unwrap_or(0);
                if dry_run {
                    println!(
                        "  WOULD DELETE {}/{} ({})",
                        s.spec.name,
                        junk,
                        format_size(size)
                    );
                } else {
                    println!("  DELETE {}/{} ({})", s.spec.name, junk, format_size(size));
                    let _ = std::fs::remove_file(&junk_path);
                }
                saved += size;
            }
        }
    }

    // Flag unknown directories
    let unknown = manifest::find_unknown();
    for (name, size) in &unknown {
        println!(
            "  FLAG {}/ ({}) — not in soma spec, delete manually if unwanted",
            name,
            format_size(*size)
        );
    }

    let action = if dry_run { "Would save" } else { "Saved" };
    println!("\n{action}: {}", format_size(saved));
    if dry_run && saved > 0 {
        println!("Run with --execute to apply: cyb-llm clean --execute");
    }
}

fn run_fetch(name_filter: Option<String>, tier_filter: Option<String>) {
    use cyb_llm::manifest::{self, format_size, Tier};

    let tier_match: Option<Tier> = match tier_filter.as_deref() {
        Some("tier0" | "0") => Some(Tier::Tier0),
        Some("tier1" | "1") => Some(Tier::Tier1),
        Some("tier2" | "2") => Some(Tier::Tier2),
        Some("media") => Some(Tier::Media),
        Some(other) => {
            eprintln!("Unknown tier: {other}. Use: tier0, tier1, tier2, media");
            return;
        }
        None => None,
    };

    let base = manifest::models_dir();
    std::fs::create_dir_all(&base).ok();

    let api = match hf_hub::api::sync::Api::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("HF API init failed: {e}");
            return;
        }
    };

    for spec in manifest::MANIFEST {
        // Filter
        if let Some(ref name) = name_filter {
            if !spec.name.contains(name.as_str()) {
                continue;
            }
        }
        if let Some(target_tier) = tier_match {
            if spec.tier != target_tier {
                continue;
            }
        }

        if !spec.hf_download || spec.hf_repo.is_empty() {
            println!("SKIP {} — not an HF download", spec.name);
            continue;
        }

        let model_dir = base.join(spec.name);

        // Check if already complete
        let status = manifest::inspect_model(spec);
        if status.exists && status.missing_shards.is_none() {
            println!("OK   {} ({})", spec.name, format_size(status.size_bytes));
            continue;
        }

        if status.exists && status.missing_shards.is_some() {
            println!(
                "FIX  {} — {}",
                spec.name,
                status.missing_shards.as_deref().unwrap_or("incomplete")
            );
        } else {
            println!("PULL {} ← {}", spec.name, spec.hf_repo);
        }

        // Download all files from repo
        let repo = api.model(spec.hf_repo.to_string());

        // Use hf-hub's info endpoint to list files, then download each
        match repo.info() {
            Ok(info) => {
                std::fs::create_dir_all(&model_dir).ok();
                let siblings = info.siblings;
                let mut downloaded = 0u64;
                for file_info in &siblings {
                    let filename = &file_info.rfilename;

                    // Skip large unnecessary files
                    if filename.ends_with(".h5")
                        || filename.ends_with(".msgpack")
                        || filename.contains("pytorch_model")
                        || filename.contains("flax_model")
                        || filename == ".gitattributes"
                    {
                        continue;
                    }

                    let target = model_dir.join(filename);
                    if target.exists() {
                        continue;
                    }

                    // Create subdirectories
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }

                    match repo.get(filename) {
                        Ok(cached_path) => {
                            // hf-hub caches to its own dir; copy/link to our dir
                            if cached_path != target {
                                if let Err(e) = std::fs::copy(&cached_path, &target) {
                                    eprintln!("  WARN: copy {filename} failed: {e}");
                                    continue;
                                }
                            }
                            let fsize = target.metadata().map(|m| m.len()).unwrap_or(0);
                            downloaded += fsize;
                            println!("  GET  {filename} ({})", format_size(fsize));
                        }
                        Err(e) => {
                            eprintln!("  FAIL {filename}: {e}");
                        }
                    }
                }
                if downloaded > 0 {
                    println!(
                        "  DONE {} (downloaded {})",
                        spec.name,
                        format_size(downloaded)
                    );
                }
            }
            Err(e) => {
                eprintln!("  FAIL listing {}: {e}", spec.hf_repo);
            }
        }
    }
}

fn run_pack(name: &str) {
    use cyb_llm::manifest::{self, format_size, MANIFEST};
    use cyb_llm::import;

    let base = manifest::models_dir();

    let targets: Vec<&manifest::ModelSpec> = if name == "all" {
        MANIFEST
            .iter()
            .filter(|s| base.join(s.name).exists())
            .collect()
    } else {
        MANIFEST.iter().filter(|s| s.name.contains(name)).collect()
    };

    if targets.is_empty() {
        eprintln!("No models matched '{name}'");
        return;
    }

    let mut total_in = 0u64;
    let mut total_out = 0u64;
    let mut packed = 0usize;

    for spec in &targets {
        let model_dir = base.join(spec.name);

        // Check if .cyb already exists
        let cyb_path = model_dir.join(format!("{}.cyb", spec.name));
        if cyb_path.exists() {
            let size = cyb_path.metadata().map(|m| m.len()).unwrap_or(0);
            println!("SKIP {:<28} already packed ({})", spec.name, format_size(size));
            continue;
        }

        let start = std::time::Instant::now();
        match import::convert_to_cyb(&model_dir) {
            Ok((path, in_size, out_size)) => {
                let elapsed = start.elapsed().as_secs_f64();
                let ratio = if out_size > 0 {
                    in_size as f64 / out_size as f64
                } else {
                    0.0
                };
                println!(
                    "PACK {:<28} {} → {} ({:.1}x, {:.1}s) → {}",
                    spec.name,
                    format_size(in_size),
                    format_size(out_size),
                    ratio,
                    elapsed,
                    path.file_name().unwrap_or_default().to_string_lossy(),
                );
                total_in += in_size;
                total_out += out_size;
                packed += 1;
            }
            Err(e) => {
                eprintln!("FAIL {:<28} {e}", spec.name);
            }
        }
    }

    if packed > 0 {
        println!(
            "\nPacked {packed} models. Total: {} → {} ({:.1}x)",
            format_size(total_in),
            format_size(total_out),
            if total_out > 0 {
                total_in as f64 / total_out as f64
            } else {
                0.0
            },
        );
    }
}

fn run_import(name: &str) {
    use cyb_llm::manifest::{self, format_size, MANIFEST};
    use cyb_llm::import;

    let base = manifest::models_dir();

    let targets: Vec<&manifest::ModelSpec> = if name == "all" {
        MANIFEST.iter().filter(|s| {
            let dir = base.join(s.name);
            dir.exists()
        }).collect()
    } else {
        MANIFEST.iter().filter(|s| s.name.contains(name)).collect()
    };

    if targets.is_empty() {
        eprintln!("No models matched '{name}'");
        return;
    }

    let mut total_deleted = 0usize;
    let mut total_converted = 0usize;

    for spec in &targets {
        let model_dir = base.join(spec.name);
        if !model_dir.exists() {
            println!("SKIP {} — not downloaded", spec.name);
            continue;
        }

        let before = import::dir_size_pub(&model_dir);
        let result = import::canonicalize(&model_dir);

        // Repack .cyb files: normalize names + quantize to Q4
        for entry in std::fs::read_dir(&model_dir).into_iter().flatten().flatten() {
            if entry.path().extension().map(|e| e == "cyb").unwrap_or(false) {
                if let Err(e) = import::repack_cyb(&entry.path()) {
                    eprintln!("  repack {}: {e}", entry.path().display());
                }
            }
        }

        let after = import::dir_size_pub(&model_dir);

        total_deleted += result.files_deleted;
        total_converted += result.configs_converted;

        let saved = if before > after { before - after } else { 0 };
        let status = if result.errors.is_empty() { "ok" } else { "WARN" };

        println!(
            "{:<4} {:<28} {} → {} (−{}, {}cfg, −{}files, weights: {})",
            status,
            spec.name,
            format_size(before),
            format_size(after),
            format_size(saved),
            result.configs_converted,
            result.files_deleted,
            result.weights_format,
        );

        for err in &result.errors {
            eprintln!("       {err}");
        }
    }

    println!("\nImported {} models. Converted {} configs, deleted {} junk files.",
        targets.len(), total_converted, total_deleted);
}

fn run_convert(name: &str, quant_override: Option<&str>, execute: bool, cleanup: bool) {
    use cyb_llm::manifest::{self, format_size, Quant, MANIFEST};
    use cyb_llm::quantize::{self, QuantType};

    let base = manifest::models_dir();

    let quant_from_str = |s: &str| -> Option<QuantType> {
        match s {
            "q4" | "Q4" | "q4_0" => Some(QuantType::Q4_0),
            "q8" | "Q8" | "q8_0" => Some(QuantType::Q8_0),
            "f16" | "F16" => Some(QuantType::F16),
            _ => None,
        }
    };

    let quant_for_spec = |spec: &manifest::ModelSpec| -> Option<QuantType> {
        if let Some(qs) = quant_override {
            return quant_from_str(qs);
        }
        match spec.target_quant {
            Quant::Q4 => Some(QuantType::Q4_0),
            Quant::Q8 => Some(QuantType::Q8_0),
            Quant::F16 => Some(QuantType::F16),
            Quant::Native => None,
        }
    };

    let targets: Vec<&manifest::ModelSpec> = if name == "all" {
        MANIFEST
            .iter()
            .filter(|s| {
                let status = manifest::inspect_model(s);
                status.exists
                    && status.has_safetensors
                    && s.target_quant != Quant::Native
                    && status.missing_shards.is_none()
            })
            .collect()
    } else {
        MANIFEST.iter().filter(|s| s.name.contains(name)).collect()
    };

    if targets.is_empty() {
        eprintln!("No models matched '{name}'");
        return;
    }

    for spec in &targets {
        let model_dir = base.join(spec.name);
        if !model_dir.exists() {
            println!("SKIP {} — not downloaded", spec.name);
            continue;
        }

        let qt = match quant_for_spec(spec) {
            Some(q) => q,
            None => {
                println!("SKIP {} — native format", spec.name);
                continue;
            }
        };

        let status = manifest::inspect_model(spec);
        if status.missing_shards.is_some() {
            println!(
                "SKIP {} — {}",
                spec.name,
                status.missing_shards.as_deref().unwrap_or("incomplete")
            );
            continue;
        }
        if !status.has_safetensors {
            println!("SKIP {} — no safetensors to convert", spec.name);
            continue;
        }

        let output_name = format!("{}.gguf", spec.name);
        let output_path = model_dir.join(&output_name);

        if output_path.exists() {
            let size = output_path.metadata().map(|m| m.len()).unwrap_or(0);
            println!("SKIP {} — already converted ({})", spec.name, format_size(size));
            continue;
        }

        let qt_name = match qt {
            QuantType::Q4_0 => "Q4_0",
            QuantType::Q8_0 => "Q8_0",
            QuantType::F16 => "F16",
        };

        println!(
            "CONVERT {} → {} ({}, {} → ~{})",
            spec.name, output_name, qt_name,
            format_size(status.size_bytes),
            format_size((spec.expected_gb * 1e9) as u64),
        );

        if !execute {
            continue;
        }

        let start = std::time::Instant::now();
        match quantize::convert_safetensors_to_gguf(&model_dir, &output_path, qt, true) {
            Ok(stats) => {
                let elapsed = start.elapsed().as_secs_f64();
                let out_size = output_path.metadata().map(|m| m.len()).unwrap_or(0);
                println!(
                    "  OK  {} tensors, {:.1}x compression, {} → {} in {:.1}s",
                    stats.n_tensors, stats.compression_ratio(),
                    format_size(stats.input_bytes), format_size(out_size), elapsed,
                );
                println!(
                    "       Q4: {}, Q8: {}, F32(norms): {}, F16: {}",
                    stats.quantized_q4, stats.quantized_q8, stats.kept_f32, stats.kept_f16,
                );

                if cleanup {
                    let mut freed = 0u64;
                    if let Ok(entries) = std::fs::read_dir(&model_dir) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            let is_st = p.extension().map(|e| e == "safetensors").unwrap_or(false);
                            let is_index = p.file_name().and_then(|n| n.to_str())
                                .map(|n| n.contains("index")).unwrap_or(false);
                            if is_st || is_index {
                                let sz = p.metadata().map(|m| m.len()).unwrap_or(0);
                                if let Err(e) = std::fs::remove_file(&p) {
                                    eprintln!("  WARN: rm {:?}: {e}", p.file_name());
                                } else {
                                    freed += sz;
                                }
                            }
                        }
                    }
                    if freed > 0 {
                        println!("  FREED {} (deleted safetensors)", format_size(freed));
                    }
                }
            }
            Err(e) => {
                eprintln!("  FAIL {}: {e}", spec.name);
                let _ = std::fs::remove_file(&output_path);
            }
        }
    }

    if !execute {
        println!("\nDry run. Add --execute to convert: cyb-llm convert {} --execute", name);
    }
}

fn glob_dir(dir: &std::path::Path, pattern: &str) -> Vec<std::path::PathBuf> {
    // Simple glob: supports * and ** prefix
    let mut results = Vec::new();
    let target_name = pattern.replace("*/", "").replace("**/", "");

    fn visit(dir: &std::path::Path, target: &str, results: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, target, results);
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == target
                        || (target.starts_with('*') && name.ends_with(&target[1..]))
                    {
                        results.push(path);
                    }
                }
            }
        }
    }

    visit(dir, &target_name, &mut results);
    results
}

fn dir_size_recursive(path: &std::path::Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size_recursive(&p);
            } else {
                total += p.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

/// Resolve model path — keep symlink parent (don't canonicalize to blobs/)
fn resolve_model_path(path: &std::path::Path) -> std::path::PathBuf {
    // Don't canonicalize — HF cache uses symlinks from snapshots/ to blobs/
    // We want the snapshot directory for finding config.json and tokenizer.json
    path.to_path_buf()
}

// ── extract-tensors command ─────────────────────────────────────

fn run_extract_tensors(name: &str) {
    use cyb_llm::manifest::{self, MANIFEST};
    use cyb_llm::cyb_format;

    let base = manifest::models_dir();
    let targets: Vec<&manifest::ModelSpec> = if name == "all" {
        MANIFEST.iter().filter(|s| base.join(s.name).exists()).collect()
    } else {
        MANIFEST.iter().filter(|s| s.name.contains(name)).collect()
    };

    for spec in &targets {
        let dir = base.join(spec.name);
        let cyb_path = dir.join(format!("{}.cyb", spec.name));
        if !cyb_path.exists() {
            eprintln!("SKIP {:<28} no .cyb file", spec.name);
            continue;
        }

        match cyb_format::extract_tensor_index(&cyb_path) {
            Ok(tensors) => {
                let toml = cyb_format::tensor_index_to_toml(&tensors);
                let out_path = dir.join("tensors.toml");
                if let Err(e) = std::fs::write(&out_path, &toml) {
                    eprintln!("FAIL {:<28} write tensors.toml: {e}", spec.name);
                    continue;
                }
                println!("OK   {:<28} {} tensors → tensors.toml", spec.name, tensors.len());
            }
            Err(e) => {
                eprintln!("FAIL {:<28} {e}", spec.name);
            }
        }
    }
}

// ── model-pack command ──────────────────────────────────────────

fn run_model_pack(name: &str) {
    use cyb_llm::manifest::{self, format_size, MANIFEST};
    use cyb_llm::cyb_format;

    let base = manifest::models_dir();
    let targets: Vec<&manifest::ModelSpec> = if name == "all" {
        MANIFEST.iter().filter(|s| base.join(s.name).exists()).collect()
    } else {
        MANIFEST.iter().filter(|s| s.name.contains(name)).collect()
    };

    for spec in &targets {
        let dir = base.join(spec.name);

        // Read all 7 required files
        let read = |filename: &str| -> String {
            let p = dir.join(filename);
            std::fs::read_to_string(&p).unwrap_or_else(|_| {
                eprintln!("  WARN: missing {filename}");
                String::new()
            })
        };

        let card = read("card.md");
        let config = read("config.toml");
        let program = read("program.rs");
        let tensors_toml = read("tensors.toml");
        let vocab = read("vocab.toml");
        let eval = read("eval.toml");

        // Read weights from .cyb binary
        let cyb_path = dir.join(format!("{}.cyb", spec.name));
        let weights = if cyb_path.exists() {
            match cyb_format::extract_tensor_data(&cyb_path) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("FAIL {:<28} read weights: {e}", spec.name);
                    continue;
                }
            }
        } else {
            eprintln!("SKIP {:<28} no .cyb file", spec.name);
            continue;
        };

        let output_path = base.join(format!("{}.model", spec.name));
        match cyb_format::write_model_file(
            &output_path, spec.name,
            &card, &config, &program, "rs",
            &tensors_toml, &vocab, &eval, &weights,
        ) {
            Ok(()) => {
                let size = output_path.metadata().map(|m| m.len()).unwrap_or(0);
                println!("PACK {:<28} → {}.model ({})", spec.name, spec.name, format_size(size));
            }
            Err(e) => {
                eprintln!("FAIL {:<28} {e}", spec.name);
            }
        }
    }
}
