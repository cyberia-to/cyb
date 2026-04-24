use crate::util::{bench_ollama, format_size, pad_bench, probe_arch, quick_bench, read_header_meta, short_reason};
use run::manifest::MANIFEST;
use std::time::Instant;

pub fn run() {
    let base = run::manifest::models_dir();
    println!();
    println!("  \x1b[1mmr status\x1b[0m — {}", base.display());
    println!();
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

        let disk_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let t0 = Instant::now();
        let (layers, ctx) = read_header_meta(&path);
        let load_ms = t0.elapsed().as_millis();

        let layers_str = if layers == 0 { "—".into() } else { layers.to_string() };
        let ctx_str = if ctx == 0 { "—".into() }
            else if ctx >= 1000 { format!("{}K", ctx / 1000) }
            else { ctx.to_string() };

        let clear = "\x1b[2K\r";
        eprint!("{clear}  probe {}...", e.name);
        let probe = probe_arch(&path);
        if let Err(reason) = probe {
            eprint!("{clear}");
            let unsupported_msg = format!("\x1b[33munsupported\x1b[0m: {}", short_reason(&reason));
            let llama_str = match e.ollama_tag {
                Some(tag) => format!("{:>6}", bench_ollama(tag)),
                None => format!("{:>6}", "—"),
            };
            println!(
                "  {:<24} {:<12} {:<8} {:>6} {:>3} {:>5} {:>6}ms  {}  {}",
                e.name, e.family, e.role, format_size(disk_bytes),
                layers_str, ctx_str, load_ms, unsupported_msg, llama_str
            );
            continue;
        }

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
            let b = run::backend::cpu::CpuBackend::new();
            quick_bench(&path, &b)
        });
        let wgpu_str = bench_col("wgpu+rs", 9, &|| match run::backend::wgpu::WgpuRsBackend::new() {
            Ok(b) => quick_bench(&path, &b),
            Err(_) => ("—".into(), "—".into()),
        });
        #[cfg(target_os = "macos")]
        let honey_str = bench_col("honeycrisp", 10, &|| match run::backend::honeycrisp::HoneycrispBackend::new() {
            Ok(b) => quick_bench(&path, &b),
            Err(_) => ("—".into(), "—".into()),
        });
        #[cfg(not(target_os = "macos"))]
        let honey_str = format!("{:>10}", "—");

        let llama_str = match e.ollama_tag {
            Some(tag) => { eprint!("{clear}  bench llama {}...", e.name); format!("{:>6}", bench_ollama(tag)) }
            None => format!("{:>6}", "—"),
        };

        eprint!("{clear}");
        println!(
            "  {:<24} {:<12} {:<8} {:>6} {:>3} {:>5} {:>6}ms {} {} {} {}",
            e.name, e.family, e.role, format_size(disk_bytes),
            layers_str, ctx_str, load_ms, cpu_str, wgpu_str, honey_str, llama_str
        );
    }
    println!("  {}", "─".repeat(width));
    println!();
    println!("  legend: \x1b[32m✓\x1b[0m clean answer  ·  \x1b[33m?\x1b[0m fragmented/rambling  ·  \x1b[31m✗\x1b[0m garbled");
    println!();
}
