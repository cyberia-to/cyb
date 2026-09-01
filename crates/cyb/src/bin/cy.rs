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
            // `cy wire up [<peer-id> <ip:port>…] [cell.log]` — one verb: a
            // node is always both sides. The peer id (64 hex) marks the
            // bootstrap; args with ':' are its sockets; the rest is the cell.
            let res = match sub {
                "up" | "listen" | "dial" => {
                    let rest = &args[2..];
                    let peer = rest.iter().find(|a| a.len() == 64 && a.chars().all(|c| c.is_ascii_hexdigit())).cloned();
                    let socks: Vec<String> = rest.iter().filter(|a| a.contains(':')).cloned().collect();
                    let cell = rest
                        .iter()
                        .find(|a| !a.contains(':') && Some(*a) != peer.as_ref().map(|p| p).map(|p| p))
                        .filter(|a| a.len() != 64)
                        .cloned();
                    let bootstrap = peer.map(|p| (p, socks));
                    rt.block_on(cyb::wire::up(cell.as_deref(), bootstrap))
                }
                _ => {
                    eprintln!("usage:");
                    eprintln!("  cy wire up [cell.log]                     # hold a cell open");
                    eprintln!("  cy wire up <peer-id> <ip:port> [cell.log] # first contact; after that the graph dials");
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
    println!("  cy wire up [cell.log]                     # a cell on the wire");
    println!("  cy wire up <peer-id> <ip:port> [cell.log] # bootstrap; then the graph dials");
    println!("  cy version");
    println!();
    println!("product probe:");
    println!("  cyber sync                 # true-cyber → spacepussy-test");
    println!();
    println!("  cargo install true-cyber");
    println!("  docs https://cyber.page/install");
}
