//! Tier 1: per-op unit tests.
//!
//! Each op is exercised on every available backend with fixed-seed inputs
//! and golden values. Goldens are the CPU reference output — the
//! correctness authority (see reference/runtime/test.md).
//!
//! Spec: reference/runtime/ops.md, reference/runtime/test.md

use mr::{Backend, DType, Op, Tensor};

/// Build list of backends available on current platform.
fn backends() -> Vec<(&'static str, Box<dyn Backend>)> {
    let mut v: Vec<(&'static str, Box<dyn Backend>)> = Vec::new();
    v.push(("cpu", Box::new(mr::cpu::CpuBackend::new())));
    if let Ok(b) = mr::wgpu_rs::WgpuRsBackend::new() {
        v.push(("wgpu+rs", Box::new(b)));
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(b) = mr::honeycrisp::HoneycrispBackend::new() {
            v.push(("honeycrisp", Box::new(b)));
        }
    }
    v
}

fn assert_close(a: &[f32], b: &[f32], eps: f32, ctx: &str) {
    assert_eq!(a.len(), b.len(), "{ctx}: length mismatch {} vs {}", a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        let diff = (x - y).abs();
        assert!(
            diff <= eps,
            "{ctx}: index {i} diff={diff} (a={x}, b={y}, eps={eps})"
        );
    }
}

// ───────────────────────────────────────────────────────────────
// RmsNorm
// ───────────────────────────────────────────────────────────────

#[test]
fn rmsnorm_all_backends() {
    let x = Tensor::from_f32(vec![2, 4], vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0]);
    let g = Tensor::from_f32(vec![4], vec![1.0, 0.5, 2.0, 1.0]);
    let eps = 1e-6;

    // Golden from CPU reference.
    let cpu = mr::cpu::CpuBackend::new();
    let golden = cpu
        .execute(&Op::RmsNorm { eps }, &[&x, &g])
        .expect("cpu rmsnorm")
        .remove(0)
        .to_f32_vec();

    for (name, b) in backends() {
        let out = b
            .execute(&Op::RmsNorm { eps }, &[&x, &g])
            .unwrap_or_else(|e| panic!("{name}: RmsNorm failed: {e}"))
            .remove(0);
        let out_f32 = b
            .download_f32(&out)
            .unwrap_or_else(|e| panic!("{name}: download_f32 failed: {e}"));
        assert_close(&out_f32, &golden, 1e-5, &format!("backend={name} RmsNorm"));
    }
}

#[test]
fn rmsnorm_eps_before_sqrt() {
    // Regression for the spec rule: ε is added to mean-of-squares, not after sqrt.
    // x = [0, 0], g = [1, 1]: without proper ε-before-sqrt, division by zero.
    let x = Tensor::from_f32(vec![2], vec![0.0, 0.0]);
    let g = Tensor::from_f32(vec![2], vec![1.0, 1.0]);
    for (name, b) in backends() {
        let out = b
            .execute(&Op::RmsNorm { eps: 1e-6 }, &[&x, &g])
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .remove(0);
        let v = b.download_f32(&out).unwrap();
        for (i, val) in v.iter().enumerate() {
            assert!(val.is_finite(), "{name}: RmsNorm output[{i}] not finite: {val}");
            assert!((val).abs() < 1e-3, "{name}: RmsNorm zero input should give ~0");
        }
    }
}

// ───────────────────────────────────────────────────────────────
// Matmul
// ───────────────────────────────────────────────────────────────

#[test]
fn matmul_small_all_backends() {
    // x: [3, 2], W: [2, 2] → y: [3, 2]
    // x @ W^T with x=[[1,2],[3,4],[5,6]], W=[[1,2],[3,4]]
    // Expected: [[5,11],[11,25],[17,39]]
    let x = Tensor::from_f32(vec![3, 2], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let w = Tensor::from_f32(vec![2, 2], vec![1.0, 2.0, 3.0, 4.0]);
    let expected = [5.0, 11.0, 11.0, 25.0, 17.0, 39.0];

    for (name, b) in backends() {
        let out = b
            .execute(&Op::Matmul, &[&x, &w])
            .unwrap_or_else(|e| panic!("{name}: Matmul failed: {e}"))
            .remove(0);
        let v = b.download_f32(&out).unwrap();
        assert_close(&v, &expected, 1e-5, &format!("backend={name} Matmul"));
    }
}

#[test]
fn matmul_shape_mismatch_errors() {
    let x = Tensor::from_f32(vec![3, 2], vec![0.0; 6]);
    let w = Tensor::from_f32(vec![2, 3], vec![0.0; 6]); // K=3, but x last=2
    for (name, b) in backends() {
        assert!(
            b.execute(&Op::Matmul, &[&x, &w]).is_err(),
            "backend={name}: expected Matmul shape error"
        );
    }
}

// ───────────────────────────────────────────────────────────────
// Softmax
// ───────────────────────────────────────────────────────────────

#[test]
fn softmax_last_dim() {
    let x = Tensor::from_f32(vec![2, 3], vec![1.0, 1.0, 1.0, 0.0, 0.0, 1000.0]);
    for (name, b) in backends() {
        let out = b
            .execute(&Op::Softmax { dim: -1 }, &[&x])
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .remove(0);
        let v = b.download_f32(&out).unwrap();
        // Row 0: uniform
        for j in 0..3 {
            assert!(
                (v[j] - 1.0 / 3.0).abs() < 1e-5,
                "{name}: row0 col{j} = {}",
                v[j]
            );
        }
        // Row 1: concentrated at index 2
        assert!(v[3] < 1e-5, "{name}: row1 col0 should be near zero");
        assert!(v[5] > 0.99, "{name}: row1 col2 should be near 1");
    }
}

// ───────────────────────────────────────────────────────────────
// Silu, Gelu
// ───────────────────────────────────────────────────────────────

#[test]
fn silu_all_backends() {
    let x = Tensor::from_f32(vec![4], vec![-2.0, -0.5, 0.5, 2.0]);
    let cpu = mr::cpu::CpuBackend::new();
    let golden = cpu
        .execute(&Op::Silu, &[&x])
        .unwrap()
        .remove(0)
        .to_f32_vec();
    for (name, b) in backends() {
        let out = b.execute(&Op::Silu, &[&x]).unwrap().remove(0);
        let v = b.download_f32(&out).unwrap();
        assert_close(&v, &golden, 1e-5, &format!("backend={name} Silu"));
    }
}

#[test]
fn gelu_exact_and_tanh_agree_near_zero() {
    let x = Tensor::from_f32(vec![5], vec![-0.1, -0.05, 0.0, 0.05, 0.1]);
    for (name, b) in backends() {
        let exact = b
            .execute(&Op::Gelu { approximate: false }, &[&x])
            .unwrap()
            .remove(0);
        let tanh_ = b
            .execute(&Op::Gelu { approximate: true }, &[&x])
            .unwrap()
            .remove(0);
        let ve = b.download_f32(&exact).unwrap();
        let vt = b.download_f32(&tanh_).unwrap();
        assert_close(&ve, &vt, 5e-3, &format!("backend={name} Gelu variants agree"));
    }
}

// ───────────────────────────────────────────────────────────────
// Rope
// ───────────────────────────────────────────────────────────────

#[test]
fn rope_pos_zero_is_identity() {
    let x = Tensor::from_f32(vec![1, 4], vec![1.0, 2.0, 3.0, 4.0]);
    let pos = Tensor::from_f32(vec![1], vec![0.0]);
    for (name, b) in backends() {
        let out = b
            .execute(
                &Op::Rope {
                    head_dim: 4,
                    rope_dim: 4,
                    base: 10000.0,
                },
                &[&x, &pos],
            )
            .unwrap()
            .remove(0);
        let v = b.download_f32(&out).unwrap();
        assert_close(
            &v,
            &[1.0, 2.0, 3.0, 4.0],
            1e-5,
            &format!("backend={name} Rope(pos=0) identity"),
        );
    }
}

#[test]
fn rope_odd_head_dim_errors() {
    let x = Tensor::from_f32(vec![1, 3], vec![0.0; 3]);
    let pos = Tensor::from_f32(vec![1], vec![0.0]);
    for (name, b) in backends() {
        assert!(
            b.execute(
                &Op::Rope {
                    head_dim: 3,
                    rope_dim: 3,
                    base: 10000.0,
                },
                &[&x, &pos]
            )
            .is_err(),
            "{name}: odd head_dim should error"
        );
    }
}
