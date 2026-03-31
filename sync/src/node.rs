//! Sync node: device sync over TCP.
//!
//! Each device runs one SyncNode. Nodes discover peers manually (add-peer)
//! or via broadcast. Chunks transfer as content-addressed blobs over TCP.
//! Transport is modular — swap to iroh/radio when ready.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

use crate::erasure;
use crate::store::{FileEntry, GSet};

/// Wire protocol message types.
const MSG_PING: u8 = 1;
const MSG_PONG: u8 = 2;
const MSG_LIST_FILES: u8 = 3;
const MSG_FILE_LIST: u8 = 4;
const MSG_GET_CHUNK: u8 = 5;
const MSG_CHUNK_DATA: u8 = 6;
const MSG_CHUNK_NOT_FOUND: u8 = 7;
const MSG_REGISTRY: u8 = 8;

/// Shared state between async tasks.
pub struct SharedState {
    pub registry: GSet,
    pub data_dir: PathBuf,
    pub k: usize,
    pub n: usize,
    pub peers: Vec<String>,
    /// Per-peer capacity allocation in bytes (mirrors peers vec by index).
    pub peer_capacities: Vec<u64>,
}

/// A sync node running on one device.
pub struct SyncNode {
    state: Arc<RwLock<SharedState>>,
    port: u16,
}

impl SyncNode {
    /// Start a new sync node.
    pub async fn start(data_dir: &Path, port: u16, k: usize, n: usize) -> Result<Self> {
        assert!(n.is_power_of_two());
        assert!(k >= 1 && k <= n);

        std::fs::create_dir_all(data_dir)?;
        std::fs::create_dir_all(data_dir.join("chunks"))?;

        // Load registry.
        let registry_path = data_dir.join("registry.json");
        let registry = if registry_path.exists() {
            let data = std::fs::read_to_string(&registry_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            GSet::new()
        };

        // Load peers.
        let peers_path = data_dir.join("peers.json");
        let peers: Vec<String> = if peers_path.exists() {
            let data = std::fs::read_to_string(&peers_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Load peer capacities.
        let caps_path = data_dir.join("capacities.json");
        let peer_capacities: Vec<u64> = if caps_path.exists() {
            let data = std::fs::read_to_string(&caps_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            vec![0; peers.len()]
        };

        let state = Arc::new(RwLock::new(SharedState {
            registry,
            data_dir: data_dir.to_path_buf(),
            k,
            n,
            peers,
            peer_capacities,
        }));

        println!("node started on port {}", port);
        println!("  dir: {}", data_dir.display());
        println!("  erasure: k={}, n={}", k, n);

        Ok(Self { state, port })
    }

    /// Run the daemon: listen for connections + background auto-sync.
    pub async fn run_daemon(&self, sync_interval_secs: u64) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", self.port)).await?;
        println!("listening on 0.0.0.0:{}", self.port);

        // Spawn TCP listener.
        let state = self.state.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let state = state.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, state).await {
                                eprintln!("connection from {} error: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => eprintln!("accept error: {}", e),
                }
            }
        });

        // Spawn background auto-sync.
        if sync_interval_secs > 0 {
            let state = self.state.clone();
            println!("auto-sync every {}s", sync_interval_secs);
            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(sync_interval_secs));
                interval.tick().await; // skip first immediate tick
                loop {
                    interval.tick().await;
                    if let Err(e) = background_sync(&state).await {
                        eprintln!("auto-sync error: {}", e);
                    }
                }
            });
        }

        println!("running... press ctrl-c to stop");
        tokio::signal::ctrl_c().await?;
        println!("\nshutting down...");
        self.save().await?;
        Ok(())
    }

    /// Add a file: erasure encode and store chunks with capacity-weighted placement.
    pub async fn put_file(&self, name: &str, data: &[u8]) -> Result<()> {
        let mut state = self.state.write().await;
        let shards = erasure::encode(data, state.k, state.n);

        let chunks_dir = state.data_dir.join("chunks");
        let mut shard_hashes = Vec::with_capacity(state.n);

        // Compute placement: which shards go to which device.
        let placement = capacity_weighted_placement(
            state.n,
            &state.peer_capacities,
        );

        for shard in &shards {
            let bytes = shard_to_bytes(shard);
            let hash = cyber_hemera::hash(&bytes);
            let hex = hash.to_hex();

            // Always store locally (this device is implicit peer 0).
            // Placement tells us which additional shards to keep vs skip.
            let keep_local = placement.get(shard.index).copied().unwrap_or(0) == 0;
            if keep_local || state.peers.is_empty() {
                let path = chunks_dir.join(&hex);
                std::fs::write(&path, &bytes)?;
            }

            shard_hashes.push(hex);
        }

        let entry = FileEntry {
            name: name.to_string(),
            original_len: data.len(),
            k: state.k,
            n: state.n,
            shard_hashes,
        };

        state.registry.insert(entry);
        self.save_registry(&state)?;

        let local_count = placement.iter().filter(|&&d| d == 0).count();
        println!(
            "stored '{}' ({} bytes, {} shards, {} local)",
            name,
            data.len(),
            state.n,
            local_count
        );
        Ok(())
    }

    /// Get a file: collect shards locally and from peers, then decode.
    pub async fn get_file(&self, name: &str) -> Result<Vec<u8>> {
        let state = self.state.read().await;
        let entry = state
            .registry
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("file '{}' not found", name))?
            .clone();
        let peers = state.peers.clone();
        let chunks_dir = state.data_dir.join("chunks");
        drop(state);

        let mut available_shards = Vec::new();

        for (idx, hash_hex) in entry.shard_hashes.iter().enumerate() {
            // Try local.
            let path = chunks_dir.join(hash_hex);
            if path.exists() {
                let bytes = std::fs::read(&path)?;
                available_shards.push(bytes_to_shard(idx, &bytes));
                if available_shards.len() >= entry.k {
                    break;
                }
                continue;
            }

            // Try peers.
            for peer in &peers {
                match fetch_chunk_from_peer(peer, hash_hex).await {
                    Ok(bytes) => {
                        // Cache locally.
                        std::fs::write(&path, &bytes)?;
                        available_shards.push(bytes_to_shard(idx, &bytes));
                        break;
                    }
                    Err(_) => continue,
                }
            }

            if available_shards.len() >= entry.k {
                break;
            }
        }

        if available_shards.len() < entry.k {
            anyhow::bail!(
                "only {} of {} required shards available for '{}'",
                available_shards.len(),
                entry.k,
                name
            );
        }

        Ok(erasure::decode(
            &available_shards,
            entry.k,
            entry.n,
            entry.original_len,
        ))
    }

    /// Sync with a peer: exchange registries, fetch missing chunks.
    pub async fn sync_with(&self, peer: &str) -> Result<()> {
        println!("syncing with {}...", peer);

        // Get peer's registry.
        let remote_registry = fetch_registry(peer).await?;
        let remote_count = remote_registry.len();

        // Merge.
        let mut state = self.state.write().await;
        let before = state.registry.len();
        state.registry.merge(&remote_registry);
        let after = state.registry.len();
        self.save_registry(&state)?;

        println!(
            "registry: {} local + {} remote → {} merged ({} new)",
            before,
            remote_count,
            after,
            after - before
        );

        // Fetch missing chunks.
        let chunks_dir = state.data_dir.join("chunks");
        let mut fetched = 0;

        for entry in state.registry.files.values() {
            for hash_hex in &entry.shard_hashes {
                let path = chunks_dir.join(hash_hex);
                if !path.exists() {
                    match fetch_chunk_from_peer(peer, hash_hex).await {
                        Ok(bytes) => {
                            std::fs::write(&path, &bytes)?;
                            fetched += 1;
                        }
                        Err(_) => {}
                    }
                }
            }
        }

        println!("fetched {} chunks from {}", fetched, peer);
        Ok(())
    }

    /// Add a peer address (host:port) with optional capacity in bytes.
    pub async fn add_peer(&self, addr: &str, capacity: u64) -> Result<()> {
        let mut state = self.state.write().await;
        if !state.peers.contains(&addr.to_string()) {
            state.peers.push(addr.to_string());
            state.peer_capacities.push(capacity);
            save_peers(&state)?;
            println!("added peer: {} (capacity: {})", addr, format_bytes(capacity));
        } else {
            println!("peer already known: {}", addr);
        }
        Ok(())
    }

    /// Sync with all known peers.
    pub async fn sync_all(&self) -> Result<()> {
        let state = self.state.read().await;
        let peers = state.peers.clone();
        drop(state);

        if peers.is_empty() {
            println!("no peers configured. use 'add-peer' first.");
            return Ok(());
        }

        for peer in &peers {
            if let Err(e) = self.sync_with(peer).await {
                eprintln!("sync with {} failed: {}", peer, e);
            }
        }
        Ok(())
    }

    /// List files.
    pub async fn list_files(&self) -> Vec<String> {
        let state = self.state.read().await;
        state.registry.list().iter().map(|s| s.to_string()).collect()
    }

    /// Status info.
    pub async fn status(&self) -> (usize, usize, usize, usize) {
        let state = self.state.read().await;
        let files = state.registry.len();
        let peers = state.peers.len();
        let chunks = std::fs::read_dir(state.data_dir.join("chunks"))
            .map(|d| d.count())
            .unwrap_or(0);
        (files, peers, chunks, self.port as usize)
    }

    pub async fn save(&self) -> Result<()> {
        let state = self.state.read().await;
        self.save_registry(&state)
    }

    fn save_registry(&self, state: &SharedState) -> Result<()> {
        let path = state.data_dir.join("registry.json");
        let json = serde_json::to_string_pretty(&state.registry)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

// ── Background auto-sync ──

async fn background_sync(state: &Arc<RwLock<SharedState>>) -> Result<()> {
    // Reload registry from disk first (CLI may have updated it).
    {
        let mut s = state.write().await;
        let path = s.data_dir.join("registry.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            if let Ok(disk_reg) = serde_json::from_str::<GSet>(&data) {
                s.registry.merge(&disk_reg);
            }
        }
    }

    let s = state.read().await;
    let peers = s.peers.clone();
    let chunks_dir = s.data_dir.join("chunks");
    drop(s);

    if peers.is_empty() {
        return Ok(());
    }

    let mut total_new_files = 0;
    let mut total_fetched = 0;

    for peer in &peers {
        let remote_registry = match fetch_registry(peer).await {
            Ok(r) => r,
            Err(_) => continue,
        };

        let mut s = state.write().await;
        let before = s.registry.len();
        s.registry.merge(&remote_registry);
        let after = s.registry.len();
        total_new_files += after - before;

        if after > before {
            let path = s.data_dir.join("registry.json");
            let json = serde_json::to_string_pretty(&s.registry)?;
            std::fs::write(path, json)?;
        }

        for entry in s.registry.files.values() {
            for hash_hex in &entry.shard_hashes {
                let path = chunks_dir.join(hash_hex);
                if !path.exists() {
                    if let Ok(bytes) = fetch_chunk_from_peer(peer, hash_hex).await {
                        std::fs::write(&path, &bytes)?;
                        total_fetched += 1;
                    }
                }
            }
        }
    }

    if total_new_files > 0 || total_fetched > 0 {
        println!(
            "[auto-sync] {} new files, {} chunks fetched",
            total_new_files, total_fetched
        );
    }
    Ok(())
}

// ── Capacity-weighted placement ──

/// Assign each shard to a device index (0 = local, 1..n = peers).
/// Devices with more capacity get more shards.
///
/// If no capacities are set (all zeros or empty), falls back to round-robin.
fn capacity_weighted_placement(n_shards: usize, peer_capacities: &[u64]) -> Vec<usize> {
    let n_devices = peer_capacities.len() + 1; // +1 for local device (index 0)
    if n_devices == 0 || n_shards == 0 {
        return vec![0; n_shards];
    }

    // Build capacity list with local device at index 0.
    // Local device has "infinite" capacity (u64::MAX) if no explicit cap.
    let mut caps: Vec<u64> = Vec::with_capacity(n_devices);
    caps.push(u64::MAX); // local always accepts
    caps.extend_from_slice(peer_capacities);

    let total_cap: u128 = caps.iter().map(|&c| c.max(1) as u128).sum();

    if total_cap == 0 {
        // Fallback: round-robin.
        return (0..n_shards).map(|i| i % n_devices).collect();
    }

    // Proportional allocation: device d gets floor(n_shards * cap_d / total_cap) shards.
    let mut alloc = vec![0usize; n_devices];
    let mut assigned = 0;
    for d in 0..n_devices {
        let share = (n_shards as u128 * caps[d].max(1) as u128 / total_cap) as usize;
        alloc[d] = share;
        assigned += share;
    }

    // Distribute remainder to largest-capacity devices.
    let mut remainder = n_shards.saturating_sub(assigned);
    let mut order: Vec<usize> = (0..n_devices).collect();
    order.sort_by(|&a, &b| caps[b].cmp(&caps[a]));
    for &d in &order {
        if remainder == 0 {
            break;
        }
        alloc[d] += 1;
        remainder -= 1;
    }

    // Build placement vector: shard i → device index.
    let mut placement = Vec::with_capacity(n_shards);
    for (device_idx, &count) in alloc.iter().enumerate() {
        for _ in 0..count {
            placement.push(device_idx);
        }
    }
    placement.truncate(n_shards);
    placement
}

fn save_peers(state: &SharedState) -> Result<()> {
    let path = state.data_dir.join("peers.json");
    let json = serde_json::to_string_pretty(&state.peers)?;
    std::fs::write(&path, json)?;
    let caps_path = state.data_dir.join("capacities.json");
    let json = serde_json::to_string_pretty(&state.peer_capacities)?;
    std::fs::write(caps_path, json)?;
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "unlimited".to_string();
    }
    const GB: u64 = 1_000_000_000;
    const MB: u64 = 1_000_000;
    const KB: u64 = 1_000;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ── Server: handle incoming connections ──

async fn handle_connection(mut stream: TcpStream, state: Arc<RwLock<SharedState>>) -> Result<()> {
    let msg_type = stream.read_u8().await?;

    match msg_type {
        MSG_PING => {
            stream.write_u8(MSG_PONG).await?;
        }
        MSG_LIST_FILES => {
            let state = state.read().await;
            let json = serde_json::to_string(&state.registry)?;
            stream.write_u8(MSG_FILE_LIST).await?;
            write_bytes(&mut stream, json.as_bytes()).await?;
        }
        MSG_GET_CHUNK => {
            let hash_hex = read_string(&mut stream).await?;
            let state = state.read().await;
            let path = state.data_dir.join("chunks").join(&hash_hex);
            if path.exists() {
                let data = std::fs::read(&path)?;
                stream.write_u8(MSG_CHUNK_DATA).await?;
                write_bytes(&mut stream, &data).await?;
            } else {
                stream.write_u8(MSG_CHUNK_NOT_FOUND).await?;
            }
        }
        MSG_REGISTRY => {
            // Always serve from disk — CLI processes may have updated it.
            let state = state.read().await;
            let path = state.data_dir.join("registry.json");
            let json = if path.exists() {
                std::fs::read_to_string(&path)?
            } else {
                serde_json::to_string(&state.registry)?
            };
            stream.write_u8(MSG_FILE_LIST).await?;
            write_bytes(&mut stream, json.as_bytes()).await?;
        }
        _ => {}
    }

    Ok(())
}

// ── Client: fetch from peers ──

async fn fetch_chunk_from_peer(peer: &str, hash_hex: &str) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect(peer)
        .await
        .context("failed to connect to peer")?;

    stream.write_u8(MSG_GET_CHUNK).await?;
    write_bytes(&mut stream, hash_hex.as_bytes()).await?;

    let response = stream.read_u8().await?;
    if response == MSG_CHUNK_DATA {
        read_bytes(&mut stream).await
    } else {
        anyhow::bail!("chunk not found on peer")
    }
}

async fn fetch_registry(peer: &str) -> Result<GSet> {
    let mut stream = TcpStream::connect(peer)
        .await
        .context("failed to connect to peer")?;

    stream.write_u8(MSG_REGISTRY).await?;

    let response = stream.read_u8().await?;
    if response == MSG_FILE_LIST {
        let data = read_bytes(&mut stream).await?;
        let json = String::from_utf8(data)?;
        Ok(serde_json::from_str(&json)?)
    } else {
        anyhow::bail!("unexpected response")
    }
}

// ── Wire helpers: length-prefixed messages ──

async fn write_bytes(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    stream.write_u32(data.len() as u32).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn read_bytes(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let len = stream.read_u32().await? as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn read_string(stream: &mut TcpStream) -> Result<String> {
    let bytes = read_bytes(stream).await?;
    Ok(String::from_utf8(bytes)?)
}

fn shard_to_bytes(shard: &erasure::Shard) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(shard.data.len() * 8);
    for &elem in &shard.data {
        bytes.extend_from_slice(&elem.as_u64().to_le_bytes());
    }
    bytes
}

fn bytes_to_shard(index: usize, bytes: &[u8]) -> erasure::Shard {
    let mut data = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks(8) {
        if chunk.len() == 8 {
            let val = u64::from_le_bytes(chunk.try_into().unwrap());
            data.push(nebu::Goldilocks::new(val));
        }
    }
    erasure::Shard { index, data }
}
