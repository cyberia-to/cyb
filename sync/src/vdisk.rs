//! Virtual disk manager.
//!
//! Two user-facing primitives:
//! - create-disk: define a logical volume with redundancy parameters
//! - attach: allocate physical space from a device into a disk
//!
//! The system handles chunk placement, transparent fetch, and rechunking.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::erasure;
use crate::das;
use crate::store::{self, ChunkStore, FileEntry, GSet};

/// Cache eviction policy.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CachePolicy {
    /// Keep frequently accessed data local.
    Lru,
    /// Fetch on demand, don't cache.
    Cold,
    /// Replicate everything locally.
    Hot,
}

/// Tier level (matches whitepaper tiers).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Tier {
    /// Keys, seeds, irreplaceable. Full replication.
    Critical,
    /// Active working files. Erasure coded.
    Active,
    /// Archive, media. Erasure coded, cold fetch.
    Archive,
}

/// A virtual disk definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiskConfig {
    pub name: String,
    /// Max device failures to tolerate. "max" = n-1 (full replication).
    pub redundancy: Redundancy,
    pub tier: Tier,
    pub cache_policy: CachePolicy,
}

/// Redundancy specification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Redundancy {
    /// Tolerate f device failures.
    Tolerate(usize),
    /// Full replication (f = n-1).
    Max,
}

/// A device attached to one or more disks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceAttachment {
    pub device_name: String,
    pub disk_name: String,
    /// Allocated capacity in bytes.
    pub capacity: u64,
}

/// Computed disk status.
#[derive(Clone, Debug)]
pub struct DiskStatus {
    pub name: String,
    pub devices: usize,
    pub total_capacity: u64,
    pub k: usize,
    pub n: usize,
    pub f: usize,
    pub per_file_overhead: f64,
    pub healthy: bool,
    pub message: String,
}

/// The virtual disk manager.
pub struct VDiskManager {
    /// Disk configurations.
    disks: HashMap<String, DiskConfig>,
    /// Device attachments.
    attachments: Vec<DeviceAttachment>,
    /// Per-device chunk stores.
    stores: HashMap<String, ChunkStore>,
    /// Global file registry (G-Set CRDT).
    registry: GSet,
    /// Base directory for device storage.
    base_dir: PathBuf,
}

impl VDiskManager {
    /// Create a new VDisk manager with a base storage directory.
    pub fn new(base_dir: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(base_dir)?;
        Ok(Self {
            disks: HashMap::new(),
            attachments: Vec::new(),
            stores: HashMap::new(),
            registry: GSet::new(),
            base_dir: base_dir.to_path_buf(),
        })
    }

    /// Create a virtual disk.
    pub fn create_disk(&mut self, config: DiskConfig) -> Result<(), String> {
        if self.disks.contains_key(&config.name) {
            return Err(format!("disk '{}' already exists", config.name));
        }
        self.disks.insert(config.name.clone(), config);
        Ok(())
    }

    /// Attach a device to a disk with given capacity.
    pub fn attach(
        &mut self,
        device_name: &str,
        disk_name: &str,
        capacity: u64,
    ) -> Result<(), String> {
        if !self.disks.contains_key(disk_name) {
            return Err(format!("disk '{}' does not exist", disk_name));
        }

        let attachment = DeviceAttachment {
            device_name: device_name.to_string(),
            disk_name: disk_name.to_string(),
            capacity,
        };
        self.attachments.push(attachment);

        // Create chunk store for device if not exists.
        if !self.stores.contains_key(device_name) {
            let device_dir = self.base_dir.join(device_name);
            let store = ChunkStore::new(&device_dir, capacity)
                .map_err(|e| e.to_string())?;
            self.stores.insert(device_name.to_string(), store);
        }

        Ok(())
    }

    /// Get computed status for a disk.
    pub fn status(&self, disk_name: &str) -> Result<DiskStatus, String> {
        let config = self
            .disks
            .get(disk_name)
            .ok_or_else(|| format!("disk '{}' not found", disk_name))?;

        let devices: Vec<&DeviceAttachment> = self
            .attachments
            .iter()
            .filter(|a| a.disk_name == disk_name)
            .collect();

        let n = devices.len();
        let total_capacity: u64 = devices.iter().map(|a| a.capacity).sum();

        let f = match &config.redundancy {
            Redundancy::Max => if n > 0 { n - 1 } else { 0 },
            Redundancy::Tolerate(f) => *f,
        };

        let k = if n > f { n - f } else { 1 };
        let per_file_overhead = if k > 0 { n as f64 / k as f64 } else { 0.0 };

        let healthy = n > f && f > 0;
        let message = if n == 0 {
            "no devices attached".to_string()
        } else if n <= f {
            format!("need {} more devices for f={}", f - n + 1, f)
        } else if f == 0 {
            "no redundancy (f=0)".to_string()
        } else {
            format!("healthy (f={})", f)
        };

        Ok(DiskStatus {
            name: disk_name.to_string(),
            devices: n,
            total_capacity,
            k,
            n,
            f,
            per_file_overhead,
            healthy,
            message,
        })
    }

    /// Store a file on a virtual disk: erasure code and distribute chunks.
    pub fn put_file(
        &mut self,
        disk_name: &str,
        file_name: &str,
        data: &[u8],
    ) -> Result<das::DasCommitment, String> {
        let config = self
            .disks
            .get(disk_name)
            .ok_or_else(|| format!("disk '{}' not found", disk_name))?
            .clone();

        let device_names: Vec<String> = self
            .attachments
            .iter()
            .filter(|a| a.disk_name == disk_name)
            .map(|a| a.device_name.clone())
            .collect();

        let n_devices = device_names.len();
        if n_devices == 0 {
            return Err("no devices attached to disk".to_string());
        }

        let f = match &config.redundancy {
            Redundancy::Max => n_devices - 1,
            Redundancy::Tolerate(f) => *f,
        };
        let k = n_devices - f;

        // Round n up to next power of 2 for NTT.
        let n_ntt = n_devices.next_power_of_two();

        // Erasure encode.
        let shards = erasure::encode(data, k, n_ntt);

        // Commit via DAS.
        let commitment = das::commit(&shards, k, data.len());

        // Distribute shards to devices (round-robin, capacity-weighted later).
        let mut shard_hashes = Vec::with_capacity(n_ntt);
        for shard in &shards {
            let device_idx = shard.index % n_devices;
            let device_name = &device_names[device_idx];

            if let Some(store) = self.stores.get_mut(device_name) {
                let hash = store.put(shard).map_err(|e| e.to_string())?;
                shard_hashes.push(hash.to_hex());
            }
        }

        // Register file in G-Set.
        let entry = FileEntry {
            name: file_name.to_string(),
            original_len: data.len(),
            k,
            n: n_ntt,
            shard_hashes,
        };
        self.registry.insert(entry);

        Ok(commitment)
    }

    /// Retrieve a file from a virtual disk, reconstructing from available shards.
    pub fn get_file(&self, file_name: &str) -> Result<Vec<u8>, String> {
        let entry = self
            .registry
            .get(file_name)
            .ok_or_else(|| format!("file '{}' not found", file_name))?;

        // Collect available shards from all device stores.
        let mut available_shards = Vec::new();
        for (shard_idx, hash_hex) in entry.shard_hashes.iter().enumerate() {
            let hash = hex_to_hash(hash_hex)?;
            for store in self.stores.values() {
                if let Ok(bytes) = store.get(&hash) {
                    let shard = store::bytes_to_shard(shard_idx, &bytes);
                    available_shards.push(shard);
                    break;
                }
            }
        }

        if available_shards.len() < entry.k {
            return Err(format!(
                "only {} of {} required shards available",
                available_shards.len(),
                entry.k
            ));
        }

        Ok(erasure::decode(
            &available_shards,
            entry.k,
            entry.n,
            entry.original_len,
        ))
    }

    /// List all files in the registry.
    pub fn list_files(&self) -> Vec<&str> {
        self.registry.list()
    }

    /// List all disks.
    pub fn list_disks(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.disks.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Get the file registry (for CRDT merge with other devices).
    pub fn registry(&self) -> &GSet {
        &self.registry
    }

    /// Merge another device's registry into ours.
    pub fn merge_registry(&mut self, other: &GSet) {
        self.registry.merge(other);
    }
}

/// Convert hex string to Hash (simplified).
fn hex_to_hash(hex: &str) -> Result<cyber_hemera::Hash, String> {
    // Hash internally stores [u8; 32]. Parse from hex.
    if hex.len() != 64 {
        return Err("invalid hash hex length".to_string());
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| e.to_string())?;
    }
    Ok(cyber_hemera::Hash::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, VDiskManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = VDiskManager::new(dir.path()).unwrap();
        (dir, mgr)
    }

    #[test]
    fn create_disk_and_attach() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "work".into(),
            redundancy: Redundancy::Tolerate(1),
            tier: Tier::Active,
            cache_policy: CachePolicy::Lru,
        })
        .unwrap();

        mgr.attach("laptop", "work", 1_000_000).unwrap();
        mgr.attach("phone", "work", 500_000).unwrap();

        let status = mgr.status("work").unwrap();
        assert_eq!(status.devices, 2);
        assert_eq!(status.k, 1);
        assert_eq!(status.n, 2);
        assert_eq!(status.f, 1);
    }

    #[test]
    fn end_to_end_store_and_retrieve() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "work".into(),
            redundancy: Redundancy::Tolerate(1),
            tier: Tier::Active,
            cache_policy: CachePolicy::Lru,
        })
        .unwrap();

        mgr.attach("device_a", "work", 10_000_000).unwrap();
        mgr.attach("device_b", "work", 10_000_000).unwrap();

        let data = b"hello virtual disk! this is an end-to-end test of erasure coded storage.";
        mgr.put_file("work", "hello.txt", data).unwrap();

        let recovered = mgr.get_file("hello.txt").unwrap();
        assert_eq!(&recovered, &data[..]);
    }

    #[test]
    fn survive_device_loss() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "resilient".into(),
            redundancy: Redundancy::Tolerate(1),
            tier: Tier::Active,
            cache_policy: CachePolicy::Lru,
        })
        .unwrap();

        mgr.attach("dev_a", "resilient", 10_000_000).unwrap();
        mgr.attach("dev_b", "resilient", 10_000_000).unwrap();
        mgr.attach("dev_c", "resilient", 10_000_000).unwrap();

        let data = b"this data must survive losing one device";
        mgr.put_file("resilient", "important.txt", data).unwrap();

        // Simulate losing dev_a by removing its store.
        mgr.stores.remove("dev_a");

        // Should still reconstruct from dev_b + dev_c.
        let recovered = mgr.get_file("important.txt").unwrap();
        assert_eq!(&recovered, &data[..]);
    }

    #[test]
    fn full_replication_tier0() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "keys".into(),
            redundancy: Redundancy::Max,
            tier: Tier::Critical,
            cache_policy: CachePolicy::Hot,
        })
        .unwrap();

        mgr.attach("phone", "keys", 1_000_000).unwrap();
        mgr.attach("laptop", "keys", 1_000_000).unwrap();
        mgr.attach("server", "keys", 1_000_000).unwrap();

        let status = mgr.status("keys").unwrap();
        assert_eq!(status.f, 2); // survive loss of 2 out of 3
        assert_eq!(status.k, 1); // every device has full copy
    }

    #[test]
    fn file_registry_merge() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "work".into(),
            redundancy: Redundancy::Tolerate(1),
            tier: Tier::Active,
            cache_policy: CachePolicy::Lru,
        })
        .unwrap();

        mgr.attach("dev_a", "work", 10_000_000).unwrap();
        mgr.attach("dev_b", "work", 10_000_000).unwrap();

        mgr.put_file("work", "a.txt", b"file from device A")
            .unwrap();

        // Simulate another device's registry.
        let mut other = GSet::new();
        other.insert(FileEntry {
            name: "b.txt".into(),
            original_len: 10,
            k: 1,
            n: 2,
            shard_hashes: vec![],
        });

        mgr.merge_registry(&other);
        assert_eq!(mgr.list_files().len(), 2);
    }

    #[test]
    fn disk_status_messages() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "empty".into(),
            redundancy: Redundancy::Tolerate(1),
            tier: Tier::Active,
            cache_policy: CachePolicy::Lru,
        })
        .unwrap();

        let status = mgr.status("empty").unwrap();
        assert!(!status.healthy);
        assert!(status.message.contains("no devices"));
    }
}
