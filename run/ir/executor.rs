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

        // Pre-compute last_use[tensor_name] = max node index that reads it.
        // After a node runs, any tensor whose last_use == that index is freed.
        let last_use = build_last_use(&self.graph.nodes);

        for (node_idx, node) in self.graph.nodes.iter().enumerate() {
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
                // Layout-only ops handled in the executor — no backend dispatch.
                Op::Reshape { shape } => {
                    let x = lookup(&tensors, &self.weights, &node.inputs[0])?.clone();
                    let out = exec_reshape(x, shape)?;
                    tensors.insert(node.outputs[0].clone(), out);
                }
                Op::TokenEmbed => {
                    let ids = lookup(&tensors, &self.weights, &node.inputs[0])?.clone();
                    let w   = lookup(&tensors, &self.weights, &node.inputs[1])?.clone();
                    let out = exec_token_embed(&ids, &w)?;
                    tensors.insert(node.outputs[0].clone(), out);
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

            // Free any intermediates whose last consumer just ran.
            for input_name in &node.inputs {
                if last_use.get(input_name.as_str()) == Some(&node_idx) {
                    tensors.remove(input_name);
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

/// Pre-compute `last_use[tensor_name]` = the highest node index that lists
/// the tensor as an input. Weights are not tracked (they live for the whole
/// forward). Only intermediate tensors need freeing.
fn build_last_use(nodes: &[super::graph::Node]) -> HashMap<String, usize> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        for name in &node.inputs {
            map.entry(name.clone())
                .and_modify(|prev| { if i > *prev { *prev = i; } })
                .or_insert(i);
        }
    }
    map
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

/// Pure view — reinterpret the tensor's data with a new shape.
/// Handles one `-1` inference dim. Shares the underlying data arc (zero-copy).
fn exec_reshape(x: Tensor, shape: &[i64]) -> Result<Tensor, BackendError> {
    let x_numel = x.numel();
    let mut out_shape: Vec<usize> = Vec::with_capacity(shape.len());
    let mut infer_idx: Option<usize> = None;
    let mut product = 1usize;
    for (i, &d) in shape.iter().enumerate() {
        if d == -1 {
            if infer_idx.is_some() {
                return Err(BackendError::InvalidInput {
                    op: "Reshape",
                    reason: "at most one -1 dim allowed".into(),
                });
            }
            infer_idx = Some(i);
            out_shape.push(0);
        } else if d >= 0 {
            out_shape.push(d as usize);
            product *= d as usize;
        } else {
            return Err(BackendError::InvalidInput {
                op: "Reshape",
                reason: format!("invalid dim {d}: only -1 is allowed as inferred dim"),
            });
        }
    }
    if let Some(idx) = infer_idx {
        if product == 0 || x_numel % product != 0 {
            return Err(BackendError::InvalidInput {
                op: "Reshape",
                reason: format!("cannot infer dim: {x_numel} / {product} has remainder"),
            });
        }
        out_shape[idx] = x_numel / product;
    } else if out_shape.iter().product::<usize>() != x_numel {
        return Err(BackendError::InvalidInput {
            op: "Reshape",
            reason: format!(
                "element count mismatch: shape {:?} = {} ≠ input {}",
                out_shape,
                out_shape.iter().product::<usize>(),
                x_numel
            ),
        });
    }
    Ok(Tensor { shape: out_shape, dtype: x.dtype, data: x.data })
}

/// Embedding table lookup: ids as f32 [seq], weight [vocab, hidden] → [seq, hidden].
fn exec_token_embed(ids: &Tensor, w: &Tensor) -> Result<Tensor, BackendError> {
    if ids.rank() != 1 {
        return Err(BackendError::InvalidInput {
            op: "TokenEmbed",
            reason: format!("ids must be rank 1, got rank {}", ids.rank()),
        });
    }
    if w.rank() != 2 {
        return Err(BackendError::InvalidInput {
            op: "TokenEmbed",
            reason: format!("weight must be rank 2, got {:?}", w.shape),
        });
    }
    let vocab = w.shape[0];
    let hidden = w.shape[1];
    let id_data = ids.as_f32();
    let w_data = w.as_f32();
    let seq = ids.shape[0];
    let mut out = vec![0f32; seq * hidden];
    for (s, &tok_f) in id_data.iter().enumerate() {
        let tok = tok_f as usize;
        if tok >= vocab {
            return Err(BackendError::InvalidInput {
                op: "TokenEmbed",
                reason: format!("token ID {tok} out of vocab range {vocab}"),
            });
        }
        let src = &w_data[tok * hidden..(tok + 1) * hidden];
        out[s * hidden..(s + 1) * hidden].copy_from_slice(src);
    }
    Ok(Tensor::from_f32(vec![seq, hidden], out))
}
