use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::State;
use uhash_core::lithium_header;
use uhash_prover::cpu::ParallelCpuSolver;
use uhash_prover::Solver;

#[cfg(all(feature = "gpu-metal", target_os = "macos"))]
use uhash_prover::metal_miner::MetalMiner;

#[cfg(feature = "gpu-cuda")]
use uhash_prover::cuda_miner::CudaMiner;

#[cfg(feature = "gpu-wgpu")]
use uhash_prover::wgpu_solver::WgpuSolver;

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
    pub mining: AtomicBool,
    hash_count: AtomicU64,
    start_time: Mutex<Option<Instant>>,
    pending_proofs: Mutex<Vec<FoundProof>>,
    params: Mutex<Option<MiningParams>>,
    active_backend: Mutex<String>,
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
            active_backend: Mutex::new(String::new()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum MiningBackend {
    Auto,
    Cpu,
    Metal,
    Cuda,
    Wgpu,
}

impl MiningBackend {
    fn from_str_opt(s: Option<&str>) -> Self {
        match s {
            Some("cpu") => Self::Cpu,
            Some("metal") => Self::Metal,
            Some("cuda") => Self::Cuda,
            Some("wgpu") => Self::Wgpu,
            _ => Self::Auto,
        }
    }

    fn auto_fallback_chain() -> &'static [MiningBackend] {
        if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            &[
                MiningBackend::Metal,
                MiningBackend::Wgpu,
                MiningBackend::Cpu,
            ]
        } else {
            &[
                MiningBackend::Cuda,
                MiningBackend::Wgpu,
                MiningBackend::Cpu,
            ]
        }
    }
}

fn try_create_solver(
    backend: MiningBackend,
    threads: usize,
) -> Option<(Box<dyn Solver + Send>, &'static str)> {
    match backend {
        MiningBackend::Cpu => {
            let solver = ParallelCpuSolver::new(threads);
            Some((Box::new(solver), "cpu"))
        }
        MiningBackend::Metal => {
            #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
            {
                match MetalMiner::new() {
                    Ok(m) => Some((Box::new(m), "metal")),
                    Err(e) => {
                        eprintln!("[Mining] Metal init failed: {}", e);
                        None
                    }
                }
            }
            #[cfg(not(all(feature = "gpu-metal", target_os = "macos")))]
            {
                None
            }
        }
        MiningBackend::Cuda => {
            #[cfg(feature = "gpu-cuda")]
            {
                match CudaMiner::new() {
                    Ok(m) => Some((Box::new(m), "cuda")),
                    Err(e) => {
                        eprintln!("[Mining] CUDA init failed: {}", e);
                        None
                    }
                }
            }
            #[cfg(not(feature = "gpu-cuda"))]
            {
                None
            }
        }
        MiningBackend::Wgpu => {
            #[cfg(feature = "gpu-wgpu")]
            {
                match WgpuSolver::new() {
                    Ok(m) => Some((Box::new(m), "wgpu")),
                    Err(e) => {
                        eprintln!("[Mining] WGPU init failed: {}", e);
                        None
                    }
                }
            }
            #[cfg(not(feature = "gpu-wgpu"))]
            {
                None
            }
        }
        MiningBackend::Auto => None, // handled by fallback chain
    }
}

fn create_solver(
    backend: MiningBackend,
    threads: usize,
) -> Result<(Box<dyn Solver + Send>, &'static str), String> {
    if backend == MiningBackend::Auto {
        for &candidate in MiningBackend::auto_fallback_chain() {
            if let Some(result) = try_create_solver(candidate, threads) {
                return Ok(result);
            }
        }
        return Err("No mining backend available".into());
    }

    try_create_solver(backend, threads).ok_or_else(|| {
        format!(
            "Backend {:?} not available (not compiled or init failed)",
            backend
        )
    })
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
    backend: Option<String>,
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

    let header = lithium_header(&address, &block_hash, &cyberlinks_merkle);

    let num_threads = threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4)
    }) as usize;

    let requested_backend = MiningBackend::from_str_opt(backend.as_deref());
    let (mut solver, backend_name) = match create_solver(requested_backend, num_threads) {
        Ok(s) => s,
        Err(e) => return serde_json::json!({ "success": false, "error": e }),
    };

    // Store params so JS can restore refs after component remount
    *state.params.lock().unwrap() = Some(MiningParams {
        address,
        block_hash_hex,
        cyberlinks_merkle_hex,
        difficulty,
        epoch_id: epoch_id.unwrap_or(0),
        block_timestamp: block_timestamp.unwrap_or(0),
    });

    *state.active_backend.lock().unwrap() = backend_name.to_string();
    state.mining.store(true, Ordering::SeqCst);
    state.hash_count.store(0, Ordering::SeqCst);
    *state.start_time.lock().unwrap() = Some(Instant::now());
    state.pending_proofs.lock().unwrap().clear();

    let lanes = solver.recommended_lanes(0);

    let state_clone = state.inner().clone();

    std::thread::spawn(move || {
        let mut nonce: u64 = 0;

        while state_clone.mining.load(Ordering::Relaxed) {
            match solver.find_proof_batch(&header, nonce, lanes, difficulty) {
                Ok((Some((found_nonce, hash)), actual)) => {
                    let proof = FoundProof {
                        hash: hex::encode(hash),
                        nonce: found_nonce,
                    };
                    state_clone.pending_proofs.lock().unwrap().push(proof);
                    state_clone
                        .hash_count
                        .fetch_add(actual as u64, Ordering::Relaxed);
                }
                Ok((None, actual)) => {
                    state_clone
                        .hash_count
                        .fetch_add(actual as u64, Ordering::Relaxed);
                }
                Err(e) => {
                    eprintln!("[Mining] Solver error: {}", e);
                    break;
                }
            }
            nonce = nonce.saturating_add(lanes as u64);
        }
    });

    serde_json::json!({
        "success": true,
        "threads": num_threads,
        "backend": backend_name
    })
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

    let backend = state.active_backend.lock().unwrap().clone();

    serde_json::json!({
        "success": true,
        "total_hashes": count,
        "elapsed_secs": elapsed,
        "avg_hashrate": hashrate,
        "backend": backend
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
    let backend = state.active_backend.lock().unwrap().clone();

    let mut result = serde_json::json!({
        "mining": is_mining,
        "total_hashes": count,
        "elapsed_secs": elapsed,
        "hashrate": hashrate,
        "pending_proofs": pending_count,
        "backend": backend
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
pub fn mining_benchmark(count: u32, backend: Option<String>) -> serde_json::Value {
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let requested_backend = MiningBackend::from_str_opt(backend.as_deref());
    let (mut solver, backend_name) = match create_solver(requested_backend, num_threads) {
        Ok(s) => s,
        Err(e) => return serde_json::json!({ "error": e }),
    };

    let header = [0u8; 32];
    let lanes = solver.recommended_lanes(0);
    let total_lanes = (count as usize).max(lanes);

    let start = Instant::now();
    let mut nonce: u64 = 0;
    let mut total_hashed: usize = 0;

    while total_hashed < total_lanes {
        let batch = lanes.min(total_lanes - total_hashed);
        match solver.benchmark_hashes(&header, nonce, batch) {
            Ok(done) => {
                total_hashed += done;
                nonce += batch as u64;
            }
            Err(e) => {
                return serde_json::json!({
                    "error": format!("Benchmark error: {}", e),
                    "backend": backend_name
                });
            }
        }
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    let hashrate = total_hashed as f64 / elapsed.as_secs_f64();

    serde_json::json!({
        "count": total_hashed,
        "elapsed_ms": elapsed_ms,
        "hashrate": hashrate,
        "backend": backend_name
    })
}

#[tauri::command]
pub fn get_mining_params() -> serde_json::Value {
    #[allow(unused_mut)]
    let mut available = vec!["cpu"];

    #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
    available.push("metal");

    #[cfg(feature = "gpu-cuda")]
    available.push("cuda");

    #[cfg(feature = "gpu-wgpu")]
    available.push("wgpu");

    serde_json::json!({
        "chains": uhash_core::CHAINS,
        "scratchpad_kb": uhash_core::SCRATCHPAD_SIZE / 1024,
        "total_mb": uhash_core::TOTAL_MEMORY / (1024 * 1024),
        "rounds": uhash_core::ROUNDS,
        "block_size": uhash_core::BLOCK_SIZE,
        "available_backends": available
    })
}
