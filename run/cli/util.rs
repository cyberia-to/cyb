//! Shared CLI utilities: model resolution, backend selection, formatting.

use run::backend::Backend;
use run::format::LoadedModel;
use run::arch::decoder::LlamaModel;
use run::generate::{sample, SampleConfig, SampleKind};
use run::tokenizer::build_tokenizer;
use std::path::PathBuf;
use std::time::Instant;

pub fn resolve_model_path(name: &str) -> PathBuf {
    let p = PathBuf::from(name);
    if p.exists() {
        return p;
    }
    if let Ok(home) = std::env::var("HOME") {
        let candidate = PathBuf::from(home).join("llm").join(format!("{name}.model"));
        if candidate.exists() {
            return candidate;
        }
    }
    p
}

pub fn pick_backend(name: &str) -> Box<dyn Backend> {
    match name {
        "cpu" => Box::new(run::backend::cpu::CpuBackend::new()),
        "wgpu+rs" | "wgpu" => match run::backend::wgpu::WgpuRsBackend::new() {
            Ok(b) => Box::new(b),
            Err(e) => {
                eprintln!("wgpu+rs unavailable ({e}), falling back to cpu");
                Box::new(run::backend::cpu::CpuBackend::new())
            }
        },
        #[cfg(target_os = "macos")]
        "honeycrisp" => match run::backend::honeycrisp::HoneycrispBackend::new() {
            Ok(b) => Box::new(b),
            Err(e) => {
                eprintln!("honeycrisp unavailable ({e}), falling back to cpu");
                Box::new(run::backend::cpu::CpuBackend::new())
            }
        },
        "auto" | "" => {
            #[cfg(target_os = "macos")]
            {
                if let Ok(b) = run::backend::honeycrisp::HoneycrispBackend::new() {
                    return Box::new(b);
                }
            }
            if let Ok(b) = run::backend::wgpu::WgpuRsBackend::new() {
                return Box::new(b);
            }
            Box::new(run::backend::cpu::CpuBackend::new())
        }
        other => {
            eprintln!("unknown backend: {other}");
            std::process::exit(2);
        }
    }
}

pub fn format_size(bytes: u64) -> String {
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

pub fn pad_bench(tok_s: &str, sane: &str, width: usize) -> String {
    let visible = visible_len(tok_s) + 2;
    let pad = width.saturating_sub(visible);
    format!("{:>pad$}{tok_s} {sane}", "", pad = pad)
}

pub fn visible_len(s: &str) -> usize {
    let mut count = 0usize;
    let mut in_esc = false;
    for c in s.chars() {
        if c == '\x1b' { in_esc = true; continue; }
        if in_esc { if c == 'm' { in_esc = false; } continue; }
        count += 1;
    }
    count
}

pub fn find_int(text: &str, key: &str) -> Option<i64> {
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

pub fn extract_json_u64(json: &str, key: &str) -> u64 {
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

pub fn bench_ollama(tag: &str) -> String {
    let body = format!(
        r#"{{"model":"{tag}","prompt":"What is 2+2? /no_think","stream":false,"options":{{"num_predict":64,"temperature":0}}}}"#
    );
    let out = std::process::Command::new("curl")
        .args(["-s", "--max-time", "30", "http://localhost:11434/api/generate", "-d", &body])
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

pub fn probe_arch(path: &std::path::Path) -> Result<(), String> {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let lm = LoadedModel::load(path).map_err(|e| format!("{e}"))?;
        LlamaModel::from_loaded(&lm).map(|_| ()).map_err(|e| format!("{e}"))
    }));
    std::panic::set_hook(prev);
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("panic during arch build".into()),
    }
}

pub fn short_reason(err: &str) -> String {
    if err.contains("q_proj.weight") && err.contains("declared shape") {
        return "layer-variant q_dim (gemma4)".into();
    }
    if err.contains("post_attention_norm") || err.contains("post_ffw_norm") || err.contains("layer_output_scale") {
        return "extra per-layer norms (gemma4)".into();
    }
    let trimmed = err.trim().replace('\n', " ");
    if trimmed.len() > 60 { format!("{}…", &trimmed[..60]) } else { trimmed }
}

pub fn read_header_meta(path: &std::path::Path) -> (usize, u64) {
    use std::io::Read;
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (0, 0),
    };
    let mut buf = vec![0u8; 2 * 1024 * 1024];
    let n = f.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    let text = String::from_utf8_lossy(&buf);
    let layers = find_int(&text, "num_hidden_layers").unwrap_or(0) as usize;
    let ctx = find_int(&text, "max_position_embeddings")
        .or_else(|| find_int(&text, "context_length"))
        .unwrap_or(0) as u64;
    (layers, ctx)
}

pub fn validate_math_answer(text: &str) -> &'static str {
    let text = text.trim().to_lowercase();
    let body = if let Some(pos) = text.find("</think>") { &text[pos + 8..] } else { &text[..] };
    let body = body.trim();
    let frag_patterns = ["|im_", "|im ", "|endof", "|user", "|assistant", "|fim"];
    let frag_count: usize = frag_patterns.iter().map(|p| body.matches(p).count()).sum();
    let has_four = body.contains('4') || body.contains("four");
    let repetitive = body.len() > 20 && {
        let first_20: String = body.chars().take(20).collect();
        body.matches(&first_20[..]).count() > 2
    };
    if body.is_empty() { "\x1b[31m✗\x1b[0m" }
    else if frag_count >= 3 || repetitive { "\x1b[31m✗\x1b[0m" }
    else if has_four && frag_count == 0 && !repetitive { "\x1b[32m✓\x1b[0m" }
    else if has_four { "\x1b[33m?\x1b[0m" }
    else { "\x1b[31m✗\x1b[0m" }
}

pub fn validate_code_answer(text: &str) -> &'static str {
    let t = text.trim();
    if t.is_empty() { return "\x1b[31m✗\x1b[0m"; }
    let frag_patterns = ["|im_", "|im ", "|endof", "|user", "|assistant"];
    let frag_count: usize = frag_patterns.iter().map(|p| t.matches(p).count()).sum();
    if frag_count >= 2 { return "\x1b[31m✗\x1b[0m"; }
    let repetitive = t.len() > 20 && {
        let first_20: String = t.chars().take(20).collect();
        t.matches(&first_20[..]).count() > 2
    };
    if repetitive { return "\x1b[31m✗\x1b[0m"; }
    let has_code = t.contains("    ") || t.contains('\t') || t.contains("return")
        || t.contains("if ") || t.contains("def ") || t.contains("fn ") || t.contains("=> ");
    if has_code { "\x1b[32m✓\x1b[0m" } else { "\x1b[33m?\x1b[0m" }
}

pub fn quick_bench(path: &std::path::Path, backend: &dyn Backend, eval: run::manifest::EvalKind) -> (String, String) {
    use run::tokenizer::ChatMessage;
    use run::manifest::EvalKind;
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
    let prompt = match eval {
        EvalKind::Math => {
            let msgs = vec![ChatMessage { role: "user".into(), content: "What is 2+2? /no_think".into() }];
            tok.apply_chat_template(&msgs, true)
        }
        EvalKind::Code => "def fibonacci(n):\n".into(),
    };
    let max_gen = match eval { EvalKind::Math => 32, EvalKind::Code => 48 };
    let cfg = SampleConfig { method: SampleKind::Greedy, temperature: 1.0, top_p: 0.95, top_k: 40 };
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
    let mut generated: Vec<u32> = Vec::with_capacity(max_gen);
    for _ in 0..max_gen {
        let next = sample(&logits, cfg);
        if tok.is_eos(next) { break; }
        generated.push(next);
        match model.forward(next, backend) {
            Ok(l) => logits = l,
            Err(_) => break,
        }
    }
    let dt = t_decode.elapsed().as_secs_f64();
    let tok_s = if !generated.is_empty() && dt > 0.01 {
        format!("{:.0}", generated.len() as f64 / dt)
    } else {
        "0".into()
    };
    let text = tok.decode(&generated, true);
    let quality = match eval {
        EvalKind::Math => validate_math_answer(&text),
        EvalKind::Code => validate_code_answer(&text),
    };
    (tok_s, quality.into())
}
