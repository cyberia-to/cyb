use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::State;
use uhash_core::{lithium_header, UniversalHash};

/// Parameters passed to start_mining, stored for JS resume after component remount.
#[derive(Clone, Serialize, Default)]
pub struct MiningParams {
    pub address: String,
    pub block_hash_hex: String,
    pub cyberlinks_merkle_hex: String,
    pub difficulty: u32,
    pub epoch_id: u64,
    pub block_timestamp: u64,
}

pub struct MiningState {
    mining: AtomicBool,
    hash_count: AtomicU64,
    start_time: Mutex<Option<Instant>>,
    pending_proofs: Mutex<Vec<FoundProof>>,
    params: Mutex<Option<MiningParams>>,
}

#[derive(Clone, Serialize)]
pub struct FoundProof {
    pub hash: String,
    pub nonce: u64,
}

impl MiningState {
    pub fn new() -> Self {
        Self {
            mining: AtomicBool::new(false),
            hash_count: AtomicU64::new(0),
            start_time: Mutex::new(None),
            pending_proofs: Mutex::new(Vec::new()),
            params: Mutex::new(None),
        }
    }
}

fn meets_difficulty(hash: &[u8], difficulty: u32) -> bool {
    let mut leading_zeros = 0u32;
    for byte in hash {
        if *byte == 0 {
            leading_zeros += 8;
        } else {
            leading_zeros += byte.leading_zeros();
            break;
        }
    }
    leading_zeros >= difficulty
}

fn decode_hex_32(label: &str, hex_str: &str) -> Result<[u8; 32], String> {
    let raw = hex::decode(hex_str).map_err(|e| format!("invalid {} hex: {}", label, e))?;
    if raw.len() != 32 {
        return Err(format!(
            "{} must be exactly 32 bytes (got {})",
            label,
            raw.len()
        ));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&raw);
    Ok(out)
}

#[tauri::command]
pub fn start_mining(
    address: String,
    block_hash_hex: String,
    cyberlinks_merkle_hex: String,
    difficulty: u32,
    epoch_id: Option<u64>,
    block_timestamp: Option<u64>,
    threads: Option<u32>,
    state: State<Arc<MiningState>>,
) -> serde_json::Value {
    if state.mining.load(Ordering::SeqCst) {
        return serde_json::json!({ "success": false, "error": "Already mining" });
    }

    let block_hash = match decode_hex_32("block_hash", &block_hash_hex) {
        Ok(b) => b,
        Err(e) => return serde_json::json!({ "success": false, "error": e }),
    };
    let cyberlinks_merkle = match decode_hex_32("cyberlinks_merkle", &cyberlinks_merkle_hex) {
        Ok(b) => b,
        Err(e) => return serde_json::json!({ "success": false, "error": e }),
    };

    // Pre-compute the 32-byte header: SHA256(address || block_hash || cyberlinks_merkle)
    let header = lithium_header(&address, &block_hash, &cyberlinks_merkle);

    // Store params so JS can restore refs after component remount
    *state.params.lock().unwrap() = Some(MiningParams {
        address: address.clone(),
        block_hash_hex: block_hash_hex.clone(),
        cyberlinks_merkle_hex: cyberlinks_merkle_hex.clone(),
        difficulty,
        epoch_id: epoch_id.unwrap_or(0),
        block_timestamp: block_timestamp.unwrap_or(0),
    });

    state.mining.store(true, Ordering::SeqCst);
    state.hash_count.store(0, Ordering::SeqCst);
    *state.start_time.lock().unwrap() = Some(Instant::now());
    state.pending_proofs.lock().unwrap().clear();

    let num_threads = threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4)
    });

    let state_clone = state.inner().clone();

    for thread_id in 0..num_threads {
        let mining_flag = state_clone.clone();

        std::thread::spawn(move || {
            let mut hasher = UniversalHash::new();
            let mut nonce: u64 = thread_id as u64;

            while mining_flag.mining.load(Ordering::Relaxed) {
                // Lithium v1 input: header(32) || nonce_le(8) = 40 bytes
                let mut input = [0u8; 40];
                input[..32].copy_from_slice(&header);
                input[32..].copy_from_slice(&nonce.to_le_bytes());
                let hash = hasher.hash(&input);

                mining_flag.hash_count.fetch_add(1, Ordering::Relaxed);

                if meets_difficulty(&hash, difficulty) {
                    let proof = FoundProof {
                        hash: hex::encode(hash),
                        nonce,
                    };
                    mining_flag.pending_proofs.lock().unwrap().push(proof);
                }

                nonce += num_threads as u64;
            }
        });
    }

    serde_json::json!({ "success": true, "threads": num_threads })
}

#[tauri::command]
pub fn stop_mining(state: State<Arc<MiningState>>) -> serde_json::Value {
    state.mining.store(false, Ordering::SeqCst);
    *state.params.lock().unwrap() = None;

    let elapsed = state
        .start_time
        .lock()
        .unwrap()
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(0.0);

    let count = state.hash_count.load(Ordering::SeqCst);
    let hashrate = if elapsed > 0.0 {
        count as f64 / elapsed
    } else {
        0.0
    };

    serde_json::json!({
        "success": true,
        "total_hashes": count,
        "elapsed_secs": elapsed,
        "avg_hashrate": hashrate
    })
}

#[tauri::command]
pub fn get_mining_status(state: State<Arc<MiningState>>) -> serde_json::Value {
    let is_mining = state.mining.load(Ordering::SeqCst);
    let count = state.hash_count.load(Ordering::SeqCst);

    let elapsed = state
        .start_time
        .lock()
        .unwrap()
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(0.0);

    let hashrate = if elapsed > 0.0 {
        count as f64 / elapsed
    } else {
        0.0
    };

    let pending_count = state.pending_proofs.lock().unwrap().len();
    let params = state.params.lock().unwrap();

    let mut result = serde_json::json!({
        "mining": is_mining,
        "total_hashes": count,
        "elapsed_secs": elapsed,
        "hashrate": hashrate,
        "pending_proofs": pending_count
    });

    if let Some(ref p) = *params {
        result["block_hash_hex"] = serde_json::json!(p.block_hash_hex);
        result["cyberlinks_merkle_hex"] = serde_json::json!(p.cyberlinks_merkle_hex);
        result["epoch_id"] = serde_json::json!(p.epoch_id);
        result["block_timestamp"] = serde_json::json!(p.block_timestamp);
    }

    result
}

#[tauri::command]
pub fn take_proofs(state: State<Arc<MiningState>>) -> serde_json::Value {
    let proofs: Vec<FoundProof> = std::mem::take(&mut *state.pending_proofs.lock().unwrap());
    serde_json::json!(proofs)
}

#[tauri::command]
pub fn mining_benchmark(count: u32) -> serde_json::Value {
    let mut hasher = UniversalHash::new();

    let start = Instant::now();

    for i in 0..count {
        let input = format!("benchmark_input_{}", i);
        let _ = hasher.hash(input.as_bytes());
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    let hashrate = count as f64 / elapsed.as_secs_f64();

    serde_json::json!({
        "count": count,
        "elapsed_ms": elapsed_ms,
        "hashrate": hashrate
    })
}

#[tauri::command]
pub fn get_mining_params() -> serde_json::Value {
    serde_json::json!({
        "chains": uhash_core::CHAINS,
        "scratchpad_kb": uhash_core::SCRATCHPAD_SIZE / 1024,
        "total_mb": uhash_core::TOTAL_MEMORY / (1024 * 1024),
        "rounds": uhash_core::ROUNDS,
        "block_size": uhash_core::BLOCK_SIZE
    })
}
