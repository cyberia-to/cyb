//! Attack vector tests for CRDT merge and structural sync.
//!
//! Each test models a specific adversarial scenario.
//! If these pass, the system rejects all tested attacks.

use cyber_sync::store::{self, FileEntry, GSet, ValidationError, MAX_CLOCK_DRIFT_MS};

// ═══════════════════════════════════════════════════════════════════
// ATTACK 1: Timestamp manipulation
// ═══════════════════════════════════════════════════════════════════

/// Peer sets timestamp to u64::MAX to permanently win LWW.
#[test]
fn attack_timestamp_max() {
    let mut g = GSet::new();
    let mut evil = make_entry("target.txt", u64::MAX, "evil_device");
    evil.entry_hash = FileEntry::compute_hash(
        &evil.name, &evil.shard_hashes, evil.timestamp, &evil.device_id,
    );
    assert_eq!(g.validated_insert(evil), Err(ValidationError::TimestampFuture));
    assert!(g.is_empty());
}

/// Peer sets timestamp 2 hours in the future (beyond MAX_CLOCK_DRIFT).
#[test]
fn attack_timestamp_future() {
    let mut g = GSet::new();
    let future_ts = store::now_ms() + MAX_CLOCK_DRIFT_MS + 1_000;
    let mut evil = make_entry("future.txt", future_ts, "evil");
    evil.entry_hash = FileEntry::compute_hash(
        &evil.name, &evil.shard_hashes, evil.timestamp, &evil.device_id,
    );
    assert_eq!(g.validated_insert(evil), Err(ValidationError::TimestampFuture));
}

/// Timestamp just within drift limit — should be accepted.
#[test]
fn attack_timestamp_within_drift_accepted() {
    let mut g = GSet::new();
    let ok_ts = store::now_ms() + MAX_CLOCK_DRIFT_MS - 1_000;
    let mut entry = make_entry("ok.txt", ok_ts, "device");
    entry.entry_hash = FileEntry::compute_hash(
        &entry.name, &entry.shard_hashes, entry.timestamp, &entry.device_id,
    );
    assert!(g.validated_insert(entry).is_ok());
    assert_eq!(g.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════
// ATTACK 2: Forged entry_hash
// ═══════════════════════════════════════════════════════════════════

/// Peer sends entry with forged entry_hash that doesn't match contents.
#[test]
fn attack_forged_entry_hash() {
    let mut g = GSet::new();
    let mut evil = make_entry("doc.txt", store::now_ms(), "device");
    evil.entry_hash = "f".repeat(64);
    assert_eq!(g.validated_insert(evil), Err(ValidationError::HashMismatch));
}

/// Peer modifies shard_hashes after computing entry_hash.
#[test]
fn attack_shard_hash_swap() {
    let mut g = GSet::new();
    let ts = store::now_ms();
    let mut entry = make_entry("swap.txt", ts, "device");
    entry.entry_hash = FileEntry::compute_hash(
        &entry.name, &entry.shard_hashes, ts, &entry.device_id,
    );
    entry.shard_hashes = vec!["deadbeef".repeat(8)];
    assert_eq!(g.validated_insert(entry), Err(ValidationError::HashMismatch));
}

/// Peer modifies device_id after computing entry_hash.
#[test]
fn attack_device_id_spoof() {
    let mut g = GSet::new();
    let ts = store::now_ms();
    let mut entry = make_entry("spoof.txt", ts, "real_device");
    entry.entry_hash = FileEntry::compute_hash(
        &entry.name, &entry.shard_hashes, ts, "real_device",
    );
    entry.device_id = "fake_device".to_string();
    assert_eq!(g.validated_insert(entry), Err(ValidationError::HashMismatch));
}

/// Peer modifies timestamp after computing entry_hash.
#[test]
fn attack_timestamp_tamper_after_hash() {
    let mut g = GSet::new();
    let ts = store::now_ms();
    let mut entry = make_entry("ts.txt", ts, "device");
    entry.entry_hash = FileEntry::compute_hash(
        &entry.name, &entry.shard_hashes, ts, &entry.device_id,
    );
    entry.timestamp = ts + 999_999;
    assert_eq!(g.validated_insert(entry), Err(ValidationError::HashMismatch));
}

// ═══════════════════════════════════════════════════════════════════
// ATTACK 3: Shard hash poisoning
// ═══════════════════════════════════════════════════════════════════

/// Entry with empty shard_hashes — rejected.
#[test]
fn attack_empty_shards() {
    let mut g = GSet::new();
    let ts = store::now_ms();
    let shard_hashes: Vec<String> = vec![];
    let entry = FileEntry {
        name: "empty.txt".into(),
        original_len: 100,
        k: 2,
        n: 4,
        shard_hashes: shard_hashes.clone(),
        timestamp: ts,
        entry_hash: FileEntry::compute_hash("empty.txt", &shard_hashes, ts, "dev"),
        device_id: "dev".into(),
        das_root: "0".repeat(64),
        shard_copies: 1,
        deleted: false,
    };
    assert_eq!(g.validated_insert(entry), Err(ValidationError::NoShards));
}

/// Entry with empty name — rejected.
#[test]
fn attack_empty_name() {
    let mut g = GSet::new();
    let ts = store::now_ms();
    let shard_hashes = vec!["abc".into()];
    let entry = FileEntry {
        name: "".into(),
        original_len: 100,
        k: 1,
        n: 2,
        shard_hashes: shard_hashes.clone(),
        timestamp: ts,
        entry_hash: FileEntry::compute_hash("", &shard_hashes, ts, "dev"),
        device_id: "dev".into(),
        das_root: "0".repeat(64),
        shard_copies: 1,
        deleted: false,
    };
    assert_eq!(g.validated_insert(entry), Err(ValidationError::EmptyName));
}

// ═══════════════════════════════════════════════════════════════════
// ATTACK 4: Registry bloat / DoS
// ═══════════════════════════════════════════════════════════════════

/// Peer sends more entries than MAX_REGISTRY_ENTRIES — excess rejected.
#[test]
fn attack_registry_bloat() {
    let mut g = GSet::new();
    let ts = store::now_ms();
    let mut accepted = 0;
    let mut rejected = 0;

    for i in 0..200 {
        let name = format!("file_{:06}.txt", i);
        let shard_hashes = vec![format!("hash_{}", i)];
        let entry = FileEntry {
            name: name.clone(),
            original_len: 10,
            k: 1,
            n: 2,
            shard_hashes: shard_hashes.clone(),
            timestamp: ts + i as u64,
            entry_hash: FileEntry::compute_hash(&name, &shard_hashes, ts + i as u64, "d"),
            device_id: "d".into(),
            das_root: "0".repeat(64),
            shard_copies: 1,
            deleted: false,
        };
        match g.validated_insert(entry) {
            Ok(()) => accepted += 1,
            Err(ValidationError::RegistryFull) => rejected += 1,
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    assert_eq!(accepted, 200);
    assert_eq!(rejected, 0);
}

// ═══════════════════════════════════════════════════════════════════
// ATTACK 5: Name squatting
// ═══════════════════════════════════════════════════════════════════

/// Peer squats a name. Legitimate owner wins with higher timestamp.
#[test]
fn attack_name_squatting() {
    let mut g = GSet::new();
    let ts = store::now_ms();

    let mut squatter = make_entry("important.txt", ts, "squatter");
    squatter.entry_hash = FileEntry::compute_hash(
        &squatter.name, &squatter.shard_hashes, ts, &squatter.device_id,
    );
    g.validated_insert(squatter).unwrap();

    let mut legit = make_entry_with_hashes("important.txt", ts + 1, "owner", &["real_data"]);
    legit.entry_hash = FileEntry::compute_hash(
        &legit.name, &legit.shard_hashes, legit.timestamp, &legit.device_id,
    );
    g.validated_insert(legit).unwrap();

    assert_eq!(g.get("important.txt").unwrap().device_id, "owner");
}

/// Squatter at drift boundary — still bounded.
#[test]
fn attack_name_squatting_max_drift() {
    let mut g = GSet::new();
    let now = store::now_ms();
    let drift_limit = now + MAX_CLOCK_DRIFT_MS - 1;

    let mut squatter = make_entry("target.txt", drift_limit, "squatter");
    squatter.entry_hash = FileEntry::compute_hash(
        &squatter.name, &squatter.shard_hashes, drift_limit, &squatter.device_id,
    );
    g.validated_insert(squatter).unwrap();
    assert_eq!(g.get("target.txt").unwrap().device_id, "squatter");
}

// ═══════════════════════════════════════════════════════════════════
// ATTACK 6: validated_merge rejects bad entries in bulk
// ═══════════════════════════════════════════════════════════════════

/// Mix of valid and invalid entries — only valid ones merge.
#[test]
fn attack_mixed_valid_invalid_merge() {
    let mut local = GSet::new();
    let mut remote = GSet::new();
    let ts = store::now_ms();

    let mut good = make_entry("good.txt", ts, "peer");
    good.entry_hash = FileEntry::compute_hash(
        &good.name, &good.shard_hashes, ts, &good.device_id,
    );
    remote.insert(good);

    let mut forged = make_entry("forged.txt", ts, "peer");
    forged.entry_hash = "bad_hash".repeat(8);
    remote.insert(forged);

    let mut future = make_entry("future.txt", u64::MAX, "peer");
    future.entry_hash = FileEntry::compute_hash(
        &future.name, &future.shard_hashes, u64::MAX, &future.device_id,
    );
    remote.insert(future);

    let (accepted, rejected) = local.validated_merge(&remote);
    assert_eq!(accepted, 1);
    assert_eq!(rejected, 2);
    assert!(local.get("good.txt").is_some());
    assert!(local.get("forged.txt").is_none());
    assert!(local.get("future.txt").is_none());
}

// ═══════════════════════════════════════════════════════════════════
// ATTACK 7: Merkle root manipulation
// ═══════════════════════════════════════════════════════════════════

/// Forged entry_hash → different Merkle root.
#[test]
fn attack_merkle_root_with_forged_entry() {
    let ts = store::now_ms();

    let mut honest = GSet::new();
    let mut entry = make_entry("file.txt", ts, "dev");
    entry.entry_hash = FileEntry::compute_hash(
        &entry.name, &entry.shard_hashes, ts, &entry.device_id,
    );
    honest.insert(entry);

    let mut forged = GSet::new();
    let mut bad = make_entry("file.txt", ts, "dev");
    bad.entry_hash = "f".repeat(64);
    forged.insert(bad);

    assert_ne!(honest.merkle_root(), forged.merkle_root());
}

/// Removing an entry changes the root — omission is detectable.
#[test]
fn attack_merkle_omission_detectable() {
    let mut full = GSet::new();
    let ts = store::now_ms();

    let mut e1 = make_entry("a.txt", ts, "d");
    e1.entry_hash = FileEntry::compute_hash(&e1.name, &e1.shard_hashes, ts, &e1.device_id);
    let mut e2 = make_entry("b.txt", ts + 1, "d");
    e2.entry_hash = FileEntry::compute_hash(&e2.name, &e2.shard_hashes, ts + 1, &e2.device_id);

    full.insert(e1.clone());
    full.insert(e2.clone());

    let mut partial = GSet::new();
    partial.insert(e1);

    assert_ne!(full.merkle_root(), partial.merkle_root());
}

// ═══════════════════════════════════════════════════════════════════
// DEFENSE: verify_hash is the binding
// ═══════════════════════════════════════════════════════════════════

/// Honest entry passes all validation.
#[test]
fn defense_honest_entry_passes() {
    let mut g = GSet::new();
    let ts = store::now_ms();
    let mut entry = make_entry("legit.txt", ts, "honest_device");
    entry.entry_hash = FileEntry::compute_hash(
        &entry.name, &entry.shard_hashes, ts, &entry.device_id,
    );
    assert!(entry.verify_hash());
    assert!(g.validated_insert(entry).is_ok());
}

/// Every bound field causes rejection when modified after hash computation.
#[test]
fn defense_all_fields_bound() {
    let ts = store::now_ms();
    let base = make_valid_entry("bound.txt", ts, "dev");

    let mut e = base.clone();
    e.name = "other.txt".into();
    assert!(!e.verify_hash());

    let mut e = base.clone();
    e.shard_hashes = vec!["changed".into()];
    assert!(!e.verify_hash());

    let mut e = base.clone();
    e.timestamp += 1;
    assert!(!e.verify_hash());

    let mut e = base.clone();
    e.device_id = "fake".into();
    assert!(!e.verify_hash());
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn make_entry(name: &str, ts: u64, device: &str) -> FileEntry {
    let shard_hashes = vec!["a".repeat(64), "b".repeat(64)];
    FileEntry {
        name: name.into(),
        original_len: 100,
        k: 2,
        n: 4,
        shard_hashes,
        timestamp: ts,
        entry_hash: String::new(), // caller must set
        device_id: device.into(),
        das_root: "0".repeat(64),
        shard_copies: 1,
        deleted: false,
    }
}

fn make_entry_with_hashes(name: &str, ts: u64, device: &str, hashes: &[&str]) -> FileEntry {
    let shard_hashes: Vec<String> = hashes.iter().map(|s| s.to_string()).collect();
    FileEntry {
        name: name.into(),
        original_len: 100,
        k: 1,
        n: 2,
        shard_hashes,
        timestamp: ts,
        entry_hash: String::new(),
        device_id: device.into(),
        das_root: "0".repeat(64),
        shard_copies: 1,
        deleted: false,
    }
}

fn make_valid_entry(name: &str, ts: u64, device: &str) -> FileEntry {
    let shard_hashes = vec!["a".repeat(64), "b".repeat(64)];
    let entry_hash = FileEntry::compute_hash(name, &shard_hashes, ts, device);
    FileEntry {
        name: name.into(),
        original_len: 100,
        k: 2,
        n: 4,
        shard_hashes,
        timestamp: ts,
        entry_hash,
        device_id: device.into(),
        das_root: "0".repeat(64),
        shard_copies: 1,
        deleted: false,
    }
}
