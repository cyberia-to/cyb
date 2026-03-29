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

    /// Run native wgpu inference (Qwen3-0.6B Q4)
    RunNative {
        /// Model ID (e.g. onnx-community/Qwen3-0.6B-ONNX)
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

        Commands::RunNative {
            model,
            prompt,
            max_tokens,
            temperature,
        } => {
            use std::sync::Arc;
            use cyb_inference::runtime::GpuRuntime;
            use cyb_inference::runtime::model::{NativeModel, sample_top_p, argmax};

            println!("Loading model (native wgpu): {model}");

            // Download model and tokenizer
            let model_path = match cyb_inference::hub::download_model(&model) {
                Ok(p) => p,
                Err(e) => { eprintln!("Model download failed: {e}"); return; }
            };

            let tokenizer_path = match cyb_inference::hub::download_tokenizer(&model) {
                Ok(p) => p,
                Err(e) => { eprintln!("Tokenizer download failed: {e}"); return; }
            };

            // Initialize GPU runtime
            let runtime = GpuRuntime::new();
            let pipelines = Arc::new(runtime.pipelines);

            // Load model weights
            let load_start = std::time::Instant::now();
            let mut native_model = match NativeModel::load_from_onnx(&model_path, pipelines) {
                Ok(m) => m,
                Err(e) => { eprintln!("Model load failed: {e}"); return; }
            };
            println!("Model loaded in {:.1}s", load_start.elapsed().as_secs_f64());

            // Load tokenizer
            let tokenizer = match tokenizers::Tokenizer::from_file(&tokenizer_path) {
                Ok(t) => t,
                Err(e) => { eprintln!("Tokenizer load failed: {e}"); return; }
            };

            // Encode prompt
            let encoding = match tokenizer.encode(prompt.as_str(), false) {
                Ok(e) => e,
                Err(e) => { eprintln!("Tokenization failed: {e}"); return; }
            };
            let mut token_ids: Vec<u32> = encoding.get_ids().to_vec();
            log::info!("Prompt tokens ({} tokens): {:?}", token_ids.len(), token_ids);

            println!("---");
            print!("{prompt}");
            use std::io::Write;
            std::io::stdout().flush().ok();

            let gen_start = std::time::Instant::now();

            // Prefill: process prompt tokens one at a time (safe path)
            let mut logits = Vec::new();
            for t in 0..token_ids.len() {
                logits = native_model.forward(&[token_ids[t]]);
            }

            // Autoregressive generation
            let eos_tokens: &[u32] = &[151643, 151645]; // Qwen3 EOS tokens

            for step in 0..max_tokens {
                let next_token = if temperature <= 0.0 {
                    argmax(&logits)
                } else {
                    sample_top_p(&logits, temperature, 0.9)
                };

                if eos_tokens.contains(&next_token) {
                    break;
                }

                token_ids.push(next_token);

                let decoded = match tokenizer.decode(&[next_token], false) {
                    Ok(s) => s,
                    Err(_) => String::from("?"),
                };
                print!("{decoded}");
                std::io::stdout().flush().ok();

                // Decode: forward pass with single new token
                logits = native_model.forward(&[next_token]);

                if step > 0 && step % 20 == 0 {
                    let elapsed = gen_start.elapsed().as_secs_f64();
                    let tps = (step as f64) / elapsed;
                    log::debug!("Step {step}: {tps:.1} tok/s");
                }
            }

            let elapsed = gen_start.elapsed().as_secs_f64();
            let generated = token_ids.len() - encoding.get_ids().len();
            println!();
            println!("---");
            println!(
                "Generated {generated} tokens in {elapsed:.1}s ({:.1} tok/s)",
                generated as f64 / elapsed
            );
        }
    }
}
