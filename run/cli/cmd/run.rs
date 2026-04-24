use crate::util::{pick_backend, resolve_model_path};
use run::arch::decoder::LlamaModel;
use run::backend::Backend;
use run::format::LoadedModel;
use run::generate::{generate, SampleConfig, SampleKind};
use run::tokenizer::{build_tokenizer, ChatMessage};
use std::time::Instant;

pub fn run(args: Vec<String>) {
    if args.is_empty() {
        eprintln!("usage: mr run <model> [--prompt TEXT] [--max-tokens N] ...");
        std::process::exit(2);
    }
    let model_arg = &args[0];
    let mut prompt = String::new();
    let mut max_tokens: usize = usize::MAX;
    let mut temperature: f32 = 0.0;
    let mut backend_name: String = "auto".into();
    let mut use_chat = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--prompt" => { i += 1; prompt = args[i].clone(); }
            "--max-tokens" => { i += 1; max_tokens = args[i].parse().unwrap_or(usize::MAX); }
            "--temperature" => { i += 1; temperature = args[i].parse().unwrap_or(0.0); }
            "--backend" => { i += 1; backend_name = args[i].clone(); }
            "--no-chat" => use_chat = false,
            other => { eprintln!("unknown flag: {other}"); std::process::exit(2); }
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
    let lm = LoadedModel::load(&path).unwrap_or_else(|e| { eprintln!("load failed: {e}"); std::process::exit(1); });
    let mut model = LlamaModel::from_loaded(&lm).unwrap_or_else(|e| { eprintln!("model build failed: {e}"); std::process::exit(1); });
    let tok = build_tokenizer(&lm).unwrap_or_else(|e| { eprintln!("tokenizer build failed: {e}"); std::process::exit(1); });
    println!(
        "Loaded in {:.1}s  [{}, {} layers, hidden={}, heads={}/{}, head_dim={}, qk_norm={}]",
        t_load.elapsed().as_secs_f64(),
        model.config.model_type, model.config.num_hidden_layers,
        model.config.hidden_size, model.config.num_attention_heads,
        model.config.num_key_value_heads, model.config.head_dim,
        model.config.has_qk_norm,
    );

    let final_prompt = if use_chat {
        tok.apply_chat_template(&[ChatMessage { role: "user".into(), content: prompt.clone() }], true)
    } else {
        prompt.clone()
    };

    let cfg = SampleConfig {
        method: if temperature <= 0.0 { SampleKind::Greedy } else { SampleKind::TopP },
        temperature: if temperature <= 0.0 { 1.0 } else { temperature },
        top_p: 0.95,
        top_k: 40,
    };

    let backend: Box<dyn Backend> = pick_backend(&backend_name);
    println!("Backend: {}", backend.kind().as_str());
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
        Err(e) => { eprintln!("generate failed: {e}"); std::process::exit(1); }
    }
}
