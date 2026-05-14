//! Content-addressed chunk store with CRDT merge.
//!
//! Layer 4: erasure-coded blob storage, content-addressed chunks.
//! Layer 5: LWW-Element-Set — last-writer-wins by timestamp, deterministic merge.
//! Layer 3: Registry has a Hemera Merkle root for completeness verification.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cyber_hemera::Hash;
use serde::{Deserialize, Serialize};

use crate::erasure::Shard;

/// A file tracked by the system.
///
/// Layer 4: content-addressed blob entry with erasure coding and DAS commitment.
/// Layer 5: carries timestamp for LWW merge and device_id for ownership.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub original_len: usize,
    pub k: usize,
    pub n: usize,
    /// Hemera hash of each shard (indexed by shard index).
    pub shard_hashes: Vec<String>,
    /// Unix timestamp in milliseconds when this entry was created (LWW key).
    pub timestamp: u64,
    /// Hash of this entry itself (H(name || shard_hashes || timestamp || device_id)).
    pub entry_hash: String,
    /// Device that created this entry.
    pub device_id: String,
    /// DAS commitment root over erasure-coded shards (layer 4).
    pub das_root: String,
    /// Number of shard copies per device (controls D/k tradeoff).
    #[serde(default = "default_shard_copies")]
    pub shard_copies: usize,
    /// Tombstone: true = this file has been deleted. Shards can be GC'd.
    #[serde(default)]
    pub deleted: bool,
}

fn default_shard_copies() -> usize { 1 }

/// Maximum allowed clock drift from local time (1 hour in ms).
/// Entries with timestamps beyond now + MAX_CLOCK_DRIFT are rejected.
pub const MAX_CLOCK_DRIFT_MS: u64 = 3_600_000;

/// Maximum entries per registry to prevent bloat attacks.
pub const MAX_REGISTRY_ENTRIES: usize = 100_000;

/// Validation errors for incoming entries.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// entry_hash does not match recomputed hash.
    HashMismatch,
    /// Timestamp is too far in the future.
    TimestampFuture,
    /// Entry name is empty.
    EmptyName,
    /// Shard hashes list is empty.
    NoShards,
    /// Registry is full.
    RegistryFull,
}

impl FileEntry {
    /// Compute the entry hash deterministically.
    pub fn compute_hash(
        name: &str,
        shard_hashes: &[String],
        timestamp: u64,
        device_id: &str,
    ) -> String {
        let mut data = Vec::new();
        data.extend_from_slice(name.as_bytes());
        for h in shard_hashes {
            data.extend_from_slice(h.as_bytes());
        }
        data.extend_from_slice(&timestamp.to_le_bytes());
        data.extend_from_slice(device_id.as_bytes());
        cyber_hemera::hash(&data).to_hex()
    }

    /// Verify this entry's entry_hash matches its contents.
    pub fn verify_hash(&self) -> bool {
        let expected = Self::compute_hash(
            &self.name,
            &self.shard_hashes,
            self.timestamp,
            &self.device_id,
        );
        self.entry_hash == expected
    }

    /// Validate an entry from an untrusted source.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.name.is_empty() {
            return Err(ValidationError::EmptyName);
        }
        if self.shard_hashes.is_empty() {
            return Err(ValidationError::NoShards);
        }
        if !self.verify_hash() {
            return Err(ValidationError::HashMismatch);
        }
        let now = now_ms();
        if self.timestamp > now + MAX_CLOCK_DRIFT_MS {
            return Err(ValidationError::TimestampFuture);
        }
        Ok(())
    }
}

/// Content-addressed chunk store backed by a directory.
pub struct ChunkStore {
    root: PathBuf,
    index: HashMap<String, Vec<u8>>,
    capacity: u64,
    used: u64,
}

impl ChunkStore {
    pub fn new(root: &Path, capacity: u64) -> std::io::Result<Self> {
        std::fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            index: HashMap::new(),
            capacity,
            used: 0,
        })
    }

    pub fn put(&mut self, shard: &Shard) -> std::io::Result<Hash> {
        let bytes = shard_to_bytes(shard);
        let hash = cyber_hemera::hash(&bytes);
        let hex = hash.to_hex();

        let size = bytes.len() as u64;
        if self.capacity > 0 && self.used + size > self.capacity {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "chunk store capacity exceeded",
            ));
        }

        let path = self.root.join(&hex);
        if !path.exists() {
            std::fs::write(&path, &bytes)?;
            self.used += size;
        }
        self.index.insert(hex, bytes);
        Ok(hash)
    }

    pub fn get(&self, hash: &Hash) -> std::io::Result<Vec<u8>> {
        let hex = hash.to_hex();
        if let Some(data) = self.index.get(&hex) {
            return Ok(data.clone());
        }
        let path = self.root.join(&hex);
        std::fs::read(&path)
    }

    pub fn has(&self, hash: &Hash) -> bool {
        let hex = hash.to_hex();
        self.index.contains_key(&hex) || self.root.join(&hex).exists()
    }

    /// Remove a chunk by hash hex. Returns bytes freed.
    pub fn remove(&mut self, hash_hex: &str) -> std::io::Result<u64> {
        let path = self.root.join(hash_hex);
        let size = if path.exists() {
            let meta = std::fs::metadata(&path)?;
            std::fs::remove_file(&path)?;
            meta.len()
        } else {
            0
        };
        self.index.remove(hash_hex);
        self.used = self.used.saturating_sub(size);
        Ok(size)
    }

    /// Garbage collect: remove chunks not in the live set.
    /// Returns (removed_count, bytes_freed).
    pub fn gc(&mut self, live_hashes: &std::collections::HashSet<String>) -> std::io::Result<(usize, u64)> {
        // List all chunks on disk.
        let all_chunks: Vec<String> = std::fs::read_dir(&self.root)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.len() == 64 { Some(name) } else { None }
            })
            .collect();

        let mut removed = 0;
        let mut freed = 0u64;

        for hash_hex in &all_chunks {
            if !live_hashes.contains(hash_hex) {
                freed += self.remove(hash_hex)?;
                removed += 1;
            }
        }

        Ok((removed, freed))
    }

    /// Integrity audit: verify all local chunks match their hash.
    /// Returns (ok_count, corrupt_count, corrupt_hashes).
    pub fn audit(&self) -> std::io::Result<(usize, usize, Vec<String>)> {
        let mut ok = 0;
        let mut corrupt = Vec::new();

        let entries: Vec<String> = std::fs::read_dir(&self.root)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.len() == 64 { Some(name) } else { None }
            })
            .collect();

        for hash_hex in &entries {
            let path = self.root.join(hash_hex);
            if let Ok(bytes) = std::fs::read(&path) {
                if verify_chunk(&bytes, hash_hex) {
                    ok += 1;
                } else {
                    corrupt.push(hash_hex.clone());
                }
            }
        }

        Ok((ok, corrupt.len(), corrupt))
    }

    /// List all chunk hashes on disk.
    pub fn list_chunks(&self) -> std::io::Result<Vec<String>> {
        let entries: Vec<String> = std::fs::read_dir(&self.root)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.len() == 64 { Some(name) } else { None }
            })
            .collect();
        Ok(entries)
    }

    pub fn used(&self) -> u64 {
        self.used
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

/// LWW-Element-Set: last-writer-wins by timestamp.
///
/// Layer 5 (merge): commutative, associative, idempotent.
/// For same-name conflicts, highest timestamp wins.
/// Ties broken by entry_hash (deterministic).
///
/// Layer 3 (completeness): registry_root is the Hemera Merkle root
/// over sorted entries. Peers verify this root on sync.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GSet {
    pub files: HashMap<String, FileEntry>,
}

impl GSet {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Insert an entry (trusted — from local put_file). No validation.
    pub fn insert(&mut self, entry: FileEntry) {
        self.lww_insert(entry);
    }

    /// Insert an entry from an untrusted source with full validation.
    /// Returns Err if entry is invalid (forged hash, future timestamp, etc).
    pub fn validated_insert(&mut self, entry: FileEntry) -> Result<(), ValidationError> {
        entry.validate()?;
        if self.files.len() >= MAX_REGISTRY_ENTRIES && !self.files.contains_key(&entry.name) {
            return Err(ValidationError::RegistryFull);
        }
        self.lww_insert(entry);
        Ok(())
    }

    /// Merge with another set from an untrusted source.
    /// Validates every entry. Returns count of (accepted, rejected).
    pub fn validated_merge(&mut self, other: &GSet) -> (usize, usize) {
        let mut accepted = 0;
        let mut rejected = 0;
        for entry in other.files.values() {
            match self.validated_insert(entry.clone()) {
                Ok(()) => accepted += 1,
                Err(_) => rejected += 1,
            }
        }
        (accepted, rejected)
    }

    /// Merge with trusted source (same device, from disk). No validation.
    pub fn merge(&mut self, other: &GSet) {
        for entry in other.files.values() {
            self.lww_insert(entry.clone());
        }
    }

    /// LWW insert logic: higher timestamp wins, tie-break by entry_hash.
    fn lww_insert(&mut self, entry: FileEntry) {
        match self.files.get(&entry.name) {
            Some(existing) => {
                if entry.timestamp > existing.timestamp
                    || (entry.timestamp == existing.timestamp
                        && entry.entry_hash > existing.entry_hash)
                {
                    self.files.insert(entry.name.clone(), entry);
                }
            }
            None => {
                self.files.insert(entry.name.clone(), entry);
            }
        }
    }

    /// Compute Merkle root over sorted entries (layer 3: completeness).
    ///
    /// Root = H(sorted entry hashes concatenated).
    /// Two registries with the same entries produce the same root.
    pub fn merkle_root(&self) -> String {
        let mut hashes: Vec<&str> = self.files.values().map(|e| e.entry_hash.as_str()).collect();
        hashes.sort();
        let mut data = Vec::new();
        for h in &hashes {
            data.extend_from_slice(h.as_bytes());
        }
        if data.is_empty() {
            return "0".repeat(64);
        }
        cyber_hemera::hash(&data).to_hex()
    }

    /// List live (non-deleted) file names.
    pub fn list(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.files
            .values()
            .filter(|e| !e.deleted)
            .map(|e| e.name.as_str())
            .collect();
        names.sort();
        names
    }

    /// Get a live (non-deleted) file entry by name.
    pub fn get(&self, name: &str) -> Option<&FileEntry> {
        self.files.get(name).filter(|e| !e.deleted)
    }

    /// Get entry including tombstones (for GC/internal use).
    pub fn get_raw(&self, name: &str) -> Option<&FileEntry> {
        self.files.get(name)
    }

    /// Number of live (non-deleted) entries.
    pub fn len(&self) -> usize {
        self.files.values().filter(|e| !e.deleted).count()
    }

    /// Number of tombstones.
    pub fn tombstone_count(&self) -> usize {
        self.files.values().filter(|e| e.deleted).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Compact: remove tombstones older than `max_age_ms`.
    /// Only safe after all devices have synced past the tombstone.
    pub fn compact(&mut self, max_age_ms: u64) -> usize {
        let now = now_ms();
        let before = self.files.len();
        self.files.retain(|_, e| {
            !(e.deleted && now.saturating_sub(e.timestamp) > max_age_ms)
        });
        before - self.files.len()
    }

    /// Delta: entries newer than `since_timestamp`.
    /// Used for delta sync — only send what changed.
    pub fn delta_since(&self, since_timestamp: u64) -> GSet {
        let mut delta = GSet::new();
        for entry in self.files.values() {
            if entry.timestamp > since_timestamp {
                delta.insert(entry.clone());
            }
        }
        delta
    }

    /// All shard hashes referenced by live entries (for GC).
    pub fn live_shard_hashes(&self) -> std::collections::HashSet<String> {
        let mut hashes = std::collections::HashSet::new();
        for entry in self.files.values() {
            if !entry.deleted {
                for h in &entry.shard_hashes {
                    hashes.insert(h.clone());
                }
            }
        }
        hashes
    }

}

/// Serialize shard data for hashing/storage.
fn shard_to_bytes(shard: &Shard) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(shard.data.len() * 8);
    for &elem in &shard.data {
        bytes.extend_from_slice(&elem.as_u64().to_le_bytes());
    }
    bytes
}

/// Deserialize bytes back to a Shard.
pub fn bytes_to_shard(index: usize, bytes: &[u8]) -> Shard {
    let mut data = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks(8) {
        if chunk.len() == 8 {
            let val = u64::from_le_bytes(chunk.try_into().unwrap());
            data.push(nebu::Goldilocks::new(val));
        }
    }
    Shard { index, data }
}

/// Verify chunk bytes against expected Hemera hash.
pub fn verify_chunk(bytes: &[u8], expected_hash_hex: &str) -> bool {
    let computed = cyber_hemera::hash(bytes);
    computed.to_hex() == expected_hash_hex
}

/// Current time in milliseconds.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erasure;

    fn make_entry(name: &str, ts: u64) -> FileEntry {
        let shard_hashes = vec!["aaa".into(), "bbb".into()];
        let entry_hash = FileEntry::compute_hash(name, &shard_hashes, ts, "dev1");
        FileEntry {
            name: name.into(),
            original_len: 100,
            k: 2,
            n: 4,
            shard_hashes,
            timestamp: ts,
            entry_hash,
            device_id: "dev1".into(),
            das_root: "0".repeat(64),
            shard_copies: 1, deleted: false,
        }
    }

    #[test]
    fn store_and_retrieve() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ChunkStore::new(dir.path(), 0).unwrap();
        let data = b"chunk store test data";
        let shards = erasure::encode(data, 2, 4);
        let hash = store.put(&shards[0]).unwrap();
        assert!(store.has(&hash));
        let retrieved = store.get(&hash).unwrap();
        let expected = shard_to_bytes(&shards[0]);
        assert_eq!(retrieved, expected);
    }

    #[test]
    fn capacity_limit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ChunkStore::new(dir.path(), 10).unwrap();
        let data = b"this data is definitely more than 10 bytes when encoded";
        let shards = erasure::encode(data, 2, 4);
        assert!(store.put(&shards[0]).is_err());
    }

    #[test]
    fn lww_merge_higher_timestamp_wins() {
        let mut a = GSet::new();
        let mut b = GSet::new();

        a.insert(make_entry("file.txt", 100));
        b.insert(make_entry("file.txt", 200));

        a.merge(&b);
        assert_eq!(a.get("file.txt").unwrap().timestamp, 200);

        // Reverse: b merges a, same result.
        b.merge(&GSet {
            files: [("file.txt".into(), make_entry("file.txt", 100))]
                .into_iter()
                .collect(),
        });
        assert_eq!(b.get("file.txt").unwrap().timestamp, 200);
    }

    #[test]
    fn gset_merge_union() {
        let mut a = GSet::new();
        let mut b = GSet::new();
        a.insert(make_entry("x", 1));
        b.insert(make_entry("y", 2));
        a.merge(&b);
        assert_eq!(a.len(), 2);
        a.merge(&b); // idempotent
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn gset_commutativity() {
        let mut a = GSet::new();
        let mut b = GSet::new();
        a.insert(make_entry("x", 1));
        b.insert(make_entry("y", 2));
        let mut ab = a.clone();
        ab.merge(&b);
        let mut ba = b.clone();
        ba.merge(&a);
        assert_eq!(ab.list(), ba.list());
    }

    #[test]
    fn merkle_root_deterministic() {
        let mut a = GSet::new();
        let mut b = GSet::new();
        let e1 = make_entry("x", 1);
        let e2 = make_entry("y", 2);
        // Insert in different order.
        a.insert(e1.clone());
        a.insert(e2.clone());
        b.insert(e2);
        b.insert(e1);
        assert_eq!(a.merkle_root(), b.merkle_root());
    }

    #[test]
    fn merkle_root_changes_on_insert() {
        let mut g = GSet::new();
        let r1 = g.merkle_root();
        g.insert(make_entry("x", 1));
        let r2 = g.merkle_root();
        assert_ne!(r1, r2);
    }

    #[test]
    fn verify_chunk_works() {
        let data = b"test chunk verification";
        let shards = erasure::encode(data, 2, 4);
        let bytes = shard_to_bytes(&shards[0]);
        let hash = cyber_hemera::hash(&bytes).to_hex();
        assert!(verify_chunk(&bytes, &hash));
        let mut bad = bytes.clone();
        bad[0] ^= 0xFF;
        assert!(!verify_chunk(&bad, &hash));
    }
}
