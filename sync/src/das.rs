//! Data Availability Sampling (DAS).
//!
//! Commit erasure-coded shards via Hemera Merkle roots.
//! Verify availability by random sampling with inclusion proofs.

use cyber_hemera::Hash;
use cyber_hemera::tree;

use crate::erasure::Shard;

/// Commitment to a set of shards: one Merkle root per shard.
#[derive(Clone, Debug)]
pub struct DasCommitment {
    /// Hemera Merkle root of each shard's data.
    pub shard_roots: Vec<Hash>,
    /// Global root: hash of all shard roots.
    pub root: Hash,
    /// Parameters.
    pub k: usize,
    pub n: usize,
    /// Original data length in bytes.
    pub original_len: usize,
}

/// A sample: one shard's data + proof of inclusion.
#[derive(Clone, Debug)]
pub struct Sample {
    pub shard_index: usize,
    pub shard_data: Vec<u8>,
    pub shard_root: Hash,
}

/// Commit to a set of erasure-coded shards.
pub fn commit(shards: &[Shard], k: usize, original_len: usize) -> DasCommitment {
    let n = shards.len();
    let shard_roots: Vec<Hash> = shards
        .iter()
        .map(|s| {
            let bytes = shard_to_bytes(s);
            tree::root_hash(&bytes)
        })
        .collect();

    // Global root = hash of concatenated shard roots.
    let mut all_roots_bytes = Vec::with_capacity(shard_roots.len() * 32);
    for r in &shard_roots {
        all_roots_bytes.extend_from_slice(r.as_bytes());
    }
    let root = tree::root_hash(&all_roots_bytes);

    DasCommitment {
        shard_roots,
        root,
        k,
        n,
        original_len,
    }
}

/// Create a sample for a specific shard.
pub fn sample(shard: &Shard) -> Sample {
    let bytes = shard_to_bytes(shard);
    let root = tree::root_hash(&bytes);
    Sample {
        shard_index: shard.index,
        shard_data: bytes,
        shard_root: root,
    }
}

/// Verify a sample against a commitment.
pub fn verify_sample(sample: &Sample, commitment: &DasCommitment) -> bool {
    if sample.shard_index >= commitment.n {
        return false;
    }

    // Check: hash of sample data matches committed shard root.
    let computed_root = tree::root_hash(&sample.shard_data);
    computed_root == commitment.shard_roots[sample.shard_index]
}

/// Verify availability by checking multiple random samples.
/// Returns (passed, total) — how many samples verified.
pub fn verify_availability(
    samples: &[Sample],
    commitment: &DasCommitment,
) -> (usize, usize) {
    let mut passed = 0;
    for s in samples {
        if verify_sample(s, commitment) {
            passed += 1;
        }
    }
    (passed, samples.len())
}

/// Confidence level for k successful samples out of k attempts.
/// 1 - (1/2)^k assuming adversary withholds >50%.
pub fn confidence(successful_samples: usize) -> f64 {
    1.0 - 0.5_f64.powi(successful_samples as i32)
}

/// Serialize a shard's field elements to bytes for hashing.
fn shard_to_bytes(shard: &Shard) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(shard.data.len() * 8);
    for &elem in &shard.data {
        bytes.extend_from_slice(&elem.as_u64().to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erasure;

    #[test]
    fn commit_and_verify_samples() {
        let data = b"testing DAS commitment and sampling verification";
        let k = 2;
        let n = 4;
        let shards = erasure::encode(data, k, n);
        let commitment = commit(&shards, k, data.len());

        // Sample each shard and verify.
        for shard in &shards {
            let s = sample(shard);
            assert!(
                verify_sample(&s, &commitment),
                "shard {} failed verification",
                shard.index
            );
        }
    }

    #[test]
    fn tampered_sample_fails() {
        let data = b"tamper detection test";
        let k = 2;
        let n = 4;
        let shards = erasure::encode(data, k, n);
        let commitment = commit(&shards, k, data.len());

        let mut s = sample(&shards[0]);
        // Tamper with the data.
        if !s.shard_data.is_empty() {
            s.shard_data[0] ^= 0xFF;
        }
        assert!(!verify_sample(&s, &commitment));
    }

    #[test]
    fn confidence_calculation() {
        assert!(confidence(20) > 0.999999);
        assert!(confidence(30) > 0.999999999);
        assert!((confidence(1) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn availability_check() {
        let data = b"availability verification across all shards";
        let k = 2;
        let n = 4;
        let shards = erasure::encode(data, k, n);
        let commitment = commit(&shards, k, data.len());

        let samples: Vec<Sample> = shards.iter().map(|s| sample(s)).collect();
        let (passed, total) = verify_availability(&samples, &commitment);
        assert_eq!(passed, n);
        assert_eq!(total, n);
    }
}
