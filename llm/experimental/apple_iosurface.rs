//! Apple Silicon zero-copy backend — unimem + acpu + aruminium + rane
//!
//! All computation happens on physically pinned IOSurface memory.
//! CPU (AMX/NEON), GPU (Metal), and ANE read/write the same DRAM.
//! Zero copies between pipeline stages.
//!
//! Memory layout (unimem::Layout):
//!   weights tape — model parameters, warm-allocated, never cleared
//!   scratch tape — per-token activations, cleared after each token
//!   history tape — KV cache, cleared per conversation

#[cfg(target_os = "macos")]
use std::collections::HashMap;

#[cfg(target_os = "macos")]
use crate::ir::{DType, Graph, WeightData};

/// Apple Silicon inference engine — zero-copy across CPU/GPU/ANE
#[cfg(target_os = "macos")]
pub struct AppleEngine {
    /// Memory layout: weights + scratch + history
    pub layout: unimem::Layout,
    /// Weight offsets in the weights tape: name → (offset, size)
    weight_map: HashMap<String, (usize, usize)>,
    /// Model config
    hidden_size: usize,
    num_heads: usize,
    kv_heads: usize,
    head_dim: usize,
    num_layers: usize,
    vocab_size: usize,
    intermediate_size: usize,
}

#[cfg(target_os = "macos")]
impl AppleEngine {
    /// Load a model from a Graph (from .cyb) into zero-copy memory.
    pub fn load(graph: Graph, config: AppleConfig) -> Result<Self, String> {
        // Calculate total weight size
        let total_weight_bytes: usize = graph.weights.values()
            .map(|w| w.data.len())
            .sum();

        // Scratch: enough for one forward pass activations
        // Rough estimate: 4 * hidden_size * max_seq_len * sizeof(f32)
        let scratch_bytes = config.hidden_size * 4096 * 4 * 4; // generous

        // History: KV cache
        // num_layers * 2 * kv_heads * head_dim * max_context * sizeof(f16)
        let history_bytes = config.num_layers * 2 * config.kv_heads * config.head_dim * 4096 * 2;

        let layout = unimem::Layout::new(total_weight_bytes, scratch_bytes, history_bytes)
            .map_err(|e| format!("unimem layout: {e}"))?;

        // Copy weights into the weights tape
        let mut weight_map = HashMap::new();

        // Sort by name for deterministic layout
        let mut names: Vec<&String> = graph.weights.keys().collect();
        names.sort();

        for name in &names {
            let w = &graph.weights[*name];
            let align = 64; // AMX alignment
            let ptr = layout.weights().take(w.data.len(), align)
                .ok_or_else(|| format!("weights tape full at {name}"))?;

            // Copy weight data into pinned memory
            unsafe {
                std::ptr::copy_nonoverlapping(w.data.as_ptr(), ptr, w.data.len());
            }

            let offset = ptr as usize - layout.weights().block().address() as usize;
            weight_map.insert(name.to_string(), (offset, w.data.len()));
        }

        log::info!(
            "Apple engine: {}MB weights in pinned memory, {}MB scratch, {}MB history",
            total_weight_bytes / 1_000_000,
            scratch_bytes / 1_000_000,
            history_bytes / 1_000_000,
        );

        Ok(Self {
            layout,
            weight_map,
            hidden_size: config.hidden_size,
            num_heads: config.num_heads,
            kv_heads: config.kv_heads,
            head_dim: config.head_dim,
            num_layers: config.num_layers,
            vocab_size: config.vocab_size,
            intermediate_size: config.intermediate_size,
        })
    }

    /// Get a weight as f32 slice (zero-copy if already f32, convert if f16)
    pub fn weight_f32(&self, name: &str) -> Option<&[f32]> {
        let (offset, size) = self.weight_map.get(name)?;
        let base = self.layout.weights().block().address();
        let ptr = unsafe { base.add(*offset) };
        let count = size / 4;
        Some(unsafe { std::slice::from_raw_parts(ptr as *const f32, count) })
    }

    /// Get raw weight bytes
    pub fn weight_raw(&self, name: &str) -> Option<&[u8]> {
        let (offset, size) = self.weight_map.get(name)?;
        let base = self.layout.weights().block().address();
        let ptr = unsafe { base.add(*offset) };
        Some(unsafe { std::slice::from_raw_parts(ptr, *size) })
    }

    /// Allocate scratch space for one tensor (cleared per token)
    pub fn scratch_f32(&self, count: usize) -> Option<&mut [f32]> {
        let bytes = count * 4;
        let ptr = self.layout.scratch().take(bytes, 64)?;
        Some(unsafe { std::slice::from_raw_parts_mut(ptr as *mut f32, count) })
    }

    /// Clear scratch tape (after each token decode)
    pub fn clear_scratch(&self) {
        self.layout.clear_pass();
    }

    /// Clear KV cache (new conversation)
    pub fn clear_history(&self) {
        self.layout.clear_talk();
    }

    /// Run one forward pass: token_id → logits
    /// Uses acpu (AMX/NEON) for all compute — zero-copy on pinned memory.
    pub fn forward(&self, _token_ids: &[u32]) -> Result<Vec<f32>, String> {
        // TODO: Full transformer forward pass using acpu ops
        // For now: skeleton showing the zero-copy data flow

        // Each layer:
        // 1. RMSNorm (NEON)
        // 2. QKV projection (AMX matmul)
        // 3. RoPE (NEON rotate)
        // 4. Attention (AMX matmul + NEON softmax)
        // 5. O projection (AMX matmul)
        // 6. RMSNorm (NEON)
        // 7. FFN gate+up (AMX matmul)
        // 8. SiLU (NEON)
        // 9. FFN down (AMX matmul)
        // 10. Residual add (NEON)

        // All on the same physical memory — zero copies.

        Err("Apple forward pass not yet implemented — use wgpu backend".into())
    }

    /// Memory statistics
    pub fn memory_stats(&self) -> String {
        let stat = self.layout.stat();
        format!(
            "weights: {}/{} MB, scratch: {}/{} MB, history: {}/{} MB",
            stat.weights_used / 1_000_000, stat.weights_total / 1_000_000,
            stat.scratch_used / 1_000_000, stat.scratch_total / 1_000_000,
            stat.history_used / 1_000_000, stat.history_total / 1_000_000,
        )
    }
}

/// Config for AppleEngine
#[cfg(target_os = "macos")]
pub struct AppleConfig {
    pub hidden_size: usize,
    pub num_heads: usize,
    pub kv_heads: usize,
    pub head_dim: usize,
    pub num_layers: usize,
    pub vocab_size: usize,
    pub intermediate_size: usize,
}
