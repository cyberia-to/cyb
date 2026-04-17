//! Rotary Position Embedding — NeoX-style pairing.
//!
//! Spec: reference/runtime/ops.md §3

use crate::backend::BackendError;
use crate::tensor::Tensor;

/// Apply RoPE to the last `head_dim` axis.
///
/// Input shape: [..., num_heads, head_dim] or [..., head_dim] (heads optional).
/// `pos`: [..., seq] tensor of i64 (we accept f32 tensor; values cast to i64).
/// `head_dim` must be even.
///
/// Convention: NeoX pairing — first half paired with second half.
pub fn rope_f32(
    x: &Tensor,
    pos: &Tensor,
    head_dim: usize,
    base: f32,
) -> Result<Tensor, BackendError> {
    if head_dim % 2 != 0 {
        return Err(BackendError::InvalidInput {
            op: "Rope",
            reason: format!("head_dim must be even, got {head_dim}"),
        });
    }
    if x.shape.last() != Some(&head_dim) {
        return Err(BackendError::ShapeMismatch {
            op: "Rope",
            expected: vec![head_dim],
            got: x.shape.clone(),
        });
    }
    let half = head_dim / 2;

    // Flatten leading dims into (outer, seq, heads, head_dim).
    // For simplicity: assume x is [..., seq, head_dim] or [..., seq, heads, head_dim].
    // pos corresponds to one position per "seq" row.
    let n = x.numel() / head_dim;
    let pos_len = pos.numel();
    if n % pos_len != 0 {
        return Err(BackendError::InvalidInput {
            op: "Rope",
            reason: format!(
                "cannot broadcast pos (len={pos_len}) across x ({} rows of head_dim)",
                n
            ),
        });
    }
    let heads_per_pos = n / pos_len;

    let x_data = x.as_f32();
    let pos_data = pos.as_f32();
    let mut out = vec![0f32; x.numel()];

    for row in 0..n {
        let p_idx = row / heads_per_pos;
        let p = pos_data[p_idx];
        let x_row = &x_data[row * head_dim..(row + 1) * head_dim];
        let y_row = &mut out[row * head_dim..(row + 1) * head_dim];
        for j in 0..half {
            let theta = p / base.powf(2.0 * j as f32 / head_dim as f32);
            let (s, c) = theta.sin_cos();
            let x1 = x_row[j];
            let x2 = x_row[j + half];
            y_row[j] = x1 * c - x2 * s;
            y_row[j + half] = x1 * s + x2 * c;
        }
    }

    Ok(Tensor::from_f32(x.shape.clone(), out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pos_zero_is_identity() {
        // pos=0 → theta=0 → cos=1, sin=0 → identity
        let x = Tensor::from_f32(vec![1, 4], vec![1.0, 2.0, 3.0, 4.0]);
        let pos = Tensor::from_f32(vec![1], vec![0.0]);
        let y = rope_f32(&x, &pos, 4, 10000.0).unwrap();
        let v = y.to_f32_vec();
        assert!((v[0] - 1.0).abs() < 1e-6);
        assert!((v[1] - 2.0).abs() < 1e-6);
        assert!((v[2] - 3.0).abs() < 1e-6);
        assert!((v[3] - 4.0).abs() < 1e-6);
    }

    #[test]
    fn odd_head_dim_error() {
        let x = Tensor::from_f32(vec![1, 3], vec![0.0; 3]);
        let pos = Tensor::from_f32(vec![1], vec![0.0]);
        assert!(rope_f32(&x, &pos, 3, 10000.0).is_err());
    }

    #[test]
    fn rotation_90_at_specific_pos() {
        // At j=0, theta = pos * base^0 = pos. For theta=π/2 we need pos=π/2.
        // base=1 gives theta=pos, so pos=π/2 gives 90° rotation of first pair.
        let x = Tensor::from_f32(vec![1, 2], vec![1.0, 0.0]); // head_dim=2, pair (x[0], x[1])
        let pos = Tensor::from_f32(vec![1], vec![std::f32::consts::FRAC_PI_2]);
        let y = rope_f32(&x, &pos, 2, 1.0).unwrap();
        let v = y.to_f32_vec();
        // Pair (x[0]=1, x[1]=0) rotated by 90° → (0, 1)
        assert!(v[0].abs() < 1e-6);
        assert!((v[1] - 1.0).abs() < 1e-6);
    }
}
