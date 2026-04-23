//! Rotary Position Embedding — NeoX-style pairing.
//!
//! Spec: reference/runtime/ops.md §3

use crate::backend::BackendError;
use crate::tensor::Tensor;

/// Apply RoPE to the last `head_dim` axis (NeoX pairing).
///
/// Input shape: [..., num_heads, head_dim] or [..., head_dim].
/// `pos`: tensor with f32 position values, broadcast across rows.
///
/// `rope_dim` is the count of rotated dimensions (must be even, ≤ head_dim).
/// Standard RoPE has `rope_dim == head_dim`; partial-rotary (Gemma-4 full
/// layers) has `rope_dim < head_dim` — the trailing `head_dim - rope_dim`
/// dimensions pass through unchanged.
pub fn rope_f32(
    x: &Tensor,
    pos: &Tensor,
    head_dim: usize,
    rope_dim: usize,
    base: f32,
) -> Result<Tensor, BackendError> {
    if rope_dim == 0 || rope_dim % 2 != 0 || rope_dim > head_dim {
        return Err(BackendError::InvalidInput {
            op: "Rope",
            reason: format!("rope_dim must be even and ≤ head_dim, got {rope_dim} / {head_dim}"),
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
    let rope_half = rope_dim / 2;

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
        // NeoX pairing across the FULL head_dim: dim j is paired with
        // dim (j + head_dim/2). Partial rotary (rope_dim < head_dim) only
        // rotates the first rope_half pairs; the remainder is identity.
        // The frequency uses head_dim as the divisor (matches HF
        // _compute_proportional_rope_parameters).
        for j in 0..rope_half {
            let theta = p / base.powf(2.0 * j as f32 / head_dim as f32);
            let (s, c) = theta.sin_cos();
            let x1 = x_row[j];
            let x2 = x_row[j + half];
            y_row[j] = x1 * c - x2 * s;
            y_row[j + half] = x1 * s + x2 * c;
        }
        // Pass-through pairs (rope_half..half) — identity rotation.
        for j in rope_half..half {
            y_row[j] = x_row[j];
            y_row[j + half] = x_row[j + half];
        }
    }

    Ok(Tensor::from_f32(x.shape.clone(), out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pos_zero_is_identity() {
        let x = Tensor::from_f32(vec![1, 4], vec![1.0, 2.0, 3.0, 4.0]);
        let pos = Tensor::from_f32(vec![1], vec![0.0]);
        let y = rope_f32(&x, &pos, 4, 4, 10000.0).unwrap();
        let v = y.to_f32_vec();
        assert!((v[0] - 1.0).abs() < 1e-6);
        assert!((v[1] - 2.0).abs() < 1e-6);
        assert!((v[2] - 3.0).abs() < 1e-6);
        assert!((v[3] - 4.0).abs() < 1e-6);
    }

    #[test]
    fn odd_rope_dim_error() {
        let x = Tensor::from_f32(vec![1, 4], vec![0.0; 4]);
        let pos = Tensor::from_f32(vec![1], vec![0.0]);
        assert!(rope_f32(&x, &pos, 4, 3, 10000.0).is_err());
    }

    #[test]
    fn rotation_90_at_specific_pos() {
        let x = Tensor::from_f32(vec![1, 2], vec![1.0, 0.0]);
        let pos = Tensor::from_f32(vec![1], vec![std::f32::consts::FRAC_PI_2]);
        let y = rope_f32(&x, &pos, 2, 2, 1.0).unwrap();
        let v = y.to_f32_vec();
        assert!(v[0].abs() < 1e-6);
        assert!((v[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn partial_rotary_passes_trailing_dims() {
        // head_dim=8 (half=4), rope_dim=4 (rope_half=2): NeoX pairing across
        // the full half=4 width — dim j paired with dim j+4. Only the first
        // 2 pairs (j=0,1) get rotation; pairs (j=2,3) pass through.
        // Matches HF _compute_proportional_rope_parameters (zero inv_freq for
        // the trailing pairs → identity rotation).
        // pos=0 → all rotations identity, output equals input.
        let x = Tensor::from_f32(vec![1, 8], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        let pos = Tensor::from_f32(vec![1], vec![0.0]);
        let y = rope_f32(&x, &pos, 8, 4, 10000.0).unwrap().to_f32_vec();
        for (i, v) in y.iter().enumerate() {
            assert!((v - (i as f32 + 1.0)).abs() < 1e-6, "y[{i}]={v}");
        }

        // pos=π/2, base=1, head_dim=8, rope_dim=4 → rotate pairs (0,4) and (1,5)
        // by 90° (theta = π/2). Pair (2,6) and (3,7) pass through.
        // Pair (x[0]=1, x[4]=5): y[0] = 1*cos - 5*sin = -5; y[4] = 1*sin + 5*cos = 1.
        // Pair (x[1]=2, x[5]=6): y[1] = -6; y[5] = 2.
        let pos90 = Tensor::from_f32(vec![1], vec![std::f32::consts::FRAC_PI_2]);
        let y2 = rope_f32(&x, &pos90, 8, 4, 1.0).unwrap().to_f32_vec();
        assert!((y2[0] - -5.0).abs() < 1e-5, "y2[0]={}", y2[0]);
        assert!((y2[4] - 1.0).abs() < 1e-5, "y2[4]={}", y2[4]);
        assert!((y2[1] - -6.0).abs() < 1e-5, "y2[1]={}", y2[1]);
        assert!((y2[5] - 2.0).abs() < 1e-5, "y2[5]={}", y2[5]);
        // Pairs (2,6) and (3,7) pass through unchanged.
        assert!((y2[2] - 3.0).abs() < 1e-6, "y2[2]={}", y2[2]);
        assert!((y2[6] - 7.0).abs() < 1e-6, "y2[6]={}", y2[6]);
        assert!((y2[3] - 4.0).abs() < 1e-6, "y2[3]={}", y2[3]);
        assert!((y2[7] - 8.0).abs() < 1e-6, "y2[7]={}", y2[7]);
    }
}
