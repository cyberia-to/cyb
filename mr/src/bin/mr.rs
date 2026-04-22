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
        "profile" => profile(args),
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
    println!("  profile <model> [--steps N] [--backend X]    per-op time breakdown");
    println!("  run <model> --prompt <text>      generate text from a model");
    println!("    options:");
    println!("      --max-tokens N               (default 64)");
    println!("      --temperature T              (default 0 = greedy)");
    println!("      --backend NAME               cpu|wgpu+rs|honeycrisp");
    println!("      --no-chat                    skip chat template, use raw prompt");
}

fn profile(args: Vec<String>) {
    use mr::format::LoadedModel;
    use mr::llama_style::LlamaModel;
    if args.is_empty() {
        eprintln!("usage: mr profile <model> [--steps N] [--backend X]");
        std::process::exit(2);
    }
    let model_arg = &args[0];
    let mut steps: usize = 8;
    let mut backend_name = "auto".to_string();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--steps" => {
                i += 1;
                steps = args[i].parse().unwrap_or(8);
            }
            "--backend" => {
                i += 1;
                backend_name = args[i].clone();
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
    let backend = pick_backend(&backend_name);
    println!();
    println!(
        "  \x1b[1mmr profile\x1b[0m — {} on {} ({} steps)",
        path.display(),
        backend.kind().as_str(),
        steps
    );
    println!();

    let lm = LoadedModel::load(&path).expect("load");
    let mut model = LlamaModel::from_loaded(&lm).expect("build");
    model.to_backend(&*backend).expect("upload");
    model.enable_prof();

    // Warmup first — kernel compile, allocator prime.
    let _ = model.forward(0, &*backend).expect("warmup");
    // Reset so warmup doesn't pollute numbers.
    model.enable_prof();

    for i in 0..steps {
        let tok = (i + 1) as u32;
        let _ = model.forward(tok, &*backend).expect("forward");
    }

    println!("{}", model.prof.summary());
    println!();
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
    use mr::manifest::MANIFEST;

    let base = mr::manifest::models_dir();
    println!();
    println!("  \x1b[1mmr status\x1b[0m — {}", base.display());
    println!();
    // Widths: MODEL 24, FAMILY 12, ROLE 8, SIZE 6, L 3, CTX 5, LOAD 7, CPU 9, WGPU+RS 9, HONEY 10, LLAMA 6
    let hdr = format!(
        "  {:<24} {:<12} {:<8} {:>6} {:>3} {:>5} {:>7} {:>9} {:>9} {:>10} {:>6}",
        "MODEL", "FAMILY", "ROLE", "SIZE", "L", "CTX", "LOAD", "CPU", "WGPU+RS", "HONEYCRISP", "LLAMA"
    );
    let width = 113;
    println!("{hdr}");
    println!("  {}", "─".repeat(width));

    for e in MANIFEST {
        let path = base.join(format!("{}.model", e.name));
        if !path.exists() {
            println!(
                "  {:<24} {:<12} {:<8} {:>6} {:>3} {:>5} \x1b[33m{:>7}\x1b[0m {:>9} {:>9} {:>10} {:>6}",
                e.name, e.family, e.role, "—", "—", "—", "missing", "—", "—", "—", "—"
            );
            continue;
        }

        // Read config (fast — text only) for size / layers / ctx
        let disk_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let t0 = Instant::now();
        let (layers, ctx) = read_header_meta(&path);
        let load_ms = t0.elapsed().as_millis();

        let layers_str = if layers == 0 { "—".into() } else { layers.to_string() };
        let ctx_str = if ctx == 0 {
            "—".into()
        } else if ctx >= 1000 {
            format!("{}K", ctx / 1000)
        } else {
            ctx.to_string()
        };

        // Progress indicator on stderr so stdout stays clean.
        let clear = "\x1b[2K\r";
        let bench_col = |label: &str, width: usize, f: &dyn Fn() -> (String, String)| -> String {
            eprint!("{clear}  bench {label} {}...", e.name);
            let prev = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            std::panic::set_hook(prev);
            match result {
                Ok((tok_s, sane)) => pad_bench(&tok_s, &sane, width),
                Err(_) => {
                    let pad = width.saturating_sub(3);
                    format!("{:>pad$}\x1b[31merr\x1b[0m", "", pad = pad)
                }
            }
        };

        let cpu_str = bench_col("cpu", 9, &|| {
            let b = mr::cpu::CpuBackend::new();
            quick_bench(&path, &b)
        });
        let wgpu_str = bench_col("wgpu+rs", 9, &|| match mr::wgpu_rs::WgpuRsBackend::new() {
            Ok(b) => quick_bench(&path, &b),
            Err(_) => ("—".into(), "—".into()),
        });
        #[cfg(target_os = "macos")]
        let honey_str = bench_col("honeycrisp", 10, &|| match mr::honeycrisp::HoneycrispBackend::new() {
            Ok(b) => quick_bench(&path, &b),
            Err(_) => ("—".into(), "—".into()),
        });
        #[cfg(not(target_os = "macos"))]
        let honey_str = format!("{:>10}", "—");

        let llama_str = match e.ollama_tag {
            Some(tag) => {
                eprint!("{clear}  bench llama {}...", e.name);
                format!("{:>6}", bench_ollama(tag))
            }
            None => format!("{:>6}", "—"),
        };

        eprint!("{clear}");
        println!(
            "  {:<24} {:<12} {:<8} {:>6} {:>3} {:>5} {:>6}ms {} {} {} {}",
            e.name,
            e.family,
            e.role,
            format_size(disk_bytes),
            layers_str,
            ctx_str,
            load_ms,
            cpu_str,
            wgpu_str,
            honey_str,
            llama_str
        );
    }
    println!("  {}", "─".repeat(width));
    println!();
    println!("  legend: \x1b[32m✓\x1b[0m clean answer  ·  \x1b[33m?\x1b[0m fragmented/rambling  ·  \x1b[31m✗\x1b[0m garbled");
    println!();
}

/// Pad bench result "tok/s sanity" into `width` visible chars (ignoring ANSI).
fn pad_bench(tok_s: &str, sane: &str, width: usize) -> String {
    // Visible length = tok_s (strip ANSI) + " " + 1 (sanity glyph)
    let visible = visible_len(tok_s) + 2;
    let pad = width.saturating_sub(visible);
    format!("{:>pad$}{tok_s} {sane}", "", pad = pad)
}

fn visible_len(s: &str) -> usize {
    // Strip CSI sequences \x1b[...m
    let mut count = 0usize;
    let mut in_esc = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_esc = true;
            continue;
        }
        if in_esc {
            if c == 'm' {
                in_esc = false;
            }
            continue;
        }
        count += 1;
    }
    count
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        format!("{:.1}G", bytes as f64 / (1u64 << 30) as f64)
    } else if bytes >= 1 << 20 {
        format!("{}M", bytes >> 20)
    } else if bytes >= 1 << 10 {
        format!("{}K", bytes >> 10)
    } else {
        format!("{}B", bytes)
    }
}

/// Quick benchmark: generate 32 tokens on "What is 2+2? /no_think" and
/// return (tok/s text, colored sanity glyph).
fn quick_bench(path: &std::path::Path, backend: &dyn Backend) -> (String, String) {
    let lm = match LoadedModel::load(path) {
        Ok(m) => m,
        Err(_) => return ("\x1b[31merr\x1b[0m".into(), "—".into()),
    };
    let mut model = match LlamaModel::from_loaded(&lm) {
        Ok(m) => m,
        Err(_) => return ("\x1b[31merr\x1b[0m".into(), "—".into()),
    };
    let tok = match build_tokenizer(&lm) {
        Ok(t) => t,
        Err(_) => return ("\x1b[31merr\x1b[0m".into(), "—".into()),
    };
    if model.to_backend(backend).is_err() {
        return ("\x1b[31merr\x1b[0m".into(), "—".into());
    }

    let msgs = vec![ChatMessage {
        role: "user".into(),
        content: "What is 2+2? /no_think".into(),
    }];
    let prompt = tok.apply_chat_template(&msgs, true);
    let cfg = SampleConfig {
        method: SampleKind::Greedy,
        temperature: 1.0,
        top_p: 0.95,
        top_k: 40,
    };

    // Prefill + decode, measure decode-only time.
    model.reset_kv_cache();
    let prompt_ids = tok.encode(&prompt);
    let mut logits: Vec<f32> = Vec::new();
    for &tid in &prompt_ids {
        match model.forward(tid, backend) {
            Ok(l) => logits = l,
            Err(_) => return ("\x1b[31merr\x1b[0m".into(), "—".into()),
        }
    }

    let t_decode = Instant::now();
    let mut generated: Vec<u32> = Vec::with_capacity(32);
    for _ in 0..32 {
        let next = mr::generate::sample(&logits, cfg);
        if tok.is_eos(next) {
            break;
        }
        generated.push(next);
        match model.forward(next, backend) {
            Ok(l) => logits = l,
            Err(_) => break,
        }
    }
    let dt = t_decode.elapsed().as_secs_f64();

    let tok_s = if generated.len() > 0 && dt > 0.01 {
        format!("{:.0}", generated.len() as f64 / dt)
    } else {
        "0".into()
    };

    // Strip specials so validator sees what a user would see (ollama does the same).
    let text = tok.decode(&generated, true);
    let sane = validate_math_answer(&text);
    (tok_s, sane.into())
}

/// Classify generation quality for "What is 2+2?" prompt.
fn validate_math_answer(text: &str) -> &'static str {
    let text = text.trim().to_lowercase();
    let body = if let Some(pos) = text.find("</think>") {
        &text[pos + 8..]
    } else {
        &text[..]
    };
    let body = body.trim();

    let frag_patterns = ["|im_", "|im ", "|endof", "|user", "|assistant", "|fim"];
    let frag_count: usize = frag_patterns.iter().map(|p| body.matches(p).count()).sum();

    let has_four = body.contains('4') || body.contains("four");

    let repetitive = body.len() > 20 && {
        let first_20: String = body.chars().take(20).collect();
        body.matches(&first_20[..]).count() > 2
    };

    if body.is_empty() {
        "\x1b[31m✗\x1b[0m"
    } else if frag_count >= 3 || repetitive {
        "\x1b[31m✗\x1b[0m"
    } else if has_four && frag_count == 0 && !repetitive {
        // "4" alone, "is 4", "= 4", "equals 4", "2+2 = 4" — all clean.
        "\x1b[32m✓\x1b[0m"
    } else if has_four {
        "\x1b[33m?\x1b[0m"
    } else {
        "\x1b[31m✗\x1b[0m"
    }
}

/// Call local ollama, return tok/s as string.
fn bench_ollama(tag: &str) -> String {
    let body = format!(
        r#"{{"model":"{tag}","prompt":"What is 2+2? /no_think","stream":false,"options":{{"num_predict":64,"temperature":0}}}}"#
    );
    let out = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "30",
            "http://localhost:11434/api/generate",
            "-d",
            &body,
        ])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let eval_count = extract_json_u64(&text, "eval_count");
            let eval_dur_ns = extract_json_u64(&text, "eval_duration");
            if eval_count > 0 && eval_dur_ns > 0 {
                format!("{:.0}", eval_count as f64 / (eval_dur_ns as f64 / 1e9))
            } else {
                "—".into()
            }
        }
        _ => "—".into(),
    }
}

/// Parse (num_hidden_layers, max_position_embeddings) from the text header
/// without loading weights. Returns zeros on failure.
fn read_header_meta(path: &std::path::Path) -> (usize, u64) {
    use std::io::Read;
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (0, 0),
    };
    let mut buf = vec![0u8; 2 * 1024 * 1024];
    let n = f.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    // Arbitrary read boundary may split a UTF-8 codepoint; lossy is fine —
    // we only scan line-based integers in the TOML header.
    let text = String::from_utf8_lossy(&buf);
    let layers = find_int(&text, "num_hidden_layers").unwrap_or(0) as usize;
    let ctx = find_int(&text, "max_position_embeddings")
        .or_else(|| find_int(&text, "context_length"))
        .unwrap_or(0) as u64;
    (layers, ctx)
}

fn find_int(text: &str, key: &str) -> Option<i64> {
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                let v = v.trim().trim_matches('"');
                if let Ok(n) = v.parse::<i64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn extract_json_u64(json: &str, key: &str) -> u64 {
    let pat = format!("\"{key}\":");
    json.find(&pat)
        .and_then(|p| {
            let after = &json[p + pat.len()..];
            let num: String = after
                .chars()
                .skip_while(|c| c.is_whitespace())
                .take_while(|c| c.is_ascii_digit())
                .collect();
            num.parse().ok()
        })
        .unwrap_or(0)
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
