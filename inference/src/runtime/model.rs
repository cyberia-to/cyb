//! NativeModel — Qwen3-0.6B forward pass on pure wgpu
//! Loads ONNX Q4 weights, runs transformer layers via WGSL compute shaders

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::onnx_proto::onnx::ModelProto;
use prost::Message;

use super::ops;
use super::pipelines::Pipelines;

/// Transformer model config detected from weight shapes
struct ModelConfig {
    hidden_size: usize,
    num_heads: usize,
    kv_num_heads: usize,
    head_dim: usize,
    num_layers: usize,
    vocab_size: usize,
    block_size: usize,
}

/// Per-layer weights, all on GPU
struct LayerWeights {
    // Attention
    input_norm_weight: wgpu::Buffer,
    q_proj_packed: wgpu::Buffer,
    q_proj_scales: wgpu::Buffer,
    q_n: u32,
    k_proj_packed: wgpu::Buffer,
    k_proj_scales: wgpu::Buffer,
    k_n: u32,
    v_proj_packed: wgpu::Buffer,
    v_proj_scales: wgpu::Buffer,
    v_n: u32,
    q_norm_weight: wgpu::Buffer,
    k_norm_weight: wgpu::Buffer,
    o_proj_packed: wgpu::Buffer,
    o_proj_scales: wgpu::Buffer,
    o_n: u32,

    // FFN
    post_norm_weight: wgpu::Buffer,
    gate_proj_packed: wgpu::Buffer,
    gate_proj_scales: wgpu::Buffer,
    gate_n: u32,
    up_proj_packed: wgpu::Buffer,
    up_proj_scales: wgpu::Buffer,
    up_n: u32,
    down_proj_packed: wgpu::Buffer,
    down_proj_scales: wgpu::Buffer,
    down_n: u32,
}

/// KV cache for one layer
struct KVCache {
    key: Option<wgpu::Buffer>,
    value: Option<wgpu::Buffer>,
}

/// Native wgpu model — holds all weights on GPU and runs forward pass
pub struct NativeModel {
    config: ModelConfig,
    pipelines: Arc<Pipelines>,

    // Weights
    embed_table: wgpu::Buffer,
    layers: Vec<LayerWeights>,
    final_norm_weight: wgpu::Buffer,
    lm_head_packed: wgpu::Buffer,
    lm_head_scales: wgpu::Buffer,
    lm_head_n: u32,

    // Precomputed RoPE caches (full, sliced per forward call)
    cos_cache: Vec<f32>, // [max_seq, head_dim/2] on CPU for slicing
    sin_cache: Vec<f32>,

    // KV cache
    kv_cache: Vec<KVCache>,
    past_seq_len: usize,

    /// When true, argmax is computed on GPU (reads 4 bytes instead of 608KB)
    pub greedy_mode: bool,

    // Pre-allocated scratch buffers (reused every forward pass)
    scratch: ScratchBuffers,
}

/// Pre-allocated GPU buffers for decode step (seq=1)
struct ScratchBuffers {
    hidden: wgpu::Buffer,     // [hidden_size]
    normed: wgpu::Buffer,     // [hidden_size]
    q: wgpu::Buffer,          // [q_n] = num_heads * head_dim
    k: wgpu::Buffer,          // [k_n] = kv_heads * head_dim
    v: wgpu::Buffer,          // [k_n]
    q_normed: wgpu::Buffer,   // [q_n]
    k_normed: wgpu::Buffer,   // [k_n]
    q_roped: wgpu::Buffer,    // [q_n]
    k_roped: wgpu::Buffer,    // [k_n]
    attn_out: wgpu::Buffer,   // [q_n]
    attn_proj: wgpu::Buffer,  // [hidden_size]
    normed2: wgpu::Buffer,    // [hidden_size]
    gate: wgpu::Buffer,       // [gate_n]
    up: wgpu::Buffer,         // [gate_n]
    ffn: wgpu::Buffer,        // [gate_n]
    ffn_out: wgpu::Buffer,    // [hidden_size]
    residual: wgpu::Buffer,   // [hidden_size]
    logits: wgpu::Buffer,     // [vocab_size]
}

/// Load and parse an ONNX model protobuf
fn load_model_proto(path: &Path) -> Result<ModelProto, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buf)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    ModelProto::decode(&*buf)
        .map_err(|e| format!("Failed to decode ONNX protobuf: {e}"))
}

/// Read raw bytes from a TensorProto, supporting external data
fn read_tensor_raw(
    tp: &crate::onnx_proto::onnx::TensorProto,
    model_dir: &Path,
) -> Result<Vec<u8>, String> {
    if tp.data_location == 1 {
        // External data
        let mut location = String::new();
        let mut offset: u64 = 0;
        let mut length: u64 = 0;

        for entry in &tp.external_data {
            match entry.key.as_str() {
                "location" => location = entry.value.clone(),
                "offset" => offset = entry.value.parse().unwrap_or(0),
                "length" => length = entry.value.parse().unwrap_or(0),
                _ => {}
            }
        }

        if location.is_empty() || length == 0 {
            return Err(format!("External tensor {} has no location/length", tp.name));
        }

        let data_path = model_dir.join(&location);
        let file = std::fs::File::open(&data_path)
            .map_err(|e| format!("Cannot open external data {}: {e}", data_path.display()))?;
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(|e| format!("Cannot mmap {}: {e}", data_path.display()))?
        };
        let end = (offset + length) as usize;
        if end > mmap.len() {
            return Err(format!(
                "External data {} out of bounds: offset={offset} length={length} file_size={}",
                tp.name,
                mmap.len()
            ));
        }

        Ok(mmap[offset as usize..end].to_vec())
    } else if !tp.raw_data.is_empty() {
        Ok(tp.raw_data.clone())
    } else if !tp.float_data.is_empty() {
        Ok(bytemuck::cast_slice(&tp.float_data).to_vec())
    } else {
        Err(format!("Tensor {} has no data", tp.name))
    }
}

/// Convert raw bytes to f32 based on data_type
fn raw_to_f32(raw: &[u8], data_type: i32) -> Vec<f32> {
    match data_type {
        1 => {
            // FLOAT
            raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        }
        10 => {
            // FLOAT16
            raw.chunks_exact(2)
                .map(|c| half::f16::from_le_bytes([c[0], c[1]]).to_f32())
                .collect()
        }
        2 => {
            // UINT8 -> f32
            raw.iter().map(|&b| b as f32).collect()
        }
        _ => {
            log::warn!("Unsupported data type {data_type}, treating as f32 bytes");
            raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        }
    }
}

/// Pack raw UINT8 bytes into u32 (4 bytes -> 1 u32, little-endian)
fn pack_bytes_to_u32(raw: &[u8]) -> Vec<u32> {
    raw.chunks(4)
        .map(|chunk| {
            let mut val = 0u32;
            for (i, &b) in chunk.iter().enumerate() {
                val |= (b as u32) << (i * 8);
            }
            val
        })
        .collect()
}

impl NativeModel {
    /// Load model from ONNX file
    pub fn load_from_onnx(path: &Path, pipelines: Arc<Pipelines>) -> Result<Self, String> {
        let model = load_model_proto(path)?;
        let graph = model.graph.ok_or("No graph in model")?;
        let model_dir = path.parent().unwrap_or(Path::new("."));

        log::info!(
            "ONNX model: {} initializers, {} nodes",
            graph.initializer.len(),
            graph.node.len()
        );

        // Index all initializers by name
        let mut tensors: HashMap<String, &crate::onnx_proto::onnx::TensorProto> = HashMap::new();
        for init in &graph.initializer {
            tensors.insert(init.name.clone(), init);
        }

        // Detect model config from known weight shapes
        let embed_tp = tensors
            .get("model.embed_tokens.weight")
            .ok_or("Missing model.embed_tokens.weight")?;
        let vocab_size = embed_tp.dims[0] as usize;
        let hidden_size = embed_tp.dims[1] as usize;

        // Detect number of layers
        let num_layers = (0..)
            .take_while(|i| {
                tensors.contains_key(&format!("model.layers.{i}.input_layernorm.weight"))
            })
            .count();

        // Detect head_dim from q_norm weight
        let q_norm_tp = tensors
            .get("model.layers.0.attn.q_norm.layernorm.weight")
            .ok_or("Missing q_norm weight for layer 0")?;
        let head_dim = q_norm_tp.dims[0] as usize;

        // Detect block_size from MatMulNBits nodes
        let block_size = graph
            .node
            .iter()
            .find(|n| n.op_type == "MatMulNBits")
            .and_then(|n| {
                n.attribute
                    .iter()
                    .find(|a| a.name == "block_size")
                    .map(|a| a.i as usize)
            })
            .unwrap_or(32);

        // Detect num_heads from Q projection output size
        // Q proj: [hidden_size, q_hidden] where q_hidden = num_heads * head_dim
        // For MatMulNBits, N attribute tells us the output dim
        let q_n = graph
            .node
            .iter()
            .find(|n| {
                n.op_type == "MatMulNBits"
                    && n.name.contains("layers.0")
                    && n.name.contains("q_proj")
            })
            .and_then(|n| n.attribute.iter().find(|a| a.name == "N").map(|a| a.i as usize))
            .unwrap_or(hidden_size);
        let num_heads = q_n / head_dim;

        // Detect kv_num_heads from K projection
        let k_n = graph
            .node
            .iter()
            .find(|n| {
                n.op_type == "MatMulNBits"
                    && n.name.contains("layers.0")
                    && n.name.contains("k_proj")
            })
            .and_then(|n| n.attribute.iter().find(|a| a.name == "N").map(|a| a.i as usize))
            .unwrap_or(hidden_size);
        let kv_num_heads = k_n / head_dim;

        let config = ModelConfig {
            hidden_size,
            num_heads,
            kv_num_heads,
            head_dim,
            num_layers,
            vocab_size,
            block_size,
        };

        log::info!(
            "Model config: hidden={}, heads={}, kv_heads={}, head_dim={}, layers={}, vocab={}, block_size={}",
            config.hidden_size,
            config.num_heads,
            config.kv_num_heads,
            config.head_dim,
            config.num_layers,
            config.vocab_size,
            config.block_size,
        );

        // Load embedding table
        let embed_raw = read_tensor_raw(embed_tp, model_dir)?;
        let embed_f32 = raw_to_f32(&embed_raw, embed_tp.data_type);
        let embed_table = pipelines.upload_f32(&embed_f32);
        log::info!("Loaded embedding table: [{vocab_size}, {hidden_size}]");

        // Helper closures
        let load_f32_weight = |name: &str| -> Result<wgpu::Buffer, String> {
            let tp = tensors.get(name).ok_or(format!("Missing {name}"))?;
            let raw = read_tensor_raw(tp, model_dir)?;
            let f32_data = raw_to_f32(&raw, tp.data_type);
            Ok(pipelines.upload_f32(&f32_data))
        };

        let load_q4_packed = |name: &str| -> Result<wgpu::Buffer, String> {
            let tp = tensors.get(name).ok_or(format!("Missing {name}"))?;
            let raw = read_tensor_raw(tp, model_dir)?;
            // ONNX stores Q4 weights as UINT8 raw_data. Pack 4 bytes -> 1 u32.
            let packed = pack_bytes_to_u32(&raw);
            Ok(pipelines.upload_u32(&packed))
        };

        let load_scales = |name: &str| -> Result<wgpu::Buffer, String> {
            let tp = tensors.get(name).ok_or(format!("Missing {name}"))?;
            let raw = read_tensor_raw(tp, model_dir)?;
            let f32_data = raw_to_f32(&raw, tp.data_type);
            Ok(pipelines.upload_f32(&f32_data))
        };

        // Get N (output dim) for a MatMulNBits node by name pattern
        let get_matmul_n = |layer: usize, proj: &str| -> u32 {
            graph
                .node
                .iter()
                .find(|n| {
                    n.op_type == "MatMulNBits"
                        && n.name.contains(&format!("layers.{layer}"))
                        && n.name.contains(proj)
                })
                .and_then(|n| n.attribute.iter().find(|a| a.name == "N").map(|a| a.i as u32))
                .unwrap_or(0)
        };

        // Get N for lm_head
        let lm_head_n_val = graph
            .node
            .iter()
            .find(|n| n.op_type == "MatMulNBits" && n.name.contains("lm_head"))
            .and_then(|n| n.attribute.iter().find(|a| a.name == "N").map(|a| a.i as u32))
            .unwrap_or(vocab_size as u32);

        // Load layers
        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            log::info!("Loading layer {i}/{num_layers}...");

            let input_norm_weight =
                load_f32_weight(&format!("model.layers.{i}.input_layernorm.weight"))?;
            let q_proj_packed =
                load_q4_packed(&format!("model.layers.{i}.attn.q_proj.MatMul.weight_Q4"))?;
            let q_proj_scales =
                load_scales(&format!("model.layers.{i}.attn.q_proj.MatMul.weight_scales"))?;
            let layer_q_n = get_matmul_n(i, "q_proj");

            let k_proj_packed =
                load_q4_packed(&format!("model.layers.{i}.attn.k_proj.MatMul.weight_Q4"))?;
            let k_proj_scales =
                load_scales(&format!("model.layers.{i}.attn.k_proj.MatMul.weight_scales"))?;
            let layer_k_n = get_matmul_n(i, "k_proj");

            let v_proj_packed =
                load_q4_packed(&format!("model.layers.{i}.attn.v_proj.MatMul.weight_Q4"))?;
            let v_proj_scales =
                load_scales(&format!("model.layers.{i}.attn.v_proj.MatMul.weight_scales"))?;
            let layer_v_n = get_matmul_n(i, "v_proj");

            let q_norm_weight =
                load_f32_weight(&format!("model.layers.{i}.attn.q_norm.layernorm.weight"))?;
            let k_norm_weight =
                load_f32_weight(&format!("model.layers.{i}.attn.k_norm.layernorm.weight"))?;

            let o_proj_packed =
                load_q4_packed(&format!("model.layers.{i}.attn.o_proj.MatMul.weight_Q4"))?;
            let o_proj_scales =
                load_scales(&format!("model.layers.{i}.attn.o_proj.MatMul.weight_scales"))?;
            let layer_o_n = get_matmul_n(i, "o_proj");

            let post_norm_weight =
                load_f32_weight(&format!("model.layers.{i}.post_attention_layernorm.weight"))?;

            let gate_proj_packed =
                load_q4_packed(&format!("model.layers.{i}.mlp.gate_proj.MatMul.weight_Q4"))?;
            let gate_proj_scales =
                load_scales(&format!("model.layers.{i}.mlp.gate_proj.MatMul.weight_scales"))?;
            let layer_gate_n = get_matmul_n(i, "gate_proj");

            let up_proj_packed =
                load_q4_packed(&format!("model.layers.{i}.mlp.up_proj.MatMul.weight_Q4"))?;
            let up_proj_scales =
                load_scales(&format!("model.layers.{i}.mlp.up_proj.MatMul.weight_scales"))?;
            let layer_up_n = get_matmul_n(i, "up_proj");

            let down_proj_packed =
                load_q4_packed(&format!("model.layers.{i}.mlp.down_proj.MatMul.weight_Q4"))?;
            let down_proj_scales =
                load_scales(&format!("model.layers.{i}.mlp.down_proj.MatMul.weight_scales"))?;
            let layer_down_n = get_matmul_n(i, "down_proj");

            layers.push(LayerWeights {
                input_norm_weight,
                q_proj_packed,
                q_proj_scales,
                q_n: layer_q_n,
                k_proj_packed,
                k_proj_scales,
                k_n: layer_k_n,
                v_proj_packed,
                v_proj_scales,
                v_n: layer_v_n,
                q_norm_weight,
                k_norm_weight,
                o_proj_packed,
                o_proj_scales,
                o_n: layer_o_n,
                post_norm_weight,
                gate_proj_packed,
                gate_proj_scales,
                gate_n: layer_gate_n,
                up_proj_packed,
                up_proj_scales,
                up_n: layer_up_n,
                down_proj_packed,
                down_proj_scales,
                down_n: layer_down_n,
            });
        }

        // Final norm — Qwen3 uses "model.layers.{num_layers}.final_norm_layernorm.weight"
        let final_norm_name = format!("model.layers.{num_layers}.final_norm_layernorm.weight");
        let final_norm_weight = load_f32_weight(&final_norm_name)?;

        // LM head — Qwen3 ties embed_tokens weight (transposed) as lm_head
        // We reuse embed_table transposed for lm_head
        // Since it's f32 [vocab, hidden], we need to do regular matmul, not Q4
        // Store a flag that lm_head is tied
        let lm_head_packed = pipelines.upload_u32(&[0]); // dummy — tied weights use embed_table
        let lm_head_scales = pipelines.upload_f32(&[0.0]); // dummy

        // RoPE caches
        let cos_tp = tensors.get("cos_cache").ok_or("Missing cos_cache")?;
        let cos_raw = read_tensor_raw(cos_tp, model_dir)?;
        let cos_cache = raw_to_f32(&cos_raw, cos_tp.data_type);

        let sin_tp = tensors.get("sin_cache").ok_or("Missing sin_cache")?;
        let sin_raw = read_tensor_raw(sin_tp, model_dir)?;
        let sin_cache = raw_to_f32(&sin_raw, sin_tp.data_type);

        // Initialize empty KV caches
        let kv_cache = (0..num_layers)
            .map(|_| KVCache {
                key: None,
                value: None,
            })
            .collect();

        log::info!("Model loaded successfully");

        let p_ref = pipelines.clone();
        Ok(Self {
            config,
            pipelines,
            embed_table,
            layers,
            final_norm_weight,
            lm_head_packed,
            lm_head_scales,
            lm_head_n: lm_head_n_val,
            cos_cache,
            sin_cache,
            kv_cache,
            past_seq_len: 0,
            greedy_mode: false,
            scratch: {
                let q_n = num_heads * head_dim;
                let k_n = kv_num_heads * head_dim;
                let gate_n = if num_layers > 0 {
                    graph.node.iter()
                        .find(|n| n.op_type == "MatMulNBits" && n.name.contains("gate_proj") && n.name.contains("layers.0"))
                        .and_then(|n| n.attribute.iter().find(|a| a.name == "N").map(|a| a.i as usize))
                        .unwrap_or(hidden_size * 3)
                } else { hidden_size * 3 };
                let alloc = |size: usize| p_ref.alloc((size as u64) * 4);
                ScratchBuffers {
                    hidden: alloc(hidden_size),
                    normed: alloc(hidden_size),
                    q: alloc(q_n),
                    k: alloc(k_n),
                    v: alloc(k_n),
                    q_normed: alloc(q_n),
                    k_normed: alloc(k_n),
                    q_roped: alloc(q_n),
                    k_roped: alloc(k_n),
                    attn_out: alloc(q_n),
                    attn_proj: alloc(hidden_size),
                    normed2: alloc(hidden_size),
                    gate: alloc(gate_n),
                    up: alloc(gate_n),
                    ffn: alloc(gate_n),
                    ffn_out: alloc(hidden_size),
                    residual: alloc(hidden_size),
                    logits: alloc(vocab_size),
                }
            },
        })
    }

    /// Reset KV cache for a new conversation
    pub fn reset_kv_cache(&mut self) {
        self.past_seq_len = 0;
        for cache in &mut self.kv_cache {
            cache.key = None;
            cache.value = None;
        }
    }

    /// Run one forward pass: token_ids -> logits buffer on GPU
    /// Returns logits as Vec<f32> of size [vocab_size] (last position only)
    ///
    /// Decode path (seq_len=1): uses batched compute passes — ONE pass per phase
    /// per layer instead of one pass per op. Reduces ~500 passes → ~30.
    /// Prefill path (seq_len>1): uses legacy one-pass-per-op for simplicity.
    pub fn forward(&mut self, token_ids: &[u32]) -> Vec<f32> {
        let p = self.pipelines.clone();
        p.begin_frame(); // Reset buffer pool — reuse all previous step's buffers

        let seq_len = token_ids.len();
        let pos_offset = self.past_seq_len;
        let total_seq = pos_offset + seq_len;
        let hidden_size = self.config.hidden_size as u32;
        let head_dim = self.config.head_dim as u32;
        let half_dim = head_dim / 2;
        let num_heads = self.config.num_heads as u32;
        let kv_num_heads = self.config.kv_num_heads as u32;
        let block_size = self.config.block_size as u32;

        // Single command encoder for ALL GPU work in this forward pass
        let mut enc = p.device.create_command_encoder(&Default::default());

        // 1. Upload token IDs as f32 (embed shader reads f32 token_ids)
        let ids_f32: Vec<f32> = token_ids.iter().map(|&id| id as f32).collect();
        let ids_buf = p.upload_f32(&ids_f32);

        // 2. Embedding lookup
        let mut hidden = ops::embed(&p, &mut enc, &self.embed_table, &ids_buf, hidden_size, seq_len as u32);

        // 3. Slice RoPE caches for current positions [pos_offset..pos_offset+seq_len]
        //    cos_cache layout: [max_seq, half_dim]
        let cos_slice: Vec<f32> = (0..seq_len)
            .flat_map(|s| {
                let pos = pos_offset + s;
                let start = pos * (half_dim as usize);
                let end = start + half_dim as usize;
                self.cos_cache[start..end].iter().copied()
            })
            .collect();
        let sin_slice: Vec<f32> = (0..seq_len)
            .flat_map(|s| {
                let pos = pos_offset + s;
                let start = pos * (half_dim as usize);
                let end = start + half_dim as usize;
                self.sin_cache[start..end].iter().copied()
            })
            .collect();
        let cos_buf = p.upload_f32(&cos_slice);
        let sin_buf = p.upload_f32(&sin_slice);

        // 4. Transformer layers
        let num_layers = self.layers.len();

        if seq_len == 1 {
            // ================================================================
            // DECODE PATH — batched compute passes (ONE pass per phase)
            // ================================================================
            for i in 0..num_layers {
                // ---- Phase 1: Prepare all bind groups for pre-attention ----
                // All output buffers and bind groups are created BEFORE the
                // compute pass begins. Dispatches run in order within the pass,
                // so norm output is written before matmul reads it, etc.

                // a. Input RMS Norm: hidden → normed
                let (normed, norm_bg, norm_wg) = ops::rms_norm_prepare(
                    &p, &hidden, &self.layers[i].input_norm_weight,
                    1, hidden_size, 1e-6,
                );

                // b. Q/K/V projections: normed → q, k, v
                let (q_buf, q_bg, q_wg) = ops::q4_matmul_prepare(
                    &p, &normed, &self.layers[i].q_proj_packed,
                    &self.layers[i].q_proj_scales,
                    self.layers[i].q_n, hidden_size, block_size,
                );
                let (k_buf, k_bg, k_wg) = ops::q4_matmul_prepare(
                    &p, &normed, &self.layers[i].k_proj_packed,
                    &self.layers[i].k_proj_scales,
                    self.layers[i].k_n, hidden_size, block_size,
                );
                let (v_buf, v_bg, v_wg) = ops::q4_matmul_prepare(
                    &p, &normed, &self.layers[i].v_proj_packed,
                    &self.layers[i].v_proj_scales,
                    self.layers[i].v_n, hidden_size, block_size,
                );

                // c. Q/K per-head RMS Norm
                let (q_normed, qn_bg, qn_wg) = ops::rms_norm_prepare(
                    &p, &q_buf, &self.layers[i].q_norm_weight,
                    num_heads, head_dim, 1e-6,
                );
                let (k_normed, kn_bg, kn_wg) = ops::rms_norm_prepare(
                    &p, &k_buf, &self.layers[i].k_norm_weight,
                    kv_num_heads, head_dim, 1e-6,
                );

                // d. RoPE
                let q_elements = self.layers[i].q_n;
                let k_elements = self.layers[i].k_n;

                let (q_roped, qr_bg, qr_wg) = ops::rope_prepare(
                    &p, &q_normed, &cos_buf, &sin_buf,
                    q_elements, head_dim, 1,
                );
                let (k_roped, kr_bg, kr_wg) = ops::rope_prepare(
                    &p, &k_normed, &cos_buf, &sin_buf,
                    k_elements, head_dim, 1,
                );

                // ---- KV cache via compute (stays in same pass later) ----
                let past_seq_u32 = self.past_seq_len as u32;
                let empty_buf = p.alloc(4);
                let past_k_ref = self.kv_cache[i].key.as_ref().unwrap_or(&empty_buf);
                let past_v_ref = self.kv_cache[i].value.as_ref().unwrap_or(&empty_buf);

                let (full_k, kv_k_bg, kv_k_wg) = ops::kv_append_prepare(
                    &p, past_k_ref, &k_roped, kv_num_heads, head_dim, past_seq_u32, seq_len as u32);
                let (full_v, kv_v_bg, kv_v_wg) = ops::kv_append_prepare(
                    &p, past_v_ref, &v_buf, kv_num_heads, head_dim, past_seq_u32, seq_len as u32);

                // GQA head expansion via compute
                let (attn_k, exp_k_bg, exp_k_wg, attn_v, exp_v_bg, exp_v_wg) = if num_heads != kv_num_heads {
                    let (ek, ekb, ekw) = ops::kv_expand_prepare(&p, &full_k, kv_num_heads, num_heads, head_dim, total_seq as u32);
                    let (ev, evb, evw) = ops::kv_expand_prepare(&p, &full_v, kv_num_heads, num_heads, head_dim, total_seq as u32);
                    (ek, ekb, ekw, ev, evb, evw)
                } else {
                    // No expansion needed — create dummy bind groups
                    let dummy_bg = p.create_bind_group(&p.kv_expand, &[
                        full_k.as_entire_binding(), full_k.as_entire_binding(),
                        p.upload_uniform(&[0u8; 32]).as_entire_binding(),
                    ]);
                    (full_k.clone(), dummy_bg.clone(), (0,0,0),
                     full_v.clone(), dummy_bg, (0,0,0))
                };

                let scale = 1.0 / (head_dim as f32).sqrt();

                // Attention decode: q_roped × K^T → softmax → V
                let (attn_out, attn_bg, attn_wg) = ops::attention_decode_prepare(
                    &p, &q_roped, &attn_k, &attn_v,
                    num_heads, head_dim, total_seq as u32, scale,
                );

                // O projection: attn_out → attn_proj
                let (attn_proj, oproj_bg, oproj_wg) = ops::q4_matmul_prepare(
                    &p, &attn_out, &self.layers[i].o_proj_packed,
                    &self.layers[i].o_proj_scales,
                    self.layers[i].o_n, self.layers[i].q_n, block_size,
                );

                // Residual: hidden + attn_proj → residual1
                let (residual1, res1_bg, res1_wg) = ops::add_prepare(
                    &p, &hidden, &attn_proj, hidden_size,
                );

                // Post-attention RMS Norm: residual1 → normed2
                let (normed2, norm2_bg, norm2_wg) = ops::rms_norm_prepare(
                    &p, &residual1, &self.layers[i].post_norm_weight,
                    1, hidden_size, 1e-6,
                );

                // Gate + Up projections: normed2 → gate, up
                let (gate, gate_bg, gate_wg) = ops::q4_matmul_prepare(
                    &p, &normed2, &self.layers[i].gate_proj_packed,
                    &self.layers[i].gate_proj_scales,
                    self.layers[i].gate_n, hidden_size, block_size,
                );
                let (up, up_bg, up_wg) = ops::q4_matmul_prepare(
                    &p, &normed2, &self.layers[i].up_proj_packed,
                    &self.layers[i].up_proj_scales,
                    self.layers[i].up_n, hidden_size, block_size,
                );

                // SwiGLU: gate, up → ffn
                let (ffn, silu_bg, silu_wg) = ops::silu_mul_prepare(
                    &p, &gate, &up, self.layers[i].gate_n,
                );

                // Down projection: ffn → ffn_out
                let (ffn_out, down_bg, down_wg) = ops::q4_matmul_prepare(
                    &p, &ffn, &self.layers[i].down_proj_packed,
                    &self.layers[i].down_proj_scales,
                    self.layers[i].down_n, self.layers[i].gate_n, block_size,
                );

                // Final residual: residual1 + ffn_out → hidden
                let (new_hidden, res2_bg, res2_wg) = ops::add_prepare(
                    &p, &residual1, &ffn_out, hidden_size,
                );

                // ---- ALL ops in ONE compute pass per layer! ----
                {
                    let mut pass = enc.begin_compute_pass(&Default::default());

                    // Pre-attention: norm → QKV → Q/K norm → RoPE
                    p.dispatch_in_pass(&mut pass, &p.rms_norm, &norm_bg, norm_wg);
                    p.dispatch_in_pass(&mut pass, &p.q4_matmul, &q_bg, q_wg);
                    p.dispatch_in_pass(&mut pass, &p.q4_matmul, &k_bg, k_wg);
                    p.dispatch_in_pass(&mut pass, &p.q4_matmul, &v_bg, v_wg);
                    p.dispatch_in_pass(&mut pass, &p.rms_norm, &qn_bg, qn_wg);
                    p.dispatch_in_pass(&mut pass, &p.rms_norm, &kn_bg, kn_wg);
                    p.dispatch_in_pass(&mut pass, &p.rope, &qr_bg, qr_wg);
                    p.dispatch_in_pass(&mut pass, &p.rope, &kr_bg, kr_wg);

                    // KV cache append + expand (compute, not buffer copies!)
                    p.dispatch_in_pass(&mut pass, &p.kv_append, &kv_k_bg, kv_k_wg);
                    p.dispatch_in_pass(&mut pass, &p.kv_append, &kv_v_bg, kv_v_wg);
                    if num_heads != kv_num_heads {
                        p.dispatch_in_pass(&mut pass, &p.kv_expand, &exp_k_bg, exp_k_wg);
                        p.dispatch_in_pass(&mut pass, &p.kv_expand, &exp_v_bg, exp_v_wg);
                    }

                    // Post-attention: attn → O proj → residual → norm → FFN → residual
                    p.dispatch_in_pass(&mut pass, &p.attention, &attn_bg, attn_wg);
                    p.dispatch_in_pass(&mut pass, &p.q4_matmul, &oproj_bg, oproj_wg);
                    p.dispatch_in_pass(&mut pass, &p.add, &res1_bg, res1_wg);
                    p.dispatch_in_pass(&mut pass, &p.rms_norm, &norm2_bg, norm2_wg);
                    p.dispatch_in_pass(&mut pass, &p.q4_matmul, &gate_bg, gate_wg);
                    p.dispatch_in_pass(&mut pass, &p.q4_matmul, &up_bg, up_wg);
                    p.dispatch_in_pass(&mut pass, &p.silu_mul, &silu_bg, silu_wg);
                    p.dispatch_in_pass(&mut pass, &p.q4_matmul, &down_bg, down_wg);
                    p.dispatch_in_pass(&mut pass, &p.add, &res2_bg, res2_wg);
                }

                // KV cache save (buffer copies, outside pass)
                let kv_heads_usize = self.config.kv_num_heads;
                let head_dim_usize = self.config.head_dim;
                let total_kv_elements = total_seq * kv_heads_usize * head_dim_usize;
                let cache_k = p.alloc_permanent((total_kv_elements as u64) * 4);
                let cache_v = p.alloc_permanent((total_kv_elements as u64) * 4);
                enc.copy_buffer_to_buffer(&full_k, 0, &cache_k, 0, (total_kv_elements as u64) * 4);
                enc.copy_buffer_to_buffer(&full_v, 0, &cache_v, 0, (total_kv_elements as u64) * 4);
                self.kv_cache[i].key = Some(cache_k);
                self.kv_cache[i].value = Some(cache_v);

                hidden = new_hidden;
            }
        } else {
            // ================================================================
            // PREFILL PATH — legacy one-pass-per-op (unchanged)
            // ================================================================
            for i in 0..num_layers {
                // a. Input RMS Norm
                let normed = ops::rms_norm(
                    &p, &mut enc, &hidden, &self.layers[i].input_norm_weight,
                    seq_len as u32, hidden_size, 1e-6,
                );

                // b. Q/K/V projections: per-position matvec
                let layer_q_n = self.layers[i].q_n;
                let layer_k_n = self.layers[i].k_n;
                let layer_v_n = self.layers[i].v_n;
                let q_size = (layer_q_n as u64) * (seq_len as u64) * 4;
                let k_size = (layer_k_n as u64) * (seq_len as u64) * 4;
                let v_size = (layer_v_n as u64) * (seq_len as u64) * 4;
                let q_combined = p.alloc(q_size);
                let k_combined = p.alloc(k_size);
                let v_combined = p.alloc(v_size);

                for s in 0..seq_len {
                    let normed_slice =
                        slice_buffer(&p, &mut enc, &normed, s * self.config.hidden_size, self.config.hidden_size);

                    let q_pos = ops::q4_matmul(
                        &p, &mut enc, &normed_slice,
                        &self.layers[i].q_proj_packed, &self.layers[i].q_proj_scales,
                        layer_q_n, hidden_size, block_size,
                    );
                    let k_pos = ops::q4_matmul(
                        &p, &mut enc, &normed_slice,
                        &self.layers[i].k_proj_packed, &self.layers[i].k_proj_scales,
                        layer_k_n, hidden_size, block_size,
                    );
                    let v_pos = ops::q4_matmul(
                        &p, &mut enc, &normed_slice,
                        &self.layers[i].v_proj_packed, &self.layers[i].v_proj_scales,
                        layer_v_n, hidden_size, block_size,
                    );

                    copy_into(&mut enc, &q_pos, &q_combined, s * layer_q_n as usize, layer_q_n as usize);
                    copy_into(&mut enc, &k_pos, &k_combined, s * layer_k_n as usize, layer_k_n as usize);
                    copy_into(&mut enc, &v_pos, &v_combined, s * layer_v_n as usize, layer_v_n as usize);
                }

                let (q_buf, k_buf, v_buf) = (q_combined, k_combined, v_combined);

                // c. Q/K per-head RMS Norm
                let q_positions = seq_len as u32 * num_heads;
                let k_positions = seq_len as u32 * kv_num_heads;

                let q_normed = ops::rms_norm(
                    &p, &mut enc, &q_buf, &self.layers[i].q_norm_weight,
                    q_positions, head_dim, 1e-6,
                );
                let k_normed = ops::rms_norm(
                    &p, &mut enc, &k_buf, &self.layers[i].k_norm_weight,
                    k_positions, head_dim, 1e-6,
                );

                // d. RoPE
                let q_elements = seq_len as u32 * self.layers[i].q_n;
                let k_elements = seq_len as u32 * self.layers[i].k_n;

                let q_roped = ops::rope(
                    &p, &mut enc, &q_normed, &cos_buf, &sin_buf,
                    q_elements, head_dim, seq_len as u32,
                );
                let k_roped = ops::rope(
                    &p, &mut enc, &k_normed, &cos_buf, &sin_buf,
                    k_elements, head_dim, seq_len as u32,
                );

                // e. KV cache: concat with past
                let kv_heads_usize = self.config.kv_num_heads;
                let head_dim_usize = self.config.head_dim;
                let total_kv_elements = total_seq * kv_heads_usize * head_dim_usize;

                let full_k = p.alloc((total_kv_elements as u64) * 4);
                let full_v = p.alloc((total_kv_elements as u64) * 4);

                {
                    let past_seq = self.past_seq_len;

                    for h in 0..kv_heads_usize {
                        let new_head_offset = (total_seq * head_dim_usize * h) as u64 * 4;

                        if past_seq > 0 {
                            if let Some(ref past_k) = self.kv_cache[i].key {
                                let past_head_src = (h * past_seq * head_dim_usize) as u64 * 4;
                                let past_bytes = (past_seq * head_dim_usize) as u64 * 4;
                                let past_buf_size = past_k.size();
                                if past_head_src + past_bytes > past_buf_size {
                                    log::error!("KV K copy OOB layer={i} h={h}: src_off={past_head_src} + size={past_bytes} > buf={past_buf_size}");
                                } else {
                                    enc.copy_buffer_to_buffer(past_k, past_head_src, &full_k, new_head_offset, past_bytes);
                                }
                            }
                            if let Some(ref past_v) = self.kv_cache[i].value {
                                let past_head_src = (h * past_seq * head_dim_usize) as u64 * 4;
                                let past_bytes = (past_seq * head_dim_usize) as u64 * 4;
                                let past_buf_size = past_v.size();
                                if past_head_src + past_bytes > past_buf_size {
                                    log::error!("KV V copy OOB layer={i} h={h}: src_off={past_head_src} + size={past_bytes} > buf={past_buf_size}");
                                } else {
                                    enc.copy_buffer_to_buffer(past_v, past_head_src, &full_v, new_head_offset, past_bytes);
                                }
                            }
                        }

                        for s in 0..seq_len {
                            let new_src = ((s * kv_heads_usize + h) * head_dim_usize) as u64 * 4;
                            let new_dst = new_head_offset + ((past_seq + s) * head_dim_usize) as u64 * 4;
                            let dim_bytes = (head_dim_usize as u64) * 4;
                            let k_buf_size = (seq_len * kv_heads_usize * head_dim_usize) as u64 * 4;
                            if new_src + dim_bytes > k_buf_size {
                                log::error!("K copy OOB: src={new_src}+{dim_bytes} > buf_size={k_buf_size}, h={h}, s={s}");
                                continue;
                            }
                            enc.copy_buffer_to_buffer(&k_roped, new_src, &full_k, new_dst, dim_bytes);
                            enc.copy_buffer_to_buffer(&v_buf, new_src, &full_v, new_dst, dim_bytes);
                        }
                    }
                }

                let cache_k = p.alloc_permanent((total_kv_elements as u64) * 4);
                let cache_v = p.alloc_permanent((total_kv_elements as u64) * 4);
                enc.copy_buffer_to_buffer(&full_k, 0, &cache_k, 0, (total_kv_elements as u64) * 4);
                enc.copy_buffer_to_buffer(&full_v, 0, &cache_v, 0, (total_kv_elements as u64) * 4);
                self.kv_cache[i].key = Some(cache_k);
                self.kv_cache[i].value = Some(cache_v);

                // f. Attention: GQA head expansion
                let (attn_k, attn_v) = if num_heads != kv_num_heads {
                    let expanded_k = expand_kv_heads(&p, &mut enc, &full_k, kv_num_heads, num_heads, head_dim, total_seq as u32);
                    let expanded_v = expand_kv_heads(&p, &mut enc, &full_v, kv_num_heads, num_heads, head_dim, total_seq as u32);
                    (expanded_k, expanded_v)
                } else {
                    (full_k, full_v)
                };

                let scale = 1.0 / (head_dim as f32).sqrt();

                // Prefill: use last position's Q only
                let last_q_offset = (seq_len - 1) * (self.layers[i].q_n as usize);
                let last_q = slice_buffer(&p, &mut enc, &q_roped, last_q_offset, self.layers[i].q_n as usize);
                let attn_out = ops::attention_decode(
                    &p, &mut enc, &last_q, &attn_k, &attn_v,
                    num_heads, head_dim, total_seq as u32, scale,
                );

                // g. O projection
                let attn_proj = ops::q4_matmul(
                    &p, &mut enc, &attn_out,
                    &self.layers[i].o_proj_packed, &self.layers[i].o_proj_scales,
                    self.layers[i].o_n, self.layers[i].q_n, block_size,
                );

                // h. Residual connection
                let residual_hidden = slice_buffer(
                    &p, &mut enc, &hidden,
                    (seq_len - 1) * self.config.hidden_size, self.config.hidden_size,
                );
                hidden = ops::add(&p, &mut enc, &residual_hidden, &attn_proj, hidden_size);

                // i. Post-attention RMS Norm
                let normed2 = ops::rms_norm(
                    &p, &mut enc, &hidden, &self.layers[i].post_norm_weight,
                    1, hidden_size, 1e-6,
                );

                // j. Gate + Up projections
                let gate = ops::q4_matmul(
                    &p, &mut enc, &normed2,
                    &self.layers[i].gate_proj_packed, &self.layers[i].gate_proj_scales,
                    self.layers[i].gate_n, hidden_size, block_size,
                );
                let up = ops::q4_matmul(
                    &p, &mut enc, &normed2,
                    &self.layers[i].up_proj_packed, &self.layers[i].up_proj_scales,
                    self.layers[i].up_n, hidden_size, block_size,
                );

                // k. SwiGLU
                let ffn = ops::silu_mul(&p, &mut enc, &gate, &up, self.layers[i].gate_n);

                // l. Down projection
                let ffn_out = ops::q4_matmul(
                    &p, &mut enc, &ffn,
                    &self.layers[i].down_proj_packed, &self.layers[i].down_proj_scales,
                    self.layers[i].down_n, self.layers[i].gate_n, block_size,
                );

                // m. Residual
                hidden = ops::add(&p, &mut enc, &hidden, &ffn_out, hidden_size);
            }
        }

        // 5. Final RMS Norm + LM head
        let normed = ops::rms_norm(&p, &mut enc, &hidden, &self.final_norm_weight, 1, hidden_size, 1e-6);

        // 6. LM head — GPU f32 matmul with tied embed weights
        let vocab = self.config.vocab_size as u32;
        let logits_buf = ops::f32_matmul(&p, &mut enc, &normed, &self.embed_table, vocab, hidden_size);

        // For greedy decoding: argmax on GPU, read only 1 u32
        if self.greedy_mode {
            let argmax_buf = ops::argmax_gpu(&p, &mut enc, &logits_buf, vocab);

            let t0 = std::time::Instant::now();
            p.queue.submit(std::iter::once(enc.finish()));
            self.past_seq_len = total_seq;

            // Read single u32 (4 bytes instead of 608KB!)
            let bytes = p.read_f32(&argmax_buf, 1);
            let token_id = bytes[0].to_bits(); // reinterpret f32 bits as u32
            log::info!("GPU total (greedy): {:.1}ms", t0.elapsed().as_secs_f64() * 1000.0);

            // Return fake logits with just the argmax token marked
            let mut logits = vec![0.0f32; vocab as usize];
            logits[token_id as usize] = 1.0;
            return logits;
        }

        // Full logits readback (for sampling with temperature)
        let t0 = std::time::Instant::now();
        p.queue.submit(std::iter::once(enc.finish()));
        self.past_seq_len = total_seq;

        let logits = p.read_f32(&logits_buf, vocab as usize);
        log::info!("GPU total (full readback): {:.1}ms", t0.elapsed().as_secs_f64() * 1000.0);

        logits
    }
}

// --- Free helper functions (avoid borrow conflicts with &mut self) ---

/// Slice a GPU buffer: copy `count` f32 elements from `offset` into a new buffer
fn slice_buffer(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    src: &wgpu::Buffer,
    offset_elements: usize,
    count_elements: usize,
) -> wgpu::Buffer {
    let dst = p.alloc((count_elements as u64) * 4);
    enc.copy_buffer_to_buffer(
        src,
        (offset_elements as u64) * 4,
        &dst,
        0,
        (count_elements as u64) * 4,
    );
    dst
}

/// Copy src buffer into dst buffer at given element offset
fn copy_into(
    enc: &mut wgpu::CommandEncoder,
    src: &wgpu::Buffer,
    dst: &wgpu::Buffer,
    offset_elements: usize,
    count_elements: usize,
) {
    enc.copy_buffer_to_buffer(
        src,
        0,
        dst,
        (offset_elements as u64) * 4,
        (count_elements as u64) * 4,
    );
}

/// Expand KV heads for GQA: replicate each KV head to match Q heads
/// Input: [kv_heads * total_seq * head_dim]
/// Output: [num_heads * total_seq * head_dim]
fn expand_kv_heads(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    kv: &wgpu::Buffer,
    kv_heads: u32,
    num_heads: u32,
    head_dim: u32,
    total_seq: u32,
) -> wgpu::Buffer {
    let repeats = num_heads / kv_heads;
    let output_elements = num_heads * total_seq * head_dim;
    let output = p.alloc((output_elements as u64) * 4);
    let kv_head_size = (total_seq * head_dim) as u64 * 4;

    for kv_h in 0..kv_heads {
        for r in 0..repeats {
            let dst_head = kv_h * repeats + r;
            enc.copy_buffer_to_buffer(
                kv,
                (kv_h as u64) * kv_head_size,
                &output,
                (dst_head as u64) * kv_head_size,
                kv_head_size,
            );
        }
    }

    output
}

/// Greedy argmax on CPU
pub fn argmax(logits: &[f32]) -> u32 {
    logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i as u32)
        .unwrap_or(0)
}

/// Top-p sampling on CPU
pub fn sample_top_p(logits: &[f32], temperature: f32, top_p: f32) -> u32 {
    if temperature <= 0.0 {
        return argmax(logits);
    }

    // Temperature scaling
    let scaled: Vec<f32> = logits.iter().map(|&v| v / temperature).collect();

    // Softmax
    let max_val = scaled.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp_vals: Vec<f32> = scaled.iter().map(|&v| (v - max_val).exp()).collect();
    let sum: f32 = exp_vals.iter().sum();
    let probs: Vec<f32> = exp_vals.iter().map(|&v| v / sum).collect();

    // Sort descending
    let mut indexed: Vec<(usize, f32)> = probs.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Nucleus
    let mut cumulative = 0.0;
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for (idx, prob) in &indexed {
        cumulative += prob;
        candidates.push((*idx, *prob));
        if cumulative >= top_p {
            break;
        }
    }

    // Renormalize
    let total: f32 = candidates.iter().map(|(_, p)| p).sum();

    // Simple RNG
    let r = {
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let x = seed.wrapping_mul(1103515245).wrapping_add(12345);
        (x as f32 / u32::MAX as f32).abs()
    };

    let mut acc = 0.0;
    for (idx, prob) in &candidates {
        acc += prob / total;
        if acc >= r {
            return *idx as u32;
        }
    }

    candidates.last().map(|(i, _)| *i as u32).unwrap_or(0)
}
