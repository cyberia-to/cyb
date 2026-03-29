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
