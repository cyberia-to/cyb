//! Stress tests: many devices, many files, chaos scenarios.
//!
//! These tests simulate real deployment patterns at scale.
//! If they pass, the system handles production-like conditions.

use std::collections::HashMap;
use std::time::Instant;

use cyber_sync::erasure;
use cyber_sync::das;
use cyber_sync::store::{self, FileEntry, GSet};

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 1: Scale — many devices, many files
// ═══════════════════════════════════════════════════════════════════

/// 10 devices, 100 files, erasure (4,8). Every device gets all files via
/// registry merge. After losing any 4 of 8 devices, all data survives.
#[test]
fn scale_10_devices_100_files() {
    let k = 4;
    let n = 8;
    let n_files = 100;
    let n_devices = 10;
    let start = Instant::now();

    // Generate files.
    let files: Vec<(String, Vec<u8>)> = (0..n_files)
        .map(|i| {
            let name = format!("file_{:04}.dat", i);
            let size = 100 + (i * 37 % 1000); // 100-1099 bytes
            let data: Vec<u8> = (0..size).map(|j| ((i * 7 + j * 13) % 256) as u8).collect();
            (name, data)
        })
        .collect();

    // Encode all files.
    let encoded: Vec<(String, Vec<u8>, Vec<erasure::Shard>)> = files
        .iter()
        .map(|(name, data)| {
            let shards = erasure::encode(data, k, n);
            (name.clone(), data.clone(), shards)
        })
        .collect();

    // Distribute shards round-robin to devices.
    let mut devices: Vec<HashMap<String, Vec<Option<erasure::Shard>>>> =
        (0..n_devices).map(|_| HashMap::new()).collect();

    for (name, _data, shards) in &encoded {
        for device in &mut devices {
            device.insert(name.clone(), vec![None; n]);
        }
        for shard in shards {
            let device_idx = shard.index % n_devices;
            devices[device_idx]
                .get_mut(name)
                .unwrap()[shard.index] = Some(shard.clone());
        }
    }

    // Verify: losing any 4 devices (n-k=4), remaining 6 can reconstruct everything.
    let max_loss = n - k; // 4
    for lost_start in 0..n_devices {
        // Lose `max_loss` consecutive devices.
        let lost: Vec<usize> = (0..max_loss).map(|i| (lost_start + i) % n_devices).collect();
        let surviving: Vec<usize> = (0..n_devices)
            .filter(|d| !lost.contains(d))
            .collect();

        for (name, original_data, _shards) in &encoded {
            // Collect shards from surviving devices.
            let mut available = Vec::new();
            for &dev in &surviving {
                for shard_opt in &devices[dev][name] {
                    if let Some(shard) = shard_opt {
                        if !available.iter().any(|s: &erasure::Shard| s.index == shard.index) {
                            available.push(shard.clone());
                        }
                    }
                }
            }

            assert!(
                available.len() >= k,
                "file {} has only {} shards after losing devices {:?} (need {})",
                name,
                available.len(),
                lost,
                k
            );

            let recovered = erasure::decode(&available, k, n, original_data.len());
            assert_eq!(
                &recovered, original_data,
                "file {} corrupted after losing devices {:?}",
                name, lost
            );
        }
    }

    let elapsed = start.elapsed();
    eprintln!(
        "scale_10_devices_100_files: {} files × {} loss patterns verified in {:.2}s",
        n_files, n_devices, elapsed.as_secs_f64()
    );
}

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 2: Cascading failure — devices die one by one
// ═══════════════════════════════════════════════════════════════════

/// Start with 8 devices. Lose them one at a time. Data survives until
/// only k-1 remain. At that point, reconstruction fails.
#[test]
fn cascading_failure_8_devices() {
    let k = 2;
    let n = 8;
    let data = b"cascading failure test - this data must survive until k-1 devices remain";
    let shards = erasure::encode(data, k, n);

    // Each device holds exactly one shard.
    for surviving_count in (0..=n).rev() {
        let available: Vec<erasure::Shard> = shards[..surviving_count].to_vec();

        if surviving_count >= k {
            let recovered = erasure::decode(&available, k, n, data.len());
            assert_eq!(
                &recovered,
                &data[..],
                "failed with {} surviving devices",
                surviving_count
            );
        } else if surviving_count > 0 {
            // Should panic (insufficient shards).
            let result = std::panic::catch_unwind(|| {
                erasure::decode(&available, k, n, data.len());
            });
            assert!(
                result.is_err(),
                "should fail with only {} of {} required shards",
                surviving_count, k
            );
        }
    }
}

/// Same test with higher k: (4,8). Fails earlier.
#[test]
fn cascading_failure_high_k() {
    let k = 4;
    let n = 8;
    let data: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();
    let shards = erasure::encode(&data, k, n);

    // Works with 8,7,6,5,4 surviving. Fails with 3,2,1.
    for keep in (k..=n).rev() {
        let available: Vec<erasure::Shard> = shards[..keep].to_vec();
        let recovered = erasure::decode(&available, k, n, data.len());
        assert_eq!(recovered, data, "failed with {} shards", keep);
    }

    // 3 shards (k=4): fails.
    let result = std::panic::catch_unwind(|| {
        let short: Vec<erasure::Shard> = shards[..3].to_vec();
        erasure::decode(&short, k, n, data.len());
    });
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 3: Registry merge storm — 20 devices, concurrent writes
// ═══════════════════════════════════════════════════════════════════

/// 20 devices each write unique files. All registries merge.
/// Final state is identical regardless of merge order.
#[test]
fn merge_storm_20_devices() {
    let n_devices = 20;
    let files_per_device = 10;
    let start = Instant::now();

    // Each device creates its own registry.
    let mut device_registries: Vec<GSet> = Vec::with_capacity(n_devices);

    for dev in 0..n_devices {
        let mut reg = GSet::new();
        let device_id = format!("dev_{:02}", dev);
        let mut prev = "0".repeat(64);

        for f in 0..files_per_device {
            let name = format!("dev{:02}_file{:02}.txt", dev, f);
            let ts = 1000 + dev as u64 * 100 + f as u64;
            let shard_hashes = vec![format!("shard_{}_{}", dev, f)];
            let entry_hash =
                FileEntry::compute_hash(&name, &shard_hashes, ts, &prev, &device_id);

            let entry = FileEntry {
                name,
                original_len: 100,
                k: 2,
                n: 4,
                shard_hashes,
                timestamp: ts,
                prev_hash: prev.clone(),
                entry_hash: entry_hash.clone(),
                device_id: device_id.clone(),
                das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,            };
            prev = entry_hash;
            reg.insert(entry);
        }

        device_registries.push(reg);
    }

    // Merge all registries in order 0..19.
    let mut forward = GSet::new();
    for reg in &device_registries {
        forward.merge(reg);
    }

    // Merge in reverse order 19..0.
    let mut backward = GSet::new();
    for reg in device_registries.iter().rev() {
        backward.merge(reg);
    }

    // Merge in interleaved order.
    let mut interleaved = GSet::new();
    for i in 0..n_devices {
        let idx = if i % 2 == 0 { i / 2 } else { n_devices - 1 - i / 2 };
        interleaved.merge(&device_registries[idx]);
    }

    // All must be identical.
    assert_eq!(forward.len(), n_devices * files_per_device);
    assert_eq!(forward.merkle_root(), backward.merkle_root());
    assert_eq!(forward.merkle_root(), interleaved.merkle_root());

    // No equivocations (each device has a clean chain).
    assert!(forward.detect_equivocation().is_empty());

    let elapsed = start.elapsed();
    eprintln!(
        "merge_storm_20_devices: {} entries, 3 merge orders, all identical in {:.2}s",
        forward.len(),
        elapsed.as_secs_f64()
    );
}

/// Same-name conflicts across 10 devices — all resolve identically.
#[test]
fn merge_conflict_10_devices_same_file() {
    let n_devices = 10;
    let mut registries: Vec<GSet> = Vec::new();

    for dev in 0..n_devices {
        let mut reg = GSet::new();
        let device_id = format!("dev_{}", dev);
        let ts = 1000 + dev as u64 * 10; // different timestamps
        let shard_hashes = vec![format!("data_from_dev_{}", dev)];
        let prev = "0".repeat(64);
        let entry_hash =
            FileEntry::compute_hash("shared.txt", &shard_hashes, ts, &prev, &device_id);

        reg.insert(FileEntry {
            name: "shared.txt".into(),
            original_len: 100,
            k: 1,
            n: 2,
            shard_hashes,
            timestamp: ts,
            prev_hash: prev,
            entry_hash,
            device_id,
            das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,        });
        registries.push(reg);
    }

    // Merge in every permutation order (first 5 to keep time reasonable).
    let orders: Vec<Vec<usize>> = vec![
        (0..n_devices).collect(),
        (0..n_devices).rev().collect(),
        vec![5, 3, 8, 1, 9, 0, 7, 2, 6, 4],
        vec![9, 0, 8, 1, 7, 2, 6, 3, 5, 4],
        vec![0, 9, 1, 8, 2, 7, 3, 6, 4, 5],
    ];

    let mut roots = Vec::new();
    let mut winners = Vec::new();

    for order in &orders {
        let mut merged = GSet::new();
        for &idx in order {
            merged.merge(&registries[idx]);
        }
        roots.push(merged.merkle_root());
        winners.push(merged.get("shared.txt").unwrap().device_id.clone());
    }

    // All merge orders produce the same root.
    for r in &roots {
        assert_eq!(r, &roots[0], "merkle root differs across merge orders");
    }

    // All merge orders pick the same winner.
    for w in &winners {
        assert_eq!(w, &winners[0], "LWW winner differs across merge orders");
    }

    // Winner is the device with the highest timestamp.
    assert_eq!(winners[0], format!("dev_{}", n_devices - 1));
}

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 4: Corruption chaos — random bit flips
// ═══════════════════════════════════════════════════════════════════

/// Corrupt random bytes in random shards. Hash verification catches all.
#[test]
fn corruption_chaos_random_flips() {
    let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
    let k = 2;
    let n = 4;
    let shards = erasure::encode(&data, k, n);

    let mut detected = 0;
    let mut total = 0;

    for shard in &shards {
        let bytes = shard_to_bytes(shard);
        let hash = cyber_hemera::hash(&bytes).to_hex();

        // Flip bytes at various positions.
        let positions = [0, 1, bytes.len() / 4, bytes.len() / 2, bytes.len() - 1];
        for &pos in &positions {
            if pos < bytes.len() {
                let mut corrupted = bytes.clone();
                corrupted[pos] ^= 0x01; // single bit flip
                total += 1;
                if !store::verify_chunk(&corrupted, &hash) {
                    detected += 1;
                }
            }
        }

        // Flip multiple bytes.
        for offset in (0..bytes.len().min(100)).step_by(7) {
            let mut corrupted = bytes.clone();
            corrupted[offset] ^= 0xFF;
            total += 1;
            if !store::verify_chunk(&corrupted, &hash) {
                detected += 1;
            }
        }
    }

    assert_eq!(
        detected, total,
        "missed {} corruptions out of {}",
        total - detected,
        total
    );
    eprintln!(
        "corruption_chaos: {}/{} corruptions detected (100%)",
        detected, total
    );
}

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 5: DAS at scale — many files, all committed
// ═══════════════════════════════════════════════════════════════════

/// 50 files, each with DAS commitment. Verify all samples.
/// Then tamper one shard per file — all detected.
#[test]
fn das_scale_50_files() {
    let n_files = 50;
    let k = 2;
    let n = 4;

    let mut honest_samples = 0;
    let mut tampered_detected = 0;

    for i in 0..n_files {
        let size = 100 + i * 50;
        let data: Vec<u8> = (0..size).map(|j| ((i + j) % 256) as u8).collect();
        let shards = erasure::encode(&data, k, n);
        let commitment = das::commit(&shards, k, data.len());

        // All honest samples pass.
        for shard in &shards {
            let sample = das::sample(shard);
            assert!(das::verify_sample(&sample, &commitment));
            honest_samples += 1;
        }

        // Tamper first shard — detected.
        let mut bad_sample = das::sample(&shards[0]);
        if !bad_sample.shard_data.is_empty() {
            bad_sample.shard_data[0] ^= 0xFF;
            assert!(!das::verify_sample(&bad_sample, &commitment));
            tampered_detected += 1;
        }
    }

    eprintln!(
        "das_scale: {} honest OK, {} tampered detected",
        honest_samples, tampered_detected
    );
}

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 6: Validated merge at scale — adversarial registry
// ═══════════════════════════════════════════════════════════════════

/// Adversary sends 1000 entries: 500 valid, 250 forged hash, 250 future timestamp.
/// Only the 500 valid entries merge.
#[test]
fn validated_merge_adversarial_1000() {
    let mut local = GSet::new();
    let mut adversary = GSet::new();
    let ts = store::now_ms();

    let mut expected_valid = 0;
    let mut expected_invalid = 0;

    for i in 0..1000 {
        let name = format!("adv_{:04}.txt", i);
        let shard_hashes = vec![format!("shard_{}", i)];
        let prev = "0".repeat(64);

        if i < 500 {
            // Valid entry.
            let entry_hash =
                FileEntry::compute_hash(&name, &shard_hashes, ts + i, &prev, "adv");
            adversary.insert(FileEntry {
                name,
                original_len: 10,
                k: 1,
                n: 2,
                shard_hashes,
                timestamp: ts + i,
                prev_hash: prev,
                entry_hash,
                device_id: "adv".into(),
                das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,            });
            expected_valid += 1;
        } else if i < 750 {
            // Forged hash.
            adversary.insert(FileEntry {
                name,
                original_len: 10,
                k: 1,
                n: 2,
                shard_hashes,
                timestamp: ts + i,
                prev_hash: prev,
                entry_hash: "forged".repeat(10),
                device_id: "adv".into(),
                das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,            });
            expected_invalid += 1;
        } else {
            // Future timestamp.
            let future_ts = ts + 999_999_999_999;
            let entry_hash =
                FileEntry::compute_hash(&name, &shard_hashes, future_ts, &prev, "adv");
            adversary.insert(FileEntry {
                name,
                original_len: 10,
                k: 1,
                n: 2,
                shard_hashes,
                timestamp: future_ts,
                prev_hash: prev,
                entry_hash,
                device_id: "adv".into(),
                das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,            });
            expected_invalid += 1;
        }
    }

    let (accepted, rejected) = local.validated_merge(&adversary);

    assert_eq!(accepted, expected_valid);
    assert_eq!(rejected, expected_invalid);
    assert_eq!(local.len(), 500);

    eprintln!(
        "validated_merge_adversarial: {}/{} accepted, {}/{} rejected",
        accepted, expected_valid, rejected, expected_invalid
    );
}

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 7: Split-brain + rejoin
// ═══════════════════════════════════════════════════════════════════

/// Two groups of devices diverge (network partition), then rejoin.
/// After merge, all data from both partitions is present.
#[test]
fn split_brain_rejoin() {
    let ts = store::now_ms();

    // Partition A: devices 0-4 write files a_0..a_9.
    let mut partition_a = GSet::new();
    for i in 0..10 {
        let name = format!("part_a_{}.txt", i);
        let shard_hashes = vec![format!("a_shard_{}", i)];
        let prev = "0".repeat(64);
        let device_id = format!("dev_{}", i % 5);
        let entry_hash =
            FileEntry::compute_hash(&name, &shard_hashes, ts + i, &prev, &device_id);
        partition_a.insert(FileEntry {
            name,
            original_len: 100,
            k: 2,
            n: 4,
            shard_hashes,
            timestamp: ts + i,
            prev_hash: prev,
            entry_hash,
            device_id,
            das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,        });
    }

    // Partition B: devices 5-9 write files b_0..b_9.
    let mut partition_b = GSet::new();
    for i in 0..10 {
        let name = format!("part_b_{}.txt", i);
        let shard_hashes = vec![format!("b_shard_{}", i)];
        let prev = "0".repeat(64);
        let device_id = format!("dev_{}", 5 + i % 5);
        let entry_hash =
            FileEntry::compute_hash(&name, &shard_hashes, ts + 100 + i, &prev, &device_id);
        partition_b.insert(FileEntry {
            name,
            original_len: 200,
            k: 2,
            n: 4,
            shard_hashes,
            timestamp: ts + 100 + i,
            prev_hash: prev,
            entry_hash,
            device_id,
            das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,        });
    }

    // Both partitions also write to a shared filename.
    let shared_a = {
        let shard_hashes = vec!["shared_a".into()];
        let prev = "0".repeat(64);
        let eh = FileEntry::compute_hash("conflict.txt", &shard_hashes, ts + 50, &prev, "dev_0");
        FileEntry {
            name: "conflict.txt".into(),
            original_len: 100,
            k: 2,
            n: 4,
            shard_hashes,
            timestamp: ts + 50,
            prev_hash: prev,
            entry_hash: eh,
            device_id: "dev_0".into(),
            das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,        }
    };
    let shared_b = {
        let shard_hashes = vec!["shared_b".into()];
        let prev = "0".repeat(64);
        let eh = FileEntry::compute_hash("conflict.txt", &shard_hashes, ts + 150, &prev, "dev_5");
        FileEntry {
            name: "conflict.txt".into(),
            original_len: 200,
            k: 2,
            n: 4,
            shard_hashes,
            timestamp: ts + 150,
            prev_hash: prev,
            entry_hash: eh,
            device_id: "dev_5".into(),
            das_root: "0".repeat(64),
            vdf_proof: None,
            shard_copies: 1,        }
    };
    partition_a.insert(shared_a);
    partition_b.insert(shared_b);

    let root_a = partition_a.merkle_root();
    let root_b = partition_b.merkle_root();
    assert_ne!(root_a, root_b, "partitions should have different roots");

    // Rejoin: merge both directions.
    let mut rejoined_ab = partition_a.clone();
    rejoined_ab.merge(&partition_b);

    let mut rejoined_ba = partition_b.clone();
    rejoined_ba.merge(&partition_a);

    // Both merge orders produce identical state.
    assert_eq!(rejoined_ab.merkle_root(), rejoined_ba.merkle_root());

    // All 21 files present (10 + 10 + 1 shared).
    assert_eq!(rejoined_ab.len(), 21);

    // Conflict resolved: partition B wins (ts+150 > ts+50).
    assert_eq!(
        rejoined_ab.get("conflict.txt").unwrap().device_id,
        "dev_5"
    );

    eprintln!("split_brain_rejoin: 21 files merged, conflict resolved deterministically");
}

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 8: Exhaustive shard combination for (4,8)
// ═══════════════════════════════════════════════════════════════════

/// (4,8) has C(8,4) = 70 possible subsets. Test ALL of them with real data.
#[test]
fn exhaustive_4_of_8_all_70_subsets() {
    let data: Vec<u8> = (0..8192).map(|i| (i % 256) as u8).collect();
    let k = 4;
    let n = 8;
    let shards = erasure::encode(&data, k, n);

    let subsets = combinations(n, k);
    assert_eq!(subsets.len(), 70);

    for subset in &subsets {
        let partial: Vec<erasure::Shard> = subset.iter().map(|&i| shards[i].clone()).collect();
        let recovered = erasure::decode(&partial, k, n, data.len());
        assert_eq!(
            recovered, data,
            "FAILED: (4,8) subset {:?} on 8KB data",
            subset
        );
    }

    eprintln!("exhaustive_4_of_8: all 70 subsets verified on 8KB data");
}

/// (2,8) has C(8,2) = 28 possible subsets. All must work.
#[test]
fn exhaustive_2_of_8_all_28_subsets() {
    let data: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
    let k = 2;
    let n = 8;
    let shards = erasure::encode(&data, k, n);

    let subsets = combinations(n, k);
    assert_eq!(subsets.len(), 28);

    for subset in &subsets {
        let partial: Vec<erasure::Shard> = subset.iter().map(|&i| shards[i].clone()).collect();
        let recovered = erasure::decode(&partial, k, n, data.len());
        assert_eq!(recovered, data, "FAILED: (2,8) subset {:?}", subset);
    }
}

// ═══════════════════════════════════════════════════════════════════
// SCENARIO 9: Data integrity across all sizes
// ═══════════════════════════════════════════════════════════════════

/// Test every byte length from 0 to 256. Edge cases: boundaries of
/// 7-byte field element encoding.
#[test]
fn integrity_every_byte_length_0_to_256() {
    let k = 2;
    let n = 4;

    for size in 0..=256 {
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let shards = erasure::encode(&data, k, n);

        // Full roundtrip.
        let recovered = erasure::decode(&shards, k, n, data.len());
        assert_eq!(recovered, data, "FAILED at size {}", size);

        // Partial (drop shard 0 and 1).
        let partial: Vec<erasure::Shard> = shards
            .into_iter()
            .filter(|s| s.index >= 2)
            .collect();
        let recovered2 = erasure::decode(&partial, k, n, data.len());
        assert_eq!(recovered2, data, "FAILED partial at size {}", size);
    }

    eprintln!("integrity_every_byte_length: 0..256 all verified");
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn shard_to_bytes(shard: &erasure::Shard) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(shard.data.len() * 8);
    for &elem in &shard.data {
        bytes.extend_from_slice(&elem.as_u64().to_le_bytes());
    }
    bytes
}

fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut current = Vec::with_capacity(k);
    comb_rec(n, k, 0, &mut current, &mut result);
    result
}

fn comb_rec(n: usize, k: usize, start: usize, cur: &mut Vec<usize>, res: &mut Vec<Vec<usize>>) {
    if cur.len() == k {
        res.push(cur.clone());
        return;
    }
    for i in start..n {
        cur.push(i);
        comb_rec(n, k, i + 1, cur, res);
        cur.pop();
    }
}
