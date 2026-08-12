//! Terminal face of cyb — product entry. Full REPL expands with mudra/wallet features.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("cy {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.first().map(|s| s.as_str()) == Some("version") {
        println!("cy {} (lib cyb {})", env!("CARGO_PKG_VERSION"), cyb::VERSION);
        return;
    }
    println!("cy {} — terminal face of cyb", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  library: cyb {} — chroma, signals, money wallet helpers", cyb::VERSION);
    println!("  stack:   cybergraph · foculus · bbg · cyber-tru · cyber-tok");
    println!();
    println!("  cargo add cyb");
    println!("  cargo install cyb");
    println!("  docs https://cyber.page/soft3/");
    println!();
    println!("Full fund/earn REPL ships next; core runtime is on crates.io.");
}
