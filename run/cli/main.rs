//! run CLI entry point.

mod cmd;
mod util;

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
        "backends" => cmd::backends::run(),
        "graph"   => cmd::graph::run(args),
        "run"     => cmd::run::run(args),
        "status"  => cmd::status::run(),
        "bench"   => cmd::bench::run(args),
        "profile" => cmd::profile::run(args),
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
    println!("  bench <model> [--steps N] [--max-secs N]   phase breakdown + tok/s per backend");
    println!("  profile <model> [--steps N] [--backend X]  per-op time breakdown");
    println!("  graph <model> [--verify]         embed IR graph section into .model file");
    println!("  run <model> --prompt <text>      generate text from a model");
    println!("    options:");
    println!("      --max-tokens N               (default: unlimited — stops at EOS / turn boundary)");
    println!("      --temperature T              (default 0 = greedy)");
    println!("      --backend NAME               cpu|wgpu+rs|honeycrisp");
    println!("      --no-chat                    skip chat template, use raw prompt");
    println!("      --path=graph                 force graph executor path (slower, for testing)");
}
