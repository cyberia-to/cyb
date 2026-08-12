//! Terminal face of cyb. Default network: space-pussy.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("help");
    match cmd {
        "-V" | "--version" | "version" => {
            println!("cy {}", env!("CARGO_PKG_VERSION"));
        }
        "network" | "net" => {
            let n = args
                .get(1)
                .and_then(|s| cyb::network::Network::parse(s))
                .unwrap_or_default();
            println!("network {}", n.chain_id());
            println!("  rpc  {}", n.rpc());
            println!("  lcd  {}", n.lcd());
            if n == cyb::network::Network::DEFAULT {
                println!("  (product default)");
            }
        }
        "help" | "-h" | "--help" => print_help(),
        _ if args.is_empty() => print_help(),
        other => {
            eprintln!("unknown `{other}` — try `cy help` or `soft3 sync`");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    let n = cyb::network::Network::DEFAULT;
    println!("cy {} — terminal face of cyb", env!("CARGO_PKG_VERSION"));
    println!();
    println!("default network: {} ({})", n.chain_id(), n.rpc());
    println!();
    println!("  cy network [space-pussy|bostrom]");
    println!("  cy version");
    println!();
    println!("sync / status of the default network:");
    println!("  soft3 sync                 # probes space-pussy RPC");
    println!("  soft3 sync --network bostrom");
    println!();
    println!("  cargo install soft3");
    println!("  cargo install cyb");
    println!("  docs https://cyber.page/install");
}
