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

    /// Encode audio with a Whisper model (encoder only, outputs embeddings)
    Encode {
        /// Path to GGML whisper model (.bin)
        model: String,
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

        Commands::Encode { model } => {
            run_encode(&model);
        }
    }
}

/// Run BERT embedding via GraphModel
fn run_embed(model_path: &str, text: &str) {
    use cyb_llm::ir::templates::{bert_encoder, BertConfig};
    use cyb_llm::backend::wgpu_backend::graph_model::{GraphModel, GraphModelConfig, Architecture};

    let path = std::path::Path::new(model_path);
    let model_dir = path.parent().unwrap_or(std::path::Path::new("."));

    // Read config.json for model parameters
    let config_path = model_dir.join("config.json");
    let bert_config = if config_path.exists() {
        let config_str = std::fs::read_to_string(&config_path).expect("Cannot read config.json");
        let config_json: serde_json::Value = serde_json::from_str(&config_str).expect("Invalid config.json");

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
        }
    } else {
        println!("No config.json found, using BERT-base defaults");
        BertConfig::default()
    };

    println!("BERT config: hidden={}, heads={}, layers={}, vocab={}",
        bert_config.hidden_size, bert_config.num_heads, bert_config.num_layers, bert_config.vocab_size);

    // Load weights
    println!("Loading weights from {}...", path.display());
    let graph_with_weights = cyb_llm::loader::load_model(path).expect("Failed to load model");

    // Build graph from template
    let graph = bert_encoder(&bert_config);
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

    // Run embedding
    let embed_start = std::time::Instant::now();
    let output = model.encode(&input_ids, "pooler_output", bert_config.hidden_size)
        .expect("Encoding failed");
    let elapsed = embed_start.elapsed().as_secs_f64();

    println!("---");
    println!("Pooler output ({} dims), computed in {:.3}s:", output.len(), elapsed);
    // Print first 10 values
    let preview: Vec<String> = output.iter().take(10).map(|v| format!("{:.4}", v)).collect();
    println!("  [{}{}]", preview.join(", "),
        if output.len() > 10 { ", ..." } else { "" });
}

/// Run Whisper encoder via GraphModel (outputs encoder embeddings)
fn run_encode(model_path: &str) {
    use cyb_llm::ir::templates::{whisper_encoder_decoder, WhisperConfig};
    use cyb_llm::backend::wgpu_backend::graph_model::{GraphModel, GraphModelConfig, Architecture};

    let path = std::path::Path::new(model_path);

    // Detect format
    let is_ggml = model_path.ends_with(".bin");
    if !is_ggml {
        eprintln!("Whisper encode currently only supports GGML (.bin) format");
        return;
    }

    // Load GGML model (reads hparams + weights)
    println!("Loading Whisper GGML model from {}...", path.display());
    let graph_with_weights = cyb_llm::loader::ggml::load_ggml(path).expect("Failed to load GGML model");

    // Extract hparams from weight shapes to build config
    // The GGML loader puts hparams in the log, but we can detect from weights
    let audio_state = graph_with_weights.weights.get("encoder.conv1.bias")
        .map(|w| w.shape[0])
        .unwrap_or(768);
    let n_audio_head = if audio_state == 1280 { 20 } else if audio_state == 1024 { 16 } else if audio_state == 512 { 8 } else { 12 };
    let n_text_state = graph_with_weights.weights.get("decoder.ln.weight")
        .map(|w| w.shape[0])
        .unwrap_or(audio_state);
    let n_text_head = if n_text_state == 1280 { 20 } else if n_text_state == 1024 { 16 } else if n_text_state == 512 { 8 } else { 12 };
    let n_vocab = graph_with_weights.weights.get("decoder.token_embedding.weight")
        .map(|w| w.shape[0])
        .unwrap_or(51865);
    let n_audio_ctx = graph_with_weights.weights.get("encoder.positional_embedding")
        .map(|w| w.shape[0])
        .unwrap_or(1500);
    let n_mels = graph_with_weights.weights.get("encoder.conv1.weight")
        .map(|w| w.shape[1])
        .unwrap_or(80);

    // Count encoder layers
    let n_audio_layer = (0..100)
        .take_while(|i| graph_with_weights.weights.contains_key(&format!("encoder.blocks.{i}.attn_ln.weight")))
        .count();
    let n_text_layer = (0..100)
        .take_while(|i| graph_with_weights.weights.contains_key(&format!("decoder.blocks.{i}.attn_ln.weight")))
        .count();
    let n_text_ctx = graph_with_weights.weights.get("decoder.positional_embedding")
        .map(|w| if w.shape.len() >= 2 { w.shape[1] } else { w.shape[0] })
        .unwrap_or(448);

    let whisper_config = WhisperConfig {
        n_audio_state: audio_state,
        n_audio_head,
        n_audio_layer,
        n_audio_ctx,
        n_text_state,
        n_text_head,
        n_text_layer,
        n_text_ctx,
        n_vocab,
        n_mels,
        eps: 1e-5,
    };

    println!("Whisper config: audio_state={}, audio_layers={}, text_state={}, text_layers={}, vocab={}",
        whisper_config.n_audio_state, whisper_config.n_audio_layer,
        whisper_config.n_text_state, whisper_config.n_text_layer, whisper_config.n_vocab);

    // Build graph from template
    let graph = whisper_encoder_decoder(&whisper_config);
    println!("Graph template: {} nodes", graph.len());

    // Initialize GPU
    let backend = cyb_llm::backend::create_wgpu_backend();
    let pipelines = backend.pipelines;

    let gm_config = GraphModelConfig {
        hidden_size: whisper_config.n_audio_state as u32,
        num_heads: whisper_config.n_audio_head as u32,
        kv_num_heads: whisper_config.n_audio_head as u32,
        head_dim: (whisper_config.n_audio_state / whisper_config.n_audio_head) as u32,
        vocab_size: whisper_config.n_vocab as u32,
        num_layers: whisper_config.n_text_layer as u32, // decoder layers for KV cache
        block_size: 32,
        rope_theta: 0.0,  // Whisper uses sinusoidal positional embeddings, not RoPE
        max_seq_len: whisper_config.n_text_ctx as u32,
        has_qk_norm: false,
    };

    let load_start = std::time::Instant::now();
    let _model = GraphModel::new(
        graph,
        &graph_with_weights.weights,
        pipelines,
        Architecture::EncoderDecoder,
        gm_config,
    ).expect("Failed to create GraphModel");
    println!("Model loaded in {:.1}s ({} weights on GPU)",
        load_start.elapsed().as_secs_f64(),
        graph_with_weights.weights.len());

    println!("---");
    println!("Whisper model ready. Encoder has {} layers, decoder has {} layers.",
        whisper_config.n_audio_layer, whisper_config.n_text_layer);
    println!("To run actual transcription, audio preprocessing (mel spectrogram) is needed.");
    println!("GraphModel loaded successfully — encoder/decoder architecture wired to executor.");
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
