//! mr CLI.

use mr::backend::{Backend, BackendKind};
use mr::format::LoadedModel;
use mr::generate::{generate, SampleConfig, SampleKind};
use mr::llama_style::LlamaModel;
use mr::tokenizer::{build_tokenizer, ChatMessage};
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    env_logger::init();
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        help();
        return;
    }
    let cmd = args[1].clone();
    args.drain(..2);

    match cmd.as_str() {
        "backends" => list_backends(),
        "run" => run(args),
        "help" | "--help" | "-h" => help(),
        other => {
            eprintln!("unknown command: {other}");
            help();
            std::process::exit(2);
        }
    }
}

fn help() {
    println!("mr — modelruntime");
    println!();
    println!("usage: mr <command> [args]");
    println!();
    println!("commands:");
    println!("  backends                         list available backends");
    println!("  run <model> --prompt <text>      generate text from a model");
    println!("    options:");
    println!("      --max-tokens N               (default 64)");
    println!("      --temperature T              (default 0 = greedy)");
    println!("      --backend NAME               cpu|wgpu+rs|honeycrisp");
    println!("      --no-chat                    skip chat template, use raw prompt");
}

fn list_backends() {
    let cpu = mr::cpu::CpuBackend::new();
    println!("  {}         reference library, always available", cpu.kind().as_str());
    match mr::wgpu_rs::WgpuRsBackend::new() {
        Ok(b) => println!("  {}     portable default", b.kind().as_str()),
        Err(e) => println!("  {}     unavailable: {e}", BackendKind::WgpuRs.as_str()),
    }
    #[cfg(target_os = "macos")]
    {
        match mr::honeycrisp::HoneycrispBackend::new() {
            Ok(b) => println!("  {}   Apple Silicon turbo", b.kind().as_str()),
            Err(e) => println!("  {}   unavailable: {e}", BackendKind::Honeycrisp.as_str()),
        }
    }
    #[cfg(not(target_os = "macos"))]
    println!("  honeycrisp   macOS-only");
    println!("  nox          future (trident-compiled bytecode VM)");
}

fn run(args: Vec<String>) {
    if args.is_empty() {
        eprintln!("usage: mr run <model> [--prompt TEXT] [--max-tokens N] ...");
        std::process::exit(2);
    }
    let model_arg = &args[0];
    let mut prompt = String::new();
    let mut max_tokens: usize = 64;
    let mut temperature: f32 = 0.0;
    let mut backend_name: String = "auto".into();
    let mut use_chat = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--prompt" => {
                i += 1;
                prompt = args[i].clone();
            }
            "--max-tokens" => {
                i += 1;
                max_tokens = args[i].parse().unwrap_or(64);
            }
            "--temperature" => {
                i += 1;
                temperature = args[i].parse().unwrap_or(0.0);
            }
            "--backend" => {
                i += 1;
                backend_name = args[i].clone();
            }
            "--no-chat" => use_chat = false,
            other => {
                eprintln!("unknown flag: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let path = resolve_model_path(model_arg);
    if !path.exists() {
        eprintln!("model not found: {}", path.display());
        std::process::exit(1);
    }

    println!("Loading: {}", path.display());
    let t_load = Instant::now();
    let lm = LoadedModel::load(&path).unwrap_or_else(|e| {
        eprintln!("load failed: {e}");
        std::process::exit(1);
    });
    let mut model = LlamaModel::from_loaded(&lm).unwrap_or_else(|e| {
        eprintln!("model build failed: {e}");
        std::process::exit(1);
    });
    let tok = build_tokenizer(&lm).unwrap_or_else(|e| {
        eprintln!("tokenizer build failed: {e}");
        std::process::exit(1);
    });
    println!(
        "Loaded in {:.1}s  [{}, {} layers, hidden={}, heads={}/{}, head_dim={}, qk_norm={}]",
        t_load.elapsed().as_secs_f64(),
        model.config.model_type,
        model.config.num_hidden_layers,
        model.config.hidden_size,
        model.config.num_attention_heads,
        model.config.num_key_value_heads,
        model.config.head_dim,
        model.config.has_qk_norm,
    );

    let final_prompt = if use_chat {
        let msgs = vec![ChatMessage {
            role: "user".into(),
            content: prompt.clone(),
        }];
        tok.apply_chat_template(&msgs, true)
    } else {
        prompt.clone()
    };

    let cfg = SampleConfig {
        method: if temperature <= 0.0 {
            SampleKind::Greedy
        } else {
            SampleKind::TopP
        },
        temperature: if temperature <= 0.0 { 1.0 } else { temperature },
        top_p: 0.95,
        top_k: 40,
    };

    let backend: Box<dyn Backend> = pick_backend(&backend_name);
    println!("Backend: {}", backend.kind().as_str());
    // Upload weights once to the backend (persistent GPU memory).
    if let Err(e) = model.to_backend(&*backend) {
        eprintln!("weight upload failed: {e}");
    }
    println!("---");
    println!("{final_prompt}");
    println!("---");

    let t_gen = Instant::now();
    match generate(&mut model, &tok, &*backend, &final_prompt, max_tokens, cfg) {
        Ok((text, count)) => {
            let dt = t_gen.elapsed().as_secs_f64();
            println!("{text}");
            println!("---");
            println!("Generated {count} tokens in {dt:.1}s ({:.1} tok/s)", count as f64 / dt);
        }
        Err(e) => {
            eprintln!("generate failed: {e}");
            std::process::exit(1);
        }
    }
}

fn pick_backend(name: &str) -> Box<dyn Backend> {
    match name {
        "cpu" => Box::new(mr::cpu::CpuBackend::new()),
        "wgpu+rs" | "wgpu" => match mr::wgpu_rs::WgpuRsBackend::new() {
            Ok(b) => Box::new(b),
            Err(e) => {
                eprintln!("wgpu+rs unavailable ({e}), falling back to cpu");
                Box::new(mr::cpu::CpuBackend::new())
            }
        },
        #[cfg(target_os = "macos")]
        "honeycrisp" => match mr::honeycrisp::HoneycrispBackend::new() {
            Ok(b) => Box::new(b),
            Err(e) => {
                eprintln!("honeycrisp unavailable ({e}), falling back to cpu");
                Box::new(mr::cpu::CpuBackend::new())
            }
        },
        "auto" | "" => {
            // Default: honeycrisp on macOS if available, else wgpu+rs, else cpu.
            #[cfg(target_os = "macos")]
            {
                if let Ok(b) = mr::honeycrisp::HoneycrispBackend::new() {
                    return Box::new(b);
                }
            }
            if let Ok(b) = mr::wgpu_rs::WgpuRsBackend::new() {
                return Box::new(b);
            }
            Box::new(mr::cpu::CpuBackend::new())
        }
        other => {
            eprintln!("unknown backend: {other}");
            std::process::exit(2);
        }
    }
}

fn resolve_model_path(name: &str) -> PathBuf {
    let p = PathBuf::from(name);
    if p.exists() {
        return p;
    }
    // Try ~/llm/<name>.model
    if let Ok(home) = std::env::var("HOME") {
        let candidate = PathBuf::from(home)
            .join("llm")
            .join(format!("{name}.model"));
        if candidate.exists() {
            return candidate;
        }
    }
    p
}
