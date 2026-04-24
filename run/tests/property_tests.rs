//! Property-based tests: random inputs verified across backends.
//!
//! For every supported op, generate N random inputs with random shapes,
//! run on every available backend, check all outputs match within ε.
//!
//! Catches bugs that fixed-input tests miss: edge cases in sizes,
//! accumulation precision, backend-specific overflow / rounding.
//!
//! Spec: reference/runtime/test.md

use run::{Backend, Op, Tensor};

const ITERATIONS: usize = 16;

/// Seeded xorshift64 PRNG — deterministic across test runs.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f32(&mut self) -> f32 {
        (self.next_u64() as f32) / (u64::MAX as f32) - 0.5
    }
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next_u64() as usize) % (hi - lo + 1)
    }
    fn tensor(&mut self, shape: Vec<usize>, scale: f32) -> Tensor {
        let n: usize = shape.iter().product();
        let data: Vec<f32> = (0..n).map(|_| self.f32() * scale).collect();
        Tensor::from_f32(shape, data)
    }
}

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

/// Compare a multi-backend execution: produce outputs on each backend,
/// assert they all match within ε.
fn check_cross_backend(
    op: &Op,
    inputs: &[&Tensor],
    eps: f32,
    ctx: &str,
) {
    let backends = backends();
    if backends.len() < 2 {
        eprintln!("skipping {ctx}: need ≥2 backends, got {}", backends.len());
        return;
    }
    let mut outputs: Vec<(String, Vec<f32>)> = Vec::new();
    for (name, b) in &backends {
        let out = b
            .execute(op, inputs)
            .unwrap_or_else(|e| panic!("{ctx} / {name}: {e}"));
        let vals = b
            .download_f32(&out[0])
            .unwrap_or_else(|e| panic!("{ctx} / {name} download: {e}"));
        outputs.push(((*name).to_string(), vals));
    }
    let (_, ref0) = &outputs[0];
    for (name, vals) in &outputs[1..] {
        assert_eq!(
            ref0.len(),
            vals.len(),
            "{ctx}: length mismatch {} vs {}",
            backends[0].0,
            name
        );
        let mut max_diff = 0f32;
        for (i, (a, b)) in ref0.iter().zip(vals.iter()).enumerate() {
            let diff = (a - b).abs();
            if diff > max_diff {
                max_diff = diff;
            }
            assert!(
                diff <= eps,
                "{ctx}: {} vs {} diverge at idx {i}: {a} vs {b} (diff {diff}, eps {eps})",
                backends[0].0,
                name
            );
        }
    }
}

#[test]
fn property_matmul() {
    let mut rng = Rng::new(0xDEADBEEF);
    for i in 0..ITERATIONS {
        let batch = rng.range(1, 4);
        let n = rng.range(1, 64);
        let k = rng.range(1, 64);
        let x = rng.tensor(vec![batch, k], 0.1);
        let w = rng.tensor(vec![n, k], 0.1);
        check_cross_backend(&Op::Matmul, &[&x, &w], 1e-4, &format!("matmul[{i}] {batch}×{k} × {n}×{k}"));
    }
}

#[test]
fn property_rmsnorm() {
    let mut rng = Rng::new(0xC0FFEE);
    for i in 0..ITERATIONS {
        let batch = rng.range(1, 8);
        let d = rng.range(2, 256);
        let x = rng.tensor(vec![batch, d], 1.0);
        let g = rng.tensor(vec![d], 0.5);
        check_cross_backend(
            &Op::RmsNorm { eps: 1e-6 },
            &[&x, &g],
            1e-4,
            &format!("rmsnorm[{i}] batch={batch} d={d}"),
        );
    }
}

#[test]
fn property_silu() {
    let mut rng = Rng::new(0x1234);
    for i in 0..ITERATIONS {
        let n = rng.range(1, 1000);
        let x = rng.tensor(vec![n], 2.0);
        check_cross_backend(&Op::Silu, &[&x], 1e-5, &format!("silu[{i}] n={n}"));
    }
}

#[test]
fn property_rope() {
    let mut rng = Rng::new(0xABCDEF);
    for i in 0..ITERATIONS {
        // head_dim must be even
        let head_dim = rng.range(1, 16) * 2;
        let num_heads = rng.range(1, 4);
        let x = rng.tensor(vec![num_heads, head_dim], 0.5);
        let pos = Tensor::from_f32(vec![1], vec![rng.range(0, 100) as f32]);
        check_cross_backend(
            &Op::Rope {
                head_dim: head_dim as u32,
                rope_dim: head_dim as u32,
                base: 10000.0,
            },
            &[&x, &pos],
            1e-4,
            &format!("rope[{i}] heads={num_heads} head_dim={head_dim}"),
        );
    }
}

#[test]
fn property_softmax() {
    let mut rng = Rng::new(0x55555);
    for i in 0..ITERATIONS {
        let batch = rng.range(1, 4);
        let d = rng.range(2, 128);
        let x = rng.tensor(vec![batch, d], 5.0); // larger scale probes stability
        check_cross_backend(
            &Op::Softmax { dim: -1 },
            &[&x],
            1e-5,
            &format!("softmax[{i}] batch={batch} d={d}"),
        );
    }
}

#[test]
fn property_matmul_large_k() {
    // Accumulation precision stress: K large so errors accumulate.
    let mut rng = Rng::new(0x77777);
    let x = rng.tensor(vec![1, 1024], 0.01);
    let w = rng.tensor(vec![64, 1024], 0.01);
    // Tighter scale keeps magnitudes from saturating F32 quickly.
    check_cross_backend(&Op::Matmul, &[&x, &w], 5e-4, "matmul large K=1024");
}

#[test]
fn property_rmsnorm_small_x() {
    // eps-before-sqrt test: x→0 must not explode.
    let mut rng = Rng::new(0x99999);
    for i in 0..4 {
        let d = rng.range(16, 128);
        let data: Vec<f32> = (0..d).map(|_| rng.f32() * 1e-8).collect();
        let x = Tensor::from_f32(vec![1, d], data);
        let g = rng.tensor(vec![d], 1.0);
        check_cross_backend(
            &Op::RmsNorm { eps: 1e-6 },
            &[&x, &g],
            1e-3,
            &format!("rmsnorm-tiny[{i}] d={d}"),
        );
    }
}
