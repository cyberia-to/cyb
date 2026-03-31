//! Content-addressed chunk store with G-Set CRDT merge.
//!
//! Chunks are stored by their Hemera hash. The chunk index tracks which
//! chunks are local. The G-Set (grow-only set) CRDT provides deterministic
//! merge for file metadata across devices.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cyber_hemera::Hash;
use serde::{Deserialize, Serialize};

use crate::erasure::Shard;

/// A file tracked by the system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEntry {
    /// Human-readable name / path.
    pub name: String,
    /// Original file size in bytes.
    pub original_len: usize,
    /// Erasure coding parameters.
    pub k: usize,
    pub n: usize,
    /// Hemera hash of each shard (indexed by shard index).
    pub shard_hashes: Vec<String>,
}

/// Content-addressed chunk store backed by a directory.
pub struct ChunkStore {
    /// Root directory for chunk storage.
    root: PathBuf,
    /// In-memory index: hash hex → shard index for each file.
    index: HashMap<String, Vec<u8>>,
    /// Capacity limit in bytes (0 = unlimited).
    capacity: u64,
    /// Current usage in bytes.
    used: u64,
}

impl ChunkStore {
    /// Create a new chunk store at the given directory.
    pub fn new(root: &Path, capacity: u64) -> std::io::Result<Self> {
        std::fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            index: HashMap::new(),
            capacity,
            used: 0,
        })
    }

    /// Store a shard's data, indexed by its Hemera hash.
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

    /// Retrieve a shard's data by its hash.
    pub fn get(&self, hash: &Hash) -> std::io::Result<Vec<u8>> {
        let hex = hash.to_hex();

        // Try in-memory first.
        if let Some(data) = self.index.get(&hex) {
            return Ok(data.clone());
        }

        // Try on disk.
        let path = self.root.join(&hex);
        std::fs::read(&path)
    }

    /// Check if a chunk exists locally.
    pub fn has(&self, hash: &Hash) -> bool {
        let hex = hash.to_hex();
        self.index.contains_key(&hex) || self.root.join(&hex).exists()
    }

    /// Current usage in bytes.
    pub fn used(&self) -> u64 {
        self.used
    }

    /// Capacity in bytes.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

/// G-Set CRDT: grow-only set of file entries.
///
/// Merge is union — commutative, associative, idempotent.
/// This is the simplest CRDT for an append-only file registry.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GSet {
    /// Files indexed by name.
    pub files: HashMap<String, FileEntry>,
}

impl GSet {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Add a file entry.
    pub fn insert(&mut self, entry: FileEntry) {
        self.files.insert(entry.name.clone(), entry);
    }

    /// Merge with another G-Set (union).
    pub fn merge(&mut self, other: &GSet) {
        for (name, entry) in &other.files {
            if !self.files.contains_key(name) {
                self.files.insert(name.clone(), entry.clone());
            }
        }
    }

    /// List all file names.
    pub fn list(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.files.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Get file entry by name.
    pub fn get(&self, name: &str) -> Option<&FileEntry> {
        self.files.get(name)
    }

    /// Number of files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erasure;

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
        let mut store = ChunkStore::new(dir.path(), 10).unwrap(); // 10 bytes limit

        let data = b"this data is definitely more than 10 bytes when encoded";
        let shards = erasure::encode(data, 2, 4);

        let result = store.put(&shards[0]);
        assert!(result.is_err());
    }

    #[test]
    fn gset_merge() {
        let mut a = GSet::new();
        let mut b = GSet::new();

        a.insert(FileEntry {
            name: "file1.txt".into(),
            original_len: 100,
            k: 2,
            n: 4,
            shard_hashes: vec![],
        });

        b.insert(FileEntry {
            name: "file2.txt".into(),
            original_len: 200,
            k: 2,
            n: 4,
            shard_hashes: vec![],
        });

        a.merge(&b);
        assert_eq!(a.len(), 2);
        assert!(a.get("file1.txt").is_some());
        assert!(a.get("file2.txt").is_some());

        // Idempotent: merge again changes nothing.
        a.merge(&b);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn gset_commutativity() {
        let mut a = GSet::new();
        let mut b = GSet::new();

        a.insert(FileEntry {
            name: "x".into(),
            original_len: 1,
            k: 1,
            n: 2,
            shard_hashes: vec![],
        });
        b.insert(FileEntry {
            name: "y".into(),
            original_len: 2,
            k: 1,
            n: 2,
            shard_hashes: vec![],
        });

        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        assert_eq!(ab.list(), ba.list());
    }
}
