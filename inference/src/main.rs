use clap::{Parser, Subcommand};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "cyb-inference")]
#[command(about = "Pure Rust ONNX inference engine on wgpu")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run inference on an ONNX model from HuggingFace
    Run {
        /// Model ID (e.g. onnx-community/Llama-3.2-1B-Instruct-ONNX)
        model: String,

        /// Prompt for text generation
        #[arg(short, long, default_value = "Hello, world!")]
        prompt: String,

        /// Maximum tokens to generate
        #[arg(short = 'n', long, default_value_t = 128)]
        max_tokens: usize,

        /// Temperature for sampling
        #[arg(short, long, default_value_t = 0.7)]
        temperature: f32,
    },

    /// List cached models
    List,

    /// Show ONNX graph info for a local file
    Info {
        /// Path to local ONNX file
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
            println!("Loading model: {model}");
            println!("Prompt: {prompt}");
            println!("Max tokens: {max_tokens}, temperature: {temperature}");

            // Download model
            let model_path = match cyb_inference::hub::download_model(&model) {
                Ok(p) => {
                    println!("Model path: {}", p.display());
                    p
                }
                Err(e) => {
                    eprintln!("Download failed: {e}");
                    return;
                }
            };

            // Init GPU
            let engine = cyb_inference::InferenceEngine::new();
            println!("GPU device: {:?}", engine.device);

            // Parse ONNX
            println!("Parsing ONNX graph...");
            let mut executor = match cyb_inference::graph::OnnxExecutor::from_file(&model_path) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Parse failed: {e}");
                    return;
                }
            };

            // Load weights
            println!("Loading weights...");
            executor.load_from_file(&model_path, &engine.device)
                .unwrap_or_else(|e| eprintln!("Warning loading weights: {e}"));

            // Tokenize input
            println!("Preparing input...");
            let mut inputs = HashMap::new();
            // Simple token IDs for "Hello" — GPT-2 tokenizer: [15496]
            // TODO: use proper tokenizer
            let token_ids: Vec<f32> = vec![15496.0]; // "Hello"
            let seq_len = token_ids.len();
            let input_ids_data = burn::tensor::TensorData::new(token_ids, vec![1, seq_len]);
            inputs.insert(
                "input_ids".to_string(),
                cyb_inference::graph::value::Value::Float2(
                    burn::prelude::Tensor::from_data(input_ids_data, &engine.device),
                ),
            );
            // Attention mask — all 1s (attend to everything)
            let mask: Vec<f32> = vec![1.0; seq_len];
            let mask_data = burn::tensor::TensorData::new(mask, vec![1, seq_len]);
            inputs.insert(
                "attention_mask".to_string(),
                cyb_inference::graph::value::Value::Float2(
                    burn::prelude::Tensor::from_data(mask_data, &engine.device),
                ),
            );
            match executor.run(inputs, &engine.device) {
                Ok(outputs) => {
                    println!("Outputs: {} tensors", outputs.len());
                    for (name, val) in &outputs {
                        println!("  {name}: shape={:?}", val.shape());
                    }
                }
                Err(e) => {
                    eprintln!("Inference failed: {e}");
                }
            }
        }

        Commands::List => {
            println!("Cached models in ~/.cache/huggingface/hub/");
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
                                    println!("  {}", s.replace("models--", "").replace("--", "/"));
                                }
                            }
                        }
                    }
                }
            } else {
                println!("  (no cache directory found)");
            }
        }

        Commands::Info { path } => {
            match cyb_inference::graph::load_onnx_info(&path) {
                Ok(info) => println!("{info}"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        Commands::Download { model } => {
            println!("Downloading {model}...");
            match cyb_inference::hub::download_model(&model) {
                Ok(path) => println!("Downloaded to: {}", path.display()),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }
}
