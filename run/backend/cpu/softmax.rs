//! Softmax — numerically stable (subtract max).
//!
//! Spec: reference/runtime/ops.md §4

use crate::backend::BackendError;
use crate::core::tensor::Tensor;

pub fn softmax_f32(x: &Tensor, dim: i32) -> Result<Tensor, BackendError> {
    let rank = x.rank() as i32;
    let dim = if dim < 0 { rank + dim } else { dim };
    if dim < 0 || dim >= rank {
        return Err(BackendError::InvalidInput {
            op: "Softmax",
            reason: format!("dim {dim} out of range for rank {rank}"),
        });
    }
    // For simplicity: softmax over last dim. Reshape if dim != last.
    if dim != rank - 1 {
        return Err(BackendError::Internal(format!(
            "Softmax currently only supports last-dim (dim={dim}, rank={rank})"
        )));
    }
    let d = *x.shape.last().unwrap();
    let batch: usize = x.shape[..x.shape.len() - 1].iter().product();
    let x_data = x.as_f32();
    let mut out = vec![0f32; batch * d];
    for b in 0..batch {
        let xs = &x_data[b * d..(b + 1) * d];
        let ys = &mut out[b * d..(b + 1) * d];
        let max = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0f32;
        for (i, &v) in xs.iter().enumerate() {
            let e = (v - max).exp();
            ys[i] = e;
            sum += e;
        }
        let inv = 1.0 / sum;
        for y in ys.iter_mut() {
            *y *= inv;
        }
    }
    Ok(Tensor::from_f32(x.shape.clone(), out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_dist_averages() {
        let x = Tensor::from_f32(vec![3], vec![1.0, 1.0, 1.0]);
        let y = softmax_f32(&x, -1).unwrap().to_f32_vec();
        for v in y {
            assert!((v - 1.0 / 3.0).abs() < 1e-6);
        }
    }

    #[test]
    fn stable_at_large_values() {
        // Without max subtraction, exp(1000) = Inf → NaN in denom.
        let x = Tensor::from_f32(vec![3], vec![1000.0, 1000.0, 1000.0]);
        let y = softmax_f32(&x, -1).unwrap().to_f32_vec();
        for v in y {
            assert!((v - 1.0 / 3.0).abs() < 1e-6);
        }
    }

    #[test]
    fn dominant_at_one() {
        let x = Tensor::from_f32(vec![3], vec![0.0, 100.0, 0.0]);
        let y = softmax_f32(&x, -1).unwrap().to_f32_vec();
        assert!(y[0] < 1e-6);
        assert!(y[1] > 0.99);
        assert!(y[2] < 1e-6);
    }
}
