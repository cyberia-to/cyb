use crate::util::{bench_ollama, quick_bench, resolve_model_path};
use run::backend::Backend;

/// Default time budget for subsequent forward steps, per backend.
/// After the warmup first-forward, step count is capped so total
/// subsequent time stays under this limit. Keeps bench fast on large models.
const DEFAULT_BUDGET_SECS: f64 = 20.0;

/// Run a single backend and print one result line. Called by the subprocess path.
fn run_one_backend(name: &str, path: &std::path::Path, steps: usize, budget_secs: f64) {
    let make_result: Result<Box<dyn Backend>, String> = match name {
        "cpu" => Ok(Box::new(run::backend::cpu::CpuBackend::new())),
        "wgpu+rs" => run::backend::wgpu::WgpuRsBackend::new()
            .map(|b| Box::new(b) as Box<dyn Backend>)
            .map_err(|e| format!("{e}")),
        #[cfg(target_os = "macos")]
        "honeycrisp" => run::backend::honeycrisp::HoneycrispBackend::new()
            .map(|b| Box::new(b) as Box<dyn Backend>)
            .map_err(|e| format!("{e}")),
        other => Err(format!("unknown backend {other}")),
    };
    match make_result {
        Err(e) => println!("  {:<12}  \x1b[31munavailable ({e})\x1b[0m", name),
        Ok(b) => match run::bench::bench_e2e(path, &*b, steps, budget_secs) {
            Err(e) => println!("  {:<12}  \x1b[31merror ({e})\x1b[0m", name),
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

pub fn run(args: Vec<String>) {
    if args.is_empty() {
        eprintln!("usage: mr bench <model> [--steps N] [--max-secs N] [--backend cpu|wgpu|honeycrisp]");
        std::process::exit(2);
    }
    let model_arg = &args[0];
    let mut steps: usize = 10;
    let mut budget_secs: f64 = DEFAULT_BUDGET_SECS;
    let mut only_backend: Option<String> = None;
    // Internal flag: run exactly one backend inline (subprocess mode, no header printed).
    let mut subprocess_backend: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--steps"    => { i += 1; steps = args[i].parse().unwrap_or(10); }
            "--max-secs" => { i += 1; budget_secs = args[i].parse().unwrap_or(DEFAULT_BUDGET_SECS); }
            "--backend"  => { i += 1; only_backend = Some(args[i].clone()); }
            "--subprocess-backend" => { i += 1; subprocess_backend = Some(args[i].clone()); }
            other => { eprintln!("unknown flag: {other}"); std::process::exit(2); }
        }
        i += 1;
    }
    let path = resolve_model_path(model_arg);
    if !path.exists() {
        eprintln!("model not found: {}", path.display());
        std::process::exit(1);
    }

    // Subprocess mode: run one backend, print one line, exit. Memory is isolated.
    if let Some(ref bname) = subprocess_backend {
        run_one_backend(bname, &path, steps, budget_secs);
        return;
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

    let all_backends = {
        let mut v = vec!["cpu", "wgpu+rs"];
        #[cfg(target_os = "macos")]
        v.push("honeycrisp");
        v
    };

    let exe = std::env::current_exe().expect("current_exe");

    for name in all_backends {
        if let Some(ref only) = only_backend {
            if !name.starts_with(only.as_str()) && !only.starts_with(name) {
                continue;
            }
        }
        // Spawn a fresh subprocess for each backend so its entire memory footprint
        // (model weights, GPU buffers) is released by the OS before the next backend
        // starts. This prevents OOM when benchmarking large models across all backends.
        let mut child_args = vec![
            "bench".to_string(),
            path.to_string_lossy().to_string(),
            "--subprocess-backend".to_string(), name.to_string(),
            "--steps".to_string(), steps.to_string(),
            "--max-secs".to_string(), format!("{budget_secs}"),
        ];
        // Forward any env vars that affect behavior (HC_TIMING, FUSED_DEBUG, etc.)
        let status = std::process::Command::new(&exe)
            .args(&child_args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status();
        if let Err(e) = status {
            println!("  {:<12}  \x1b[31mfailed to spawn subprocess: {e}\x1b[0m", name);
        }
        // Non-zero exit from subprocess already printed its error line; nothing extra needed.
        let _ = child_args; // suppress unused warning
    }
    println!();
}

pub fn quick(path: &std::path::Path, backend: &dyn Backend) -> (String, String) {
    quick_bench(path, backend, run::manifest::EvalKind::Math)
}

pub fn ollama(tag: &str) -> String {
    bench_ollama(tag)
}
