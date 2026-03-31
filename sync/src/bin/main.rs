use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use cyber_sync::node::SyncNode;

#[derive(Parser)]
#[command(name = "cyber-sync", about = "erasure-coded device sync")]
struct Cli {
    /// Data directory for this device.
    #[arg(short, long, default_value = "~/.cyber-sync")]
    dir: String,

    /// Listen port.
    #[arg(short, long, default_value = "4200")]
    port: u16,

    /// Data shards (k). Any k of n shards reconstruct the file.
    #[arg(short, long, default_value = "2")]
    k: usize,

    /// Total shards (n). Must be power of 2.
    #[arg(short, long, default_value = "4")]
    n: usize,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the sync daemon.
    Daemon,

    /// Store a file.
    Put {
        /// Path to the file.
        file: PathBuf,
        /// Name in registry (defaults to filename).
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Retrieve a file.
    Get {
        /// Name in registry.
        name: String,
        /// Output path.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List files.
    Ls,

    /// Show status.
    Status,

    /// Add a peer (host:port).
    AddPeer {
        /// Peer address.
        addr: String,
    },

    /// Sync with all peers (or a specific one).
    Sync {
        /// Specific peer (host:port). Omit to sync with all.
        peer: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let dir = expand_tilde(&cli.dir);
    let node = SyncNode::start(&dir, cli.port, cli.k, cli.n).await?;

    match cli.command {
        Command::Daemon => {
            node.run_daemon().await?;
        }
        Command::Put { file, name } => {
            let data = std::fs::read(&file)?;
            let name = name.unwrap_or_else(|| {
                file.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });
            node.put_file(&name, &data).await?;
        }
        Command::Get { name, output } => {
            let data = node.get_file(&name).await?;
            if let Some(path) = output {
                std::fs::write(&path, &data)?;
                println!("wrote {} bytes to {}", data.len(), path.display());
            } else {
                use std::io::Write;
                std::io::stdout().write_all(&data)?;
            }
        }
        Command::Ls => {
            let files = node.list_files().await;
            if files.is_empty() {
                println!("(no files)");
            } else {
                for f in &files {
                    println!("{}", f);
                }
            }
        }
        Command::Status => {
            let (files, peers, chunks, port) = node.status().await;
            println!("port:   {}", port);
            println!("files:  {}", files);
            println!("peers:  {}", peers);
            println!("chunks: {}", chunks);
            println!("erasure: k={}, n={}", cli.k, cli.n);
        }
        Command::AddPeer { addr } => {
            node.add_peer(&addr).await?;
        }
        Command::Sync { peer } => {
            if let Some(p) = peer {
                node.sync_with(&p).await?;
            } else {
                let (_, peers_count, _, _) = node.status().await;
                if peers_count == 0 {
                    println!("no peers configured. use 'add-peer' first.");
                } else {
                    let state = node.list_files().await;
                    drop(state);
                    // Read peers from state
                    println!("syncing with all peers...");
                    // For now, sync with each peer sequentially
                    // This is a placeholder — full impl would read peers list
                    println!("use 'sync <host:port>' to sync with a specific peer");
                }
            }
        }
    }

    node.save().await?;
    Ok(())
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}
