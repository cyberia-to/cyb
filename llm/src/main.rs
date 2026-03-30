//! cyb-llm CLI — run LLMs on wgpu

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cyb-llm")]
#[command(about = "Pure Rust LLM runtime on wgpu")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
            let model_path_buf = std::path::PathBuf::from(&model);
            let is_local = model_path_buf.exists()
                || model.ends_with(".safetensors")
                || model.ends_with(".onnx")
                || model.ends_with(".gguf");

            if is_local {
                // Local file path — detect format
                let model_path = resolve_model_path(&model_path_buf);
                let model_dir = model_path.parent().unwrap_or(std::path::Path::new("."));

                let is_safetensors = model_path
                    .extension()
                    .map(|e| e == "safetensors")
                    .unwrap_or(false);
                let is_gguf = model_path
                    .extension()
                    .map(|e| e == "gguf")
                    .unwrap_or(false);

                println!("Loading local model: {}", model_path.display());

                // Find tokenizer in model directory
                let tokenizer_path = find_tokenizer(model_dir);
                let tokenizer_path = match tokenizer_path {
                    Some(p) => p,
                    None => {
                        eprintln!("No tokenizer.json found in {} or parent directories", model_dir.display());
                        return;
                    }
                };
                log::info!("Using tokenizer: {}", tokenizer_path.display());

                // Initialize GPU backend
                let backend = cyb_llm::backend::create_wgpu_backend();
                let pipelines = backend.pipelines;

                let load_start = std::time::Instant::now();
                let mut generator = if is_gguf {
                    match cyb_llm::generate::TextGenerator::new_gguf(
                        &model_path,
                        &tokenizer_path,
                        pipelines,
                    ) {
                        Ok(g) => g,
                        Err(e) => {
                            eprintln!("Model load failed: {e}");
                            return;
                        }
                    }
                } else if is_safetensors {
                    match cyb_llm::generate::TextGenerator::new_safetensors(
                        &model_path,
                        &tokenizer_path,
                        pipelines,
                    ) {
                        Ok(g) => g,
                        Err(e) => {
                            eprintln!("Model load failed: {e}");
                            return;
                        }
                    }
                } else {
                    match cyb_llm::generate::TextGenerator::new(
                        &model_path,
                        &tokenizer_path,
                        pipelines,
                    ) {
                        Ok(g) => g,
                        Err(e) => {
                            eprintln!("Model load failed: {e}");
                            return;
                        }
                    }
                };
                println!(
                    "Model loaded in {:.1}s",
                    load_start.elapsed().as_secs_f64()
                );

                // Generate
                println!("---");
                match generator.generate(&prompt, max_tokens, temperature) {
                    Ok(_text) => {}
                    Err(e) => {
                        eprintln!("\nGeneration failed: {e}");
                    }
                }
            } else {
                // HuggingFace model ID — download ONNX model
                println!("Loading model (native wgpu): {model}");

                let model_path = match cyb_llm::hub::download_model(&model) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Model download failed: {e}");
                        return;
                    }
                };

                let tokenizer_path = match cyb_llm::hub::download_tokenizer(&model) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Tokenizer download failed: {e}");
                        return;
                    }
                };

                // Initialize GPU backend
                let backend = cyb_llm::backend::create_wgpu_backend();
                let pipelines = backend.pipelines;

                let load_start = std::time::Instant::now();
                let mut generator =
                    match cyb_llm::generate::TextGenerator::new(
                        &model_path,
                        &tokenizer_path,
                        pipelines,
                    ) {
                        Ok(g) => g,
                        Err(e) => {
                            eprintln!("Model load failed: {e}");
                            return;
                        }
                    };
                println!(
                    "Model loaded in {:.1}s",
                    load_start.elapsed().as_secs_f64()
                );

                // Generate
                println!("---");
                match generator.generate(&prompt, max_tokens, temperature) {
                    Ok(_text) => {}
                    Err(e) => {
                        eprintln!("\nGeneration failed: {e}");
                    }
                }
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
    }
}

/// Run BERT embedding via GraphModel
fn run_embed(model_path: &str, text: &str) {
    use cyb_llm::ir::templates::{bert_encoder, modernbert_encoder, BertConfig};
    use cyb_llm::backend::wgpu_backend::graph_model::{GraphModel, GraphModelConfig, Architecture};

    let path = std::path::Path::new(model_path);
    let model_dir = path.parent().unwrap_or(std::path::Path::new("."));

    // Read config.json for model parameters
    let config_path = model_dir.join("config.json");
    let bert_config = if config_path.exists() {
        let config_str = std::fs::read_to_string(&config_path).expect("Cannot read config.json");
        let config_json: serde_json::Value = serde_json::from_str(&config_str).expect("Invalid config.json");

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

    // Detect model type for template selection
    let model_type = if config_path.exists() {
        let s = std::fs::read_to_string(&config_path).unwrap_or_default();
        let j: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
        j["model_type"].as_str().unwrap_or("").to_string()
    } else {
        String::new()
    };

    let is_modernbert = model_type == "modernbert";

    // For ModernBERT: split fused QKV and Wi weights into separate Q/K/V and gate/up
    if is_modernbert {
        use cyb_llm::ir::{DType, WeightData};
        let hidden = bert_config.hidden_size;
        let inter = bert_config.intermediate_size;
        let keys: Vec<String> = graph_with_weights.weights.keys().cloned().collect();
        let qkv_keys: Vec<String> = keys.iter().filter(|k| k.ends_with(".attn.Wqkv.weight")).cloned().collect();
        let wi_keys: Vec<String> = keys.iter().filter(|k| k.ends_with(".mlp.Wi.weight")).cloned().collect();

        for key in qkv_keys {
            if let Some(w) = graph_with_weights.weights.remove(&key) {
                let elem_size = w.dtype.element_size();
                let row_bytes = hidden * elem_size;
                let prefix = key.strip_suffix(".Wqkv.weight").unwrap().to_string();
                for (idx, name) in ["q", "k", "v"].iter().enumerate() {
                    let start = idx * hidden * row_bytes;
                    let end = start + hidden * row_bytes;
                    if end <= w.data.len() {
                        graph_with_weights.weights.insert(
                            format!("{prefix}.{name}.weight"),
                            WeightData { data: w.data[start..end].to_vec(), shape: vec![hidden, hidden], dtype: w.dtype },
                        );
                    }
                }
            }
        }
        for key in wi_keys {
            if let Some(w) = graph_with_weights.weights.remove(&key) {
                let elem_size = w.dtype.element_size();
                let row_bytes = hidden * elem_size;
                let prefix = key.strip_suffix(".Wi.weight").unwrap().to_string();
                let half = inter * row_bytes;
                if half * 2 <= w.data.len() {
                    graph_with_weights.weights.insert(
                        format!("{prefix}.Wi_gate.weight"),
                        WeightData { data: w.data[..half].to_vec(), shape: vec![inter, hidden], dtype: w.dtype },
                    );
                    graph_with_weights.weights.insert(
                        format!("{prefix}.Wi_up.weight"),
                        WeightData { data: w.data[half..half*2].to_vec(), shape: vec![inter, hidden], dtype: w.dtype },
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
                        dtype: DType::F32,
                    });
                }
            }
            for suffix in &["attn_norm.bias", "mlp_norm.bias"] {
                let name = format!("model.layers.{i}.{suffix}");
                if !graph_with_weights.weights.contains_key(&name) {
                    graph_with_weights.weights.insert(name, WeightData {
                        data: vec![0u8; hidden * 4],
                        shape: vec![hidden],
                        dtype: DType::F32,
                    });
                }
            }
        }
        // Embed norm bias
        if !graph_with_weights.weights.contains_key("model.embeddings.norm.bias") {
            graph_with_weights.weights.insert("model.embeddings.norm.bias".to_string(), WeightData {
                data: vec![0u8; hidden * 4],
                shape: vec![hidden],
                dtype: DType::F32,
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
                dtype: DType::F16,
            });
        }
        let type_name = format!("{wp}embeddings.token_type_embeddings.weight");
        if !graph_with_weights.weights.contains_key(&type_name) {
            graph_with_weights.weights.insert(type_name, WeightData {
                data: vec![0u8; 2 * hidden * 2],  // F16 zeros, 2 types
                shape: vec![2, hidden],
                dtype: DType::F16,
            });
        }
    }

    // Build graph from template
    let graph = if is_modernbert {
        modernbert_encoder(&bert_config)
    } else {
        bert_encoder(&bert_config)
    };
    println!("Graph template: {} nodes", graph.len());

    // Find tokenizer
    let tokenizer_path = find_tokenizer(model_dir);
    let tokenizer_path = match tokenizer_path {
        Some(p) => p,
        None => {
            eprintln!("No tokenizer.json found in {} or parent directories", model_dir.display());
            return;
        }
    };

    let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
        .expect("Failed to load tokenizer");

    // Tokenize input
    let encoding = tokenizer.encode(text, true).expect("Tokenization failed");
    let input_ids: Vec<u32> = encoding.get_ids().to_vec();
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
    let (output_name, output_size) = if is_modernbert {
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
    // Extract [CLS] token embedding (first hidden_size values)
    let output: Vec<f32> = output_raw[..bert_config.hidden_size.min(output_raw.len())].to_vec();
    let elapsed = embed_start.elapsed().as_secs_f64();

    println!("---");
    println!("Pooler output ({} dims), computed in {:.3}s:", output.len(), elapsed);
    // Print first 10 values
    let preview: Vec<String> = output.iter().take(10).map(|v| format!("{:.4}", v)).collect();
    println!("  [{}{}]", preview.join(", "),
        if output.len() > 10 { ", ..." } else { "" });
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

/// Resolve model path — keep symlink parent (don't canonicalize to blobs/)
fn resolve_model_path(path: &std::path::Path) -> std::path::PathBuf {
    // Don't canonicalize — HF cache uses symlinks from snapshots/ to blobs/
    // We want the snapshot directory for finding config.json and tokenizer.json
    path.to_path_buf()
}

/// Find tokenizer.json in the model directory or parent directories
fn find_tokenizer(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = Some(dir);
    while let Some(d) = current {
        let candidate = d.join("tokenizer.json");
        if candidate.exists() {
            return Some(candidate);
        }
        current = d.parent();
    }
    None
}
