//! Benchmarking — per-op, per-layer, end-to-end.
//!
//! Three levels:
//! - micro: one op, N iterations, reports ns/op per backend
//! - layer: one decode step broken down by op
//! - e2e: load + prefill + decode phases

use crate::backend::Backend;
use crate::llama_style::LlamaModel;
use crate::op::Op;
use crate::tensor::Tensor;
use std::time::{Duration, Instant};

pub struct OpBench {
    pub op_name: String,
    pub shape: String,
    pub backend: String,
    pub iterations: usize,
    pub total: Duration,
}

impl OpBench {
    pub fn per_op_us(&self) -> f64 {
        self.total.as_secs_f64() * 1e6 / self.iterations as f64
    }
}

/// Microbenchmark: run `op` with `inputs` on `backend`, `iters` times.
pub fn bench_op(
    op: &Op,
    inputs: &[&Tensor],
    backend: &dyn Backend,
    iters: usize,
    shape_desc: &str,
) -> OpBench {
    // Warm up (compile pipelines, allocate pools).
    for _ in 0..3 {
        let _ = backend.execute(op, inputs).expect("bench warmup");
    }
    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = backend.execute(op, inputs).expect("bench op");
    }
    OpBench {
        op_name: op.name().into(),
        shape: shape_desc.into(),
        backend: backend.kind().as_str().into(),
        iterations: iters,
        total: t0.elapsed(),
    }
}

#[derive(Clone)]
pub struct E2EBench {
    pub backend: String,
    pub load_ms: f64,
    pub to_backend_ms: f64,
    pub first_forward_ms: f64,
    pub subsequent_forwards_ms: Vec<f64>,
}

impl E2EBench {
    pub fn prefill_tok_s(&self, prefill_tokens: usize) -> f64 {
        if prefill_tokens == 0 {
            return 0.0;
        }
        let total_ms: f64 = self.first_forward_ms
            + self.subsequent_forwards_ms[..prefill_tokens.saturating_sub(1)]
                .iter()
                .sum::<f64>();
        (prefill_tokens as f64) / (total_ms / 1000.0)
    }
    pub fn decode_tok_s(&self, prefill_tokens: usize) -> f64 {
        let decode = &self.subsequent_forwards_ms[prefill_tokens.saturating_sub(1).min(self.subsequent_forwards_ms.len())..];
        if decode.is_empty() {
            return 0.0;
        }
        (decode.len() as f64) / (decode.iter().sum::<f64>() / 1000.0)
    }
    pub fn avg_forward_ms(&self) -> f64 {
        if self.subsequent_forwards_ms.is_empty() {
            return self.first_forward_ms;
        }
        let sum: f64 = self.subsequent_forwards_ms.iter().sum();
        sum / self.subsequent_forwards_ms.len() as f64
    }
}

/// End-to-end bench: load model, upload weights, run N forward steps.
/// Reports phase breakdown to illuminate what dominates.
pub fn bench_e2e(
    path: &std::path::Path,
    backend: &dyn Backend,
    steps: usize,
) -> Result<E2EBench, String> {
    let t_load = Instant::now();
    let lm = crate::format::LoadedModel::load(path).map_err(|e| format!("{e}"))?;
    let mut model = LlamaModel::from_loaded(&lm).map_err(|e| format!("{e}"))?;
    let load_ms = t_load.elapsed().as_secs_f64() * 1000.0;

    let t_backend = Instant::now();
    model.to_backend(backend).map_err(|e| format!("{e}"))?;
    let to_backend_ms = t_backend.elapsed().as_secs_f64() * 1000.0;

    // Single-token warmup to compile kernels + prime allocators.
    let t_first = Instant::now();
    let _ = model.forward(0, backend).map_err(|e| format!("{e}"))?;
    let first_forward_ms = t_first.elapsed().as_secs_f64() * 1000.0;

    let mut forwards = Vec::with_capacity(steps);
    for i in 0..steps {
        let tok = (i % 100) as u32;
        let t = Instant::now();
        let _ = model.forward(tok, backend).map_err(|e| format!("{e}"))?;
        forwards.push(t.elapsed().as_secs_f64() * 1000.0);
    }

    Ok(E2EBench {
        backend: backend.kind().as_str().into(),
        load_ms,
        to_backend_ms,
        first_forward_ms,
        subsequent_forwards_ms: forwards,
    })
}

/// Layer-breakdown bench: time individual ops during one forward step
/// by instrumenting a fresh model. Gives per-op ms contribution.
pub struct LayerBreakdown {
    pub rmsnorm_ms: f64,
    pub matmul_q_ms: f64,
    pub matmul_k_ms: f64,
    pub matmul_v_ms: f64,
    pub matmul_o_ms: f64,
    pub matmul_gate_ms: f64,
    pub matmul_up_ms: f64,
    pub matmul_down_ms: f64,
    pub rope_ms: f64,
    pub attention_ms: f64,
    pub total_layer_ms: f64,
}

pub fn format_e2e(b: &E2EBench, prompt_tokens: usize) -> String {
    let avg = b.avg_forward_ms();
    format!(
        "{:<12}  load {:>6.0}ms  upload {:>6.0}ms  first {:>6.0}ms  avg {:>6.1}ms/tok  →  {:>5.1} tok/s",
        b.backend,
        b.load_ms,
        b.to_backend_ms,
        b.first_forward_ms,
        avg,
        1000.0 / avg,
    )
}
