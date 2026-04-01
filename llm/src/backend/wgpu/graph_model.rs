//! GraphModel — universal model runner via Graph IR + GraphExecutor
//!
//! Unlike NativeModel (hardcoded transformer decoder forward pass), GraphModel
//! can run any architecture by interpreting the Graph IR through the executor.
//! Supports encoder-only (BERT), encoder-decoder (Whisper), and decoder-only models.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ir::{Graph, DType, WeightData};
use crate::ir::executor::{GraphExecutor, GpuWeight, GpuQuantFormat, ExecKVCache, ExecConfig};
use super::pipelines::Pipelines;

/// Architecture type — determines how inference is run
#[derive(Clone, Debug)]
pub enum Architecture {
    /// Encoder-only (BERT) — single forward pass, no autoregressive loop
    Encoder,
    /// Encoder-decoder (Whisper) — encode once, decode autoregressively
    EncoderDecoder,
    /// Decoder-only (GPT, LLaMA) — autoregressive with KV cache
    Decoder,
}

/// Universal model runner built on GraphExecutor.
///
/// Usage:
/// ```ignore
/// let model = GraphModel::new(graph, weights, pipelines, Architecture::Encoder)?;
/// let output = model.encode(&input_ids, "pooler_output", 768)?;
/// ```
pub struct GraphModel {
    graph: Graph,
    executor: GraphExecutor,
    weights: HashMap<String, GpuWeight>,
    pipelines: Arc<Pipelines>,
    architecture: Architecture,
    /// KV cache for decoder models
    kv_cache: Vec<ExecKVCache>,
    /// Past sequence length for KV cache
    past_seq_len: usize,
    /// Precomputed cos/sin caches for RoPE (empty for non-RoPE models)
    cos_cache: Vec<f32>,
    sin_cache: Vec<f32>,
    /// Config
    config: GraphModelConfig,
}

/// Configuration for GraphModel
#[derive(Clone, Debug)]
pub struct GraphModelConfig {
    pub hidden_size: u32,
    pub num_heads: u32,
    pub kv_num_heads: u32,
    pub head_dim: u32,
    pub vocab_size: u32,
    pub num_layers: u32,
    pub block_size: u32,
    pub rope_theta: f32,
    pub max_seq_len: u32,
    pub has_qk_norm: bool,
}

impl GraphModel {
    /// Create a new GraphModel from a graph, raw weights, and pipeline.
    ///
    /// Weight data is uploaded to GPU. The graph must already contain
    /// correct weight names matching the loaded weights.
    pub fn new(
        mut graph: Graph,
        raw_weights: &HashMap<String, WeightData>,
        pipelines: Arc<Pipelines>,
        architecture: Architecture,
        config: GraphModelConfig,
    ) -> Result<Self, String> {
        // Topological sort the graph
        if !graph.topological_sort() {
            return Err("Graph has cycles, cannot sort topologically".to_string());
        }

        // Determine default quantization format from weights
        let default_quant = detect_quant_format(raw_weights);

        // Upload weights to GPU
        // For encoder: force F32 (encoder uses batch f32_matmul, can't use Q4 buffers)
        let force_f32 = matches!(architecture, Architecture::Encoder);
        let gpu_weights = upload_weights(raw_weights, &pipelines, config.block_size, force_f32)?;

        // Create executor config
        let exec_config = ExecConfig {
            hidden_size: config.hidden_size,
            num_heads: config.num_heads,
            kv_num_heads: config.kv_num_heads,
            head_dim: config.head_dim,
            vocab_size: config.vocab_size,
            block_size: config.block_size,
            default_quant,
            rope_theta: config.rope_theta,
            has_qk_norm: config.has_qk_norm,
        };

        let executor = GraphExecutor::new(pipelines.clone(), exec_config);

        // Initialize KV cache
        let kv_cache: Vec<ExecKVCache> = (0..config.num_layers)
            .map(|_| ExecKVCache { key: None, value: None })
            .collect();

        // Precompute RoPE caches (only needed for decoder models)
        let (cos_cache, sin_cache) = if config.rope_theta > 0.0 && config.head_dim > 0 {
            precompute_rope_caches(
                config.head_dim as usize,
                config.max_seq_len as usize,
                config.rope_theta,
            )
        } else {
            (Vec::new(), Vec::new())
        };

        Ok(Self {
            graph,
            executor,
            weights: gpu_weights,
            pipelines,
            architecture,
            kv_cache,
            past_seq_len: 0,
            cos_cache,
            sin_cache,
            config,
        })
    }

    /// Run encoder forward pass (BERT, Whisper encoder).
    ///
    /// Returns output tensor as Vec<f32>.
    /// `output_name` specifies which tensor to read back (e.g. "pooler_output").
    pub fn encode(
        &self,
        input_ids: &[u32],
        output_name: &str,
        output_size: usize,
    ) -> Result<Vec<f32>, String> {
        let p = &self.pipelines;

        // Upload input
        let ids_f32: Vec<f32> = input_ids.iter().map(|&id| id as f32).collect();
        let ids_buf = p.upload_f32(&ids_f32);

        let mut input_bufs = HashMap::new();
        input_bufs.insert("input_ids".to_string(), ids_buf);

        // For BERT, also need position_ids and token_type_ids
        let seq_len = input_ids.len();
        let pos_ids: Vec<f32> = (0..seq_len).map(|i| i as f32).collect();
        let pos_buf = p.upload_f32(&pos_ids);
        input_bufs.insert("position_ids".to_string(), pos_buf);

        let type_ids: Vec<f32> = vec![0.0; seq_len];
        let type_buf = p.upload_f32(&type_ids);
        input_bufs.insert("token_type_ids".to_string(), type_buf);

        let result = self.executor.execute_encode_with_seq_len(
            &self.graph,
            input_bufs,
            &self.weights,
            output_name,
            output_size,
            seq_len,
        );

        Ok(result)
    }

    /// Run encoder forward pass with raw f32 input (Whisper mel spectrogram).
    ///
    /// Unlike `encode()` which expects integer token IDs (BERT),
    /// this takes a named f32 tensor as input and runs the encoder graph.
    pub fn encode_audio(
        &self,
        input_name: &str,
        input_data: &[f32],
        output_name: &str,
        output_size: usize,
    ) -> Result<Vec<f32>, String> {
        let p = &self.pipelines;

        let input_buf = p.upload_f32(input_data);

        let mut input_bufs = HashMap::new();
        input_bufs.insert(input_name.to_string(), input_buf);

        let result = self.executor.execute_encode(
            &self.graph,
            input_bufs,
            &self.weights,
            output_name,
            output_size,
        );

        Ok(result)
    }

    /// Run decoder forward pass (autoregressive step).
    ///
    /// Returns logits as Vec<f32> of size vocab_size.
    pub fn forward(&mut self, token_ids: &[u32]) -> Vec<f32> {
        let logits = self.executor.execute_decode(
            &self.graph,
            token_ids,
            &self.weights,
            &mut self.kv_cache,
            self.past_seq_len,
            &self.cos_cache,
            &self.sin_cache,
        );

        self.past_seq_len += token_ids.len();
        logits
    }

    /// Reset KV cache for a new sequence.
    pub fn reset_kv_cache(&mut self) {
        self.past_seq_len = 0;
        for cache in &mut self.kv_cache {
            cache.key = None;
            cache.value = None;
        }
    }

    /// Get the architecture type
    pub fn architecture(&self) -> &Architecture {
        &self.architecture
    }
}

// ========================================================================
// Helper functions
// ========================================================================

/// Detect the predominant quantization format from weight data
fn detect_quant_format(weights: &HashMap<String, WeightData>) -> GpuQuantFormat {
    let mut q4_count = 0;
    let mut q8_count = 0;
    let mut f16_count = 0;
    let mut f32_count = 0;

    for w in weights.values() {
        match w.dtype {
            DType::Q4 | DType::Q4_1 => q4_count += 1,
            DType::Q8 => q8_count += 1,
            DType::F16 | DType::BF16 => f16_count += 1,
            DType::F32 => f32_count += 1,
            _ => {}
        }
    }

    if q4_count > q8_count && q4_count > f16_count && q4_count > f32_count {
        GpuQuantFormat::Q4
    } else if q8_count > f16_count && q8_count > f32_count {
        GpuQuantFormat::Q8
    } else if f16_count > f32_count {
        GpuQuantFormat::F16
    } else {
        GpuQuantFormat::F32
    }
}

/// Upload raw weight data to GPU buffers
fn upload_weights(
    weights: &HashMap<String, WeightData>,
    pipelines: &Pipelines,
    block_size: u32,
    force_f32: bool,
) -> Result<HashMap<String, GpuWeight>, String> {
    let mut gpu_weights = HashMap::new();

    for (name, w) in weights {
        if force_f32 && w.shape.len() >= 2 {
            // Encoder: dequant everything to f32 for batch matmul compatibility
            let f32s = crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
            let buf = pipelines.upload_f32(&f32s);
            gpu_weights.insert(name.clone(), GpuWeight {
                buf, scales: None, quant: GpuQuantFormat::F32, shape: w.shape.clone(),
            });
            continue;
        }
        let (buf, scales, quant) = upload_single_weight(w, pipelines, block_size)?;
        gpu_weights.insert(name.clone(), GpuWeight {
            buf,
            scales,
            quant,
            shape: w.shape.clone(),
        });
    }

    log::info!("Uploaded {} weights to GPU", gpu_weights.len());
    Ok(gpu_weights)
}

/// Upload a single weight tensor to GPU
fn upload_single_weight(
    w: &WeightData,
    p: &Pipelines,
    block_size: u32,
) -> Result<(wgpu::Buffer, Option<wgpu::Buffer>, GpuQuantFormat), String> {
    match w.dtype {
        DType::F32 => {
            let buf = p.upload_bytes(&w.data);
            Ok((buf, None, GpuQuantFormat::F32))
        }
        DType::F16 => {
            let buf = p.upload_bytes(&w.data);
            Ok((buf, None, GpuQuantFormat::F16))
        }
        DType::BF16 => {
            // Convert BF16 to F32 for GPU
            let f32_data: Vec<f32> = w.data.chunks_exact(2)
                .map(|c| {
                    let bits = u16::from_le_bytes([c[0], c[1]]);
                    f32::from_bits((bits as u32) << 16)
                })
                .collect();
            let buf = p.upload_f32(&f32_data);
            Ok((buf, None, GpuQuantFormat::F32))
        }
        DType::Q4 | DType::Q4_1 => {
            // Q4_0 format: blocks of 32 elements, each block = 2 bytes scale + 16 bytes data
            let num_elements: usize = w.shape.iter().product();
            let bs = block_size as usize;
            let num_blocks = num_elements / bs;
            let block_bytes = if w.dtype == DType::Q4 { 18 } else { 20 };

            // Extract scales and packed weights
            let mut scales = Vec::with_capacity(num_blocks);
            let mut packed = Vec::new();

            for b in 0..num_blocks {
                let block_start = b * block_bytes;
                if block_start + block_bytes > w.data.len() {
                    return Err(format!("Q4 weight data too short for {} blocks", num_blocks));
                }

                let scale = f32::from(half::f16::from_le_bytes([
                    w.data[block_start],
                    w.data[block_start + 1],
                ]));
                scales.push(scale);

                let data_start = if w.dtype == DType::Q4 {
                    block_start + 2
                } else {
                    block_start + 4  // Q4_1 has min after scale
                };
                packed.extend_from_slice(&w.data[data_start..data_start + (bs / 2)]);
            }

            let packed_u32: Vec<u32> = packed.chunks(4)
                .map(|c| {
                    let mut val = 0u32;
                    for (i, &b) in c.iter().enumerate() {
                        val |= (b as u32) << (i * 8);
                    }
                    val
                })
                .collect();

            let packed_buf = p.upload_u32(&packed_u32);
            let scales_buf = p.upload_f32(&scales);

            Ok((packed_buf, Some(scales_buf), GpuQuantFormat::Q4))
        }
        DType::Q8 => {
            // Q8_0 format: blocks of 32 elements, each block = 2 bytes scale + 32 bytes data
            let num_elements: usize = w.shape.iter().product();
            let bs = block_size as usize;
            let num_blocks = num_elements / bs;

            let mut scales = Vec::with_capacity(num_blocks);
            let mut packed = Vec::new();

            for b in 0..num_blocks {
                let block_start = b * 34;
                if block_start + 34 > w.data.len() {
                    // Data doesn't match GGUF Q8_0 format — treat as raw int8/f32 fallback
                    log::warn!("Q8 format mismatch (expected {} bytes, got {}), using f32 fallback",
                        num_blocks * 34, w.data.len());
                    let f32s = crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
                    let buf = p.upload_f32(&f32s);
                    return Ok((buf, None, GpuQuantFormat::F32));
                }

                let scale = f32::from(half::f16::from_le_bytes([
                    w.data[block_start],
                    w.data[block_start + 1],
                ]));
                scales.push(scale);
                packed.extend_from_slice(&w.data[block_start + 2..block_start + 34]);
            }

            let packed_u32: Vec<u32> = packed.chunks(4)
                .map(|c| {
                    let mut val = 0u32;
                    for (i, &b) in c.iter().enumerate() {
                        val |= (b as u32) << (i * 8);
                    }
                    val
                })
                .collect();

            let packed_buf = p.upload_u32(&packed_u32);
            let scales_buf = p.upload_f32(&scales);

            Ok((packed_buf, Some(scales_buf), GpuQuantFormat::Q8))
        }
        _ => {
            // Default: treat as F32
            let buf = p.upload_bytes(&w.data);
            Ok((buf, None, GpuQuantFormat::F32))
        }
    }
}

/// Precompute RoPE cos/sin caches
fn precompute_rope_caches(
    head_dim: usize,
    max_seq_len: usize,
    theta: f32,
) -> (Vec<f32>, Vec<f32>) {
    let half = head_dim / 2;
    let mut cos_cache = vec![0.0f32; max_seq_len * half];
    let mut sin_cache = vec![0.0f32; max_seq_len * half];

    for pos in 0..max_seq_len {
        for i in 0..half {
            let freq = 1.0 / theta.powf(2.0 * i as f32 / head_dim as f32);
            let angle = pos as f32 * freq;
            cos_cache[pos * half + i] = angle.cos();
            sin_cache[pos * half + i] = angle.sin();
        }
    }

    (cos_cache, sin_cache)
}

