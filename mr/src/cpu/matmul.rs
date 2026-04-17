//! Matmul: y = x @ W^T
//!
//! Spec: reference/runtime/ops.md §1

use crate::backend::BackendError;
use crate::tensor::Tensor;

/// Compute `y = x @ W^T`.
/// `x`: [..., K]
/// `W`: [N, K]
/// `y`: [..., N]
///
/// F32 reference implementation.
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

    for b in 0..batch {
        let x_row = &x_data[b * k..(b + 1) * k];
        let out_row = &mut out[b * n..(b + 1) * n];
        for i in 0..n {
            let w_row = &w_data[i * k..(i + 1) * k];
            let mut acc = 0f32;
            for j in 0..k {
                acc += x_row[j] * w_row[j];
            }
            out_row[i] = acc;
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
}
