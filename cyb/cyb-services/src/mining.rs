use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uhash_core::UniversalHash;

pub struct MiningState {
    mining: AtomicBool,
    hash_count: AtomicU64,
    start_time: Mutex<Option<Instant>>,
    pending_proofs: Mutex<Vec<FoundProof>>,
}

#[derive(Clone, Serialize)]
pub struct FoundProof {
    pub hash: String,
    pub nonce: u64,
    pub timestamp: u64,
}

impl MiningState {
    pub fn new() -> Self {
        Self {
            mining: AtomicBool::new(false),
            hash_count: AtomicU64::new(0),
            start_time: Mutex::new(None),
            pending_proofs: Mutex::new(Vec::new()),
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

pub fn start_mining(
    state: &Arc<MiningState>,
    seed: String,
    address: String,
    timestamp: u64,
    difficulty: u32,
    threads: Option<u32>,
) -> serde_json::Value {
    if state.mining.load(Ordering::SeqCst) {
        return serde_json::json!({ "success": false, "error": "Already mining" });
    }

    state.mining.store(true, Ordering::SeqCst);
    state.hash_count.store(0, Ordering::SeqCst);
    *state.start_time.lock().unwrap() = Some(Instant::now());
    state.pending_proofs.lock().unwrap().clear();

    let num_threads = threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4)
    });

    for thread_id in 0..num_threads {
        let mining_flag = Arc::clone(state);
        let seed = seed.clone();
        let address = address.clone();

        std::thread::spawn(move || {
            let mut hasher = UniversalHash::new();
            let mut nonce: u64 = thread_id as u64;

            let seed_bytes = hex::decode(&seed).unwrap_or_else(|_| seed.as_bytes().to_vec());

            while mining_flag.mining.load(Ordering::Relaxed) {
                let mut input = Vec::with_capacity(seed_bytes.len() + address.len() + 16);
                input.extend_from_slice(&seed_bytes);
                input.extend_from_slice(address.as_bytes());
                input.extend_from_slice(&timestamp.to_le_bytes());
                input.extend_from_slice(&nonce.to_le_bytes());
                let hash = hasher.hash(&input);

                mining_flag.hash_count.fetch_add(1, Ordering::Relaxed);

                if meets_difficulty(&hash, difficulty) {
                    let proof = FoundProof {
                        hash: hex::encode(hash),
                        nonce,
                        timestamp,
                    };
                    mining_flag.pending_proofs.lock().unwrap().push(proof);
                }

                nonce += num_threads as u64;
            }
        });
    }

    serde_json::json!({ "success": true, "threads": num_threads })
}

pub fn stop_mining(state: &Arc<MiningState>) -> serde_json::Value {
    state.mining.store(false, Ordering::SeqCst);

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

pub fn get_mining_status(state: &Arc<MiningState>) -> serde_json::Value {
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

    serde_json::json!({
        "mining": is_mining,
        "total_hashes": count,
        "elapsed_secs": elapsed,
        "hashrate": hashrate,
        "pending_proofs": pending_count
    })
}

pub fn take_proofs(state: &Arc<MiningState>) -> serde_json::Value {
    let proofs: Vec<FoundProof> = std::mem::take(&mut *state.pending_proofs.lock().unwrap());
    serde_json::json!(proofs)
}

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

pub fn get_mining_params() -> serde_json::Value {
    serde_json::json!({
        "chains": uhash_core::CHAINS,
        "scratchpad_kb": uhash_core::SCRATCHPAD_SIZE / 1024,
        "total_mb": uhash_core::TOTAL_MEMORY / (1024 * 1024),
        "rounds": uhash_core::ROUNDS,
        "block_size": uhash_core::BLOCK_SIZE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meets_difficulty() {
        assert!(meets_difficulty(&[0, 0, 0, 1], 24));
        assert!(!meets_difficulty(&[0, 0, 1, 0], 24));
        assert!(meets_difficulty(&[0, 0, 0, 0], 32));
        assert!(meets_difficulty(&[0x0F], 0));
        assert!(!meets_difficulty(&[0x0F], 5));
        assert!(meets_difficulty(&[0x0F], 4));
    }
}
