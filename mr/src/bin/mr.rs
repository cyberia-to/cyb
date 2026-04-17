//! mr CLI — list backends, run self-tests, eventually: import/run models.

use mr::backend::{Backend, BackendKind};

fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "backends" => list_backends(),
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
    println!("usage: mr <command>");
    println!();
    println!("commands:");
    println!("  backends    list available backends on this platform");
    println!("  help        show this help");
}

fn list_backends() {
    let cpu = mr::cpu::CpuBackend::new();
    println!("  {}  \t{}", cpu.kind().as_str(), "reference library, always available");

    match mr::wgpu_rs::WgpuRsBackend::new() {
        Ok(b) => println!("  {}  \t{}", b.kind().as_str(), "portable default"),
        Err(e) => println!("  {}  \t{}", BackendKind::WgpuRs.as_str(), format!("unavailable: {e}")),
    }

    #[cfg(target_os = "macos")]
    {
        match mr::honeycrisp::HoneycrispBackend::new() {
            Ok(b) => println!("  {}  \t{}", b.kind().as_str(), "Apple Silicon turbo"),
            Err(e) => println!(
                "  {}  \t{}",
                BackendKind::Honeycrisp.as_str(),
                format!("unavailable: {e}")
            ),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        println!("  honeycrisp\tmacOS-only");
    }

    println!("  nox       \tfuture (trident-compiled bytecode VM)");
}
