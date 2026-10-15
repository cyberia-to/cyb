//! Verified-streaming blob decode. `feature = "blobs"`.
//!
//! Critical dependency #3 in the launch tracker named radio as
//! "a dependency of nothing in the phase-1 binaries." This is cyb-core's
//! first dependency on the radio family, scoped to `cyber-bao`'s
//! synchronous BAO decode — no networking, no `iroh` transport (that
//! stays out, same as the `foculus` note above it in `Cargo.toml`).
//!
//! Row 22 needs a file cyb never had, fetched from a peer, checked
//! before it touches anything downstream. This is the "checked" half:
//! given BAO-combined bytes and the particle they claim to decode to,
//! verify every chunk against that particle before returning anything.
//! The fetch-from-a-peer half is radio's, not this crate's.

use cyber_bao::hash::Poseidon2Backend;
use cyber_bao::io::decode;
use cyber_bao::tree::BlockSize;
use cyber_hemera::Hash;

/// Decode BAO-combined bytes, verifying every chunk against `particle`.
/// `None` on truncation, a malformed tree, or any hash mismatch — the
/// caller never sees unverified bytes.
pub fn decode_verified(encoded: &[u8], particle: &Hash) -> Option<Vec<u8>> {
    decode::decode(&Poseidon2Backend, encoded, particle, BlockSize::ZERO).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyber_bao::io::encode;

    #[test]
    fn round_trips_through_encode_then_decode() {
        let data = b"the graph comes home as files and cyberlinks".to_vec();
        let (root, encoded) = encode::encode(&Poseidon2Backend, &data, BlockSize::ZERO);
        assert_eq!(decode_verified(&encoded, &root), Some(data));
    }

    #[test]
    fn rejects_bytes_tampered_after_encoding() {
        let data = b"bostrom and pussy reborn".to_vec();
        let (root, mut encoded) = encode::encode(&Poseidon2Backend, &data, BlockSize::ZERO);
        let last = encoded.len() - 1;
        encoded[last] ^= 0xff;
        assert_eq!(decode_verified(&encoded, &root), None);
    }

    #[test]
    fn rejects_a_particle_that_does_not_match() {
        let data = b"one particle, three definitions".to_vec();
        let (_root, encoded) = encode::encode(&Poseidon2Backend, &data, BlockSize::ZERO);
        let wrong = cyber_hemera::hash(b"a different particle entirely");
        assert_eq!(decode_verified(&encoded, &wrong), None);
    }

    #[test]
    fn rejects_truncated_input() {
        assert_eq!(decode_verified(&[0u8; 3], &cyber_hemera::hash(b"x")), None);
    }
}
