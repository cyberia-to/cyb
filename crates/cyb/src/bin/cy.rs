fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("cy {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    println!("cy {} — terminal face of cyb", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  cargo install cyb");
    println!("  docs  https://cyber.page/soft3/");
    println!("  code  https://github.com/cyberia-to/cyb");
    println!();
    println!("Full REPL (fund/earn/settle) ships as dependency train publishes.");
    println!("Library: use cyb = \"{}\" in Cargo.toml", env!("CARGO_PKG_VERSION"));
}
