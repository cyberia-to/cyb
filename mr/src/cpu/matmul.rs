//! Matmul: y = x @ W^T
//!
//! CPU reference with two optimizations on top of the naive three-loop form:
//!   1. f32x8 SIMD dot product (NEON on ARM, AVX on x86) via the `wide` crate
//!   2. Rayon parallelism over output rows — each N row is independent
//!
//! Correctness is unchanged; the scalar reference is preserved in
//! `matmul_f32_scalar` for comparison tests.
//!
//! Spec: reference/runtime/ops.md §1

use crate::backend::BackendError;
use crate::tensor::Tensor;
use rayon::prelude::*;
use wide::f32x8;

/// Compute `y = x @ W^T`.
/// `x`: [..., K]
/// `W`: [N, K]
/// `y`: [..., N]
pub fn matmul_f32(x: &Tensor, w: &Tensor) -> Result<Tensor, BackendError> {
    if w.rank() != 2 {
        return Err(BackendError::ShapeMismatch {
            op: "Matmul",
            expected: vec![0, 0],
            got: w.shape.clone(),
        });
    }
    let n = w.shape[0];
    let k = w.shape[1];
    if x.shape.last() != Some(&k) {
        return Err(BackendError::ShapeMismatch {
            op: "Matmul",
            expected: vec![0, k],
            got: x.shape.clone(),
        });
    }

    let batch: usize = x.shape[..x.shape.len() - 1].iter().product();
    let x_data = x.as_f32();
    let w_data = w.as_f32();
    let mut out = vec![0f32; batch * n];

    // Outer: batch (usually 1 for decode). For each batch, parallelize over N.
    // Inner dot product uses f32x8 SIMD.
    for b in 0..batch {
        let x_row = &x_data[b * k..(b + 1) * k];
        let out_row = &mut out[b * n..(b + 1) * n];
        out_row
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, y)| {
                let w_row = &w_data[i * k..(i + 1) * k];
                *y = simd_dot_f32(x_row, w_row);
            });
    }

    let mut out_shape = x.shape.clone();
    *out_shape.last_mut().unwrap() = n;
    Ok(Tensor::from_f32(out_shape, out))
}

/// 8-wide SIMD dot product with scalar tail.
#[inline]
pub fn simd_dot_f32(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let k = a.len();
    let simd_tail = k % 8;
    let simd_end = k - simd_tail;

    let mut acc = f32x8::ZERO;
    let mut j = 0;
    while j < simd_end {
        // SAFETY: bounds ensured by simd_end = k - (k % 8)
        let va = f32x8::from(&a[j..j + 8]);
        let vb = f32x8::from(&b[j..j + 8]);
        acc = va.mul_add(vb, acc);
        j += 8;
    }
    let mut sum: f32 = acc.reduce_add();
    // Scalar tail
    while j < k {
        sum += a[j] * b[j];
        j += 1;
    }
    sum
}

/// Scalar reference for tests.
pub fn matmul_f32_scalar(x: &Tensor, w: &Tensor) -> Result<Tensor, BackendError> {
    let n = w.shape[0];
    let k = w.shape[1];
    let batch: usize = x.shape[..x.shape.len() - 1].iter().product();
    let x_data = x.as_f32();
    let w_data = w.as_f32();
    let mut out = vec![0f32; batch * n];
    for b in 0..batch {
        let x_row = &x_data[b * k..(b + 1) * k];
        for i in 0..n {
            let w_row = &w_data[i * k..(i + 1) * k];
            let mut acc = 0f32;
            for j in 0..k {
                acc += x_row[j] * w_row[j];
            }
            out[b * n + i] = acc;
        }
    }
    let mut out_shape = x.shape.clone();
    *out_shape.last_mut().unwrap() = n;
    Ok(Tensor::from_f32(out_shape, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_3x2_times_2x2() {
        // x = [[1, 2], [3, 4], [5, 6]]   shape [3, 2]
        // W = [[1, 2], [3, 4]]           shape [2, 2]   (N=2, K=2)
        // y = x @ W^T = [[5, 11], [11, 25], [17, 39]]
        let x = Tensor::from_f32(vec![3, 2], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let w = Tensor::from_f32(vec![2, 2], vec![1.0, 2.0, 3.0, 4.0]);
        let y = matmul_f32(&x, &w).unwrap();
        assert_eq!(y.shape, vec![3, 2]);
        assert_eq!(y.to_f32_vec(), vec![5.0, 11.0, 11.0, 25.0, 17.0, 39.0]);
    }

    #[test]
    fn shape_mismatch() {
        let x = Tensor::from_f32(vec![3, 2], vec![0.0; 6]);
        let w = Tensor::from_f32(vec![2, 3], vec![0.0; 6]); // K=3, but x last dim = 2
        assert!(matmul_f32(&x, &w).is_err());
    }

    #[test]
    fn simd_matches_scalar_on_realistic_shape() {
        // Shape like qwen3 q_proj: N=2048, K=1024
        let k = 1024;
        let n = 2048;
        let mut rng = 0x12345u64 | 1;
        let mut next = || -> f32 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            ((rng as f32) / (u64::MAX as f32)) - 0.5
        };
        let x_data: Vec<f32> = (0..k).map(|_| next() * 0.1).collect();
        let w_data: Vec<f32> = (0..n * k).map(|_| next() * 0.01).collect();
        let x = Tensor::from_f32(vec![1, k], x_data);
        let w = Tensor::from_f32(vec![n, k], w_data);
        let y_simd = matmul_f32(&x, &w).unwrap().to_f32_vec();
        let y_scalar = matmul_f32_scalar(&x, &w).unwrap().to_f32_vec();
        let mut worst = 0f32;
        for (a, b) in y_simd.iter().zip(y_scalar.iter()) {
            let d = (a - b).abs();
            if d > worst {
                worst = d;
            }
        }
        assert!(worst < 1e-3, "simd vs scalar worst diff: {worst}");
    }
}
