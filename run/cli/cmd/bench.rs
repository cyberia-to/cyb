use crate::util::{bench_ollama, quick_bench, resolve_model_path};
use run::backend::Backend;

/// Default time budget for subsequent forward steps, per backend.
/// After the warmup first-forward, step count is capped so total
/// subsequent time stays under this limit. Keeps bench fast on large models.
const DEFAULT_BUDGET_SECS: f64 = 20.0;

pub fn run(args: Vec<String>) {
    if args.is_empty() {
        eprintln!("usage: mr bench <model> [--steps N] [--max-secs N]");
        std::process::exit(2);
    }
    let model_arg = &args[0];
    let mut steps: usize = 10;
    let mut budget_secs: f64 = DEFAULT_BUDGET_SECS;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--steps" => { i += 1; steps = args[i].parse().unwrap_or(10); }
            "--max-secs" => { i += 1; budget_secs = args[i].parse().unwrap_or(DEFAULT_BUDGET_SECS); }
            other => { eprintln!("unknown flag: {other}"); std::process::exit(2); }
        }
        i += 1;
    }
    let path = resolve_model_path(model_arg);
    if !path.exists() {
        eprintln!("model not found: {}", path.display());
        std::process::exit(1);
    }
    println!();
    println!(
        "  \x1b[1mmr bench\x1b[0m — {} (up to {} steps, {}s budget/backend)",
        path.display(), steps, budget_secs as u64,
    );
    println!();
    println!(
        "  {:<12}  {:<8}  {:<10}  {:<10}  {:<12}  {:<8}",
        "BACKEND", "LOAD", "UPLOAD", "FIRST-FWD", "AVG-FWD", "TOK/S"
    );
    println!("  {}", "─".repeat(74));

    let backends_to_try: Vec<(&str, Box<dyn Fn() -> Result<Box<dyn Backend>, String>>)> = {
        let mut v: Vec<(&str, Box<dyn Fn() -> Result<Box<dyn Backend>, String>>)> = vec![];
        v.push(("cpu", Box::new(|| Ok(Box::new(run::backend::cpu::CpuBackend::new()) as Box<dyn Backend>))));
        v.push(("wgpu+rs", Box::new(|| {
            run::backend::wgpu::WgpuRsBackend::new()
                .map(|b| Box::new(b) as Box<dyn Backend>)
                .map_err(|e| format!("{e}"))
        })));
        #[cfg(target_os = "macos")]
        v.push(("honeycrisp", Box::new(|| {
            run::backend::honeycrisp::HoneycrispBackend::new()
                .map(|b| Box::new(b) as Box<dyn Backend>)
                .map_err(|e| format!("{e}"))
        })));
        v
    };

    for (name, make) in backends_to_try {
        match make() {
            Err(e) => println!("  {:<12}  \x1b[31munavailable\x1b[0m ({e})", name),
            Ok(b) => match run::bench::bench_e2e(&path, &*b, steps, budget_secs) {
                Err(e) => println!("  {:<12}  \x1b[31merror\x1b[0m ({e})", name),
                Ok(bench) => {
                    let n = bench.subsequent_forwards_ms.len();
                    let avg = bench.avg_forward_ms();
                    println!(
                        "  {:<12}  {:>6.0}ms  {:>6.0}ms  {:>6.0}ms   {:>6.1}ms/tok  {:>5.1} tok/s  ({}steps)",
                        name, bench.load_ms, bench.to_backend_ms,
                        bench.first_forward_ms, avg, 1000.0 / avg, n,
                    );
                }
            },
        }
    }
    println!();
}

pub fn quick(path: &std::path::Path, backend: &dyn Backend) -> (String, String) {
    quick_bench(path, backend)
}

pub fn ollama(tag: &str) -> String {
    bench_ollama(tag)
}
