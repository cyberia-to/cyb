//! The body earns PUSSY: proving, not hashing.
//!
//! One ticket = one proved SpMV over THIS cyb's own cybergraph: the outer
//! sumcheck samples the claim across `log n` rounds, HyperNova folds the
//! instance into an accumulator, SuperSpartan decides, Brakedown opens —
//! zheng's whole pipeline, real Goldilocks arithmetic end to end, and the
//! ticket only counts after `verify_spmv` passes. This is the stakeless
//! PoW onramp of tru/specs/rewards.md in its offline phase: the envelope
//! (meter proven settlement work, pay pro-rata) is the spec's; the beacon,
//! cluster and difficulty target arrive with the chain. Until then the
//! accrual rate is declared in `~/cyb/rates.toml` and the page says so.
//!
//! The proof count survives restarts in `~/cyb/proofs` — work done is
//! work done, whichever session did it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use nebu::Goldilocks;
use zheng::{prove_spmv, spmv_native, verify_spmv, SparseGraph};

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
        let (graph, n, nnz) = build_graph(&axons);
        if let Ok(mut s) = self.stat.lock() {
            s.running = true;
            s.tickets = 0;
            s.failed = 0;
            s.n = n;
            s.axons = nnz;
            s.since = Some(Instant::now());
        }

        let stat = self.stat.clone();
        let run = self.run.clone();
        std::thread::Builder::new()
            .name("body-prover".into())
            .spawn(move || {
                #[cfg(target_os = "macos")]
                unsafe {
                    // QOS_CLASS_UTILITY: the prover yields to everything
                    // interactive instead of parking soma or the frame loop.
                    unsafe extern "C" {
                        fn pthread_set_qos_class_self_np(qos: u32, rel: i32) -> i32;
                    }
                    pthread_set_qos_class_self_np(0x11, 0);
                }

                let mut ticket_no = read_lifetime();
                let mut last_flush = Instant::now();
                while run.load(Ordering::Relaxed) {
                    let t0 = Instant::now();
                    // A fresh public vector per ticket, bound to the beacon:
                    // anyone holding (ticket number, chain state root) can
                    // rebuild x and re-verify. No beacon yet = seed 0, and
                    // the card says so.
                    let beacon = hub.beacon();
                    let root_seed = beacon
                        .as_ref()
                        .map(|(_, h, root)| beacon_seed(*h, root))
                        .unwrap_or(0);
                    if let Ok(mut s) = stat.lock() {
                        s.beacon = beacon;
                    }
                    let x: Vec<Goldilocks> = (0..graph.n)
                        .map(|i| {
                            Goldilocks::new(splitmix64(
                                ticket_no.wrapping_mul(0x9e37) ^ root_seed ^ i as u64,
                            ))
                        })
                        .collect();
                    let y = spmv_native(&graph, &x);
                    let ok = prove_spmv(&graph, &x, &y)
                        .map(|proof| verify_spmv(&graph, &x, &y, &proof))
                        .unwrap_or(false);

                    let ms = t0.elapsed().as_secs_f32() * 1000.0;
                    ticket_no += 1;
                    if let Ok(mut s) = stat.lock() {
                        if ok {
                            s.tickets += 1;
                            s.lifetime += 1;
                        } else {
                            s.failed += 1;
                        }
                        s.last_ms = ms;
                    }
                    // The lifetime counter reaches disk every few seconds
                    // and on the final ticket — a hard kill costs at most
                    // that much proven work, not a session.
                    if last_flush.elapsed().as_secs() >= 10 {
                        write_lifetime(ticket_no);
                        last_flush = Instant::now();
                    }
                }
                write_lifetime(ticket_no);
                if let Ok(mut s) = stat.lock() {
                    s.running = false;
                }
            })
            .expect("spawn body-prover");
    }

    pub fn stop(&self) {
        self.run.store(false, Ordering::SeqCst);
    }
}

/// The local cybergraph as zheng sees it: axon endpoints become row/col
/// indices, weights ride as field elements. An empty graph (a cyb that has
/// not lived yet) proves over a seeded ring instead — the arithmetic is
/// just as real, and the label on the page says which one is running.
fn build_graph(axons: &[([u8; 32], [u8; 32], u64)]) -> (SparseGraph, usize, usize) {
    let mut index: std::collections::HashMap<[u8; 32], usize> = std::collections::HashMap::new();
    let mut id = |index: &mut std::collections::HashMap<[u8; 32], usize>, p: [u8; 32]| {
        let next = index.len();
        *index.entry(p).or_insert(next)
    };
    let mut edges = Vec::new();
    for (from, to, amount) in axons {
        let (r, c) = (id(&mut index, *from), id(&mut index, *to));
        edges.push((r, c, Goldilocks::new((*amount).max(1))));
    }
    let n = index.len().max(2);
    let mut g = SparseGraph::empty(n);
    if edges.is_empty() {
        for i in 0..n {
            g.add(i, (i + 1) % n, Goldilocks::new(splitmix64(i as u64) | 1));
        }
    } else {
        for (r, c, w) in &edges {
            g.add(*r, *c, *w);
        }
    }
    let nnz = g.edges.len();
    (g, n, nnz)
}

/// Fold the beacon into one seed word: height plus the leading 16 hex
/// digits of the state root. Deterministic for everyone watching the
/// same chain at the same height.
fn beacon_seed(height: u64, root: &str) -> u64 {
    let hex = root.get(..16).unwrap_or(root);
    u64::from_str_radix(hex, 16).unwrap_or(0) ^ height.rotate_left(32)
}

/// splitmix64 — the standard seed scrambler; deterministic x vectors.
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One real ticket over a graph the size of a lived-in cyb, timed.
    /// Run with --nocapture to read the ms.
    #[test]
    fn one_ticket_proves_and_verifies() {
        let axons: Vec<([u8; 32], [u8; 32], u64)> = (0..255u64)
            .map(|i| {
                let mut a = [0u8; 32];
                let mut b = [0u8; 32];
                a[..8].copy_from_slice(&splitmix64(i).to_le_bytes());
                b[..8].copy_from_slice(&splitmix64(i * 7 + 1).to_le_bytes());
                (a, b, i + 1)
            })
            .collect();
        let (g, n, nnz) = build_graph(&axons);
        assert!(n > 100, "n={n}");
        assert_eq!(nnz, 255);

        let t0 = std::time::Instant::now();
        let x: Vec<Goldilocks> = (0..g.n).map(|i| Goldilocks::new(splitmix64(i as u64))).collect();
        let y = spmv_native(&g, &x);
        let proof = prove_spmv(&g, &x, &y).expect("prove");
        assert!(verify_spmv(&g, &x, &y, &proof), "verify");
        eprintln!("ticket over n={n} nnz={nnz}: {:.1} ms", t0.elapsed().as_secs_f32() * 1000.0);
    }

    #[test]
    fn empty_graph_falls_back_to_a_ring() {
        let (g, n, nnz) = build_graph(&[]);
        assert_eq!(n, 2);
        assert_eq!(nnz, 2);
        let x: Vec<Goldilocks> = (0..g.n).map(|i| Goldilocks::new(i as u64 + 3)).collect();
        let y = spmv_native(&g, &x);
        let proof = prove_spmv(&g, &x, &y).expect("prove");
        assert!(verify_spmv(&g, &x, &y, &proof));
    }

    #[test]
    fn tampered_answer_is_refused() {
        let (g, _, _) = build_graph(&[]);
        let x: Vec<Goldilocks> = (0..g.n).map(|i| Goldilocks::new(i as u64 + 3)).collect();
        let mut y = spmv_native(&g, &x);
        y[0] = y[0] + Goldilocks::ONE;
        assert!(prove_spmv(&g, &x, &y).is_err(), "a wrong claim must not prove");
    }
}
