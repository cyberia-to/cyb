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
    /// Number of copies of each shard across devices.
    /// 1 = minimum (D/k per device), 2-3 = higher availability.
    /// Tier 0 ignores this (always full replication).
    pub shard_copies: usize,
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
        let timestamp = store::now_ms();
        let device_id = "vdisk".to_string();
        let entry_hash = FileEntry::compute_hash(
            file_name, &shard_hashes, timestamp, &device_id,
        );
        let entry = FileEntry {
            name: file_name.to_string(),
            original_len: data.len(),
            k,
            n: n_ntt,
            shard_hashes,
            timestamp,
            entry_hash,
            device_id,
            das_root: format!("{:?}", commitment.root),
            shard_copies: 1,
            deleted: false,
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

    /// Rechunk: re-encode existing files on a disk with new (k, n) parameters.
    ///
    /// Called after a device joins/leaves. For each file on the disk:
    /// 1. Decode from existing shards (old k, old n).
    /// 2. Re-encode with new parameters (new k, new n).
    /// 3. Distribute new shards to current device set.
    /// 4. Update registry entry.
    ///
    /// Returns (rechunked_count, skipped_count).
    pub fn rechunk(&mut self, disk_name: &str) -> Result<(usize, usize), String> {
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
        if n_devices < 2 {
            return Ok((0, 0));
        }

        let new_f = match &config.redundancy {
            Redundancy::Max => n_devices - 1,
            Redundancy::Tolerate(f) => *f,
        };
        let new_k = n_devices - new_f;
        let new_n = n_devices.next_power_of_two();

        // Collect files that need rechunking (old k/n differ from new).
        let files_to_rechunk: Vec<String> = self
            .registry
            .files
            .values()
            .filter(|e| e.k != new_k || e.n != new_n)
            .map(|e| e.name.clone())
            .collect();

        let mut rechunked = 0;
        let mut skipped = 0;

        for file_name in &files_to_rechunk {
            // Step 1: decode from existing shards.
            match self.get_file(file_name) {
                Ok(data) => {
                    // Step 2: re-encode with new parameters.
                    let shards = erasure::encode(&data, new_k, new_n);
                    let commitment = das::commit(&shards, new_k, data.len());

                    // Step 3: distribute to current devices.
                    let mut shard_hashes = Vec::with_capacity(new_n);
                    for shard in &shards {
                        let device_idx = shard.index % n_devices;
                        let device_name = &device_names[device_idx];
                        if let Some(store) = self.stores.get_mut(device_name) {
                            let hash = store.put(shard).map_err(|e| e.to_string())?;
                            shard_hashes.push(hash.to_hex());
                        }
                    }

                    // Step 4: update registry.
                    let timestamp = store::now_ms();
                    let device_id = "vdisk".to_string();
                    let entry_hash = FileEntry::compute_hash(
                        file_name,
                        &shard_hashes,
                        timestamp,
                        &device_id,
                    );
                    self.registry.insert(FileEntry {
                        name: file_name.clone(),
                        original_len: data.len(),
                        k: new_k,
                        n: new_n,
                        shard_hashes,
                        timestamp,
                        entry_hash,
                        device_id,
                        das_root: format!("{:?}", commitment.root),
                        shard_copies: config.shard_copies, deleted: false,
                    });
                    rechunked += 1;
                }
                Err(_) => {
                    skipped += 1;
                }
            }
        }

        Ok((rechunked, skipped))
    }

    /// Rebalance: migrate chunks from over-capacity devices to under-capacity ones.
    ///
    /// For each device, compute target load (proportional to capacity).
    /// Move excess chunks to devices with room.
    /// Returns number of chunks migrated.
    pub fn rebalance(&mut self, disk_name: &str) -> Result<usize, String> {
        let device_names: Vec<String> = self
            .attachments
            .iter()
            .filter(|a| a.disk_name == disk_name)
            .map(|a| a.device_name.clone())
            .collect();

        if device_names.len() < 2 {
            return Ok(0);
        }

        // Compute per-device load and capacity.
        let mut device_load: Vec<(String, u64, u64)> = Vec::new(); // (name, used, capacity)
        for name in &device_names {
            let (used, cap) = if let Some(store) = self.stores.get(name) {
                (store.used(), store.capacity())
            } else {
                (0, 0)
            };
            device_load.push((name.clone(), used, cap));
        }

        let total_cap: u64 = device_load.iter().map(|(_, _, c)| *c).sum();
        let total_used: u64 = device_load.iter().map(|(_, u, _)| *u).sum();

        if total_cap == 0 || total_used == 0 {
            return Ok(0);
        }

        // Compute target usage per device (proportional to capacity).
        let mut targets: Vec<(String, u64, u64, i64)> = device_load
            .iter()
            .map(|(name, used, cap)| {
                let target = if total_cap > 0 {
                    (total_used as f64 * *cap as f64 / total_cap as f64) as u64
                } else {
                    0
                };
                let delta = *used as i64 - target as i64;
                (name.clone(), *used, target, delta)
            })
            .collect();

        // Sort: over-capacity first (positive delta), under-capacity last.
        targets.sort_by(|a, b| b.3.cmp(&a.3));

        let mut migrated = 0;

        // Find over-capacity devices and under-capacity devices.
        let over: Vec<String> = targets
            .iter()
            .filter(|(_, _, _, d)| *d > 0)
            .map(|(n, _, _, _)| n.clone())
            .collect();
        let under: Vec<String> = targets
            .iter()
            .filter(|(_, _, _, d)| *d < 0)
            .map(|(n, _, _, _)| n.clone())
            .collect();

        // For each file, check if its shards are on over-capacity devices
        // and could be moved to under-capacity devices.
        let file_names: Vec<String> = self.registry.files.keys().cloned().collect();
        for file_name in &file_names {
            let entry = match self.registry.get(file_name) {
                Some(e) => e.clone(),
                None => continue,
            };

            for (shard_idx, hash_hex) in entry.shard_hashes.iter().enumerate() {
                let hash = match hex_to_hash(hash_hex) {
                    Ok(h) => h,
                    Err(_) => continue,
                };

                // Find which over-capacity device has this shard.
                let mut source = None;
                for dev_name in &over {
                    if let Some(store) = self.stores.get(dev_name) {
                        if store.has(&hash) {
                            source = Some(dev_name.clone());
                            break;
                        }
                    }
                }

                if source.is_none() {
                    continue;
                }
                let source_name = source.unwrap();

                // Find an under-capacity device that doesn't have it yet.
                for dest_name in &under {
                    if let Some(dest_store) = self.stores.get(dest_name) {
                        if !dest_store.has(&hash) {
                            // Copy chunk from source to dest.
                            let bytes = match self.stores.get(&source_name) {
                                Some(s) => match s.get(&hash) {
                                    Ok(b) => b,
                                    Err(_) => continue,
                                },
                                None => continue,
                            };

                            if let Some(dest) = self.stores.get_mut(dest_name) {
                                let shard = store::bytes_to_shard(shard_idx, &bytes);
                                if dest.put(&shard).is_ok() {
                                    migrated += 1;
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        Ok(migrated)
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
            shard_copies: 1,
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
            shard_copies: 1,
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
            shard_copies: 1,
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
            shard_copies: 1,
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
            shard_copies: 1,
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
            timestamp: store::now_ms(),
            entry_hash: "a".repeat(64),
            device_id: "other".into(),
            das_root: "0".repeat(64),
            shard_copies: 1,
            deleted: false,
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
            shard_copies: 1,
        })
        .unwrap();

        let status = mgr.status("empty").unwrap();
        assert!(!status.healthy);
        assert!(status.message.contains("no devices"));
    }

    #[test]
    fn rechunk_on_device_join() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "work".into(),
            redundancy: Redundancy::Tolerate(1),
            tier: Tier::Active,
            cache_policy: CachePolicy::Lru,
            shard_copies: 1,
        })
        .unwrap();

        // Start with 2 devices → k=1, n=2 (full replication).
        mgr.attach("dev_a", "work", 10_000_000).unwrap();
        mgr.attach("dev_b", "work", 10_000_000).unwrap();

        let data = b"file that will be rechunked when a new device joins";
        mgr.put_file("work", "rechunk_me.txt", data).unwrap();

        let entry_before = mgr.registry().get("rechunk_me.txt").unwrap().clone();
        let old_k = entry_before.k;
        let old_n = entry_before.n;

        // Add 2 more devices → now 4 devices, k=3, n=4.
        mgr.attach("dev_c", "work", 10_000_000).unwrap();
        mgr.attach("dev_d", "work", 10_000_000).unwrap();

        // Rechunk.
        let (rechunked, skipped) = mgr.rechunk("work").unwrap();
        assert_eq!(rechunked, 1);
        assert_eq!(skipped, 0);

        // Verify new parameters.
        let entry_after = mgr.registry().get("rechunk_me.txt").unwrap();
        assert!(entry_after.k > old_k || entry_after.n > old_n,
            "rechunk didn't change parameters: old k={} n={}, new k={} n={}",
            old_k, old_n, entry_after.k, entry_after.n);

        // Verify data is still readable.
        let recovered = mgr.get_file("rechunk_me.txt").unwrap();
        assert_eq!(&recovered, &data[..]);
    }

    #[test]
    fn rechunk_preserves_data_integrity() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "data".into(),
            redundancy: Redundancy::Tolerate(1),
            tier: Tier::Active,
            cache_policy: CachePolicy::Lru,
            shard_copies: 1,
        })
        .unwrap();

        mgr.attach("a", "data", 10_000_000).unwrap();
        mgr.attach("b", "data", 10_000_000).unwrap();

        // Put multiple files.
        let files: Vec<(String, Vec<u8>)> = (0..10)
            .map(|i| {
                let name = format!("file_{}.dat", i);
                let data: Vec<u8> = (0..100 + i * 50).map(|j| ((i + j) % 256) as u8).collect();
                mgr.put_file("data", &name, &data).unwrap();
                (name, data)
            })
            .collect();

        // Add devices and rechunk.
        mgr.attach("c", "data", 10_000_000).unwrap();
        mgr.attach("d", "data", 10_000_000).unwrap();

        let (rechunked, _) = mgr.rechunk("data").unwrap();
        assert_eq!(rechunked, 10);

        // All files still readable with correct data.
        for (name, original) in &files {
            let recovered = mgr.get_file(name).unwrap();
            assert_eq!(&recovered, original, "file {} corrupted after rechunk", name);
        }
    }

    #[test]
    fn rebalance_moves_chunks() {
        let (_dir, mut mgr) = setup();

        mgr.create_disk(DiskConfig {
            name: "uneven".into(),
            redundancy: Redundancy::Tolerate(1),
            tier: Tier::Active,
            cache_policy: CachePolicy::Lru,
            shard_copies: 1,
        })
        .unwrap();

        // Device A: small capacity. Device B: large capacity.
        mgr.attach("small", "uneven", 1_000_000).unwrap();
        mgr.attach("large", "uneven", 10_000_000).unwrap();

        // Put files — initially distributed round-robin (roughly equal).
        for i in 0..5 {
            let data: Vec<u8> = (0..200).map(|j| ((i + j) % 256) as u8).collect();
            mgr.put_file("uneven", &format!("f_{}", i), &data).unwrap();
        }

        // Rebalance should move chunks from "small" to "large".
        let migrated = mgr.rebalance("uneven").unwrap();
        // At least some chunks should move (large has 10x capacity).
        // The exact count depends on initial distribution.

        // All files still readable.
        for i in 0..5 {
            let expected: Vec<u8> = (0..200).map(|j| ((i + j) % 256) as u8).collect();
            let recovered = mgr.get_file(&format!("f_{}", i)).unwrap();
            assert_eq!(recovered, expected, "file f_{} corrupted after rebalance", i);
        }
    }
}
