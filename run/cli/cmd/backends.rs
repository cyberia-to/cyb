use run::backend::{Backend, BackendKind};

pub fn run() {
    let cpu = run::backend::cpu::CpuBackend::new();
    println!("  {}         reference library, always available", cpu.kind().as_str());
    match run::backend::wgpu::WgpuRsBackend::new() {
        Ok(b) => println!("  {}     portable default", b.kind().as_str()),
        Err(e) => println!("  {}     unavailable: {e}", BackendKind::WgpuRs.as_str()),
    }
    #[cfg(target_os = "macos")]
    {
        match run::backend::honeycrisp::HoneycrispBackend::new() {
            Ok(b) => println!("  {}   Apple Silicon turbo", b.kind().as_str()),
            Err(e) => println!("  {}   unavailable: {e}", BackendKind::Honeycrisp.as_str()),
        }
    }
    #[cfg(not(target_os = "macos"))]
    println!("  honeycrisp   macOS-only");
    println!("  nox          future (trident-compiled bytecode VM)");
}
