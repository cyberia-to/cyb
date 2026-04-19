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
        "status" => status(),
        "bench" => bench(args),
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
    println!("  status                           honest report on manifest models");
    println!("  bench <model> [--steps N]        phase breakdown + tok/s per backend");
    println!("  run <model> --prompt <text>      generate text from a model");
    println!("    options:");
    println!("      --max-tokens N               (default 64)");
    println!("      --temperature T              (default 0 = greedy)");
    println!("      --backend NAME               cpu|wgpu+rs|honeycrisp");
    println!("      --no-chat                    skip chat template, use raw prompt");
}

fn bench(args: Vec<String>) {
    if args.is_empty() {
        eprintln!("usage: mr bench <model> [--steps N]");
        std::process::exit(2);
    }
    let model_arg = &args[0];
    let mut steps: usize = 10;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--steps" => {
                i += 1;
                steps = args[i].parse().unwrap_or(10);
            }
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
    println!();
    println!("  \x1b[1mmr bench\x1b[0m — {} ({} forward steps)", path.display(), steps);
    println!();
    println!(
        "  {:<12}  {:<8}  {:<10}  {:<10}  {:<12}  {:<8}",
        "BACKEND", "LOAD", "UPLOAD", "FIRST-FWD", "AVG-FWD", "TOK/S"
    );
    println!("  {}", "─".repeat(74));

    let backends_to_try: Vec<(&str, Box<dyn Fn() -> Result<Box<dyn Backend>, String>>)> = {
        let mut v: Vec<(&str, Box<dyn Fn() -> Result<Box<dyn Backend>, String>>)> = vec![];
        v.push((
            "cpu",
            Box::new(|| Ok(Box::new(mr::cpu::CpuBackend::new()) as Box<dyn Backend>)),
        ));
        v.push((
            "wgpu+rs",
            Box::new(|| {
                mr::wgpu_rs::WgpuRsBackend::new()
                    .map(|b| Box::new(b) as Box<dyn Backend>)
                    .map_err(|e| format!("{e}"))
            }),
        ));
        #[cfg(target_os = "macos")]
        v.push((
            "honeycrisp",
            Box::new(|| {
                mr::honeycrisp::HoneycrispBackend::new()
                    .map(|b| Box::new(b) as Box<dyn Backend>)
                    .map_err(|e| format!("{e}"))
            }),
        ));
        v
    };

    for (name, make) in backends_to_try {
        match make() {
            Err(e) => {
                println!("  {:<12}  \x1b[31munavailable\x1b[0m ({e})", name);
                continue;
            }
            Ok(b) => match mr::bench::bench_e2e(&path, &*b, steps) {
                Err(e) => println!("  {:<12}  \x1b[31merror\x1b[0m ({e})", name),
                Ok(bench) => {
                    let avg = bench.avg_forward_ms();
                    println!(
                        "  {:<12}  {:>6.0}ms  {:>6.0}ms  {:>6.0}ms   {:>6.1}ms/tok  {:>5.1} tok/s",
                        name,
                        bench.load_ms,
                        bench.to_backend_ms,
                        bench.first_forward_ms,
                        avg,
                        1000.0 / avg,
                    );
                }
            },
        }
    }
    println!();
}

fn status() {
    use mr::format::LoadedModel;
    use mr::manifest::MANIFEST;

    let base = mr::manifest::models_dir();
    println!();
    println!("  \x1b[1mmr status\x1b[0m — {}", base.display());
    println!();
    println!(
        "  {:<26} {:<12} {:<8} {:<10} {:<20} {}",
        "MODEL", "FAMILY", "ROLE", "LOAD", "FORWARD", "NOTES"
    );
    println!("  {}", "─".repeat(100));

    for e in MANIFEST {
        let path = base.join(format!("{}.model", e.name));
        if !path.exists() {
            println!(
                "  {:<26} {:<12} {:<8} \x1b[33m{:<10}\x1b[0m {:<20} {}",
                e.name, e.family, e.role, "missing", "—", e.notes
            );
            continue;
        }

        let t0 = Instant::now();
        let loaded = LoadedModel::load(&path);
        let load_s = t0.elapsed().as_secs_f64();

        let (load_status, forward_status) = match loaded {
            Err(err) => (
                format!("\x1b[31merr\x1b[0m"),
                format!("— ({err})"),
            ),
            Ok(lm) => {
                let load_ok = format!("\x1b[32m{:.1}s\x1b[0m", load_s);
                match mr::llama_style::LlamaModel::from_loaded(&lm) {
                    Err(e) => (load_ok, format!("\x1b[31march fail\x1b[0m: {e}")),
                    Ok(mut model) => {
                        let backend = mr::cpu::CpuBackend::new();
                        let t_fwd = Instant::now();
                        let result = model.forward(0, &backend);
                        let fwd_s = t_fwd.elapsed().as_secs_f64();
                        let fwd_status = match result {
                            Ok(logits) => {
                                let finite = logits.iter().all(|v| v.is_finite());
                                if finite {
                                    format!("\x1b[32m{:.1}s ok\x1b[0m", fwd_s)
                                } else {
                                    "\x1b[31mnon-finite\x1b[0m".into()
                                }
                            }
                            Err(e) => format!("\x1b[31merr\x1b[0m: {e}"),
                        };
                        (load_ok, fwd_status)
                    }
                }
            }
        };
        println!(
            "  {:<26} {:<12} {:<8} {:<10} {:<20} {}",
            e.name, e.family, e.role, load_status, forward_status, e.notes
        );
    }
    println!("  {}", "─".repeat(100));
    println!();
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
