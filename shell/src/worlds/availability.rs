//! Property 16: every particle the chain references must resolve in cyb —
//! from the local store, the burial blockstore, or a peer. This module is
//! the audit: given the particles a graph names, which of them cyb can
//! hand back as bytes today, and which are still missing.
//!
//! Today cyb has two sources: the local content store (`content::load()`,
//! `~/cyb/particles.jsonl`) and sigma's ASCII-padded names, which carry
//! their own label with no lookup at all. The burial blockstore and peer
//! fetch (properties 22, 23) are not wired in yet, so a hash that needs
//! either of those still counts as missing here — this module is the
//! measuring stick that turns green as those sources land, not a stand-in
//! for them.

use std::collections::HashMap;

use super::content;
use super::graph::decode_ascii_particle;

/// One particle's resolution: found and through what source, or missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The local content store holds the text.
    LocalStore,
    /// The hash is itself the label, padded ASCII — no lookup needed.
    AsciiName,
    /// Neither source has it.
    Missing,
}

/// A resolution pass over a set of referenced particles.
pub struct AvailabilityReport {
    pub total: usize,
    pub resolved: usize,
    pub by_source: HashMap<&'static str, usize>,
    /// The particles that did not resolve, in the order they were checked.
    pub missing: Vec<[u8; 32]>,
}

impl AvailabilityReport {
    /// The floor phase 1 measures against: the burial's own baseline,
    /// bytes present for 97.63% of bostrom's 3,143,650 particles. An empty
    /// reference set resolves fully by convention — nothing was asked for.
    pub fn resolved_fraction(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        self.resolved as f64 / self.total as f64
    }
}

fn resolve_one(hash: &[u8; 32], sidecar: &HashMap<[u8; 32], String>) -> Resolution {
    if sidecar.contains_key(hash) {
        Resolution::LocalStore
    } else if decode_ascii_particle(hash).is_some() {
        Resolution::AsciiName
    } else {
        Resolution::Missing
    }
}

/// Audit `hashes` — every particle a reference set names, e.g.
/// `BrainIndex::hashes` for the loaded graph — against `sidecar`, the
/// local content store's contents. Takes the store as a map so the audit
/// stays a pure function; callers touching the real store use
/// [`audit_local`].
pub fn audit(hashes: &[[u8; 32]], sidecar: &HashMap<[u8; 32], String>) -> AvailabilityReport {
    let mut by_source = HashMap::new();
    let mut missing = Vec::new();
    for hash in hashes {
        match resolve_one(hash, sidecar) {
            Resolution::LocalStore => *by_source.entry("local_store").or_insert(0) += 1,
            Resolution::AsciiName => *by_source.entry("ascii_name").or_insert(0) += 1,
            Resolution::Missing => missing.push(*hash),
        }
    }
    AvailabilityReport {
        total: hashes.len(),
        resolved: hashes.len() - missing.len(),
        by_source,
        missing,
    }
}

/// Convenience: audit against the local content store on disk.
pub fn audit_local(hashes: &[[u8; 32]]) -> AvailabilityReport {
    audit(hashes, &content::load())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ascii_particle(s: &str) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[..s.len()].copy_from_slice(s.as_bytes());
        h
    }

    #[test]
    fn ascii_named_particle_resolves_without_store() {
        let hash = ascii_particle("PUSSY");
        let report = audit(&[hash], &HashMap::new());
        assert_eq!(report.resolved, 1);
        assert_eq!(report.by_source.get("ascii_name"), Some(&1));
        assert!(report.missing.is_empty());
    }

    #[test]
    fn store_hit_resolves() {
        let hash = [7u8; 32];
        let mut sidecar = HashMap::new();
        sidecar.insert(hash, "some cast text".to_string());
        let report = audit(&[hash], &sidecar);
        assert_eq!(report.resolved, 1);
        assert_eq!(report.by_source.get("local_store"), Some(&1));
    }

    #[test]
    fn unresolved_particle_is_missing() {
        let hash = [9u8; 32];
        let report = audit(&[hash], &HashMap::new());
        assert_eq!(report.resolved, 0);
        assert_eq!(report.missing, vec![hash]);
        assert_eq!(report.resolved_fraction(), 0.0);
    }

    #[test]
    fn empty_set_resolves_fully_by_convention() {
        let report = audit(&[], &HashMap::new());
        assert_eq!(report.resolved_fraction(), 1.0);
    }

    #[test]
    fn mixed_set_reports_fraction_and_missing_list() {
        let ascii = ascii_particle("bob");
        let stored = [3u8; 32];
        let gone = [4u8; 32];
        let mut sidecar = HashMap::new();
        sidecar.insert(stored, "x".to_string());
        let report = audit(&[ascii, stored, gone], &sidecar);
        assert_eq!(report.total, 3);
        assert_eq!(report.resolved, 2);
        assert_eq!(report.missing, vec![gone]);
        assert!((report.resolved_fraction() - 2.0 / 3.0).abs() < 1e-9);
    }
}
