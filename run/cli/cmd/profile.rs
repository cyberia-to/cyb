use crate::util::{pick_backend, resolve_model_path};
use run::arch::decoder::LlamaModel;
use run::format::LoadedModel;

pub fn run(args: Vec<String>) {
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
            "--steps" => { i += 1; steps = args[i].parse().unwrap_or(8); }
            "--backend" => { i += 1; backend_name = args[i].clone(); }
            other => { eprintln!("unknown flag: {other}"); std::process::exit(2); }
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
        path.display(), backend.kind().as_str(), steps
    );
    println!();

    let lm = LoadedModel::load(&path).expect("load");
    let mut model = LlamaModel::from_loaded(&lm).expect("build");
    model.to_backend(&*backend).expect("upload");
    model.enable_prof();

    let _ = model.forward(0, &*backend).expect("warmup");
    model.enable_prof();

    for i in 0..steps {
        let tok = (i + 1) as u32;
        let _ = model.forward(tok, &*backend).expect("forward");
    }

    println!("{}", model.prof.summary());
    println!();
}
