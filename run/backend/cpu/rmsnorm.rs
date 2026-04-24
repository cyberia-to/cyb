//! RMS normalization.
//!
//! Spec: reference/runtime/ops.md §2
//!
//! y = (x / sqrt(mean(x²) + ε)) * g
//!
//! Critical: ε is added to mean-of-squares BEFORE sqrt.

use crate::backend::BackendError;
use crate::core::tensor::Tensor;

/// RMSNorm over the last dimension.
///
/// `x`: [..., D]
/// `g`: [D]  (learned gain)
/// `eps`: small positive scalar
pub fn rms_norm_f32(x: &Tensor, g: &Tensor, eps: f32) -> Result<Tensor, BackendError> {
    if g.rank() != 1 {
        return Err(BackendError::ShapeMismatch {
            op: "RmsNorm",
            expected: vec![0],
            got: g.shape.clone(),
        });
    }
    let d = g.shape[0];
    if x.shape.last() != Some(&d) {
        return Err(BackendError::ShapeMismatch {
            op: "RmsNorm",
            expected: vec![d],
            got: x.shape.clone(),
        });
    }

    let batch: usize = x.shape[..x.shape.len() - 1].iter().product();
    let x_data = x.as_f32();
    let g_data = g.as_f32();
    let mut out = vec![0f32; batch * d];

    for b in 0..batch {
        let xs = &x_data[b * d..(b + 1) * d];
        // Mean of squares — single pass
        let mut sum_sq = 0f32;
        for v in xs {
            sum_sq += v * v;
        }
        let rms = (sum_sq / d as f32 + eps).sqrt();
        let inv = 1.0 / rms;
        let ys = &mut out[b * d..(b + 1) * d];
        for j in 0..d {
            ys[j] = xs[j] * inv * g_data[j];
        }
    }

    Ok(Tensor::from_f32(x.shape.clone(), out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_gain_zero_eps() {
        // x = [3, 4]; rms = sqrt((9+16)/2) = sqrt(12.5)
        // y = x / rms
        let x = Tensor::from_f32(vec![2], vec![3.0, 4.0]);
        let g = Tensor::from_f32(vec![2], vec![1.0, 1.0]);
        let y = rms_norm_f32(&x, &g, 0.0).unwrap();
        let rms = (25f32 / 2.0).sqrt();
        assert!((y.to_f32_vec()[0] - 3.0 / rms).abs() < 1e-6);
        assert!((y.to_f32_vec()[1] - 4.0 / rms).abs() < 1e-6);
    }

    #[test]
    fn gain_scaling() {
        let x = Tensor::from_f32(vec![2], vec![3.0, 4.0]);
        let g = Tensor::from_f32(vec![2], vec![2.0, 0.5]);
        let y = rms_norm_f32(&x, &g, 0.0).unwrap();
        let rms = (25f32 / 2.0).sqrt();
        assert!((y.to_f32_vec()[0] - 2.0 * 3.0 / rms).abs() < 1e-6);
        assert!((y.to_f32_vec()[1] - 0.5 * 4.0 / rms).abs() < 1e-6);
    }

    #[test]
    fn eps_added_before_sqrt() {
        // Small x: ε matters.
        // x = [0.0, 0.0], g = [1, 1], eps = 1e-6
        // sum_sq = 0, sum_sq/d + eps = 1e-6, rms = 1e-3
        // y = 0 / 1e-3 = 0
        let x = Tensor::from_f32(vec![2], vec![0.0, 0.0]);
        let g = Tensor::from_f32(vec![2], vec![1.0, 1.0]);
        let y = rms_norm_f32(&x, &g, 1e-6).unwrap();
        assert_eq!(y.to_f32_vec(), vec![0.0, 0.0]);
        // But NO panic — division by zero would happen if ε were added after sqrt.
    }

    #[test]
    fn multi_batch() {
        let x = Tensor::from_f32(vec![2, 2], vec![3.0, 4.0, 6.0, 8.0]);
        let g = Tensor::from_f32(vec![2], vec![1.0, 1.0]);
        let y = rms_norm_f32(&x, &g, 0.0).unwrap();
        assert_eq!(y.shape, vec![2, 2]);
        // Row 0: rms = sqrt(12.5), y = [3, 4] / rms
        // Row 1: rms = sqrt(50),   y = [6, 8] / rms
        let r0 = (25f32 / 2.0).sqrt();
        let r1 = (100f32 / 2.0).sqrt();
        let v = y.to_f32_vec();
        assert!((v[0] - 3.0 / r0).abs() < 1e-6);
        assert!((v[2] - 6.0 / r1).abs() < 1e-6);
    }
}
