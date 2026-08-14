//! Terminal face of cyb. Default network: spacepussy-test.

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
                println!("  (product default — soft3 chaosnet)");
            }
        }
        "help" | "-h" | "--help" => print_help(),
        _ if args.is_empty() => print_help(),
        other => {
            eprintln!("unknown `{other}` — try `cy help` or `cyber sync`");
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
    println!("  soft3 chaosnet — not cosmos space-pussy on cybernode");
    println!();
    println!("  cy network [spacepussy-test]");
    println!("  cy version");
    println!();
    println!("product probe:");
    println!("  cyber sync                 # true-cyber → spacepussy-test");
    println!();
    println!("  cargo install true-cyber");
    println!("  docs https://cyber.page/install");
}
