use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use cyber_sync::node::SyncNode;

#[derive(Parser)]
#[command(name = "cyber-sync", about = "erasure-coded device sync over iroh QUIC")]
struct Cli {
    /// Data directory for this device.
    #[arg(short, long, default_value = "~/.cyber-sync")]
    dir: String,

    /// QUIC port (fixed for stable address).
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
    /// Start the sync daemon with background auto-sync.
    Daemon {
        /// Auto-sync interval in seconds (0 = disabled).
        #[arg(short, long, default_value = "30")]
        interval: u64,
    },

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

    /// Delete a file (tombstone).
    Rm { name: String },

    /// List files.
    Ls,

    /// Show status.
    Status,

    /// Add a peer by node ID.
    AddPeer {
        /// Peer node ID (public key hex from `cyber-sync status`).
        node_id: String,
        /// Peer storage capacity (e.g. 50GB, 100MB). 0 = unlimited.
        #[arg(short, long, default_value = "0")]
        capacity: String,
    },

    /// Sync with all peers (or a specific one).
    Sync {
        /// Specific peer node ID. Omit to sync with all.
        peer: Option<String>,
    },

    /// Garbage collect orphaned chunks.
    Gc,

    /// Integrity audit: hash-check all local chunks.
    Audit,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let dir = expand_tilde(&cli.dir);
    let node = SyncNode::start(&dir, cli.k, cli.n, cli.port).await?;

    match cli.command {
        Command::Daemon { interval } => {
            node.run_daemon(interval).await?;
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
        Command::Rm { name } => {
            node.delete_file(&name).await?;
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
            let (files, peers, chunks, _) = node.status().await;
            println!("node:   {}", node.node_id());
            println!("files:  {}", files);
            println!("peers:  {}", peers);
            println!("chunks: {}", chunks);
            println!("erasure: k={}, n={}", cli.k, cli.n);
        }
        Command::AddPeer { node_id, capacity } => {
            let cap = parse_capacity(&capacity)?;
            node.add_peer(&node_id, cap).await?;
        }
        Command::Sync { peer } => {
            if let Some(p) = peer {
                node.sync_with(&p).await?;
            } else {
                node.sync_all().await?;
            }
        }
        Command::Gc => {
            node.gc().await?;
        }
        Command::Audit => {
            node.audit().await?;
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

fn parse_capacity(s: &str) -> Result<u64> {
    let s = s.trim().to_uppercase();
    if s == "0" || s.is_empty() {
        return Ok(0);
    }
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("GB") {
        (n, 1_000_000_000u64)
    } else if let Some(n) = s.strip_suffix("MB") {
        (n, 1_000_000u64)
    } else if let Some(n) = s.strip_suffix("KB") {
        (n, 1_000u64)
    } else if let Some(n) = s.strip_suffix('B') {
        (n, 1u64)
    } else {
        (s.as_str(), 1u64)
    };
    let num: f64 = num_str.trim().parse()?;
    Ok((num * multiplier as f64) as u64)
}
