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
        "wire" => {
            // the milestone: two cells converge over radio, no server between
            let rt = tokio::runtime::Runtime::new().expect("tokio");
            let sub = args.get(1).map(|s| s.as_str()).unwrap_or("");
            let res = match sub {
                "listen" => rt.block_on(cyb::wire::listen(args.get(2).map(|s| s.as_str()))),
                "dial" => {
                    let id = args.get(2).cloned().unwrap_or_default();
                    // sockets are every ip:port arg; an arg without ':' is the cell path
                    let socks: Vec<String> =
                        args[3..].iter().filter(|a| a.contains(':')).cloned().collect();
                    let cell = args[3..].iter().find(|a| !a.contains(':')).cloned();
                    rt.block_on(cyb::wire::dial(&id, &socks, cell.as_deref()))
                }
                _ => {
                    eprintln!("usage:");
                    eprintln!("  cy wire listen [cell.log]");
                    eprintln!("  cy wire dial <id> <ip:port>… [cell.log]");
                    std::process::exit(2);
                }
            };
            if let Err(e) = res {
                eprintln!("wire: {e:#}");
                std::process::exit(1);
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
    println!("  cy wire listen [cell.log]      # hold a cell open for peers");
    println!("  cy wire dial <id> <ip:port>…   # converge with a listening cell");
    println!("  cy version");
    println!();
    println!("product probe:");
    println!("  cyber sync                 # true-cyber → spacepussy-test");
    println!();
    println!("  cargo install true-cyber");
    println!("  docs https://cyber.page/install");
}
