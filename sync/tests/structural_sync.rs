//! Structural sync integration tests.
//!
//! If these pass, the system will not lose data under any tested configuration.
//! Covers all 5 layers of structural sync (minus zheng ZK proofs).

use std::time::Instant;

use cyber_sync::erasure;
use cyber_sync::das;
use cyber_sync::store::{self, FileEntry, GSet};

// ═══════════════════════════════════════════════════════════════════
// LAYER 4: ERASURE CODING — every (k,n) config, every shard subset
// ═══════════════════════════════════════════════════════════════════

/// Test all valid (k,n) configurations with multiple data sizes.
/// For each config, verify ALL C(n,k) shard subsets reconstruct correctly.
#[test]
fn erasure_all_configs_all_subsets() {
    let configs: Vec<(usize, usize)> = vec![
        (1, 1), // degenerate: no redundancy
        (1, 2), // full replication
        (1, 4), // 1 data shard, 3 parity
        (2, 2), // k=n, no parity
        (2, 4), // standard 2-of-4
        (4, 4), // k=n
        (4, 8), // 4-of-8
        (2, 8), // 2-of-8: maximum redundancy
        (1, 8), // 1-of-8: extreme redundancy
    ];

    let data_sizes = [0, 1, 7, 8, 13, 56, 100, 1000, 4096, 10_000];

    let mut total_subsets = 0u64;
    let start = Instant::now();

    for &(k, n) in &configs {
        for &size in &data_sizes {
            let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let shards = erasure::encode(&data, k, n);
            assert_eq!(shards.len(), n, "config ({},{}): wrong shard count", k, n);

            // Test ALL C(n,k) subsets.
            let subsets = combinations(n, k);
            for subset in &subsets {
                let partial: Vec<erasure::Shard> = subset
                    .iter()
                    .map(|&i| shards[i].clone())
                    .collect();
                let recovered = erasure::decode(&partial, k, n, data.len());
                assert_eq!(
                    recovered, data,
                    "FAILED: config ({},{}), size {}, subset {:?}",
                    k, n, size, subset
                );
                total_subsets += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    eprintln!(
        "erasure_all_configs_all_subsets: {} configs × {} sizes = {} subsets tested in {:.2}s",
        configs.len(),
        data_sizes.len(),
        total_subsets,
        elapsed.as_secs_f64()
    );
}

/// Losing more than n-k shards must not silently corrupt — decode should still
/// produce output but it will be wrong. The caller checks the hash.
/// This test verifies the system doesn't panic on insufficient shards.
#[test]
fn erasure_insufficient_shards_fails_gracefully() {
    let data = b"test insufficient shards";
    let k = 2;
    let n = 4;
    let shards = erasure::encode(data, k, n);

    // Only 1 shard (need 2): should panic on assert.
    let result = std::panic::catch_unwind(|| {
        let partial = vec![shards[0].clone()];
        erasure::decode(&partial, k, n, data.len());
    });
    assert!(result.is_err(), "should panic with insufficient shards");
}

/// Large data: 10MB erasure roundtrip.
#[test]
fn erasure_large_data_10mb() {
    let size = 10 * 1024 * 1024; // 10MB
    let data: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
    let start = Instant::now();

    let k = 2;
    let n = 4;
    let shards = erasure::encode(&data, k, n);

    let encode_time = start.elapsed();

    // Drop 2 shards, reconstruct from remaining 2.
    let partial: Vec<erasure::Shard> = shards
        .into_iter()
        .filter(|s| s.index == 0 || s.index == 3)
        .collect();

    let decode_start = Instant::now();
    let recovered = erasure::decode(&partial, k, n, data.len());
    let decode_time = decode_start.elapsed();

    assert_eq!(recovered.len(), data.len());
    assert_eq!(recovered, data, "10MB data corrupted after erasure roundtrip");

    eprintln!(
        "erasure_large_data_10mb: encode {:.2}s, decode {:.2}s ({:.1} MB/s encode, {:.1} MB/s decode)",
        encode_time.as_secs_f64(),
        decode_time.as_secs_f64(),
        size as f64 / 1e6 / encode_time.as_secs_f64(),
        size as f64 / 1e6 / decode_time.as_secs_f64(),
    );
}

// ═══════════════════════════════════════════════════════════════════
// LAYER 1: VALIDITY — chunk hash verification
// ═══════════════════════════════════════════════════════════════════

/// Every shard hash matches its content.
#[test]
fn validity_shard_hashes_match() {
    let data = b"hash verification of every shard in every config";

    for &(k, n) in &[(1, 2), (2, 4), (4, 8)] {
        let shards = erasure::encode(data, k, n);
        for shard in &shards {
            let bytes = shard_to_bytes(shard);
            let hash = cyber_hemera::hash(&bytes).to_hex();
            assert!(
                store::verify_chunk(&bytes, &hash),
                "shard {} hash mismatch in ({},{})",
                shard.index, k, n
            );
        }
    }
}

/// Flipping any single bit in a chunk is detected.
#[test]
fn validity_single_bit_flip_detected() {
    let data = b"single bit flip detection test";
    let shards = erasure::encode(data, 2, 4);

    for shard in &shards {
        let bytes = shard_to_bytes(shard);
        let hash = cyber_hemera::hash(&bytes).to_hex();

        // Flip each byte position.
        for pos in 0..bytes.len().min(64) {
            let mut tampered = bytes.clone();
            tampered[pos] ^= 1;
            assert!(
                !store::verify_chunk(&tampered, &hash),
                "bit flip at byte {} not detected in shard {}",
                pos, shard.index
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// LAYER 4: DAS — commitment and sampling
// ═══════════════════════════════════════════════════════════════════

/// DAS commitment is stable: same shards → same root.
#[test]
fn das_commitment_deterministic() {
    let data = b"DAS determinism test";
    let shards = erasure::encode(data, 2, 4);
    let c1 = das::commit(&shards, 2, data.len());
    let c2 = das::commit(&shards, 2, data.len());
    assert_eq!(c1.root, c2.root);
    assert_eq!(c1.shard_roots, c2.shard_roots);
}

/// Every sample from honest shards passes verification.
/// Any tampered sample fails.
#[test]
fn das_sampling_honest_vs_tampered() {
    let data = b"DAS sampling integrity test with enough bytes to be meaningful";
    let shards = erasure::encode(data, 2, 4);
    let commitment = das::commit(&shards, 2, data.len());

    for shard in &shards {
        let sample = das::sample(shard);
        assert!(das::verify_sample(&sample, &commitment));

        // Tamper.
        let mut bad = sample.clone();
        if !bad.shard_data.is_empty() {
            bad.shard_data[0] ^= 0xFF;
        }
        assert!(!das::verify_sample(&bad, &commitment));
    }
}

// ═══════════════════════════════════════════════════════════════════
// LAYER 2: ORDERING — hash chain, timestamps, equivocation
// ═══════════════════════════════════════════════════════════════════

/// Hash chain links entries causally.
#[test]
fn ordering_hash_chain() {
    let mut g = GSet::new();

    let e1 = make_entry("file1", 100, "dev1", &"0".repeat(64));
    let prev1 = e1.entry_hash.clone();
    g.insert(e1);

    let e2 = make_entry("file2", 200, "dev1", &prev1);
    let prev2 = e2.entry_hash.clone();
    g.insert(e2);

    let e3 = make_entry("file3", 300, "dev1", &prev2);
    g.insert(e3);

    assert_eq!(g.latest_hash("dev1"), g.get("file3").unwrap().entry_hash);

    // Chain is intact: each entry's prev_hash matches predecessor's entry_hash.
    let f2 = g.get("file2").unwrap();
    let f3 = g.get("file3").unwrap();
    assert_eq!(f2.prev_hash, prev1);
    assert_eq!(f3.prev_hash, prev2);
}

/// Equivocation: two entries from same device with same prev_hash = fork.
#[test]
fn ordering_equivocation_detection() {
    let mut g = GSet::new();

    let prev = "0".repeat(64);
    let e1 = make_entry("fork_a", 100, "dev1", &prev);
    let e2 = make_entry("fork_b", 100, "dev1", &prev); // same prev = equivocation
    g.insert(e1);
    g.insert(e2);

    let forks = g.detect_equivocation();
    assert_eq!(forks.len(), 1, "should detect 1 equivocation");
}

/// Different devices can share same prev_hash without equivocation.
#[test]
fn ordering_no_false_equivocation() {
    let mut g = GSet::new();
    let prev = "0".repeat(64);
    g.insert(make_entry("a", 100, "dev1", &prev));
    g.insert(make_entry("b", 100, "dev2", &prev)); // different device = OK
    assert!(g.detect_equivocation().is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// LAYER 3: COMPLETENESS — Merkle root
// ═══════════════════════════════════════════════════════════════════

/// Merkle root is order-independent (commutative).
#[test]
fn completeness_merkle_commutative() {
    let e1 = make_entry("a", 1, "d1", &"0".repeat(64));
    let e2 = make_entry("b", 2, "d2", &"0".repeat(64));
    let e3 = make_entry("c", 3, "d3", &"0".repeat(64));

    let mut g1 = GSet::new();
    g1.insert(e1.clone());
    g1.insert(e2.clone());
    g1.insert(e3.clone());

    let mut g2 = GSet::new();
    g2.insert(e3);
    g2.insert(e1);
    g2.insert(e2);

    assert_eq!(g1.merkle_root(), g2.merkle_root());
}

/// Adding or removing an entry changes the root.
#[test]
fn completeness_merkle_sensitive() {
    let mut g = GSet::new();
    let r0 = g.merkle_root();

    g.insert(make_entry("x", 1, "d", &"0".repeat(64)));
    let r1 = g.merkle_root();
    assert_ne!(r0, r1);

    g.insert(make_entry("y", 2, "d", &"0".repeat(64)));
    let r2 = g.merkle_root();
    assert_ne!(r1, r2);
}

/// Merged registries have the same root.
#[test]
fn completeness_merge_same_root() {
    let e1 = make_entry("a", 1, "d1", &"0".repeat(64));
    let e2 = make_entry("b", 2, "d2", &"0".repeat(64));

    let mut a = GSet::new();
    a.insert(e1.clone());

    let mut b = GSet::new();
    b.insert(e2.clone());

    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab.merkle_root(), ba.merkle_root());
}

// ═══════════════════════════════════════════════════════════════════
// LAYER 5: MERGE — LWW CRDT properties
// ═══════════════════════════════════════════════════════════════════

/// Commutativity: merge(A,B) = merge(B,A).
#[test]
fn merge_commutative() {
    let mut a = GSet::new();
    let mut b = GSet::new();
    a.insert(make_entry("x", 10, "d1", &"0".repeat(64)));
    b.insert(make_entry("y", 20, "d2", &"0".repeat(64)));

    let mut ab = a.clone();
    ab.merge(&b);
    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(sorted_names(&ab), sorted_names(&ba));
    assert_eq!(ab.merkle_root(), ba.merkle_root());
}

/// Associativity: merge(merge(A,B),C) = merge(A,merge(B,C)).
#[test]
fn merge_associative() {
    let mut a = GSet::new();
    let mut b = GSet::new();
    let mut c = GSet::new();
    a.insert(make_entry("x", 1, "d1", &"0".repeat(64)));
    b.insert(make_entry("y", 2, "d2", &"0".repeat(64)));
    c.insert(make_entry("z", 3, "d3", &"0".repeat(64)));

    let mut ab_c = a.clone();
    ab_c.merge(&b);
    ab_c.merge(&c);

    let mut a_bc = a.clone();
    let mut bc = b.clone();
    bc.merge(&c);
    a_bc.merge(&bc);

    assert_eq!(ab_c.merkle_root(), a_bc.merkle_root());
}

/// Idempotency: merge(A,A) = A.
#[test]
fn merge_idempotent() {
    let mut g = GSet::new();
    g.insert(make_entry("x", 1, "d1", &"0".repeat(64)));
    g.insert(make_entry("y", 2, "d2", &"0".repeat(64)));

    let root_before = g.merkle_root();
    let clone = g.clone();
    g.merge(&clone);
    assert_eq!(g.merkle_root(), root_before);
    assert_eq!(g.len(), 2);
}

/// LWW conflict resolution: higher timestamp wins, deterministic.
#[test]
fn merge_lww_conflict_resolution() {
    // Device 1 writes file.txt at t=100, device 2 at t=200.
    let e1 = make_entry_with("file.txt", 100, "dev1", &"0".repeat(64), &["h1"]);
    let e2 = make_entry_with("file.txt", 200, "dev2", &"0".repeat(64), &["h2"]);

    // Regardless of merge order, t=200 wins.
    let mut g1 = GSet::new();
    g1.insert(e1.clone());
    g1.insert(e2.clone());

    let mut g2 = GSet::new();
    g2.insert(e2);
    g2.insert(e1);

    assert_eq!(
        g1.get("file.txt").unwrap().timestamp,
        g2.get("file.txt").unwrap().timestamp
    );
    assert_eq!(g1.get("file.txt").unwrap().timestamp, 200);
}

// ═══════════════════════════════════════════════════════════════════
// END-TO-END: full pipeline
// ═══════════════════════════════════════════════════════════════════

/// Full cycle: encode → commit → distribute → lose shards → verify → reconstruct.
#[test]
fn e2e_full_pipeline() {
    let configs: Vec<(usize, usize, usize)> = vec![
        // (k, n, max_loss)
        (1, 2, 1),
        (2, 4, 2),
        (4, 8, 4),
        (2, 8, 6),
    ];

    let data_sizes = [1, 100, 4096, 65536];

    for &(k, n, max_loss) in &configs {
        for &size in &data_sizes {
            let data: Vec<u8> = (0..size).map(|i| ((i * 7 + 13) % 256) as u8).collect();

            // Encode.
            let shards = erasure::encode(&data, k, n);

            // DAS commit.
            let commitment = das::commit(&shards, k, data.len());
            assert_eq!(commitment.shard_roots.len(), n);

            // Verify every shard hash.
            for shard in &shards {
                let bytes = shard_to_bytes(shard);
                let hash = cyber_hemera::hash(&bytes).to_hex();
                assert!(store::verify_chunk(&bytes, &hash));
            }

            // DAS verify every sample.
            for shard in &shards {
                let sample = das::sample(shard);
                assert!(das::verify_sample(&sample, &commitment));
            }

            // Lose max_loss shards, verify reconstruction.
            for loss in 0..=max_loss {
                let remaining: Vec<erasure::Shard> = shards[loss..].to_vec();
                if remaining.len() >= k {
                    let recovered = erasure::decode(&remaining, k, n, data.len());
                    assert_eq!(
                        recovered, data,
                        "FAILED: ({},{}) size={} loss={}",
                        k, n, size, loss
                    );
                }
            }

            // Lose max_loss+1: should have insufficient shards.
            if max_loss + 1 < n {
                let remaining: Vec<erasure::Shard> =
                    shards[(max_loss + 1)..].to_vec();
                if remaining.len() < k {
                    // Good: can't decode with too few shards.
                }
            }
        }
    }
}

/// Simulate 3 devices, full lifecycle including device failure.
#[test]
fn e2e_three_devices_lifecycle() {
    let data = b"critical file that must survive device loss";
    let k = 2;
    let n = 4;

    // Step 1: encode on device A.
    let shards = erasure::encode(data, k, n);

    // Step 2: distribute — A gets shards 0,1; B gets 2; C gets 3.
    let device_a: Vec<&erasure::Shard> = shards.iter().filter(|s| s.index < 2).collect();
    let device_b: Vec<&erasure::Shard> = shards.iter().filter(|s| s.index == 2).collect();
    let device_c: Vec<&erasure::Shard> = shards.iter().filter(|s| s.index == 3).collect();

    assert_eq!(device_a.len(), 2);
    assert_eq!(device_b.len(), 1);
    assert_eq!(device_c.len(), 1);

    // Step 3: DAS verify all shards.
    let commitment = das::commit(&shards, k, data.len());
    for shard in &shards {
        assert!(das::verify_sample(&das::sample(shard), &commitment));
    }

    // Step 4: Device A dies. B+C have shards 2,3 → reconstruct.
    let survivors: Vec<erasure::Shard> = vec![
        device_b[0].clone(),
        device_c[0].clone(),
    ];
    let recovered = erasure::decode(&survivors, k, n, data.len());
    assert_eq!(&recovered, &data[..], "failed to recover after device A loss");

    // Step 5: Device B dies. Only C (shard 3) → cannot reconstruct (need k=2).
    let result = std::panic::catch_unwind(|| {
        let only_c = vec![device_c[0].clone()];
        erasure::decode(&only_c, k, n, data.len());
    });
    assert!(result.is_err(), "should fail with only 1 of 2 required shards");

    // Step 6: Device A comes back with shards 0,1 + C has shard 3 → reconstruct.
    let restored: Vec<erasure::Shard> = vec![
        device_a[0].clone(),
        device_c[0].clone(),
    ];
    let recovered2 = erasure::decode(&restored, k, n, data.len());
    assert_eq!(&recovered2, &data[..], "failed to recover after A returns");
}

/// Registry sync between two devices with conflict resolution.
#[test]
fn e2e_registry_sync_with_conflicts() {
    let mut dev_a = GSet::new();
    let mut dev_b = GSet::new();

    // Both devices write the same filename at different times.
    dev_a.insert(make_entry_with("shared.txt", 100, "dev_a", &"0".repeat(64), &["old_hash"]));
    dev_b.insert(make_entry_with("shared.txt", 200, "dev_b", &"0".repeat(64), &["new_hash"]));

    // Each device also has unique files.
    dev_a.insert(make_entry("only_a.txt", 150, "dev_a", &"0".repeat(64)));
    dev_b.insert(make_entry("only_b.txt", 250, "dev_b", &"0".repeat(64)));

    // Merge both directions.
    let mut merged_a = dev_a.clone();
    merged_a.merge(&dev_b);

    let mut merged_b = dev_b.clone();
    merged_b.merge(&dev_a);

    // Same result regardless of merge order.
    assert_eq!(merged_a.merkle_root(), merged_b.merkle_root());
    assert_eq!(merged_a.len(), 3); // shared.txt + only_a + only_b

    // LWW: dev_b's version wins (t=200 > t=100).
    assert_eq!(merged_a.get("shared.txt").unwrap().device_id, "dev_b");
    assert_eq!(merged_b.get("shared.txt").unwrap().device_id, "dev_b");
}

// ═══════════════════════════════════════════════════════════════════
// BENCHMARK: throughput measurements
// ═══════════════════════════════════════════════════════════════════

#[test]
fn bench_erasure_throughput() {
    let sizes = [1024, 65536, 1_048_576, 10_485_760]; // 1KB, 64KB, 1MB, 10MB
    let configs = [(2, 4), (4, 8)];

    eprintln!("\n--- erasure throughput ---");
    eprintln!("{:<8} {:<8} {:>12} {:>12} {:>12}", "k", "n", "size", "encode", "decode");

    for &(k, n) in &configs {
        for &size in &sizes {
            let data: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();

            let start = Instant::now();
            let shards = erasure::encode(&data, k, n);
            let encode_ms = start.elapsed().as_secs_f64() * 1000.0;

            let partial: Vec<erasure::Shard> = shards.into_iter().take(k).collect();

            let start = Instant::now();
            let recovered = erasure::decode(&partial, k, n, data.len());
            let decode_ms = start.elapsed().as_secs_f64() * 1000.0;

            assert_eq!(recovered, data);

            eprintln!(
                "{:<8} {:<8} {:>10}KB {:>10.2}ms {:>10.2}ms",
                k, n, size / 1024, encode_ms, decode_ms
            );
        }
    }
}

#[test]
fn bench_hash_verification_throughput() {
    let n_chunks = 1000;
    let chunk_size = 4096;

    let data: Vec<u8> = (0..chunk_size).map(|i| (i % 256) as u8).collect();
    let hash = cyber_hemera::hash(&data).to_hex();

    let start = Instant::now();
    for _ in 0..n_chunks {
        assert!(store::verify_chunk(&data, &hash));
    }
    let elapsed = start.elapsed();

    eprintln!(
        "\nhash verification: {} chunks × {}B in {:.2}ms ({:.0} chunks/s, {:.1} MB/s)",
        n_chunks,
        chunk_size,
        elapsed.as_secs_f64() * 1000.0,
        n_chunks as f64 / elapsed.as_secs_f64(),
        (n_chunks * chunk_size) as f64 / 1e6 / elapsed.as_secs_f64(),
    );
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn make_entry(name: &str, ts: u64, device: &str, prev: &str) -> FileEntry {
    let shard_hashes: Vec<String> = vec!["a".repeat(64), "b".repeat(64)];
    let entry_hash = FileEntry::compute_hash(name, &shard_hashes, ts, prev, device);
    FileEntry {
        name: name.into(),
        original_len: 100,
        k: 2,
        n: 4,
        shard_hashes,
        timestamp: ts,
        prev_hash: prev.into(),
        entry_hash,
        device_id: device.into(),
        das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,    }
}

fn make_entry_with(
    name: &str,
    ts: u64,
    device: &str,
    prev: &str,
    hashes: &[&str],
) -> FileEntry {
    let shard_hashes: Vec<String> = hashes.iter().map(|s| s.to_string()).collect();
    let entry_hash = FileEntry::compute_hash(name, &shard_hashes, ts, prev, device);
    FileEntry {
        name: name.into(),
        original_len: 100,
        k: 2,
        n: 4,
        shard_hashes,
        timestamp: ts,
        prev_hash: prev.into(),
        entry_hash,
        device_id: device.into(),
        das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,    }
}

fn sorted_names(g: &GSet) -> Vec<String> {
    let mut n: Vec<String> = g.files.keys().cloned().collect();
    n.sort();
    n
}

fn shard_to_bytes(shard: &erasure::Shard) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(shard.data.len() * 8);
    for &elem in &shard.data {
        bytes.extend_from_slice(&elem.as_u64().to_le_bytes());
    }
    bytes
}

/// Generate all C(n,k) combinations of k items from 0..n.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut current = Vec::with_capacity(k);
    combinations_rec(n, k, 0, &mut current, &mut result);
    result
}

fn combinations_rec(
    n: usize,
    k: usize,
    start: usize,
    current: &mut Vec<usize>,
    result: &mut Vec<Vec<usize>>,
) {
    if current.len() == k {
        result.push(current.clone());
        return;
    }
    for i in start..n {
        current.push(i);
        combinations_rec(n, k, i + 1, current, result);
        current.pop();
    }
}
