//! Graph executor — walks a [`Graph`] node-by-node, dispatches each op to a
//! backend, threads intermediate tensors between them.
//!
//! Handles the stateful `Op::KvCache` specially: accumulates K and V rows
//! across forward calls. All other ops are dispatched via `Backend::execute`.
//!
//! Special inputs auto-injected each forward:
//!   `"pos"` — scalar f32 Tensor holding `past_seq_len` as a float.
//!
//! Spec: specs/ir.md §"Walking the graph"

use super::graph::{Graph};
use crate::backend::{Backend, BackendError};
use crate::core::op::Op;
use crate::core::tensor::Tensor;
use std::collections::HashMap;

/// Knobs for execution. Most graph invocations can leave these at default.
#[derive(Clone, Debug, Default)]
pub struct ExecConfig {
    /// Collect per-op timing into the returned [`ExecStats`]. Free when off.
    pub profile: bool,
    /// Hard cap on per-forward intermediate memory (bytes). 0 = unlimited.
    pub max_intermediate_bytes: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ExecStats {
    pub total_ms: f64,
    pub per_op_ms: HashMap<&'static str, f64>,
}

/// Prepared, executable graph. Weights are stored as dequantized f32 tensors.
/// KV cache is maintained across `run()` calls.
pub struct GraphExecutor {
    graph: Graph,
    weights: HashMap<String, Tensor>,
    // Per-KvCache-output: accumulated K rows as flat Vec<f32>.
    kv_k: HashMap<String, Vec<f32>>,
    // Per-KvCache-output: accumulated V rows as flat Vec<f32>.
    kv_v: HashMap<String, Vec<f32>>,
    // Number of f32 values per step (= kv_heads * head_dim), fixed at first step.
    kv_row_size: HashMap<String, usize>,
    past_seq_len: usize,
    backend: Box<dyn Backend>,
    #[allow(dead_code)]
    config: ExecConfig,
}

impl GraphExecutor {
    /// Build an executor. `weights` must contain dequantized f32 tensors keyed
    /// by the same names the graph template uses (HF-convention for decoders).
    pub fn prepare(
        graph: Graph,
        weights: HashMap<String, Tensor>,
        backend: Box<dyn Backend>,
        config: ExecConfig,
    ) -> Result<Self, BackendError> {
        Ok(Self {
            graph,
            weights,
            kv_k: HashMap::new(),
            kv_v: HashMap::new(),
            kv_row_size: HashMap::new(),
            past_seq_len: 0,
            backend,
            config,
        })
    }

    /// Reset KV cache and position counter (equivalent to a new conversation).
    pub fn reset(&mut self) {
        self.kv_k.clear();
        self.kv_v.clear();
        self.kv_row_size.clear();
        self.past_seq_len = 0;
    }

    /// Run one forward. `inputs` must contain `"input_ids"` as a f32 Tensor
    /// whose values are token IDs cast to f32 (shape [seq_len]).
    ///
    /// Returns a map containing at minimum `"logits"` (shape [seq_len, vocab]).
    pub fn run(
        &mut self,
        mut tensors: HashMap<String, Tensor>,
    ) -> Result<HashMap<String, Tensor>, BackendError> {
        // Auto-inject current position so Rope nodes can consume it.
        tensors.insert(
            "pos".into(),
            Tensor::from_f32(vec![1], vec![self.past_seq_len as f32]),
        );

        for node in &self.graph.nodes {
            match &node.op {
                Op::KvCache => {
                    // Stateful: append this step's K and V to the per-layer cache.
                    // Template convention: inputs[0]=K, inputs[1]=V;
                    //                     outputs[0]=kv_k, outputs[1]=kv_v.
                    let k_in_name = node.inputs.get(0).ok_or_else(|| {
                        BackendError::Internal("KvCache: missing K input".into())
                    })?;
                    let v_in_name = node.inputs.get(1).ok_or_else(|| {
                        BackendError::Internal("KvCache: missing V input".into())
                    })?;
                    let kv_k_out = node.outputs.get(0).ok_or_else(|| {
                        BackendError::Internal("KvCache: missing kv_k output name".into())
                    })?;
                    let kv_v_out = node.outputs.get(1).ok_or_else(|| {
                        BackendError::Internal("KvCache: missing kv_v output name".into())
                    })?;

                    let k_new = lookup(&tensors, &self.weights, k_in_name)?;
                    let v_new = lookup(&tensors, &self.weights, v_in_name)?;
                    let row_size = k_new.numel();

                    self.kv_row_size.entry(kv_k_out.clone()).or_insert(row_size);

                    let k_cache = self.kv_k.entry(kv_k_out.clone()).or_default();
                    k_cache.extend_from_slice(&k_new.as_f32());
                    let v_cache = self.kv_v.entry(kv_v_out.clone()).or_default();
                    v_cache.extend_from_slice(&v_new.as_f32());

                    let seq = k_cache.len() / row_size;
                    tensors.insert(
                        kv_k_out.clone(),
                        Tensor::from_f32(vec![seq, row_size], k_cache.clone()),
                    );
                    tensors.insert(
                        kv_v_out.clone(),
                        Tensor::from_f32(vec![seq, row_size], v_cache.clone()),
                    );
                }
                op => {
                    let in_refs: Vec<&Tensor> = node
                        .inputs
                        .iter()
                        .map(|name| lookup(&tensors, &self.weights, name))
                        .collect::<Result<_, _>>()?;
                    let outs = self.backend.execute(op, &in_refs)?;
                    for (name, t) in node.outputs.iter().zip(outs.into_iter()) {
                        tensors.insert(name.clone(), t);
                    }
                }
            }
        }

        self.past_seq_len += 1;

        let mut outputs = HashMap::new();
        if let Some(logits) = tensors.remove("logits") {
            outputs.insert("logits".into(), logits);
        }
        Ok(outputs)
    }
}

fn lookup<'a>(
    intermediates: &'a HashMap<String, Tensor>,
    weights: &'a HashMap<String, Tensor>,
    name: &str,
) -> Result<&'a Tensor, BackendError> {
    intermediates
        .get(name)
        .or_else(|| weights.get(name))
        .ok_or_else(|| BackendError::Internal(format!("tensor not found: {name}")))
}
