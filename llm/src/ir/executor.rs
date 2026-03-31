//! Graph IR executor — runs any Graph on the wgpu backend via jet dispatch
//!
//! Replaces the hardcoded forward pass with a generic execution engine.
//! For each node in topological order:
//!   1. Look up the op in the jet registry
//!   2. If jet found → dispatch fused GPU kernel
//!   3. If not found → warn (atom interpreter fallback is future work)
//!
//! All dispatches accumulate into a single compute pass for maximum throughput.

use std::collections::HashMap;
use std::sync::Arc;

use super::graph::{AttrValue, Graph, Node, TensorId};
use super::jets::JetRegistry;
use super::ops::Op;

use crate::backend::wgpu::dispatch;
use crate::backend::wgpu::pipelines::{ComputeShader, Pipelines};

/// Quantization format for GPU weight buffers
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuQuantFormat {
    F32,
    F16,
    Q4,
    Q8,
    Ternary,
}

/// A weight tensor uploaded to the GPU, with optional quantization scales
pub struct GpuWeight {
    /// Packed weight data buffer
    pub buf: wgpu::Buffer,
    /// Quantization scale factors (None for F32/F16)
    pub scales: Option<wgpu::Buffer>,
    /// Quantization format
    pub quant: GpuQuantFormat,
    /// Original shape [N, K] for matmul weights
    pub shape: Vec<usize>,
}

impl GpuWeight {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buf
    }

    /// Get scales buffer, panics if None (caller must check quant format)
    pub fn scales_buf(&self) -> &wgpu::Buffer {
        self.scales.as_ref().expect("No scales buffer for this weight")
    }

    /// N dimension (output features) — first dim of shape
    pub fn n(&self) -> u32 {
        self.shape.first().copied().unwrap_or(0) as u32
    }

    /// K dimension (input features) — second dim of shape
    pub fn k(&self) -> u32 {
        self.shape.get(1).copied().unwrap_or(0) as u32
    }
}

/// KV cache for one transformer layer
pub struct ExecKVCache {
    pub key: Option<wgpu::Buffer>,
    pub value: Option<wgpu::Buffer>,
}

/// A single dispatch command to be batched into one compute pass
struct DispatchCmd<'a> {
    shader: &'a ComputeShader,
    bg: wgpu::BindGroup,
    wg: (u32, u32, u32),
}

/// Configuration for the graph executor — carries model-level params
pub struct ExecConfig {
    pub hidden_size: u32,
    pub num_heads: u32,
    pub kv_num_heads: u32,
    pub head_dim: u32,
    pub vocab_size: u32,
    pub block_size: u32,
    pub default_quant: GpuQuantFormat,
    pub rope_theta: f32,
    pub has_qk_norm: bool,
}

/// Graph IR executor — runs a Graph on wgpu via jet dispatch
///
/// Usage:
/// ```ignore
/// let executor = GraphExecutor::new(pipelines.clone(), config);
/// let logits = executor.execute_decode(
///     &graph, &[token_id], &weights, &mut kv_cache, past_seq_len,
///     &cos_cache, &sin_cache,
/// );
/// ```
pub struct GraphExecutor {
    pipelines: Arc<Pipelines>,
    jet_registry: JetRegistry,
    config: ExecConfig,
}

impl GraphExecutor {
    /// Create a new executor with the given GPU pipelines and model config
    pub fn new(pipelines: Arc<Pipelines>, config: ExecConfig) -> Self {
        Self {
            pipelines,
            jet_registry: JetRegistry::new(),
            config,
        }
    }

    /// Execute a decode step (seq_len=1) on the graph.
    ///
    /// This is the hot path for autoregressive generation.
    /// All ops are prepared as dispatch commands, then executed in a single
    /// compute pass for maximum GPU utilization.
    ///
    /// Returns logits as Vec<f32> of size vocab_size.
    pub fn execute_decode(
        &self,
        graph: &Graph,
        token_ids: &[u32],
        weights: &HashMap<String, GpuWeight>,
        kv_cache: &mut Vec<ExecKVCache>,
        past_seq_len: usize,
        cos_cache: &[f32],
        sin_cache: &[f32],
    ) -> Vec<f32> {
        let p = &self.pipelines;
        let cfg = &self.config;
        p.begin_frame();

        let seq_len = token_ids.len();
        let total_seq = past_seq_len + seq_len;

        let mut enc = p.device.create_command_encoder(&Default::default());

        // Upload token IDs
        let ids_f32: Vec<f32> = token_ids.iter().map(|&id| id as f32).collect();
        let ids_buf = p.upload_f32(&ids_f32);

        // Upload RoPE cos/sin for current positions
        let half = (cfg.head_dim / 2) as usize;
        let start = past_seq_len * half;
        let end = start + seq_len * half;
        let cos_buf = p.upload_f32(&cos_cache[start..end]);
        let sin_buf = p.upload_f32(&sin_cache[start..end]);

        // Buffer map: tensor_id -> GPU buffer
        let mut buffers: HashMap<TensorId, wgpu::Buffer> = HashMap::new();
        buffers.insert("input_ids".to_string(), ids_buf);

        // Dispatch command list — all accumulated, then executed in one pass
        let mut all_dispatches: Vec<DispatchCmd> = Vec::with_capacity(graph.nodes.len() * 2);

        // Layer counter for KV cache indexing
        let mut layer_idx: usize = 0;

        // Execute nodes in order (graph must be topologically sorted)
        for node in &graph.nodes {
            self.prepare_node(
                node,
                graph,
                weights,
                &mut buffers,
                &mut all_dispatches,
                kv_cache,
                &mut layer_idx,
                past_seq_len,
                total_seq,
                seq_len,
                &cos_buf,
                &sin_buf,
            );
        }

        // Single compute pass for everything
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            for cmd in &all_dispatches {
                p.dispatch_in_pass(&mut pass, cmd.shader, &cmd.bg, cmd.wg);
            }
        }

        p.queue.submit(std::iter::once(enc.finish()));

        // Read logits from output buffer
        let logits_buf = buffers
            .get("logits")
            .expect("Graph must produce 'logits' tensor");
        p.read_f32(logits_buf, cfg.vocab_size as usize)
    }

    /// Execute an encoder forward pass (BERT, Whisper encoder).
    ///
    /// No KV cache, no RoPE. Processes a full sequence at once.
    /// Reads output from `output_tensor_name` (e.g. "pooler_output" or "enc.output").
    ///
    /// `input_bufs` maps tensor names to pre-uploaded GPU buffers.
    /// For BERT: "input_ids", "position_ids", "token_type_ids"
    /// For Whisper encoder: "audio_input"
    pub fn execute_encode(
        &self,
        graph: &Graph,
        input_bufs: HashMap<String, wgpu::Buffer>,
        weights: &HashMap<String, GpuWeight>,
        output_tensor_name: &str,
        output_size: usize,
    ) -> Vec<f32> {
        self.execute_encode_with_seq_len(graph, input_bufs, weights, output_tensor_name, output_size, 1)
    }

    /// Execute encoder with explicit sequence length (for multi-token BERT inputs).
    pub fn execute_encode_with_seq_len(
        &self,
        graph: &Graph,
        input_bufs: HashMap<String, wgpu::Buffer>,
        weights: &HashMap<String, GpuWeight>,
        output_tensor_name: &str,
        output_size: usize,
        seq_len: usize,
    ) -> Vec<f32> {
        let p = &self.pipelines;
        p.begin_frame();

        let mut buffers: HashMap<TensorId, wgpu::Buffer> = input_bufs;

        // Dummy KV cache and RoPE (not used by encoder, but prepare_node expects them)
        let mut kv_cache: Vec<ExecKVCache> = Vec::new();
        let mut layer_idx: usize = 0;
        let dummy_cos = p.alloc(4);
        let dummy_sin = p.alloc(4);

        let mut all_dispatches: Vec<DispatchCmd> = Vec::with_capacity(graph.nodes.len() * 2);

        let dispatches_before = all_dispatches.len();
        for node in &graph.nodes {
            let before = all_dispatches.len();
            self.prepare_node(
                node,
                graph,
                weights,
                &mut buffers,
                &mut all_dispatches,
                &mut kv_cache,
                &mut layer_idx,
                0,              // past_seq_len
                seq_len,        // total_seq
                seq_len,        // seq_len
                &dummy_cos,
                &dummy_sin,
            );
            let produced = all_dispatches.len() - before;
            let has_output = node.outputs.iter().any(|o| buffers.contains_key(o));
            if produced == 0 && !has_output {
                log::warn!("Node {} ({:?}) produced no dispatch and no output. inputs: {:?}", node.id, node.op, node.inputs);
            }
        }
        log::info!("Encoder: {} dispatches from {} nodes", all_dispatches.len() - dispatches_before, graph.nodes.len());

        // Single compute pass
        let mut enc = p.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            for cmd in &all_dispatches {
                p.dispatch_in_pass(&mut pass, cmd.shader, &cmd.bg, cmd.wg);
            }
        }
        p.queue.submit(std::iter::once(enc.finish()));

        // Debug: find first NaN layer
        if std::env::var("CYB_DEBUG_ENCODER").is_ok() {
            for i in 0..64 {
                for suffix in &["q_out", "q_biased", "k_biased", "v_biased", "attn_out", "attn_dense", "attn_dense_biased", "residual1", "attn_ln", "inter_out", "inter_biased", "inter_act", "output_dense", "output_biased", "residual2", "output_ln"] {
                    let name = format!("layer_{i}.{suffix}");
                    if let Some(buf) = buffers.get(&name) {
                        let vals = p.read_f32(buf, 8.min(output_size));
                        let has_nan = vals.iter().any(|v| v.is_nan());
                        if has_nan || *suffix == "output_ln" || i == 8 {
                            eprintln!("  {name} = [{:.4}, {:.4}...] nan={has_nan}",
                                vals.get(0).unwrap_or(&0.0), vals.get(1).unwrap_or(&0.0));
                        }
                        if has_nan { break; } // stop at first NaN
                    }
                }
            }
        }

        // Read output
        let output_buf = buffers
            .get(output_tensor_name)
            .or_else(|| buffers.get("logits"))
            .unwrap_or_else(|| {
                // Debug: list available buffers
                let keys: Vec<_> = buffers.keys().collect();
                log::error!("Output '{}' not found. Available: {:?}", output_tensor_name, &keys[..keys.len().min(20)]);
                // Try last buffer
                buffers.values().last()
                    .expect("No buffers at all")
            });
        p.read_f32(output_buf, output_size)
    }

    /// Prepare a single node for dispatch.
    ///
    /// Maps each IR op to the concrete wgpu dispatch function,
    /// looking up input buffers from the buffer map and inserting
    /// output buffers back.
    #[allow(clippy::too_many_arguments)]
    fn prepare_node<'a>(
        &'a self,
        node: &Node,
        _graph: &Graph,
        weights: &HashMap<String, GpuWeight>,
        buffers: &mut HashMap<TensorId, wgpu::Buffer>,
        dispatches: &mut Vec<DispatchCmd<'a>>,
        kv_cache: &mut Vec<ExecKVCache>,
        layer_idx: &mut usize,
        past_seq_len: usize,
        total_seq: usize,
        seq_len: usize,
        cos_buf: &wgpu::Buffer,
        sin_buf: &wgpu::Buffer,
    ) {
        let p = &self.pipelines;
        let cfg = &self.config;

        // Skip nodes whose inputs aren't available yet.
        // This happens when running a partial graph (e.g., encoder-only from
        // an encoder-decoder graph — decoder nodes have no inputs available).
        for input_id in &node.inputs {
            if !buffers.contains_key(input_id) && !weights.contains_key(input_id) {
                log::debug!("Skipping node {} ({}): input '{}' not available",
                    node.id, node.op.name(), input_id);
                return;
            }
        }

        match &node.op {
            // ============================================================
            // Token embedding
            // ============================================================
            Op::TokenEmbed => {
                let table_id = &node.inputs[0];
                let ids_id = if node.inputs.len() > 1 {
                    &node.inputs[1]
                } else {
                    // Template uses single input (input_ids), table is a weight
                    table_id
                };

                // Determine table and ids buffers
                let (table, ids) = if node.inputs.len() >= 2 {
                    // inputs[0] = input_ids, inputs[1] = embed_table (unusual)
                    // Check which is the weight
                    if let Some(w) = weights.get(&node.inputs[1]) {
                        (w.buffer(), buffers.get(&node.inputs[0]).unwrap() as &wgpu::Buffer)
                    } else if let Some(w) = weights.get(&node.inputs[0]) {
                        (w.buffer(), buffers.get(&node.inputs[1]).unwrap() as &wgpu::Buffer)
                    } else {
                        log::warn!("TokenEmbed: no weight found in inputs");
                        return;
                    }
                } else {
                    // Single input "input_ids" — find embed table from weights
                    // Try attr "embed_weight" first, then common names
                    let embed_name = node.attrs.get("embed_weight")
                        .and_then(|v| match v {
                            AttrValue::String(s) => Some(s.as_str()),
                            _ => None,
                        });

                    let embed_table = if let Some(name) = embed_name {
                        weights.get(name)
                    } else {
                        None
                    }
                    .or_else(|| weights.get("model.embed_tokens.weight"))
                    .or_else(|| weights.get("decoder.token_embedding.weight"))
                    .or_else(|| weights.get("embeddings.word_embeddings.weight"))
                    .or_else(|| weights.get("transformer.wte.weight"))
                    .or_else(|| weights.get("embed_tokens.weight"))
                    .or_else(|| weights.get("wte.weight"));

                    if let (Some(table), Some(ids_buf)) = (embed_table, buffers.get(ids_id)) {
                        (
                            table.buffer(),
                            ids_buf as &wgpu::Buffer,
                        )
                    } else {
                        log::debug!("TokenEmbed: skipping node {} (missing weight or input)", node.id);
                        return;
                    }
                };

                let (out, bg, wg) =
                    dispatch::embed_prepare(p, table, ids, cfg.hidden_size, seq_len as u32);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.embed,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // RMS normalization
            // ============================================================
            Op::RmsNorm { eps } => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let weight_name = &node.inputs[1];
                let weight_buf = resolve_buf(weight_name, buffers, weights);

                let positions = attr_u32(node, "positions").unwrap_or(seq_len as u32);
                let hidden = attr_u32(node, "hidden").unwrap_or(cfg.hidden_size);

                let params = precompute_rms_norm(p, positions, hidden, *eps);
                let (out, bg, wg) = dispatch::rms_norm_prepare_precomputed(
                    p,
                    input,
                    weight_buf,
                    &params.0,
                    positions,
                    hidden,
                    params.1,
                );
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.rms_norm,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // Matrix multiply (dispatch varies by quant format)
            // ============================================================
            Op::Matmul => {
                let activation_id = &node.inputs[0];
                let weight_id = &node.inputs[1];

                let activation = resolve_buf(activation_id, buffers, weights);

                // Weight can be a named GpuWeight or an intermediate buffer
                if let Some(w) = weights.get(weight_id) {
                    let n = w.n();
                    let k = w.k();
                    let quant = gpu_quant_to_dispatch(w.quant);

                    let (params_buf, wg) = precompute_matmul(p, n, k, cfg.block_size, w.quant);
                    let dummy_scales = p.alloc(4);
                    let scales = w.scales.as_ref().unwrap_or(&dummy_scales);

                    let (out, bg, wg, shader) = dispatch::prepare_matmul_for_quant(
                        p, activation, &w.buf, scales, &params_buf, n, wg, quant,
                    );
                    buffers.insert(node.outputs[0].clone(), out);
                    dispatches.push(DispatchCmd { shader, bg, wg });
                } else if let Some(weight_buf) = buffers.get(weight_id) {
                    // Intermediate buffer (e.g., tied lm_head = embed_table)
                    let n = attr_u32(node, "n").unwrap_or(cfg.vocab_size);
                    let k = attr_u32(node, "k").unwrap_or(cfg.hidden_size);
                    let (params_buf, wg) = precompute_f32_matmul(p, n, k);
                    let (out, bg, wg) = dispatch::f32_matmul_prepare_precomputed(
                        p,
                        activation,
                        weight_buf,
                        &params_buf,
                        n,
                        wg,
                    );
                    buffers.insert(node.outputs[0].clone(), out);
                    dispatches.push(DispatchCmd {
                        shader: &p.f32_matmul,
                        bg,
                        wg,
                    });
                } else {
                    log::warn!(
                        "Matmul node {}: weight '{}' not found in weights or buffers",
                        node.id,
                        weight_id
                    );
                }
            }

            // ============================================================
            // RoPE (rotary position embedding)
            // ============================================================
            Op::Rope { head_dim: hd, .. } => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let total_elements = attr_u32(node, "total_elements")
                    .unwrap_or_else(|| {
                        // Infer from context: for Q it's num_heads * head_dim,
                        // for K it's kv_num_heads * head_dim
                        let n_from_attr = attr_u32(node, "n");
                        n_from_attr.unwrap_or(cfg.num_heads * cfg.head_dim)
                    });

                let (params_buf, wg) =
                    precompute_rope(p, total_elements, *hd, seq_len as u32);
                let (out, bg, wg) = dispatch::rope_prepare_precomputed(
                    p, input, cos_buf, sin_buf, &params_buf, total_elements, wg,
                );
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.rope,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // KV cache append
            // ============================================================
            Op::KvCache => {
                // Inputs: [new_k_or_v]
                // The graph has a KvCache node for the key side; we handle
                // both key and value here since the template places them
                // together. We use the layer_idx to index into the cache.
                //
                // Convention: first KvCache in a layer = key, second = value.
                // But the template has a single KvCache node per layer that
                // handles both Q/K split. We handle the common pattern:
                // input is the rope'd tensor, we need to split Q from K
                // and cache K + V separately.
                //
                // For the executor, KvCache nodes are bookkeeping. The actual
                // KV append is handled when we encounter SDPA nodes, because
                // we need both k_roped and v_buf at that point.
                //
                // For now, just pass through the input to output.
                let input_id = &node.inputs[0];
                if let Some(buf) = buffers.get(input_id) {
                    buffers.insert(node.outputs[0].clone(), buf.clone());
                }
            }

            // ============================================================
            // Scaled dot-product attention (with KV cache)
            // ============================================================
            Op::Sdpa {
                num_heads,
                kv_heads,
                head_dim: hd,
                causal,
            } => {
                // inputs[0] = q, inputs[1] = k
                let q_buf = resolve_buf(&node.inputs[0], buffers, weights);
                let kv_id = &node.inputs[1];

                // ---- Encoder attention (no KV cache, full sequence) ----
                if !causal && kv_cache.is_empty() {
                    let k_buf = resolve_buf(kv_id, buffers, weights);
                    let empty_buf = p.alloc(4);
                    let v_buf_id = node.attrs.get("v_tensor")
                        .and_then(|v| match v {
                            AttrValue::String(s) => Some(s.clone()),
                            _ => None,
                        });
                    let v_buf = if let Some(ref vid) = v_buf_id {
                        resolve_buf(vid, buffers, weights)
                    } else {
                        &empty_buf
                    };

                    let enc_seq = attr_u32(node, "positions")
                        .or_else(|| attr_u32(node, "seq_len"))
                        .unwrap_or_else(|| {
                            // Infer from Q buffer: Q = [num_heads * seq * head_dim]
                            let buf_size = q_buf.size() as u32;
                            buf_size / (4 * *num_heads * *hd)
                        });

                    let scale = 1.0 / (*hd as f32).sqrt();
                    let (attn_out, attn_bg, attn_wg) = dispatch::attention_encode_prepare(
                        p, q_buf, k_buf, v_buf, *num_heads, *hd, enc_seq, scale,
                    );
                    dispatches.push(DispatchCmd {
                        shader: &p.attention_encode,
                        bg: attn_bg,
                        wg: attn_wg,
                    });
                    buffers.insert(node.outputs[0].clone(), attn_out);
                    *layer_idx += 1;
                    return;
                }

                // ---- Decoder attention (with KV cache) ----
                let li = *layer_idx;

                let empty_buf = p.alloc(4);
                let past_k_ref = kv_cache[li]
                    .key
                    .as_ref()
                    .unwrap_or(&empty_buf);
                let past_v_ref = kv_cache[li]
                    .value
                    .as_ref()
                    .unwrap_or(&empty_buf);

                // The k_roped and v_buf should be in the buffer map
                // from the layer's K and V matmul + rope nodes.
                // Look for them by convention names.
                let k_roped = buffers.get(kv_id).unwrap_or(&empty_buf);
                let v_buf_id = node
                    .attrs
                    .get("v_tensor")
                    .and_then(|v| match v {
                        AttrValue::String(s) => Some(s.clone()),
                        _ => None,
                    });
                let v_buf = if let Some(ref vid) = v_buf_id {
                    buffers.get(vid).unwrap_or(&empty_buf)
                } else {
                    &empty_buf
                };

                let past_seq_u32 = past_seq_len as u32;

                // KV append for key
                let (full_k, kv_k_bg, kv_k_wg) = dispatch::kv_append_prepare_permanent(
                    p,
                    past_k_ref,
                    k_roped,
                    *kv_heads,
                    *hd,
                    past_seq_u32,
                    seq_len as u32,
                );
                dispatches.push(DispatchCmd {
                    shader: &p.kv_append,
                    bg: kv_k_bg,
                    wg: kv_k_wg,
                });

                // KV append for value
                let (full_v, kv_v_bg, kv_v_wg) = dispatch::kv_append_prepare_permanent(
                    p,
                    past_v_ref,
                    v_buf,
                    *kv_heads,
                    *hd,
                    past_seq_u32,
                    seq_len as u32,
                );
                dispatches.push(DispatchCmd {
                    shader: &p.kv_append,
                    bg: kv_v_bg,
                    wg: kv_v_wg,
                });

                // Update KV cache
                kv_cache[li].key = Some(full_k.clone());
                kv_cache[li].value = Some(full_v.clone());

                // GQA head expansion if needed
                let (attn_k, attn_v) = if *num_heads != *kv_heads {
                    let (ek, ekb, ekw) = dispatch::kv_expand_prepare(
                        p,
                        &full_k,
                        *kv_heads,
                        *num_heads,
                        *hd,
                        total_seq as u32,
                    );
                    dispatches.push(DispatchCmd {
                        shader: &p.kv_expand,
                        bg: ekb,
                        wg: ekw,
                    });
                    let (ev, evb, evw) = dispatch::kv_expand_prepare(
                        p,
                        &full_v,
                        *kv_heads,
                        *num_heads,
                        *hd,
                        total_seq as u32,
                    );
                    dispatches.push(DispatchCmd {
                        shader: &p.kv_expand,
                        bg: evb,
                        wg: evw,
                    });
                    (ek, ev)
                } else {
                    (full_k, full_v)
                };

                let scale = 1.0 / (*hd as f32).sqrt();
                let (attn_out, attn_bg, attn_wg) = dispatch::attention_decode_prepare(
                    p,
                    q_buf,
                    &attn_k,
                    &attn_v,
                    *num_heads,
                    *hd,
                    total_seq as u32,
                    scale,
                );
                dispatches.push(DispatchCmd {
                    shader: &p.attention,
                    bg: attn_bg,
                    wg: attn_wg,
                });

                buffers.insert(node.outputs[0].clone(), attn_out);
                *layer_idx += 1;
            }

            // ============================================================
            // Element-wise add
            // ============================================================
            Op::Add => {
                let a = resolve_buf(&node.inputs[0], buffers, weights);
                let b = resolve_buf(&node.inputs[1], buffers, weights);
                // Infer element count from buffer sizes (handles multi-token sequences)
                let n_from_buf = (a.size().min(b.size()) / 4) as u32;
                let n = if n_from_buf > 0 && seq_len > 1 { n_from_buf } else {
                    attr_u32(node, "n").unwrap_or(cfg.hidden_size)
                };

                let (out, bg, wg) = dispatch::add_prepare(p, a, b, n);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.add,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // Element-wise multiply
            // ============================================================
            Op::Mul => {
                let a = resolve_buf(&node.inputs[0], buffers, weights);
                let b = resolve_buf(&node.inputs[1], buffers, weights);
                let n_from_buf = (a.size().min(b.size()) / 4) as u32;
                let n = if n_from_buf > 0 && seq_len > 1 { n_from_buf } else {
                    attr_u32(node, "n").unwrap_or(cfg.hidden_size)
                };

                let (out, bg, wg) = dispatch::add_prepare(p, a, b, n);
                let out2 = p.alloc((n as u64) * 4);
                let bg2 = p.create_bind_group(
                    &p.mul,
                    &[
                        a.as_entire_binding(),
                        b.as_entire_binding(),
                        out2.as_entire_binding(),
                    ],
                );
                // Drop the add-allocated buffer
                drop(out);
                drop(bg);
                buffers.insert(node.outputs[0].clone(), out2);
                dispatches.push(DispatchCmd {
                    shader: &p.mul,
                    bg: bg2,
                    wg,
                });
            }

            // ============================================================
            // SiLU activation
            // ============================================================
            Op::Silu => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let n = infer_n(node, input, seq_len, cfg.hidden_size);

                // Standalone SiLU — use silu_mul with gate=input, up=ones
                // For SwiGLU pattern, the template should use SwiGlu op or
                // the Silu+Mul pair is handled separately.
                let (out, bg, wg) = dispatch::silu_mul_prepare(p, input, input, n);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.silu_mul,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // SwiGLU: silu(gate) * up — fused
            // ============================================================
            Op::SwiGlu => {
                let gate = resolve_buf(&node.inputs[0], buffers, weights);
                let up = resolve_buf(&node.inputs[1], buffers, weights);
                let n = infer_n(node, resolve_buf(&node.inputs[0], buffers, weights), seq_len, cfg.hidden_size);

                let (out, bg, wg) = dispatch::silu_mul_prepare(p, gate, up, n);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.silu_mul,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // GELU activation
            // ============================================================
            Op::Gelu { .. } => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let n = infer_n(node, resolve_buf(&node.inputs[0], buffers, weights), seq_len, cfg.hidden_size);
                let out = p.alloc((n as u64) * 4);
                let bg = p.create_bind_group(
                    &p.gelu,
                    &[input.as_entire_binding(), out.as_entire_binding()],
                );
                let wg = ((n + 255) / 256, 1, 1);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.gelu,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // Argmax (greedy decoding)
            // ============================================================
            Op::Argmax => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let n = attr_u32(node, "n").unwrap_or(cfg.vocab_size);
                let (params_buf, wg) = precompute_argmax(p, n);
                let (out, bg, wg) = dispatch::argmax_gpu_prepare_precomputed(
                    p, input, &params_buf, wg,
                );
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.argmax,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // LayerNorm
            // ============================================================
            Op::LayerNorm { eps } => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let weight_buf = resolve_buf(&node.inputs[1], buffers, weights);
                let bias_buf = if node.inputs.len() > 2 {
                    Some(resolve_buf(&node.inputs[2], buffers, weights))
                } else {
                    None
                };
                let positions = attr_u32(node, "positions").unwrap_or(seq_len as u32);
                let hidden = attr_u32(node, "hidden").unwrap_or(cfg.hidden_size);

                // Use layernorm shader: input, weight, bias, output, params
                let out = p.alloc((positions as u64) * (hidden as u64) * 4);

                #[repr(C)]
                #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
                struct LNParams {
                    hidden: u32,
                    eps: f32,
                }

                let params_buf = p.upload_uniform(bytemuck::bytes_of(&LNParams {
                    hidden,
                    eps: *eps,
                }));

                let empty_bias = p.alloc(4);
                let bias = bias_buf.unwrap_or(&empty_bias);

                let bg = p.create_bind_group(
                    &p.layernorm,
                    &[
                        input.as_entire_binding(),
                        weight_buf.as_entire_binding(),
                        bias.as_entire_binding(),
                        out.as_entire_binding(),
                        params_buf.as_entire_binding(),
                    ],
                );
                let wg = (positions, 1, 1);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.layernorm,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // Softmax
            // ============================================================
            Op::Softmax { .. } => {
                // Softmax is typically fused into attention.
                // For standalone use, pass through for now.
                let input_id = &node.inputs[0];
                if let Some(buf) = buffers.get(input_id) {
                    buffers.insert(node.outputs[0].clone(), buf.clone());
                }
                log::warn!("Standalone Softmax not yet implemented in executor, passing through");
            }

            // ============================================================
            // Sub, Div, Relu, Sigmoid, Tanh
            // ============================================================
            Op::Sub => {
                let a = resolve_buf(&node.inputs[0], buffers, weights);
                let b = resolve_buf(&node.inputs[1], buffers, weights);
                let n = infer_n(node, resolve_buf(&node.inputs[0], buffers, weights), seq_len, cfg.hidden_size);
                let out = p.alloc((n as u64) * 4);
                let bg = p.create_bind_group(
                    &p.sub,
                    &[
                        a.as_entire_binding(),
                        b.as_entire_binding(),
                        out.as_entire_binding(),
                    ],
                );
                let wg = ((n + 255) / 256, 1, 1);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.sub,
                    bg,
                    wg,
                });
            }

            Op::Relu => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let n = infer_n(node, resolve_buf(&node.inputs[0], buffers, weights), seq_len, cfg.hidden_size);
                let out = p.alloc((n as u64) * 4);
                let bg = p.create_bind_group(
                    &p.relu,
                    &[input.as_entire_binding(), out.as_entire_binding()],
                );
                let wg = ((n + 255) / 256, 1, 1);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.relu,
                    bg,
                    wg,
                });
            }

            Op::Sigmoid => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let n = infer_n(node, resolve_buf(&node.inputs[0], buffers, weights), seq_len, cfg.hidden_size);
                let out = p.alloc((n as u64) * 4);
                let bg = p.create_bind_group(
                    &p.sigmoid,
                    &[input.as_entire_binding(), out.as_entire_binding()],
                );
                let wg = ((n + 255) / 256, 1, 1);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.sigmoid,
                    bg,
                    wg,
                });
            }

            Op::Tanh => {
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let n = infer_n(node, resolve_buf(&node.inputs[0], buffers, weights), seq_len, cfg.hidden_size);
                let out = p.alloc((n as u64) * 4);
                let bg = p.create_bind_group(
                    &p.tanh_act,
                    &[input.as_entire_binding(), out.as_entire_binding()],
                );
                let wg = ((n + 255) / 256, 1, 1);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.tanh_act,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // Layout ops (zero compute, passthrough)
            // ============================================================
            Op::Reshape { .. } | Op::Transpose { .. } | Op::Permute { .. } => {
                // Layout-only ops: the buffer is the same, just reinterpret shape.
                // In single-token decode, these are effectively no-ops.
                if let Some(buf) = buffers.get(&node.inputs[0]) {
                    buffers.insert(node.outputs[0].clone(), buf.clone());
                }
            }

            // ============================================================
            // Position embedding
            // ============================================================
            Op::PosEmbed => {
                // Position embedding lookup — same as TokenEmbed but for position IDs
                let pos_ids = buffers.get(&node.inputs[0]);

                // Find position embedding weight table
                let embed_name = node.attrs.get("embed_weight")
                    .and_then(|v| match v {
                        AttrValue::String(s) => Some(s.as_str()),
                        _ => None,
                    });
                let pos_table = if let Some(name) = embed_name {
                    weights.get(name)
                } else {
                    None
                }
                .or_else(|| weights.get("embeddings.position_embeddings.weight"))
                .or_else(|| weights.get("roberta.embeddings.position_embeddings.weight"))
                .or_else(|| weights.get("deberta.embeddings.position_embeddings.weight"));

                if let (Some(table), Some(ids)) = (pos_table, pos_ids) {
                    let (out, bg, wg) =
                        dispatch::embed_prepare(p, table.buffer(), ids, cfg.hidden_size, seq_len as u32);
                    buffers.insert(node.outputs[0].clone(), out);
                    dispatches.push(DispatchCmd {
                        shader: &p.embed,
                        bg,
                        wg,
                    });
                } else {
                    // No position embeddings (some models use RoPE or sinusoidal)
                    // Pass through position_ids as-is
                    if let Some(buf) = buffers.get(&node.inputs[0]) {
                        buffers.insert(node.outputs[0].clone(), buf.clone());
                    }
                }
            }

            // ============================================================
            // Cross-attention (encoder-decoder models)
            // ============================================================
            Op::SdpaCross {
                num_heads,
                head_dim: hd,
            } => {
                let q_buf = resolve_buf(&node.inputs[0], buffers, weights);
                let kv_buf = resolve_buf(&node.inputs[1], buffers, weights);
                // Cross-attention uses the encoder output as K/V
                let scale = 1.0 / (*hd as f32).sqrt();
                let (attn_out, attn_bg, attn_wg) = dispatch::attention_decode_prepare(
                    p,
                    q_buf,
                    kv_buf,
                    kv_buf, // K=V for simple cross-attention
                    *num_heads,
                    *hd,
                    total_seq as u32,
                    scale,
                );
                dispatches.push(DispatchCmd {
                    shader: &p.attention,
                    bg: attn_bg,
                    wg: attn_wg,
                });
                buffers.insert(node.outputs[0].clone(), attn_out);
            }

            // ============================================================
            // GeGLU
            // ============================================================
            Op::GeGlu => {
                let gate = resolve_buf(&node.inputs[0], buffers, weights);
                let up = resolve_buf(&node.inputs[1], buffers, weights);
                let n = infer_n(node, resolve_buf(&node.inputs[0], buffers, weights), seq_len, cfg.hidden_size);
                let out = p.alloc((n as u64) * 4);
                let bg = p.create_bind_group(
                    &p.geglu,
                    &[
                        gate.as_entire_binding(),
                        up.as_entire_binding(),
                        out.as_entire_binding(),
                    ],
                );
                let wg = ((n + 255) / 256, 1, 1);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.geglu,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // Fused ops
            // ============================================================
            Op::FusedSkipNorm { eps } => {
                // skip + rms_norm: inputs[0] = residual, inputs[1] = hidden, inputs[2] = weight
                let residual = resolve_buf(&node.inputs[0], buffers, weights);
                let hidden_buf = resolve_buf(&node.inputs[1], buffers, weights);
                let weight_buf = resolve_buf(&node.inputs[2], buffers, weights);
                let hidden = attr_u32(node, "hidden").unwrap_or(cfg.hidden_size);

                let out = p.alloc((hidden as u64) * 4);

                #[repr(C)]
                #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
                struct FusedSkipNormParams {
                    hidden: u32,
                    eps: f32,
                }

                let params_buf = p.upload_uniform(bytemuck::bytes_of(
                    &FusedSkipNormParams { hidden, eps: *eps },
                ));

                let bg = p.create_bind_group(
                    &p.fused_skip_norm,
                    &[
                        residual.as_entire_binding(),
                        hidden_buf.as_entire_binding(),
                        weight_buf.as_entire_binding(),
                        out.as_entire_binding(),
                        params_buf.as_entire_binding(),
                    ],
                );
                let wg = (1, 1, 1);
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.fused_skip_norm,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // Conv1d — 1D convolution (Whisper audio encoder, TTS)
            // ============================================================
            Op::Conv1d {
                kernel,
                stride,
                padding,
                groups,
            } => {
                // inputs[0] = input, inputs[1] = weight, inputs[2] = bias
                let input = resolve_buf(&node.inputs[0], buffers, weights);
                let weight_buf = resolve_buf(&node.inputs[1], buffers, weights);
                let bias_buf = if node.inputs.len() > 2 {
                    resolve_buf(&node.inputs[2], buffers, weights)
                } else {
                    &p.alloc(4) // dummy zero bias
                };

                let in_channels = attr_u32(node, "in_channels").unwrap_or(cfg.hidden_size);
                let out_channels = attr_u32(node, "out_channels").unwrap_or(cfg.hidden_size);
                let in_length = attr_u32(node, "in_length").unwrap_or_else(|| {
                    // Infer from input buffer size: buffer_bytes / (4 * in_channels)
                    let buf_size = input.size() as u32;
                    buf_size / (4 * in_channels)
                });
                let batch_size = attr_u32(node, "batch_size").unwrap_or(1);

                let (out, bg, wg) = dispatch::conv1d_prepare(
                    p,
                    input,
                    weight_buf,
                    bias_buf,
                    batch_size,
                    in_channels,
                    out_channels,
                    in_length,
                    *kernel,
                    *stride,
                    *padding,
                    *groups,
                );
                buffers.insert(node.outputs[0].clone(), out);
                dispatches.push(DispatchCmd {
                    shader: &p.conv1d,
                    bg,
                    wg,
                });
            }

            // ============================================================
            // Unhandled ops — log warning
            // ============================================================
            _ => {
                // Check jet registry to see if this op is recognized
                let jet = self.jet_registry.lookup_op(&node.op);
                if jet.is_some() {
                    log::warn!(
                        "Op '{}' has a jet but no executor dispatch yet (node {})",
                        node.op.name(),
                        node.id
                    );
                } else if !node.op.is_layout_only() {
                    log::warn!(
                        "Unhandled op '{}' in graph executor (node {})",
                        node.op.name(),
                        node.id
                    );
                }
                // Pass through first input to first output if available
                if !node.inputs.is_empty() && !node.outputs.is_empty() {
                    if let Some(buf) = buffers.get(&node.inputs[0]) {
                        buffers.insert(node.outputs[0].clone(), buf.clone());
                    }
                }
            }
        }
    }
}

// ========================================================================
// Helper functions
// ========================================================================

/// Resolve a tensor ID to a GPU buffer — checks intermediate buffers first,
/// then weight buffers
fn resolve_buf<'a>(
    id: &str,
    buffers: &'a HashMap<TensorId, wgpu::Buffer>,
    weights: &'a HashMap<String, GpuWeight>,
) -> &'a wgpu::Buffer {
    if let Some(buf) = buffers.get(id) {
        return buf;
    }
    if let Some(w) = weights.get(id) {
        return &w.buf;
    }
    panic!(
        "Tensor '{}' not found in buffers or weights. \
         Check that the graph is topologically sorted and all weights are loaded.",
        id
    );
}

/// Infer element count `n` for an elementwise op from input buffer sizes.
/// For multi-token encoder passes (seq_len > 1), the buffer may be larger
/// than the template's `n` attr (which assumes single-token).
fn infer_n(node: &Node, buf: &wgpu::Buffer, seq_len: usize, default: u32) -> u32 {
    if seq_len > 1 {
        let buf_n = (buf.size() / 4) as u32;
        if buf_n > 0 { return buf_n; }
    }
    attr_u32(node, "n").unwrap_or(default)
}

/// Get a u32 attribute from a node
fn attr_u32(node: &Node, key: &str) -> Option<u32> {
    node.attrs.get(key).and_then(|v| match v {
        AttrValue::Int(i) => Some(*i as u32),
        AttrValue::Float(f) => Some(*f as u32),
        _ => None,
    })
}

/// Convert GpuQuantFormat to dispatch::QuantFormat
fn gpu_quant_to_dispatch(q: GpuQuantFormat) -> dispatch::QuantFormat {
    match q {
        GpuQuantFormat::F32 => dispatch::QuantFormat::F32,
        GpuQuantFormat::F16 => dispatch::QuantFormat::F16,
        GpuQuantFormat::Q4 => dispatch::QuantFormat::Q4,
        GpuQuantFormat::Q8 => dispatch::QuantFormat::Q8,
        GpuQuantFormat::Ternary => dispatch::QuantFormat::Ternary,
    }
}

// ---- Precompute param buffers (mirrors model.rs precompute_* functions) ----

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RmsNormParams {
    hidden: u32,
    eps: f32,
}

fn precompute_rms_norm(
    p: &Pipelines,
    positions: u32,
    hidden: u32,
    eps: f32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = RmsNormParams { hidden, eps };
    let buf = p.upload_uniform(bytemuck::bytes_of(&params));
    (buf, (positions, 1, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RopeParams {
    half_dim: u32,
    head_dim: u32,
    seq_len: u32,
    total_elements: u32,
}

fn precompute_rope(
    p: &Pipelines,
    total_elements: u32,
    head_dim: u32,
    seq_len: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = RopeParams {
        half_dim: head_dim / 2,
        head_dim,
        seq_len,
        total_elements,
    };
    let buf = p.upload_uniform(bytemuck::bytes_of(&params));
    (buf, ((total_elements + 255) / 256, 1, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q4MatmulParams {
    n: u32,
    k: u32,
    num_blocks: u32,
    u32s_per_row: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q8MatmulParams {
    n: u32,
    k: u32,
    num_blocks: u32,
    u32s_per_row: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct F32MatmulParams {
    n: u32,
    k: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct F16MatmulParams {
    n: u32,
    k: u32,
    k_half: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TernaryMatmulParams {
    n: u32,
    k: u32,
    u32s_per_row: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ArgmaxParams {
    n: u32,
}

/// Precompute matmul params buffer and workgroups for a given quant format
fn precompute_matmul(
    p: &Pipelines,
    n: u32,
    k: u32,
    block_size: u32,
    quant: GpuQuantFormat,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    match quant {
        GpuQuantFormat::Q4 => {
            let num_blocks = k / block_size;
            let params = Q4MatmulParams {
                n,
                k,
                num_blocks,
                u32s_per_row: num_blocks * (block_size / 2) / 4,
            };
            let buf = p.upload_uniform(bytemuck::bytes_of(&params));
            let num_wg = (n + 3) / 4;
            let x = num_wg.min(65535);
            let y = (num_wg + x - 1) / x;
            (buf, (x, y, 1))
        }
        GpuQuantFormat::Q8 => {
            let num_blocks = k / block_size;
            let params = Q8MatmulParams {
                n,
                k,
                num_blocks,
                u32s_per_row: k / 4,
            };
            let buf = p.upload_uniform(bytemuck::bytes_of(&params));
            let num_wg = (n + 3) / 4;
            let x = num_wg.min(65535);
            let y = (num_wg + x - 1) / x;
            (buf, (x, y, 1))
        }
        GpuQuantFormat::F32 => precompute_f32_matmul(p, n, k),
        GpuQuantFormat::F16 => {
            let params = F16MatmulParams {
                n,
                k,
                k_half: k / 2,
                _pad: 0,
            };
            let buf = p.upload_uniform(bytemuck::bytes_of(&params));
            let x = n.min(65535);
            let y = (n + x - 1) / x;
            (buf, (x, y, 1))
        }
        GpuQuantFormat::Ternary => {
            let params = TernaryMatmulParams {
                n,
                k,
                u32s_per_row: (k + 15) / 16,
                _pad: 0,
            };
            let buf = p.upload_uniform(bytemuck::bytes_of(&params));
            let x = n.min(65535);
            let y = (n + x - 1) / x;
            (buf, (x, y, 1))
        }
    }
}

fn precompute_f32_matmul(p: &Pipelines, n: u32, k: u32) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = F32MatmulParams { n, k };
    let buf = p.upload_uniform(bytemuck::bytes_of(&params));
    let x = n.min(65535);
    let y = (n + x - 1) / x;
    (buf, (x, y, 1))
}

fn precompute_argmax(p: &Pipelines, n: u32) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = ArgmaxParams { n };
    let buf = p.upload_uniform(bytemuck::bytes_of(&params));
    (buf, (1, 1, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_quant_to_dispatch() {
        assert_eq!(
            gpu_quant_to_dispatch(GpuQuantFormat::Q4) as u8,
            dispatch::QuantFormat::Q4 as u8
        );
        assert_eq!(
            gpu_quant_to_dispatch(GpuQuantFormat::F32) as u8,
            dispatch::QuantFormat::F32 as u8
        );
    }

    #[test]
    fn test_attr_u32_extraction() {
        use crate::ir::graph::{Attrs, AttrValue, Node};
        use crate::ir::ops::Op;

        let mut attrs: Attrs = HashMap::new();
        attrs.insert("hidden".to_string(), AttrValue::Int(4096));
        attrs.insert("eps".to_string(), AttrValue::Float(1e-5));

        let node = Node {
            id: 0,
            op: Op::Add,
            inputs: vec![],
            outputs: vec![],
            attrs,
            backend_hint: None,
        };

        assert_eq!(attr_u32(&node, "hidden"), Some(4096));
        assert_eq!(attr_u32(&node, "missing"), None);
    }
}
