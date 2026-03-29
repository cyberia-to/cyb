use clap::{Parser, Subcommand};

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
        /// Model ID (e.g. optimum/gpt2)
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

    // Initialize custom Q4 compute shader pipeline
    cyb_inference::quant::init_q4_compute();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            model,
            prompt,
            max_tokens,
            temperature,
        } => {
            println!("Loading model: {model}");

            // Download model and tokenizer
            let model_path = match cyb_inference::hub::download_model(&model) {
                Ok(p) => p,
                Err(e) => { eprintln!("Model download failed: {e}"); return; }
            };

            let tokenizer_path = match cyb_inference::hub::download_tokenizer(&model) {
                Ok(p) => p,
                Err(e) => { eprintln!("Tokenizer download failed: {e}"); return; }
            };

            let engine = cyb_inference::InferenceEngine::new();

            let mut generator = match cyb_inference::generate::TextGenerator::new(
                &model_path,
                &tokenizer_path,
                engine.device,
            ) {
                Ok(g) => g,
                Err(e) => { eprintln!("Model load failed: {e}"); return; }
            };

            // Generate
            println!("---");
            print!("{prompt}");
            use std::io::Write;
            std::io::stdout().flush().ok();

            match generator.generate(&prompt, max_tokens, temperature) {
                Ok(_text) => {
                    println!("---");
                }
                Err(e) => {
                    eprintln!("\nGeneration failed: {e}");
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
                                    println!("  {}", s.replace("models--", "").replace("--", "/"));
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
