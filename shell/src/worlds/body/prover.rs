//! The body earns PUSSY: proving, not hashing.
//!
//! One ticket = one SpMV step `y = A·x` over THIS cyb's own cybergraph,
//! real Goldilocks arithmetic, with `x` drawn from the beacon and the
//! ticket number. The ticket binds the graph commitment and a digest of
//! `y`, and only counts after `cyb_core::ticket::verify` recomputes it.
//! All inputs are public, so recomputation is the sound check: zheng's
//! legacy `prove_spmv` / `verify_spmv` ran a HyperNova fold its verifier
//! never checked (see `cyb_core::ticket` for the design and costs; a
//! succinct ticket waits for the step relation of the proof repair's
//! phase 3). This is the stakeless
//! PoW onramp of tru/specs/rewards.md in its offline phase: the envelope
//! (meter proven settlement work, pay pro-rata) is the spec's; the beacon,
//! cluster and difficulty target arrive with the chain. Until then the
//! accrual rate is declared in `~/cyb/rates.toml` and the page says so.
//!
//! The proof count survives restarts in `~/cyb/proofs` — work done is
//! work done, whichever session did it.
//!
//! Proving is a FLEET: tickets are independent, so every core can carry
//! one. The duty lever is erga's, reused: `~/cyb/prover-intensity` holds
//! max (every core), eco (half) or min (one) — re-read live between
//! tickets, no restart. All workers ride utility QoS: the fleet eats idle
//! silicon, never the frame loop or the mind.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cyb_core::ticket::{self, CommittedGraph, TicketGraph};

use super::networks::NetHub;

#[derive(Clone, Debug, Default)]
pub struct ProverStat {
    pub running: bool,
    /// Verified tickets this session.
    pub tickets: u64,
    /// Tickets whose self-verification failed (should stay 0; counted
    /// honestly if it ever moves).
    pub failed: u64,
    /// Lifetime verified tickets, including past sessions.
    pub lifetime: u64,
    pub last_ms: f32,
    /// Graph the tickets are proved over.
    pub n: usize,
    pub axons: usize,
    /// Worker threads currently proving.
    pub workers: usize,
    pub since: Option<Instant>,
    /// The chain state the current tickets bind to: (network, height,
    /// state root). Empty until the first network is reached.
    pub beacon: Option<(String, u64, String)>,
}

impl ProverStat {
    pub fn tickets_per_min(&self) -> f64 {
        match self.since {
            Some(t) if self.tickets > 0 => {
                self.tickets as f64 / (t.elapsed().as_secs_f64() / 60.0).max(1e-9)
            }
            _ => 0.0,
        }
    }
}

#[derive(Clone, Default)]
pub struct Prover {
    pub stat: Arc<Mutex<ProverStat>>,
    run: Arc<AtomicBool>,
}

fn intensity_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("prover-intensity")
}

/// max = every core, eco = half, min = one. Anything unparsed is max —
/// the lever exists to hold back, not to enable.
pub fn intensity() -> String {
    std::fs::read_to_string(intensity_file())
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| ["max", "eco", "min"].contains(&s.as_str()))
        .unwrap_or_else(|| "max".into())
}

pub fn set_intensity(mode: &str) {
    let path = intensity_file();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, mode);
}

fn fleet_size(mode: &str) -> usize {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    match mode {
        "min" => 1,
        "eco" => (cores / 2).max(1),
        _ => cores,
    }
}

fn proofs_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("proofs")
}

fn read_lifetime() -> u64 {
    std::fs::read_to_string(proofs_file())
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn write_lifetime(total: u64) {
    let path = proofs_file();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, format!("{total}\n"));
}

impl Prover {
    pub fn start() -> Self {
        let p = Prover::default();
        if let Ok(mut s) = p.stat.lock() {
            s.lifetime = read_lifetime();
        }
        p
    }

    pub fn is_running(&self) -> bool {
        self.run.load(Ordering::Relaxed)
    }

    /// Begin proving over the given axons (this cyb's graph, snapshotted).
    /// One core, utility QoS — earning must not fight thinking or drawing.
    /// Every ticket's public vector is bound to the current beacon — the
    /// last block state of the first configured network — so two bodies
    /// watching the same chain provably sample the same state.
    pub fn prove(&self, axons: Vec<([u8; 32], [u8; 32], u64)>, hub: NetHub) {
        if self.run.swap(true, Ordering::SeqCst) {
            return;
        }
        let graph = TicketGraph::from_axons(&axons);
        let (n, nnz) = (graph.n, graph.edges.len());
        if let Ok(mut s) = self.stat.lock() {
            s.running = true;
            s.tickets = 0;
            s.failed = 0;
            s.n = n;
            s.axons = nnz;
            s.since = Some(Instant::now());
        }

        // Committed once: every ticket of this session binds the same graph.
        let graph = std::sync::Arc::new(CommittedGraph::new(graph));
        // One counter names every ticket across the fleet — numbers stay
        // unique and the lifetime tally is their sum by construction.
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(read_lifetime()));
        // How many workers SHOULD be alive right now (the intensity lever,
        // resolved). Workers watch it; the supervisor moves it.
        let target = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // ── the supervisor: sizes the fleet, flushes the counter ────────
        {
            let stat = self.stat.clone();
            let run = self.run.clone();
            let counter = counter.clone();
            let target = target.clone();
            let graph = graph.clone();
            let hub = hub.clone();
            std::thread::Builder::new()
                .name("body-prover-sup".into())
                .spawn(move || {
                    let mut spawned = 0usize;
                    let mut last_flush = Instant::now();
                    while run.load(Ordering::Relaxed) {
                        let want = fleet_size(&intensity());
                        target.store(want, Ordering::Relaxed);
                        // Spawn up to the target; shrinking is the workers'
                        // own exit (index >= target).
                        while spawned < want {
                            let idx = spawned;
                            spawn_worker(
                                idx,
                                graph.clone(),
                                hub.clone(),
                                stat.clone(),
                                run.clone(),
                                counter.clone(),
                                target.clone(),
                            );
                            spawned += 1;
                        }
                        if let Ok(mut s) = stat.lock() {
                            s.workers = spawned.min(want);
                        }
                        if last_flush.elapsed().as_secs() >= 10 {
                            write_lifetime(counter.load(Ordering::Relaxed));
                            last_flush = Instant::now();
                        }
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                    write_lifetime(counter.load(Ordering::Relaxed));
                    if let Ok(mut s) = stat.lock() {
                        s.running = false;
                        s.workers = 0;
                    }
                })
                .expect("spawn body-prover-sup");
        }
    }

    pub fn stop(&self) {
        self.run.store(false, Ordering::SeqCst);
    }
}

/// One oar in the fleet: proves tickets until told the fleet shrank past
/// it or stopped. Utility QoS — idle silicon only.
#[allow(clippy::too_many_arguments)]
fn spawn_worker(
    idx: usize,
    graph: std::sync::Arc<CommittedGraph>,
    hub: NetHub,
    stat: std::sync::Arc<std::sync::Mutex<ProverStat>>,
    run: std::sync::Arc<AtomicBool>,
    counter: std::sync::Arc<AtomicU64>,
    target: std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    std::thread::Builder::new()
        .name(format!("body-prover-{idx}"))
        .spawn(move || {
            #[cfg(target_os = "macos")]
            unsafe {
                unsafe extern "C" {
                    fn pthread_set_qos_class_self_np(qos: u32, rel: i32) -> i32;
                }
                pthread_set_qos_class_self_np(0x11, 0);
            }
            while run.load(Ordering::Relaxed) {
                if idx >= target.load(Ordering::Relaxed) {
                    // The lever came down; this oar rests until it rises.
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    continue;
                }
                let t0 = Instant::now();
                let beacon = hub.beacon();
                let root_seed = beacon
                    .as_ref()
                    .map(|(_, h, root)| beacon_seed(*h, root))
                    .unwrap_or(0);
                let ticket_no = counter.fetch_add(1, Ordering::Relaxed);
                let ok = ticket::prove(&graph, root_seed, ticket_no)
                    .is_some_and(|t| ticket::verify(&t, &graph));
                let ms = t0.elapsed().as_secs_f32() * 1000.0;
                if let Ok(mut s) = stat.lock() {
                    if ok {
                        s.tickets += 1;
                        s.lifetime += 1;
                    } else {
                        s.failed += 1;
                    }
                    s.last_ms = ms;
                    s.beacon = beacon;
                }
            }
        })
        .expect("spawn prover worker");
}

/// Fold the beacon into one seed word: height plus the leading 16 hex
/// digits of the state root. Deterministic for everyone watching the
/// same chain at the same height.
fn beacon_seed(height: u64, root: &str) -> u64 {
    let hex = root.get(..16).unwrap_or(root);
    u64::from_str_radix(hex, 16).unwrap_or(0) ^ height.rotate_left(32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beacon_seed_reads_height_and_root_prefix() {
        let root = "00000000000000ff".to_string() + &"0".repeat(48);
        assert_eq!(beacon_seed(1, &root), 0xff ^ (1u64 << 32));
        assert_eq!(beacon_seed(0, "zz"), 0, "unparsed root seeds zero");
    }
}
