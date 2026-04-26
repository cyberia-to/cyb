//! [`GraphRunner`] — adapts [`GraphExecutor`] to the [`ModelRunner`] interface.
//!
//! Wraps an executor with its own CPU backend so the caller can drive it
//! through the same `step(token, backend)` / `reset()` API as `LlamaModel`.
//! The `backend` argument to `step` is currently ignored — the executor was
//! prepared with its own backend at construction time.

use super::executor::{ExecConfig, GraphExecutor};
use crate::backend::{Backend, BackendError};
use crate::core::tensor::Tensor;
use crate::format::LoadedModel;
use crate::generate::ModelRunner;
use crate::ir::family_graph;
use crate::arch::decoder::LlamaConfig;
use std::collections::HashMap;

/// A [`GraphExecutor`] wrapped as a [`ModelRunner`].
pub struct GraphRunner {
    executor: GraphExecutor,
}

impl GraphRunner {
    /// Build a `GraphRunner` from a loaded model.
    ///
    /// Tries the stored `~~~graph` section first. Falls back to building the
    /// graph from `family_graph(config)` if no section is present (e.g. during
    /// testing or before `mr graph` has been run).
    ///
    /// Returns `None` if neither source is available.
    pub fn from_loaded(
        lm: &LoadedModel,
        llama_cfg: &LlamaConfig,
        backend: Box<dyn Backend>,
    ) -> Option<Result<Self, BackendError>> {
        // Source 1: stored graph section.
        let graph = if let Some(g) = lm.graph() {
            g
        } else {
            // Source 2: build from template.
            family_graph(llama_cfg)?
        };

        // Build the weight map: dequantize all tensors to f32.
        let weights = build_weight_map(lm);

        let exec = GraphExecutor::prepare(graph, weights, backend, ExecConfig::default());
        Some(exec.map(|e| Self { executor: e }))
    }
}

impl ModelRunner for GraphRunner {
    fn step(&mut self, token: u32, _backend: &dyn Backend) -> Result<Vec<f32>, BackendError> {
        let mut inputs = HashMap::new();
        inputs.insert(
            "input_ids".into(),
            Tensor::from_f32(vec![1], vec![token as f32]),
        );
        let out = self.executor.run(inputs)?;
        out.get("logits")
            .map(|t| t.as_f32().to_vec())
            .ok_or_else(|| BackendError::Internal("graph executor: no logits output".into()))
    }

    fn reset(&mut self) {
        self.executor.reset();
    }
}

/// Dequantize all model tensors to f32 and build the weight map.
///
/// This is the "correctness reference" approach: pre-dequant everything at
/// load time. Performance-sensitive production use should pass quantized
/// tensors to `backend.quant_matmul()` directly (future improvement).
///
/// For tied-embedding models (qwen3, qwen2.5 coder small variants, gemma)
/// the source has only `model.embed_tokens.weight`; the curated forward
/// path aliases lm_head to it, but the graph template references
/// `lm_head.weight` literally. Mirror that aliasing here so the graph
/// executor works for both tied and untied models.
fn build_weight_map(lm: &LoadedModel) -> HashMap<String, Tensor> {
    use crate::backend::cpu::quant::try_dequantize_to_f32;

    let mut weights = HashMap::new();
    for meta in &lm.tensors {
        let Some(bytes) = lm.tensor_bytes(&meta.name) else {
            log::warn!("weight bytes missing for {}", meta.name);
            continue;
        };
        match try_dequantize_to_f32(bytes, meta.dtype) {
            Ok(f32s) => {
                weights.insert(meta.name.clone(), Tensor::from_f32(meta.shape.clone(), f32s));
            }
            Err(e) => {
                log::warn!("dequant failed for {}: {e}", meta.name);
            }
        }
    }
    // Tied-embedding alias: when `lm_head.weight` is missing but
    // `model.embed_tokens.weight` is present, alias them. Cheap clone of
    // the f32 buffer; could be made zero-copy via Arc later.
    if !weights.contains_key("lm_head.weight") {
        if let Some(embed) = weights.get("model.embed_tokens.weight").cloned() {
            weights.insert("lm_head.weight".to_string(), embed);
        }
    }
    weights
}
