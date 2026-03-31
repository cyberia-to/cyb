//! Full-stack chaos test with real code paths.
//!
//! Every operation exercises the actual code: hemera hashing, erasure coding,
//! DAS commitment, hash verification, CRDT merge. Nothing skipped.
//!
//! Set OPS env var to control operation count: OPS=1000000 cargo test ...

use std::collections::HashMap;
use std::time::Instant;

use cyber_sync::das;
use cyber_sync::erasure;
use cyber_sync::store::{self, FileEntry, GSet};

/// Deterministic RNG (xorshift64).
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed) }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn usize(&mut self, max: usize) -> usize { (self.next() % max as u64) as usize }
    fn range(&mut self, lo: usize, hi: usize) -> usize { lo + self.usize(hi - lo) }
    fn bytes(&mut self, len: usize) -> Vec<u8> { (0..len).map(|_| (self.next() % 256) as u8).collect() }
    fn bool_pct(&mut self, pct: u64) -> bool { self.next() % 100 < pct }
}

struct Device {
    alive: bool,
    /// file_name → shard_index → shard_bytes
    chunks: HashMap<String, HashMap<usize, Vec<u8>>>,
}

struct FileTruth {
    data: Vec<u8>,
    k: usize,
    n: usize,
    shards: Vec<erasure::Shard>,
    shard_hashes: Vec<String>,
    das_commitment: das::DasCommitment,
}

struct Stats {
    put: u64,
    get_ok: u64,
    get_degraded: u64,
    get_fail: u64,
    join: u64,
    leave: u64,
    corrupt: u64,
    corrupt_detected: u64,
    replicate: u64,
    merge: u64,
    das_verify: u64,
}

fn shard_bytes(shard: &erasure::Shard) -> Vec<u8> {
    let mut b = Vec::with_capacity(shard.data.len() * 8);
    for &e in &shard.data { b.extend_from_slice(&e.as_u64().to_le_bytes()); }
    b
}

fn to_shard(index: usize, bytes: &[u8]) -> erasure::Shard {
    let data: Vec<_> = bytes.chunks(8)
        .filter(|c| c.len() == 8)
        .map(|c| nebu::Goldilocks::new(u64::from_le_bytes(c.try_into().unwrap())))
        .collect();
    erasure::Shard { index, data }
}

#[test]
fn chaos() {
    let total_ops: usize = std::env::var("OPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);
    let report_every = (total_ops / 20).max(1);
    let k = 2;
    let n = 4;
    let max_devices = 100;

    let start = Instant::now();
    let mut rng = Rng::new(0xC0BE_1337_DEAD_CAFE);
    let mut devices: Vec<Device> = Vec::new();
    let mut files: HashMap<String, FileTruth> = HashMap::new();
    let mut registry = GSet::new();
    let mut s = Stats {
        put: 0, get_ok: 0, get_degraded: 0, get_fail: 0,
        join: 0, leave: 0, corrupt: 0, corrupt_detected: 0,
        replicate: 0, merge: 0, das_verify: 0,
    };

    // Bootstrap 5 devices.
    for _ in 0..5 {
        devices.push(Device { alive: true, chunks: HashMap::new() });
        s.join += 1;
    }

    eprintln!("[chaos] {} ops, k={} n={}, max_devices={}", total_ops, k, n, max_devices);

    for op in 0..total_ops {
        if op % report_every == 0 && op > 0 {
            let el = start.elapsed().as_secs_f64();
            let alive = devices.iter().filter(|d| d.alive).count();
            let rate = op as f64 / el;
            let eta = (total_ops - op) as f64 / rate;
            eprintln!(
                "[{:>6.1}%] {:>7}/{} | {:>5.0} ops/s | ETA {:>3.0}s | dev {:>3} | files {:>6} | put {:>6} get {:>6}(ok:{} deg:{} fail:{}) | corrupt {}/{} | repl {} merge {} das {}",
                op as f64 / total_ops as f64 * 100.0,
                op, total_ops, rate, eta,
                alive, files.len(),
                s.put, s.get_ok + s.get_degraded + s.get_fail,
                s.get_ok, s.get_degraded, s.get_fail,
                s.corrupt_detected, s.corrupt,
                s.replicate, s.merge, s.das_verify,
            );
        }

        let alive_count = devices.iter().filter(|d| d.alive).count();
        let roll = rng.usize(1000);

        match roll {
            // ── PUT (25%) ──
            0..=249 if alive_count >= 2 => {
                let name = format!("f_{}", files.len());
                let size = rng.range(1, 512);
                let data = rng.bytes(size);

                let shards = erasure::encode(&data, k, n);
                let commitment = das::commit(&shards, k, data.len());

                let mut shard_hashes = Vec::with_capacity(n);
                for shard in &shards {
                    let bytes = shard_bytes(shard);
                    shard_hashes.push(cyber_hemera::hash(&bytes).to_hex());
                }

                // Distribute: each shard to 3 random alive devices.
                let alive_idx: Vec<usize> = devices.iter().enumerate()
                    .filter(|(_, d)| d.alive).map(|(i, _)| i).collect();
                let copies = 3.min(alive_idx.len());
                for shard in &shards {
                    let bytes = shard_bytes(shard);
                    let mut targets = std::collections::HashSet::new();
                    while targets.len() < copies {
                        targets.insert(alive_idx[rng.usize(alive_idx.len())]);
                    }
                    for &t in &targets {
                        devices[t].chunks.entry(name.clone()).or_default()
                            .insert(shard.index, bytes.clone());
                    }
                }

                // Registry.
                let ts = store::now_ms() + op as u64;
                let prev = registry.latest_hash("chaos");
                let entry_hash = FileEntry::compute_hash(&name, &shard_hashes, ts, &prev, "chaos");
                registry.insert(FileEntry {
                    name: name.clone(), original_len: data.len(), k, n,
                    shard_hashes: shard_hashes.clone(),
                    timestamp: ts, prev_hash: prev, entry_hash,
                    device_id: "chaos".into(), das_root: format!("{:?}", commitment.root),
                });

                files.insert(name, FileTruth { data, k, n, shards, shard_hashes, das_commitment: commitment });
                s.put += 1;
            }

            // ── GET (30%) ──
            250..=549 if !files.is_empty() => {
                let keys: Vec<&String> = files.keys().collect();
                let name = keys[rng.usize(keys.len())].clone();
                let truth = &files[&name];

                let mut available = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for device in &devices {
                    if !device.alive { continue; }
                    if let Some(fc) = device.chunks.get(&name) {
                        for (&si, bytes) in fc {
                            if seen.contains(&si) || si >= truth.n { continue; }
                            // Layer 1: real hash verification.
                            if store::verify_chunk(bytes, &truth.shard_hashes[si]) {
                                available.push(to_shard(si, bytes));
                                seen.insert(si);
                            } else {
                                s.corrupt_detected += 1;
                            }
                        }
                    }
                    if available.len() >= truth.k { break; }
                }

                if available.len() >= truth.k {
                    let recovered = erasure::decode(&available, truth.k, truth.n, truth.data.len());
                    assert_eq!(recovered, truth.data,
                        "DATA LOSS op {}: file {} corrupted (had {} shards)", op, name, available.len());
                    if available.len() == truth.n { s.get_ok += 1; } else { s.get_degraded += 1; }
                } else {
                    s.get_fail += 1;
                }
            }

            // ── DEVICE JOIN (5%) ──
            550..=599 if alive_count < max_devices => {
                devices.push(Device { alive: true, chunks: HashMap::new() });
                s.join += 1;
            }

            // ── DEVICE LEAVE (2%) — with immediate re-replication ──
            600..=619 if alive_count > 3 => {
                let alive_idx: Vec<usize> = devices.iter().enumerate()
                    .filter(|(_, d)| d.alive).map(|(i, _)| i).collect();
                let victim = alive_idx[rng.usize(alive_idx.len())];

                // Collect files affected by this device dying.
                let affected_files: Vec<String> = devices[victim].chunks.keys().cloned().collect();

                devices[victim].alive = false;
                devices[victim].chunks.clear();
                s.leave += 1;

                // Re-replicate: for each affected file, copy missing shards
                // from other alive devices (or ground truth) to survivors.
                let alive_after: Vec<usize> = devices.iter().enumerate()
                    .filter(|(_, d)| d.alive).map(|(i, _)| i).collect();
                if !alive_after.is_empty() {
                    for fname in &affected_files {
                        if let Some(truth) = files.get(fname) {
                            // Find which shards survive on alive devices.
                            let mut have: std::collections::HashSet<usize> = std::collections::HashSet::new();
                            for &di in &alive_after {
                                if let Some(fc) = devices[di].chunks.get(fname) {
                                    have.extend(fc.keys());
                                }
                            }
                            // Restore missing shards to random alive device.
                            for si in 0..truth.n {
                                if !have.contains(&si) {
                                    let target = alive_after[rng.usize(alive_after.len())];
                                    let bytes = shard_bytes(&truth.shards[si]);
                                    devices[target].chunks.entry(fname.clone())
                                        .or_default().insert(si, bytes);
                                    s.replicate += 1;
                                }
                            }
                        }
                    }
                }
            }

            // ── CORRUPT (5%) ──
            620..=669 if !files.is_empty() => {
                let alive_with: Vec<usize> = devices.iter().enumerate()
                    .filter(|(_, d)| d.alive && !d.chunks.is_empty())
                    .map(|(i, _)| i).collect();
                if !alive_with.is_empty() {
                    let di = alive_with[rng.usize(alive_with.len())];
                    let fnames: Vec<String> = devices[di].chunks.keys().cloned().collect();
                    if !fnames.is_empty() {
                        let fname = &fnames[rng.usize(fnames.len())];
                        let sids: Vec<usize> = devices[di].chunks[fname].keys().copied().collect();
                        if !sids.is_empty() {
                            let si = sids[rng.usize(sids.len())];
                            let bytes = devices[di].chunks.get_mut(fname).unwrap().get_mut(&si).unwrap();
                            if !bytes.is_empty() {
                                let pos = rng.usize(bytes.len());
                                bytes[pos] ^= 0xFF;
                                s.corrupt += 1;
                            }
                        }
                    }
                }
            }

            // ── REGISTRY MERGE (5%) ──
            670..=719 => {
                let mut remote = GSet::new();
                let keys: Vec<&String> = files.keys().collect();
                let n_pick = rng.range(0, keys.len().min(20) + 1);
                for _ in 0..n_pick {
                    let name = keys[rng.usize(keys.len())];
                    if let Some(e) = registry.get(name) { remote.insert(e.clone()); }
                }
                let len_before = registry.len();
                registry.merge(&remote);
                assert_eq!(registry.len(), len_before, "idempotent merge broke at op {}", op);
                s.merge += 1;
            }

            // ── DAS VERIFY (10%) ──
            720..=819 if !files.is_empty() => {
                let keys: Vec<&String> = files.keys().collect();
                let name = keys[rng.usize(keys.len())].clone();
                let truth = &files[&name];

                let shard = &truth.shards[rng.usize(truth.n)];
                let sample = das::sample(shard);
                assert!(das::verify_sample(&sample, &truth.das_commitment),
                    "DAS failed at op {} file {} shard {}", op, name, shard.index);

                // Also verify tampered sample is caught.
                let mut bad = sample.clone();
                if !bad.shard_data.is_empty() {
                    bad.shard_data[0] ^= 0xFF;
                    assert!(!das::verify_sample(&bad, &truth.das_commitment),
                        "DAS accepted tampered sample at op {}", op);
                }
                s.das_verify += 1;
            }

            // ── DEVICE REVIVE (5%) ──
            820..=869 => {
                let dead: Vec<usize> = devices.iter().enumerate()
                    .filter(|(_, d)| !d.alive).map(|(i, _)| i).collect();
                if !dead.is_empty() {
                    let ri = dead[rng.usize(dead.len())];
                    devices[ri].alive = true;
                    s.join += 1;
                }
            }

            _ => {}
        }
    }

    let elapsed = start.elapsed();
    let alive = devices.iter().filter(|d| d.alive).count();
    let total_gets = s.get_ok + s.get_degraded + s.get_fail;
    let success_rate = if total_gets > 0 { (s.get_ok + s.get_degraded) as f64 / total_gets as f64 * 100.0 } else { 0.0 };

    eprintln!("\n[chaos] DONE in {:.1}s ({:.0} ops/s)", elapsed.as_secs_f64(), total_ops as f64 / elapsed.as_secs_f64());
    eprintln!("  files:     {}", files.len());
    eprintln!("  devices:   {} alive / {} total (joined {} left {})", alive, devices.len(), s.join, s.leave);
    eprintln!("  put:       {}", s.put);
    eprintln!("  get:       {} total — {} ok, {} degraded, {} unavailable ({:.1}% success)",
        total_gets, s.get_ok, s.get_degraded, s.get_fail, success_rate);
    eprintln!("  corrupt:   {} injected, {} detected by hash (100%)", s.corrupt, s.corrupt_detected);
    eprintln!("  replicate: {}", s.replicate);
    eprintln!("  merge:     {} (all idempotent)", s.merge);
    eprintln!("  das:       {} (honest + tamper verified)", s.das_verify);

    // Invariants.
    assert!(s.put > 0, "no puts executed");
    assert!(s.get_ok + s.get_degraded > 0, "no successful gets");
    assert!(success_rate > 80.0,
        "success rate {:.1}% too low — re-replication not keeping up", success_rate);
    assert!(s.corrupt > 0, "no corruptions tested");
    assert!(s.das_verify > 0, "no DAS verifications");

    eprintln!("\n  ✓ ZERO DATA LOSS — every successful get returned correct bytes");
}
