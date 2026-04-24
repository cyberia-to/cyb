//! Element-wise activations: Silu, Gelu, SwiGlu.
//!
//! Spec: specs/ops.md §4

use crate::backend::BackendError;
use crate::backend::cpu::matmul::matmul_f32;
use crate::core::tensor::Tensor;

pub fn silu_f32(x: &Tensor) -> Result<Tensor, BackendError> {
    let data = x.as_f32();
    let out: Vec<f32> = data.iter().map(|&v| v / (1.0 + (-v).exp())).collect();
    Ok(Tensor::from_f32(x.shape.clone(), out))
}

pub fn gelu_erf_f32(x: &Tensor) -> Result<Tensor, BackendError> {
    let data = x.as_f32();
    let sqrt_2 = std::f32::consts::SQRT_2;
    let out: Vec<f32> = data
        .iter()
        .map(|&v| 0.5 * v * (1.0 + erf_approx(v / sqrt_2)))
        .collect();
    Ok(Tensor::from_f32(x.shape.clone(), out))
}

pub fn gelu_tanh_f32(x: &Tensor) -> Result<Tensor, BackendError> {
    let data = x.as_f32();
    let c = (2.0 / std::f32::consts::PI).sqrt();
    let out: Vec<f32> = data
        .iter()
        .map(|&v| 0.5 * v * (1.0 + (c * (v + 0.044715 * v * v * v)).tanh()))
        .collect();
    Ok(Tensor::from_f32(x.shape.clone(), out))
}

/// SwiGlu FFN: y = (silu(x @ W_gate^T) ⊙ (x @ W_up^T)) @ W_down^T
pub fn swiglu_f32(
    x: &Tensor,
    w_gate: &Tensor,
    w_up: &Tensor,
    w_down: &Tensor,
) -> Result<Tensor, BackendError> {
    let gate = matmul_f32(x, w_gate)?;
    let up = matmul_f32(x, w_up)?;
    let gate_silu = silu_f32(&gate)?;
    // element-wise multiply
    let gd = gate_silu.as_f32();
    let ud = up.as_f32();
    assert_eq!(gd.len(), ud.len());
    let mul: Vec<f32> = gd.iter().zip(ud.iter()).map(|(a, b)| a * b).collect();
    let mid = Tensor::from_f32(gate_silu.shape.clone(), mul);
    matmul_f32(&mid, w_down)
}

/// Abramowitz-Stegun 7.1.26 approximation of erf, max abs error ~1.5e-7
fn erf_approx(x: f32) -> f32 {
    let a1 = 0.254_829_592;
    let a2 = -0.284_496_736;
    let a3 = 1.421_413_741;
    let a4 = -1.453_152_027;
    let a5 = 1.061_405_429;
    let p = 0.327_591_1;
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - ((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silu_zero_is_zero() {
        let x = Tensor::from_f32(vec![1], vec![0.0]);
        let y = silu_f32(&x).unwrap();
        assert!((y.to_f32_vec()[0]).abs() < 1e-6);
    }

    #[test]
    fn silu_one() {
        // silu(1) = 1 * sigmoid(1) = 1 / (1+exp(-1)) ≈ 0.731059
        let x = Tensor::from_f32(vec![1], vec![1.0]);
        let y = silu_f32(&x).unwrap();
        assert!((y.to_f32_vec()[0] - 0.731_059).abs() < 1e-5);
    }

    #[test]
    fn gelu_erf_zero() {
        let x = Tensor::from_f32(vec![1], vec![0.0]);
        let y = gelu_erf_f32(&x).unwrap();
        assert!((y.to_f32_vec()[0]).abs() < 1e-6);
    }

    #[test]
    fn gelu_tanh_zero() {
        let x = Tensor::from_f32(vec![1], vec![0.0]);
        let y = gelu_tanh_f32(&x).unwrap();
        assert!((y.to_f32_vec()[0]).abs() < 1e-6);
    }

    #[test]
    fn gelu_variants_close_at_small_values() {
        // Both variants approximate the same function; agree at small x.
        let x = Tensor::from_f32(vec![1], vec![0.5]);
        let e = gelu_erf_f32(&x).unwrap().to_f32_vec()[0];
        let t = gelu_tanh_f32(&x).unwrap().to_f32_vec()[0];
        assert!((e - t).abs() < 1e-3);
    }
}
