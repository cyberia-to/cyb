//! Tier 2: layer composition tests.
//!
//! LlamaStyle attention + FFN sub-layers. CPU reference is the golden;
//! other backends must match within tolerance.
//!
//! Spec: reference/runtime/arch.md#llamastyle, reference/runtime/test.md

use run::{Backend, Op, Tensor};

fn backends() -> Vec<(&'static str, Box<dyn Backend>)> {
    let mut v: Vec<(&'static str, Box<dyn Backend>)> = Vec::new();
    v.push(("cpu", Box::new(run::backend::cpu::CpuBackend::new())));
    if let Ok(b) = run::backend::wgpu::WgpuRsBackend::new() {
        v.push(("wgpu+rs", Box::new(b)));
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(b) = run::backend::honeycrisp::HoneycrispBackend::new() {
            v.push(("honeycrisp", Box::new(b)));
        }
    }
    v
}

/// LlamaStyle attention input layer:
///   h_norm = RmsNorm(h, input_norm_w, eps)
///   q = h_norm @ W_q^T
///   k = h_norm @ W_k^T
///   v = h_norm @ W_v^T
fn run_attn_input(
    backend: &dyn Backend,
    h: &Tensor,
    input_norm_w: &Tensor,
    w_q: &Tensor,
    w_k: &Tensor,
    w_v: &Tensor,
    eps: f32,
) -> (Tensor, Tensor, Tensor) {
    let normed = backend
        .execute(&Op::RmsNorm { eps }, &[h, input_norm_w])
        .expect("rmsnorm")
        .remove(0);
    let q = backend
        .execute(&Op::Matmul, &[&normed, w_q])
        .expect("matmul q")
        .remove(0);
    let k = backend
        .execute(&Op::Matmul, &[&normed, w_k])
        .expect("matmul k")
        .remove(0);
    let v = backend
        .execute(&Op::Matmul, &[&normed, w_v])
        .expect("matmul v")
        .remove(0);
    (q, k, v)
}

fn rand_tensor(shape: Vec<usize>, seed: u64) -> Tensor {
    // Deterministic pseudo-random f32 in [-0.1, 0.1] — small enough to avoid overflow.
    let n: usize = shape.iter().product();
    let mut data = Vec::with_capacity(n);
    let mut state = seed;
    for _ in 0..n {
        // xorshift64
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let f = (state as f32) / (u64::MAX as f32);
        data.push((f - 0.5) * 0.2);
    }
    Tensor::from_f32(shape, data)
}

#[test]
fn llamastyle_attn_input_matches_across_backends() {
    // Realistic small dimensions: hidden=64, num_heads=4, head_dim=16, kv_heads=2
    let hidden = 64;
    let q_dim = 4 * 16;
    let kv_dim = 2 * 16;
    let eps = 1e-6;

    let h = rand_tensor(vec![1, hidden], 1);
    let input_norm_w = rand_tensor(vec![hidden], 2);
    let w_q = rand_tensor(vec![q_dim, hidden], 3);
    let w_k = rand_tensor(vec![kv_dim, hidden], 4);
    let w_v = rand_tensor(vec![kv_dim, hidden], 5);

    let cpu = run::backend::cpu::CpuBackend::new();
    let (q0, k0, v0) = run_attn_input(&cpu, &h, &input_norm_w, &w_q, &w_k, &w_v, eps);
    let q_ref = q0.to_f32_vec();
    let k_ref = k0.to_f32_vec();
    let v_ref = v0.to_f32_vec();

    for (name, b) in backends() {
        let (q, k, v) = run_attn_input(&*b, &h, &input_norm_w, &w_q, &w_k, &w_v, eps);
        let qv = b.download_f32(&q).unwrap();
        let kv = b.download_f32(&k).unwrap();
        let vv = b.download_f32(&v).unwrap();
        for (i, (a, r)) in qv.iter().zip(q_ref.iter()).enumerate() {
            assert!(
                (a - r).abs() < 5e-5,
                "{name}: q[{i}] diverges: {a} vs {r}"
            );
        }
        for (i, (a, r)) in kv.iter().zip(k_ref.iter()).enumerate() {
            assert!(
                (a - r).abs() < 5e-5,
                "{name}: k[{i}] diverges: {a} vs {r}"
            );
        }
        for (i, (a, r)) in vv.iter().zip(v_ref.iter()).enumerate() {
            assert!(
                (a - r).abs() < 5e-5,
                "{name}: v[{i}] diverges: {a} vs {r}"
            );
        }
    }
}

#[test]
fn llamastyle_swiglu_ffn_matches_across_backends() {
    let hidden = 32;
    let intermediate = 64;

    let h = rand_tensor(vec![1, hidden], 10);
    let w_gate = rand_tensor(vec![intermediate, hidden], 11);
    let w_up = rand_tensor(vec![intermediate, hidden], 12);
    let w_down = rand_tensor(vec![hidden, intermediate], 13);

    let cpu = run::backend::cpu::CpuBackend::new();
    let ref_out = cpu
        .execute(&Op::SwiGlu, &[&h, &w_gate, &w_up, &w_down])
        .unwrap()
        .remove(0)
        .to_f32_vec();

    for (name, b) in backends() {
        let out = b
            .execute(&Op::SwiGlu, &[&h, &w_gate, &w_up, &w_down])
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .remove(0);
        let v = b.download_f32(&out).unwrap();
        for (i, (a, r)) in v.iter().zip(ref_out.iter()).enumerate() {
            assert!(
                (a - r).abs() < 1e-4,
                "{name}: swiglu[{i}] diverges: {a} vs {r}"
            );
        }
    }
}
