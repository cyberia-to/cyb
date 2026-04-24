//! Graph executor — walks a [`Graph`] node-by-node, dispatches each op to a
//! backend, threads intermediate tensors between them.
//!
//! **Status: port pending.** The old llm/ runtime's executor (1500 LOC of
//! wgpu-specific dispatch) needs a ground-up redesign for mr/'s multi-backend
//! trait, not a line-by-line move. This file holds the public API surface
//! (types + entry points) so downstream code can depend on it today; the
//! implementation lives in Phase C after the GPU kernel shelf is populated.
//!
//! Design we intend to keep from the old executor:
//! - Single compute pass: accumulate `Vec<DispatchCmd>` before GPU submission
//!   so the entire forward is one `begin_compute_pass / end` block.
//! - Backend selection: node's `backend_hint` wins if the backend supports the
//!   op; else default backend; else CPU reference. Never silently fail.
//! - Tensor lifecycle: free intermediates once last consumer has executed
//!   (pre-computed from [`Graph::memory_plan`]).
//! - Weight residency: upload resident weights once at `prepare()`; reuse
//!   buffers via a frame allocator.
//!
//! Spec: specs/ir.md §"Walking the graph"

use super::graph::Graph;
use crate::backend::{Backend, BackendError};
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

/// Prepared, executable graph. Weights have been uploaded to the chosen
/// backend; a per-invocation intermediate map handles the rest.
pub struct GraphExecutor {
    // Fields intentionally left unconstructed in this stub. Real ports:
    //   backend: Box<dyn Backend>,
    //   resident_weights: HashMap<String, Tensor>,
    //   frame_allocator: BufferPool,   // Phase B — port from llm/wgpu/alloc.rs
    //   node_order: Vec<usize>,
    //   config: ExecConfig,
    _phantom: std::marker::PhantomData<()>,
}

impl GraphExecutor {
    /// Build an executor from a graph + chosen backend. Uploads resident
    /// weights and validates that every node's op is supported (or has a
    /// CPU fallback).
    ///
    /// Currently returns an error marker — implementation lands with
    /// Phase C (GPU kernels) and Phase B (frame allocator) in place.
    pub fn prepare(
        _graph: &Graph,
        _backend: &dyn Backend,
        _config: ExecConfig,
    ) -> Result<Self, BackendError> {
        Err(BackendError::Internal(
            "graph executor port pending — use LlamaModel curated path instead".into(),
        ))
    }

    /// Run one forward. `inputs` maps graph-input names (e.g. "input_ids")
    /// to host tensors. Returns the graph's outputs keyed by name.
    pub fn run(
        &mut self,
        _inputs: HashMap<String, Tensor>,
    ) -> Result<HashMap<String, Tensor>, BackendError> {
        Err(BackendError::Internal("graph executor port pending".into()))
    }
}
