use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::State;
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
    pub challenge_hex: String,
    pub difficulty: u32,
    pub block_timestamp: u64,
}

/// Rolling window snapshot for smooth hashrate calculation.
#[derive(Clone, Copy)]
pub(crate) struct HashSnapshot {
    time: Instant,
    count: u64,
}

const HASHRATE_WINDOW_SECS: f64 = 30.0;
const MAX_SNAPSHOTS: usize = 256;

pub struct MiningState {
    pub mining: AtomicBool,
    pub(crate) hash_count: AtomicU64,
    pub(crate) batch_count: AtomicU64,
    pub(crate) total_batch_time_us: AtomicU64,
    pub(crate) proofs_submitted: AtomicU64,
    pub(crate) proofs_failed: AtomicU64,
    pub(crate) start_time: Mutex<Option<Instant>>,
    pub(crate) pending_proofs: Mutex<Vec<FoundProof>>,
    pub(crate) params: Mutex<Option<MiningParams>>,
    pub(crate) active_backend: Mutex<String>,
    pub(crate) hash_snapshots: Mutex<Vec<HashSnapshot>>,
    /// Live challenge bytes — mining thread reads this on every batch.
    pub(crate) current_challenge: std::sync::RwLock<[u8; 32]>,
}

#[derive(Clone, Serialize)]
pub struct FoundProof {
    pub hash: String,
    pub nonce: u64,
    pub challenge: String,
}

impl MiningState {
    /// Record a hash count snapshot and return the rolling hashrate (H/s).
    fn record_snapshot(&self) -> f64 {
        let now = Instant::now();
        let count = self.hash_count.load(Ordering::Relaxed);
        let mut snaps = self.hash_snapshots.lock().unwrap();
        snaps.push(HashSnapshot { time: now, count });
        // Trim old entries beyond the window
        let cutoff = now - Duration::from_secs_f64(HASHRATE_WINDOW_SECS);
        snaps.retain(|s| s.time >= cutoff);
        // Rate = (newest - oldest count) / (newest - oldest time)
        if snaps.len() < 2 {
            return 0.0;
        }
        let oldest = snaps[0];
        let newest = snaps[snaps.len() - 1];
        let dt = newest.time.duration_since(oldest.time).as_secs_f64();
        if dt < 0.1 {
            return 0.0;
        }
        (newest.count - oldest.count) as f64 / dt
    }

    /// Get the rolling hashrate without recording a new snapshot.
    fn rolling_hashrate(&self) -> f64 {
        let now = Instant::now();
        let snaps = self.hash_snapshots.lock().unwrap();
        if snaps.len() < 2 {
            // Fall back to cumulative
            let count = self.hash_count.load(Ordering::Relaxed);
            let elapsed = self.start_time.lock().unwrap()
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            return if elapsed > 0.0 { count as f64 / elapsed } else { 0.0 };
        }
        let cutoff = now - Duration::from_secs_f64(HASHRATE_WINDOW_SECS);
        let oldest = snaps.iter().find(|s| s.time >= cutoff).unwrap_or(&snaps[0]);
        let newest = &snaps[snaps.len() - 1];
        let dt = newest.time.duration_since(oldest.time).as_secs_f64();
        if dt < 0.1 { return 0.0; }
        (newest.count - oldest.count) as f64 / dt
    }

    pub fn new() -> Self {
        Self {
            mining: AtomicBool::new(false),
            hash_count: AtomicU64::new(0),
            batch_count: AtomicU64::new(0),
            total_batch_time_us: AtomicU64::new(0),
            proofs_submitted: AtomicU64::new(0),
            proofs_failed: AtomicU64::new(0),
            start_time: Mutex::new(None),
            pending_proofs: Mutex::new(Vec::new()),
            params: Mutex::new(None),
            active_backend: Mutex::new(String::new()),
            hash_snapshots: Mutex::new(Vec::with_capacity(MAX_SNAPSHOTS)),
            current_challenge: std::sync::RwLock::new([0u8; 32]),
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
    challenge_hex: String,
    difficulty: u32,
    block_timestamp: Option<u64>,
    threads: Option<u32>,
    backend: Option<String>,
    state: State<Arc<MiningState>>,
) -> serde_json::Value {
    if state.mining.load(Ordering::SeqCst) {
        return serde_json::json!({ "success": false, "error": "Already mining" });
    }

    let challenge = match decode_hex_32("challenge", &challenge_hex) {
        Ok(b) => b,
        Err(e) => return serde_json::json!({ "success": false, "error": e }),
    };

    *state.current_challenge.write().unwrap() = challenge;

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

    let address_for_metrics = address.clone();

    // Store params so JS can restore refs after component remount
    *state.params.lock().unwrap() = Some(MiningParams {
        address,
        challenge_hex,
        difficulty,
        block_timestamp: block_timestamp.unwrap_or(0),
    });

    *state.active_backend.lock().unwrap() = backend_name.to_string();
    state.mining.store(true, Ordering::SeqCst);
    state.hash_count.store(0, Ordering::SeqCst);
    state.batch_count.store(0, Ordering::SeqCst);
    state.total_batch_time_us.store(0, Ordering::SeqCst);
    state.hash_snapshots.lock().unwrap().clear();
    state.proofs_submitted.store(0, Ordering::SeqCst);
    state.proofs_failed.store(0, Ordering::SeqCst);
    *state.start_time.lock().unwrap() = Some(Instant::now());
    state.pending_proofs.lock().unwrap().clear();

    let lanes = solver.recommended_lanes(0);

    let state_clone = state.inner().clone();

    let is_gpu = backend_name != "cpu";

    std::thread::spawn(move || {
        let mut nonce: u64 = 0;
        let mut local_batch_count: u64 = 0;
        let mut local_batch_time_us: u64 = 0;
        let mut consecutive_errors: u32 = 0;
        let mut metrics = crate::metrics::MetricsReporter::new(&address_for_metrics);

        println!(
            "[Mining] Started: backend={}, lanes={}, difficulty={}, gpu_yield={}",
            backend_name, lanes, difficulty, is_gpu
        );

        while state_clone.mining.load(Ordering::Relaxed) {
            let header = *state_clone.current_challenge.read().unwrap();
            let batch_start = Instant::now();
            match solver.find_proof_batch(&header, nonce, lanes, difficulty) {
                Ok((Some((found_nonce, hash)), actual)) => {
                    consecutive_errors = 0;
                    let batch_us = batch_start.elapsed().as_micros() as u64;
                    local_batch_time_us += batch_us;
                    local_batch_count += 1;
                    state_clone.batch_count.fetch_add(1, Ordering::Relaxed);
                    state_clone.total_batch_time_us.fetch_add(batch_us, Ordering::Relaxed);
                    let proof = FoundProof {
                        hash: hex::encode(hash),
                        nonce: found_nonce,
                        challenge: hex::encode(header),
                    };
                    println!(
                        "[Mining] PROOF FOUND! nonce={}, hash={}..., batch_time={}ms",
                        found_nonce,
                        &hex::encode(&hash)[..16],
                        batch_us / 1000
                    );
                    state_clone.pending_proofs.lock().unwrap().push(proof);
                    state_clone
                        .hash_count
                        .fetch_add(actual as u64, Ordering::Relaxed);
                    state_clone.record_snapshot();
                }
                Ok((None, actual)) => {
                    consecutive_errors = 0;
                    let batch_us = batch_start.elapsed().as_micros() as u64;
                    local_batch_time_us += batch_us;
                    local_batch_count += 1;
                    state_clone.batch_count.fetch_add(1, Ordering::Relaxed);
                    state_clone.total_batch_time_us.fetch_add(batch_us, Ordering::Relaxed);
                    state_clone
                        .hash_count
                        .fetch_add(actual as u64, Ordering::Relaxed);

                    // Record snapshot for rolling hashrate
                    let rolling_hr = state_clone.record_snapshot();

                    // Log every 10 batches
                    if local_batch_count % 10 == 0 {
                        let avg_ms = local_batch_time_us as f64 / local_batch_count as f64 / 1000.0;
                        let total_h = state_clone.hash_count.load(Ordering::Relaxed);
                        println!(
                            "[Mining] batch={}, lanes={}, avg_batch={:.1}ms, hashes={}, hashrate={:.0} H/s",
                            local_batch_count, lanes, avg_ms, total_h, rolling_hr
                        );
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;
                    eprintln!(
                        "[Mining] Solver error (attempt {}): {}",
                        consecutive_errors, e
                    );
                    if consecutive_errors >= 5 {
                        eprintln!("[Mining] Too many consecutive errors, stopping");
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100 * consecutive_errors as u64));
                    continue;
                }
            }
            nonce = nonce.saturating_add(lanes as u64);

            // Yield between GPU batches to prevent thermal throttling
            if is_gpu {
                std::thread::sleep(Duration::from_millis(1));
            }

            // Push metrics in batches (checks internally if enough time elapsed)
            if local_batch_count % 100 == 0 {
                metrics.maybe_push(&state_clone, backend_name);
            }
        }

        // Flush final metrics on stop
        metrics.flush(&state_clone, backend_name);

        let total_h = state_clone.hash_count.load(Ordering::Relaxed);
        let elapsed = state_clone
            .start_time
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(1.0);
        println!(
            "[Mining] Stopped: {} batches, {} hashes in {:.1}s, avg {:.0} H/s, avg_batch={:.1}ms",
            local_batch_count,
            total_h,
            elapsed,
            total_h as f64 / elapsed,
            local_batch_time_us as f64 / local_batch_count.max(1) as f64 / 1000.0
        );
    });

    serde_json::json!({
        "success": true,
        "threads": num_threads,
        "backend": backend_name
    })
}

/// Hot-swap the mining challenge without stopping the mining thread.
/// Clears pending proofs (they were mined against the old challenge).
#[tauri::command]
pub fn update_challenge(
    challenge_hex: String,
    block_timestamp: u64,
    state: State<Arc<MiningState>>,
) -> serde_json::Value {
    if !state.mining.load(Ordering::SeqCst) {
        return serde_json::json!({ "success": false, "error": "Not mining" });
    }

    let challenge = match decode_hex_32("challenge", &challenge_hex) {
        Ok(b) => b,
        Err(e) => return serde_json::json!({ "success": false, "error": e }),
    };

    *state.current_challenge.write().unwrap() = challenge;
    state.pending_proofs.lock().unwrap().clear();

    if let Some(ref mut p) = *state.params.lock().unwrap() {
        p.challenge_hex = challenge_hex;
        p.block_timestamp = block_timestamp;
    }

    serde_json::json!({ "success": true })
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

    let hashrate = state.rolling_hashrate();

    let pending_count = state.pending_proofs.lock().unwrap().len();
    let params = state.params.lock().unwrap();
    let backend = state.active_backend.lock().unwrap().clone();
    let batches = state.batch_count.load(Ordering::Relaxed);
    let batch_time_us = state.total_batch_time_us.load(Ordering::Relaxed);
    let avg_batch_ms = if batches > 0 {
        batch_time_us as f64 / batches as f64 / 1000.0
    } else {
        0.0
    };
    let proofs_submitted = state.proofs_submitted.load(Ordering::Relaxed);
    let proofs_failed = state.proofs_failed.load(Ordering::Relaxed);

    let mut result = serde_json::json!({
        "mining": is_mining,
        "total_hashes": count,
        "elapsed_secs": elapsed,
        "hashrate": hashrate,
        "pending_proofs": pending_count,
        "backend": backend,
        "batch_count": batches,
        "avg_batch_ms": avg_batch_ms,
        "proofs_submitted": proofs_submitted,
        "proofs_failed": proofs_failed
    });

    if let Some(ref p) = *params {
        result["challenge_hex"] = serde_json::json!(p.challenge_hex);
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
pub fn report_proof_submitted(state: State<Arc<MiningState>>) {
    state.proofs_submitted.fetch_add(1, Ordering::Relaxed);
}

#[tauri::command]
pub fn report_proof_failed(state: State<Arc<MiningState>>) {
    state.proofs_failed.fetch_add(1, Ordering::Relaxed);
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
