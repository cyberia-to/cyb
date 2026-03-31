//! Reed-Solomon erasure coding over the Goldilocks field.
//!
//! Encode: treat k data elements as polynomial coefficients, evaluate at n points via NTT.
//! Decode: interpolate from any k of n evaluations.
//!
//! MDS (maximum distance separable): any k of n shards reconstruct the original.

use nebu::{Goldilocks, ntt};
use nebu::field::P;

/// A Reed-Solomon coded shard: index + data.
#[derive(Clone, Debug)]
pub struct Shard {
    /// Shard index in [0, n).
    pub index: usize,
    /// Field elements of this shard.
    pub data: Vec<Goldilocks>,
}

/// Encode data into n shards such that any k suffice to reconstruct.
///
/// Data is split into groups of k field elements. Each group is treated as
/// polynomial coefficients, evaluated at n roots of unity via NTT.
/// Shard i gets the i-th evaluation from each group.
///
/// n must be a power of 2, 1 <= k <= n.
pub fn encode(data: &[u8], k: usize, n: usize) -> Vec<Shard> {
    assert!(n.is_power_of_two(), "n must be power of 2");
    assert!(k >= 1 && k <= n, "need 1 <= k <= n");

    let elements = bytes_to_elements(data);

    // Pad to multiple of k.
    let num_groups = (elements.len() + k - 1) / k;
    let mut padded = elements;
    padded.resize(num_groups * k, Goldilocks::ZERO);

    // Initialize shards.
    let mut shards: Vec<Shard> = (0..n)
        .map(|i| Shard {
            index: i,
            data: Vec::with_capacity(num_groups),
        })
        .collect();

    // For each group of k elements: NTT to get n evaluations.
    for g in 0..num_groups {
        let mut poly = vec![Goldilocks::ZERO; n];
        for i in 0..k {
            poly[i] = padded[g * k + i];
        }
        ntt::ntt(&mut poly);
        for s in 0..n {
            shards[s].data.push(poly[s]);
        }
    }

    shards
}

/// Decode original data from any k shards out of n.
pub fn decode(shards: &[Shard], k: usize, n: usize, original_len: usize) -> Vec<u8> {
    assert!(n.is_power_of_two());
    assert!(shards.len() >= k, "need at least k shards to reconstruct");

    let num_groups = shards[0].data.len();

    // Fast path: all n shards present.
    if shards.len() == n && is_complete(shards, n) {
        return decode_full(shards, k, n, num_groups, original_len);
    }

    // General path: Lagrange interpolation from k evaluations.
    let omega = Goldilocks::new(7).exp((P - 1) / n as u64);

    let available: Vec<&Shard> = shards.iter().take(k).collect();
    let eval_points: Vec<Goldilocks> = available
        .iter()
        .map(|s| omega.exp(s.index as u64))
        .collect();

    let mut result_elements = Vec::with_capacity(num_groups * k);

    for g in 0..num_groups {
        let values: Vec<Goldilocks> = available.iter().map(|s| s.data[g]).collect();
        let coeffs = lagrange_interpolate(&eval_points, &values);
        for i in 0..k {
            result_elements.push(if i < coeffs.len() {
                coeffs[i]
            } else {
                Goldilocks::ZERO
            });
        }
    }

    elements_to_bytes(&result_elements, original_len)
}

/// Fast decode when all n shards are present — just inverse NTT.
fn decode_full(
    shards: &[Shard],
    k: usize,
    n: usize,
    num_groups: usize,
    original_len: usize,
) -> Vec<u8> {
    let mut result_elements = Vec::with_capacity(num_groups * k);

    for g in 0..num_groups {
        let mut poly = vec![Goldilocks::ZERO; n];
        for shard in shards {
            poly[shard.index] = shard.data[g];
        }
        ntt::intt(&mut poly);
        for i in 0..k {
            result_elements.push(poly[i]);
        }
    }

    elements_to_bytes(&result_elements, original_len)
}

/// Check if shards form a complete set [0..n).
fn is_complete(shards: &[Shard], n: usize) -> bool {
    if shards.len() != n {
        return false;
    }
    let mut seen = vec![false; n];
    for s in shards {
        if s.index >= n || seen[s.index] {
            return false;
        }
        seen[s.index] = true;
    }
    true
}

/// Lagrange interpolation: given evaluations (x_i, y_i), recover polynomial coefficients.
fn lagrange_interpolate(points: &[Goldilocks], values: &[Goldilocks]) -> Vec<Goldilocks> {
    let k = points.len();
    assert_eq!(k, values.len());

    let mut result = vec![Goldilocks::ZERO; k];

    for i in 0..k {
        // Denominator: Π_{j≠i} (x_i - x_j)
        let mut denom = Goldilocks::ONE;
        for j in 0..k {
            if j != i {
                denom = denom * (points[i] - points[j]);
            }
        }
        let scale = values[i] * denom.inv();

        // Numerator polynomial: Π_{j≠i} (x - x_j)
        let mut basis = vec![Goldilocks::ZERO; k];
        basis[0] = Goldilocks::ONE;
        let mut deg = 0;

        for j in 0..k {
            if j != i {
                let neg_xj = -points[j];
                for d in (1..=deg + 1).rev() {
                    basis[d] = basis[d - 1] + basis[d] * neg_xj;
                }
                basis[0] = basis[0] * neg_xj;
                deg += 1;
            }
        }

        for d in 0..k {
            result[d] = result[d] + scale * basis[d];
        }
    }

    result
}

/// Convert bytes to field elements (7 bytes per element, value < 2^56 < p).
fn bytes_to_elements(data: &[u8]) -> Vec<Goldilocks> {
    let mut out = Vec::with_capacity((data.len() + 6) / 7);
    for chunk in data.chunks(7) {
        let mut val: u64 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            val |= (b as u64) << (i * 8);
        }
        out.push(Goldilocks::new(val));
    }
    out
}

/// Convert field elements back to bytes, truncating to original_len.
fn elements_to_bytes(elements: &[Goldilocks], original_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(elements.len() * 7);
    for &e in elements {
        let val = e.as_u64();
        for i in 0..7 {
            out.push((val >> (i * 8)) as u8);
        }
    }
    out.truncate(original_len);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_shards() {
        let data = b"hello world, this is a test of erasure coding over Goldilocks!";
        let k = 2;
        let n = 4;
        let shards = encode(data, k, n);
        assert_eq!(shards.len(), n);
        let recovered = decode(&shards, k, n, data.len());
        assert_eq!(&recovered, data);
    }

    #[test]
    fn roundtrip_missing_shards() {
        let data = b"erasure coding works: lose any n-k shards and still reconstruct";
        let k = 2;
        let n = 4;
        let shards = encode(data, k, n);

        let partial: Vec<Shard> = shards
            .into_iter()
            .filter(|s| s.index == 0 || s.index == 2)
            .collect();
        assert_eq!(partial.len(), k);

        let recovered = decode(&partial, k, n, data.len());
        assert_eq!(&recovered, &data[..]);
    }

    #[test]
    fn roundtrip_k_equals_n() {
        let data = b"no parity, full data on every shard";
        let k = 4;
        let n = 4;
        let shards = encode(data, k, n);
        let recovered = decode(&shards, k, n, data.len());
        assert_eq!(&recovered, data);
    }

    #[test]
    fn roundtrip_large_data() {
        let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let k = 2;
        let n = 4;
        let shards = encode(&data, k, n);

        let partial: Vec<Shard> = shards
            .into_iter()
            .filter(|s| s.index == 1 || s.index == 3)
            .collect();

        let recovered = decode(&partial, k, n, data.len());
        assert_eq!(recovered, data);
    }

    #[test]
    fn different_shard_combinations() {
        let data = b"testing all possible k-subsets for reconstruction";
        let k = 2;
        let n = 4;
        let shards = encode(data, k, n);

        // All C(4,2) = 6 combinations must work.
        for i in 0..n {
            for j in (i + 1)..n {
                let partial: Vec<Shard> = shards
                    .iter()
                    .filter(|s| s.index == i || s.index == j)
                    .cloned()
                    .collect();
                let recovered = decode(&partial, k, n, data.len());
                assert_eq!(
                    &recovered,
                    &data[..],
                    "failed with shards {i} and {j}"
                );
            }
        }
    }

    #[test]
    fn bytes_roundtrip() {
        let data = b"field element encoding roundtrip";
        let elems = bytes_to_elements(data);
        let back = elements_to_bytes(&elems, data.len());
        assert_eq!(&back, &data[..]);
    }
}
