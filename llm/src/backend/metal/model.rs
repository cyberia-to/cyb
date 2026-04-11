//! MetalModel — full transformer inference via aruminium
//!
//! All ops in MSL, all fp16, single dispatch_batch per decode step.
//! Weights: safetensors f32 → Q4 quantize-on-load (block_q4_0 format).

use std::path::Path;
use aruminium::Buffer;
use super::pipelines::MetalPipelines;

const DEFAULT_MAX_SEQ: usize = 2048;

pub struct MetalModelConfig {
    pub hidden_size: usize,
    pub num_heads: usize,
    pub kv_num_heads: usize,
    pub head_dim: usize,
    pub num_layers: usize,
    pub vocab_size: usize,
    pub intermediate_size: usize,
    pub has_qk_norm: bool,
    pub is_ternary: bool,
    pub use_f16: bool,
    pub max_seq: usize,
}

struct MetalLayerWeights {
    input_norm_weight: Buffer,
    q_proj: Buffer,
    k_proj: Buffer,
    v_proj: Buffer,
    o_proj: Buffer,
    q_proj_bias: Option<Buffer>,
    k_proj_bias: Option<Buffer>,
    v_proj_bias: Option<Buffer>,
    q_norm_weight: Option<Buffer>,
    k_norm_weight: Option<Buffer>,
    post_norm_weight: Buffer,
    gate_proj: Buffer,
    up_proj: Buffer,
    down_proj: Buffer,
    /// true = Q4_K format (use matvec_q4k kernel), false = Q4_0 (use matvec_q4)
    is_k_quant: bool,
    // Ternary weight scales (BitNet)
    weight_scales: Option<[f32; 7]>, // q,k,v,o,gate,up,down
    // Sub-layer norms (BitNet)
    attn_sub_norm: Option<Buffer>,
    ffn_sub_norm: Option<Buffer>,
}

struct MetalKVCache {
    k: Buffer, // [kv_heads, MAX_SEQ, head_dim] fp16
    v: Buffer,
}

struct Scratch {
    hidden: Buffer,
    hidden2: Buffer,
    q: Buffer,
    k: Buffer,
    v: Buffer,
    attn_out: Buffer,
    gate: Buffer,
    up: Buffer,
    down: Buffer,
    logits: Buffer,
    argmax_result: Buffer,
    argmax_partial_vals: Buffer,
    argmax_partial_idxs: Buffer,
}

pub struct MetalModel {
    pipelines: MetalPipelines,
    config: MetalModelConfig,
    embed_table: Buffer,
    lm_head_q4: Buffer,
    lm_head_is_k_quant: bool,
    lm_head_f16: Option<Buffer>,
    final_norm_weight: Buffer,
    token_buf: Buffer,      // pre-allocated, rewritten each step
    layers: Vec<MetalLayerWeights>,
    cos_cache: Buffer,
    sin_cache: Buffer,
    kv_cache: Vec<MetalKVCache>,
    past_seq_len: usize,
    scratch: Scratch,
}

fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

impl MetalModel {
    /// Load from .model file via LoadedModel abstraction
    pub fn load(path: &Path) -> Result<Self, String> {
        let lm = crate::cyb_format::LoadedModel::load(path)
            .map_err(|e| format!("Cannot read .model: {e}"))?;
        Self::from_loaded(lm)
    }

    /// Build from pre-parsed LoadedModel — shared with other backends
    pub fn from_loaded(lm: crate::cyb_format::LoadedModel) -> Result<Self, String> {
        let pipelines = MetalPipelines::new().map_err(|e| format!("Metal init: {e}"))?;

        let cj_root = lm.config_json();
        let cj = cj_root.get("architecture").unwrap_or(&cj_root);

        let hidden_size = cj["hidden_size"].as_u64().ok_or("missing hidden_size")? as usize;
        let num_heads = cj["num_attention_heads"].as_u64().ok_or("missing num_attention_heads")? as usize;
        let kv_num_heads = cj["num_key_value_heads"].as_u64().unwrap_or(num_heads as u64) as usize;
        let num_layers = cj["num_hidden_layers"].as_u64().ok_or("missing num_hidden_layers")? as usize;
        let vocab_size = cj["vocab_size"].as_u64().ok_or("missing vocab_size")? as usize;
        let intermediate_size = cj["intermediate_size"].as_u64().unwrap_or((hidden_size * 4) as u64) as usize;
        let rope_theta = cj_root.get("architecture")
            .and_then(|a| a["rope_theta"].as_f64())
            .or_else(|| cj_root["rope_theta"].as_f64())
            .unwrap_or(10000.0) as f32;
        let _rms_norm_eps = cj["rms_norm_eps"].as_f64().unwrap_or(1e-6) as f32;
        let tie_word_embeddings = cj_root.get("tie_word_embeddings")
            .or_else(|| cj.get("tie_word_embeddings"))
            .and_then(|v| v.as_bool()).unwrap_or(true);
        let max_seq = cj["max_position_embeddings"].as_u64()
            .or_else(|| cj["context_length"].as_u64())
            .map(|v| (v as usize).min(8192))
            .unwrap_or(DEFAULT_MAX_SEQ);

        let use_f16 = false; // Q4 by default

        let head_dim = cj["head_dim"].as_u64().map(|v| v as usize).unwrap_or_else(|| {
            if let Some(w) = lm.weights.get("model.layers.0.self_attn.q_proj.weight") {
                let q_dim = w.shape[0];
                q_dim / num_heads
            } else { hidden_size / num_heads }
        });

        let has_qk_norm = lm.weights.contains_key("model.layers.0.self_attn.q_norm.weight");
        let has_attn_bias = lm.weights.contains_key("model.layers.0.self_attn.q_proj.bias");

        let is_ternary = cj_root.get("model_type").and_then(|v| v.as_str()) == Some("bitnet");
        if is_ternary {
            log::info!("BitNet model detected — using native ternary weights");
        }

        let config = MetalModelConfig {
            hidden_size, num_heads, kv_num_heads, head_dim,
            num_layers, vocab_size, intermediate_size, has_qk_norm, is_ternary, use_f16, max_seq,
        };

        log::info!("Metal model: hidden={hidden_size}, heads={num_heads}, kv_heads={kv_num_heads}, layers={num_layers}, vocab={vocab_size}");

        let weights = &lm.weights;

        let weight_to_f32 = |name: &str| -> Result<Vec<f32>, String> {
            let w = weights.get(name).ok_or_else(|| format!("Missing: {name}"))?;
            let mut f32s = crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
            if w.needs_transpose && w.shape.len() == 2 {
                f32s = crate::backend::wgpu::model::transpose_f32(&f32s, w.shape[1], w.shape[0]);
            }
            Ok(f32s)
        };

        let weight_to_f16 = |name: &str| -> Result<Vec<u16>, String> {
            let w = weights.get(name).ok_or_else(|| format!("Missing: {name}"))?;
            let mut f32s = crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
            if w.needs_transpose && w.shape.len() == 2 {
                f32s = crate::backend::wgpu::model::transpose_f32(&f32s, w.shape[1], w.shape[0]);
            }
            Ok(f32s.iter().map(|&v| aruminium::f32_to_fp16(v)).collect())
        };

        // Embed table (fp16)
        let embed_f16 = weight_to_f16("model.embed_tokens.weight")?;
        let embed_table = pipelines.upload_f16(&embed_f16).map_err(|e| format!("{e}"))?;

        // LM head
        let lm_w = weights.get("lm_head.weight")
            .or_else(|| weights.get("model.embed_tokens.weight"))
            .ok_or("Missing lm_head or embed weights")?;
        let lm_head_is_k_quant = matches!(lm_w.dtype, crate::ir::DType::Q4_K | crate::ir::DType::Q6_K);
        let lm_head_q4 = if matches!(lm_w.dtype, crate::ir::DType::Q4) {
            let transposed = transpose_q4_blocks(&lm_w.data, vocab_size, hidden_size);
            pipelines.upload_bytes(&transposed).map_err(|e| format!("{e}"))?
        } else if matches!(lm_w.dtype, crate::ir::DType::Q4_K) {
            let transposed = transpose_blocks(&lm_w.data, vocab_size, hidden_size, 256, 144);
            pipelines.upload_bytes(&transposed).map_err(|e| format!("{e}"))?
        } else if matches!(lm_w.dtype, crate::ir::DType::Q6_K) {
            let transposed = transpose_blocks(&lm_w.data, vocab_size, hidden_size, 256, 210);
            pipelines.upload_bytes(&transposed).map_err(|e| format!("{e}"))?
        } else {
            let f32s = crate::backend::wgpu::model::safetensors_to_f32(&lm_w.data, lm_w.dtype);
            let packed = quantize_f32_to_block_q4_0(&f32s, vocab_size, hidden_size);
            pipelines.upload_bytes(&packed).map_err(|e| format!("{e}"))?
        };

        // LM head f16 (for f16 mode)
        let lm_head_f16 = if use_f16 {
            let lm_f16 = if !tie_word_embeddings {
                weight_to_f16("lm_head.weight").unwrap_or_else(|_| embed_f16.clone())
            } else {
                embed_f16.clone()
            };
            Some(pipelines.upload_f16(&lm_f16).map_err(|e| format!("{e}"))?)
        } else { None };

        // Final norm
        let final_norm_f16 = weight_to_f16("model.norm.weight")?;
        let final_norm_weight = pipelines.upload_f16(&final_norm_f16).map_err(|e| format!("{e}"))?;

        let q_dim = num_heads * head_dim;
        let kv_dim = kv_num_heads * head_dim;

        // Upload projection as fp16 (no quantization)
        let f16_upload = |name: &str| -> Result<Buffer, String> {
            let f16 = weight_to_f16(name)?;
            pipelines.upload_f16(&f16).map_err(|e| format!("{e}"))
        };

        // Upload quantized weights — transpose block layout for Metal kernels
        let q4_upload = |name: &str, n: usize, k: usize| -> Result<(Buffer, bool), String> {
            let w = weights.get(name).ok_or_else(|| format!("Missing: {name}"))?;
            match w.dtype {
                crate::ir::DType::Q4 => {
                    // Q4_0: 18 bytes/32 values, transpose [N][K/32] → [K/32][N] + nibble repack
                    let transposed = transpose_q4_blocks(&w.data, n, k);
                    Ok((pipelines.upload_bytes(&transposed).map_err(|e| format!("{e}"))?, false))
                }
                crate::ir::DType::Q4_K => {
                    // Q4_K: 144 bytes/256 values, transpose [N][K/256] → [K/256][N], no nibble repack
                    let transposed = transpose_blocks(&w.data, n, k, 256, 144);
                    Ok((pipelines.upload_bytes(&transposed).map_err(|e| format!("{e}"))?, true))
                }
                crate::ir::DType::Q6_K => {
                    // Q6_K: 210 bytes/256 values, transpose [N][K/256] → [K/256][N]
                    let transposed = transpose_blocks(&w.data, n, k, 256, 210);
                    Ok((pipelines.upload_bytes(&transposed).map_err(|e| format!("{e}"))?, true))
                }
                _ => {
                    // Non-quantized: dequant → requant to Q4_0
                    let f32s = crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
                    let packed = quantize_f32_to_block_q4_0(&f32s, n, k);
                    Ok((pipelines.upload_bytes(&packed).map_err(|e| format!("{e}"))?, false))
                }
            }
        };

        // Helper: load raw ternary bytes + extract weight_scale
        let ternary_upload = |name: &str| -> Result<(Buffer, f32), String> {
            let w = weights.get(name).ok_or_else(|| format!("Missing: {name}"))?;
            let buf = pipelines.upload_bytes(&w.data).map_err(|e| format!("{e}"))?;
            let scale_name = format!("{name}_scale");
            let scale = if let Some(sw) = weights.get(&scale_name) {
                let sv = crate::backend::wgpu::model::safetensors_to_f32(&sw.data, sw.dtype);
                if !sv.is_empty() { sv[0] } else { 1.0 }
            } else { 1.0 };
            Ok((buf, scale))
        };

        // Load layers
        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            log::info!("Metal: loading layer {i}/{num_layers}");

            let input_norm_weight = pipelines.upload_f16(
                &weight_to_f16(&format!("model.layers.{i}.input_layernorm.weight"))?
            ).map_err(|e| format!("{e}"))?;

            let (q_proj, k_proj, v_proj, o_proj, gate_proj, up_proj, down_proj, weight_scales);
            let mut is_k = false;
            if is_ternary {
                let (qp, qs) = ternary_upload(&format!("model.layers.{i}.self_attn.q_proj.weight"))?;
                let (kp, ks) = ternary_upload(&format!("model.layers.{i}.self_attn.k_proj.weight"))?;
                let (vp, vs) = ternary_upload(&format!("model.layers.{i}.self_attn.v_proj.weight"))?;
                let (op, os) = ternary_upload(&format!("model.layers.{i}.self_attn.o_proj.weight"))?;
                let (gp, gs) = ternary_upload(&format!("model.layers.{i}.mlp.gate_proj.weight"))?;
                let (up_, us) = ternary_upload(&format!("model.layers.{i}.mlp.up_proj.weight"))?;
                let (dp, ds) = ternary_upload(&format!("model.layers.{i}.mlp.down_proj.weight"))?;
                q_proj = qp; k_proj = kp; v_proj = vp; o_proj = op;
                gate_proj = gp; up_proj = up_; down_proj = dp;
                weight_scales = Some([qs, ks, vs, os, gs, us, ds]);
            } else if use_f16 {
                q_proj = f16_upload(&format!("model.layers.{i}.self_attn.q_proj.weight"))?;
                k_proj = f16_upload(&format!("model.layers.{i}.self_attn.k_proj.weight"))?;
                v_proj = f16_upload(&format!("model.layers.{i}.self_attn.v_proj.weight"))?;
                o_proj = f16_upload(&format!("model.layers.{i}.self_attn.o_proj.weight"))?;
                gate_proj = f16_upload(&format!("model.layers.{i}.mlp.gate_proj.weight"))?;
                up_proj = f16_upload(&format!("model.layers.{i}.mlp.up_proj.weight"))?;
                down_proj = f16_upload(&format!("model.layers.{i}.mlp.down_proj.weight"))?;
                weight_scales = None;
            } else {
                let (qp, kq) = q4_upload(&format!("model.layers.{i}.self_attn.q_proj.weight"), q_dim, hidden_size)?;
                q_proj = qp; is_k = kq;
                let (kp, _) = q4_upload(&format!("model.layers.{i}.self_attn.k_proj.weight"), kv_dim, hidden_size)?;
                k_proj = kp;
                let (vp, _) = q4_upload(&format!("model.layers.{i}.self_attn.v_proj.weight"), kv_dim, hidden_size)?;
                v_proj = vp;
                let (op, _) = q4_upload(&format!("model.layers.{i}.self_attn.o_proj.weight"), hidden_size, q_dim)?;
                o_proj = op;
                let (gp, _) = q4_upload(&format!("model.layers.{i}.mlp.gate_proj.weight"), intermediate_size, hidden_size)?;
                gate_proj = gp;
                let (up_, _) = q4_upload(&format!("model.layers.{i}.mlp.up_proj.weight"), intermediate_size, hidden_size)?;
                up_proj = up_;
                let (dp, _) = q4_upload(&format!("model.layers.{i}.mlp.down_proj.weight"), hidden_size, intermediate_size)?;
                down_proj = dp;
                weight_scales = None;
            };

            let q_proj_bias = if has_attn_bias {
                Some(pipelines.upload_f16(&weight_to_f16(&format!("model.layers.{i}.self_attn.q_proj.bias"))?).map_err(|e| format!("{e}"))?)
            } else { None };
            let k_proj_bias = if has_attn_bias {
                Some(pipelines.upload_f16(&weight_to_f16(&format!("model.layers.{i}.self_attn.k_proj.bias"))?).map_err(|e| format!("{e}"))?)
            } else { None };
            let v_proj_bias = if has_attn_bias {
                Some(pipelines.upload_f16(&weight_to_f16(&format!("model.layers.{i}.self_attn.v_proj.bias"))?).map_err(|e| format!("{e}"))?)
            } else { None };

            let q_norm_weight = if has_qk_norm {
                Some(pipelines.upload_f16(&weight_to_f16(&format!("model.layers.{i}.self_attn.q_norm.weight"))?).map_err(|e| format!("{e}"))?)
            } else { None };
            let k_norm_weight = if has_qk_norm {
                Some(pipelines.upload_f16(&weight_to_f16(&format!("model.layers.{i}.self_attn.k_norm.weight"))?).map_err(|e| format!("{e}"))?)
            } else { None };

            let post_norm_weight = pipelines.upload_f16(
                &weight_to_f16(&format!("model.layers.{i}.post_attention_layernorm.weight"))?
            ).map_err(|e| format!("{e}"))?;

            // Sub-layer norms (BitNet)
            let attn_sub_norm = if is_ternary {
                weight_to_f16(&format!("model.layers.{i}.self_attn.attn_sub_norm.weight")).ok()
                    .and_then(|d| pipelines.upload_f16(&d).ok())
            } else { None };
            let ffn_sub_norm = if is_ternary {
                weight_to_f16(&format!("model.layers.{i}.mlp.ffn_sub_norm.weight")).ok()
                    .and_then(|d| pipelines.upload_f16(&d).ok())
            } else { None };

            layers.push(MetalLayerWeights {
                input_norm_weight, q_proj, k_proj, v_proj, o_proj,
                q_proj_bias, k_proj_bias, v_proj_bias,
                q_norm_weight, k_norm_weight,
                post_norm_weight, gate_proj, up_proj, down_proj,
                is_k_quant: is_k,
                weight_scales, attn_sub_norm, ffn_sub_norm,
            });
        }

        // RoPE cache (fp16)
        let half_dim = head_dim / 2;
        let mut cos_f16 = vec![0u16; max_seq * half_dim];
        let mut sin_f16 = vec![0u16; max_seq * half_dim];
        for pos in 0..max_seq {
            for i in 0..half_dim {
                let freq = 1.0 / rope_theta.powf(2.0 * i as f32 / head_dim as f32);
                let angle = pos as f32 * freq;
                cos_f16[pos * half_dim + i] = aruminium::f32_to_fp16(angle.cos());
                sin_f16[pos * half_dim + i] = aruminium::f32_to_fp16(angle.sin());
            }
        }
        let cos_cache = pipelines.upload_f16(&cos_f16).map_err(|e| format!("{e}"))?;
        let sin_cache = pipelines.upload_f16(&sin_f16).map_err(|e| format!("{e}"))?;

        // KV cache (pre-allocated to MAX_SEQ)
        let kv_buf_size = kv_num_heads * max_seq * head_dim * 2; // fp16
        let mut kv_cache = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            kv_cache.push(MetalKVCache {
                k: pipelines.alloc(kv_buf_size).map_err(|e| format!("{e}"))?,
                v: pipelines.alloc(kv_buf_size).map_err(|e| format!("{e}"))?,
            });
        }

        // Scratch buffers (fp16)
        let alloc_f16 = |n: usize| -> Result<Buffer, String> {
            pipelines.alloc(n * 2).map_err(|e| format!("{e}"))
        };
        let scratch = Scratch {
            hidden: alloc_f16(hidden_size)?,
            hidden2: alloc_f16(hidden_size)?,
            q: alloc_f16(q_dim)?,
            k: alloc_f16(kv_dim)?,
            v: alloc_f16(kv_dim)?,
            attn_out: alloc_f16(q_dim)?,
            gate: alloc_f16(intermediate_size)?,
            up: alloc_f16(intermediate_size)?,
            down: alloc_f16(hidden_size)?,
            logits: alloc_f16(vocab_size)?,
            argmax_result: pipelines.alloc(4).map_err(|e| format!("{e}"))?, // u32
            argmax_partial_vals: pipelines.alloc(64 * 4).map_err(|e| format!("{e}"))?, // 64 floats
            argmax_partial_idxs: pipelines.alloc(64 * 4).map_err(|e| format!("{e}"))?, // 64 u32s
        };

        log::info!("Metal model loaded: {} layers, Q4 weights, fp16 activations", num_layers);

        // Pre-allocate token buffer (rewritten each step, avoids allocation)
        let token_buf = pipelines.alloc(4).map_err(|e| format!("{e}"))?;

        Ok(MetalModel {
            pipelines, config, embed_table, lm_head_q4, lm_head_is_k_quant, lm_head_f16, final_norm_weight,
            token_buf, layers, cos_cache, sin_cache, kv_cache, past_seq_len: 0, scratch,
        })
    }

    /// Decode one token — returns next token ID.
    /// All ops encoded into a single command buffer via BatchEncoder.
    pub fn forward_decode(&mut self, token_id: u32) -> u32 {
        let p = &self.pipelines;
        let c = &self.config;
        let pos = self.past_seq_len;
        let total_seq = pos + 1;
        let half_dim = c.head_dim / 2;
        let scale = 1.0 / (c.head_dim as f32).sqrt();

        // Write token ID to pre-allocated buffer (no allocation)
        self.token_buf.write(|d| {
            d[..4].copy_from_slice(&token_id.to_le_bytes());
        });

        unsafe { aruminium::autorelease_pool(|| {
            p.dispatcher.batch_raw(|batch| {
                // ── Embed ──
                let embed_params = [c.hidden_size as u32];
                batch.bind(&p.embed);
                batch.bind_buffer(&self.embed_table, 0, 0);
                batch.bind_buffer(&self.token_buf, 0, 1);
                batch.bind_buffer(&self.scratch.hidden, 0, 2);
                batch.push(bytemuck::cast_slice(&embed_params), 3);
                batch.launch_groups((div_ceil(c.hidden_size, 256), 1, 1), (256, 1, 1));

                let norm_params_bytes = {
                    let p = [c.hidden_size as u32, 0u32]; // hidden + padding for eps
                    let mut b = bytemuck::bytes_of(&p).to_vec();
                    // Replace last 4 bytes with eps as f32
                    let eps_bytes = 1e-6f32.to_le_bytes();
                    b[4..8].copy_from_slice(&eps_bytes);
                    b
                };

                // ── Layer 0 input norm (subsequent layers get norm from fused_add_norm) ──
                batch.bind(&p.rms_norm);
                batch.bind_buffer(&self.scratch.hidden, 0, 0);
                batch.bind_buffer(&self.layers[0].input_norm_weight, 0, 1);
                batch.bind_buffer(&self.scratch.hidden2, 0, 2);
                batch.push(&norm_params_bytes, 3);
                batch.launch_groups((1, 1, 1), (256, 1, 1));

                for i in 0..c.num_layers {
                    let layer = &self.layers[i];
                    let kv = &self.kv_cache[i];
                    // hidden2 is already normed (from prev layer's fused_add_norm or initial norm)

                    let q_dim_val = (c.num_heads * c.head_dim) as u32;
                    let kv_dim_val = (c.kv_num_heads * c.head_dim) as u32;

                    if c.is_ternary {
                        // ── Ternary Q/K/V projections (3 separate dispatches) ──
                        let ws = layer.weight_scales.unwrap_or([1.0; 7]);

                        let dispatch_ternary = |batch: &aruminium::Batch, input: &Buffer, w: &Buffer, out: &Buffer, n: u32, k: u32, scale: f32| {
                            let params = [n, k, scale.to_bits()];
                            batch.bind(&p.matvec_ternary);
                            batch.bind_buffer(input, 0, 0);
                            batch.bind_buffer(w, 0, 1);
                            batch.bind_buffer(out, 0, 2);
                            batch.push(bytemuck::cast_slice(&params), 3);
                            batch.launch_groups((div_ceil(n as usize, 8), 1, 1), (256, 1, 1));
                        };

                        dispatch_ternary(batch, &self.scratch.hidden2, &layer.q_proj, &self.scratch.q, q_dim_val, c.hidden_size as u32, ws[0]);
                        dispatch_ternary(batch, &self.scratch.hidden2, &layer.k_proj, &self.scratch.k, kv_dim_val, c.hidden_size as u32, ws[1]);
                        dispatch_ternary(batch, &self.scratch.hidden2, &layer.v_proj, &self.scratch.v, kv_dim_val, c.hidden_size as u32, ws[2]);
                    } else if c.use_f16 {
                        // ── f16 Q/K/V projections (3 separate dispatches) ──
                        let dispatch_f16 = |batch: &aruminium::Batch, input: &Buffer, w: &Buffer, out: &Buffer, n: u32, k: u32| {
                            let params = [n, k];
                            batch.bind(&p.f16_matvec);
                            batch.bind_buffer(input, 0, 0);
                            batch.bind_buffer(w, 0, 1);
                            batch.bind_buffer(out, 0, 2);
                            batch.push(bytemuck::cast_slice(&params), 3);
                            batch.launch_groups((div_ceil(n as usize, 4), 1, 1), (256, 1, 1));
                        };
                        dispatch_f16(batch, &self.scratch.hidden2, &layer.q_proj, &self.scratch.q, q_dim_val, c.hidden_size as u32);
                        dispatch_f16(batch, &self.scratch.hidden2, &layer.k_proj, &self.scratch.k, kv_dim_val, c.hidden_size as u32);
                        dispatch_f16(batch, &self.scratch.hidden2, &layer.v_proj, &self.scratch.v, kv_dim_val, c.hidden_size as u32);
                    } else if layer.is_k_quant {
                        // ── Q4_K Q/K/V projections (3 separate dispatches with matvec_q4k) ──
                        let dispatch_q4k = |batch: &aruminium::Batch, input: &Buffer, w: &Buffer, out: &Buffer, n: u32, k: u32| {
                            let params = [n, k];
                            batch.bind(&p.matvec_q4k);
                            batch.bind_buffer(input, 0, 0);
                            batch.bind_buffer(w, 0, 1);
                            batch.bind_buffer(out, 0, 2);
                            batch.push(bytemuck::cast_slice(&params), 3);
                            batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
                        };
                        dispatch_q4k(batch, &self.scratch.hidden2, &layer.q_proj, &self.scratch.q, q_dim_val, c.hidden_size as u32);
                        dispatch_q4k(batch, &self.scratch.hidden2, &layer.k_proj, &self.scratch.k, kv_dim_val, c.hidden_size as u32);
                        dispatch_q4k(batch, &self.scratch.hidden2, &layer.v_proj, &self.scratch.v, kv_dim_val, c.hidden_size as u32);
                    } else {
                        // ── Fused Q+K+V projection (3→1 dispatch, Q4_0) ──
                        let wg_q = div_ceil(q_dim_val as usize, 8);
                        let wg_k = div_ceil(kv_dim_val as usize, 8);
                        let wg_v = wg_k;
                        let qkv_params = [q_dim_val, kv_dim_val, kv_dim_val, c.hidden_size as u32, wg_q as u32, wg_k as u32];
                        batch.bind(&p.fused_qkv);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(&layer.q_proj, 0, 1);
                        batch.bind_buffer(&layer.k_proj, 0, 2);
                        batch.bind_buffer(&layer.v_proj, 0, 3);
                        batch.bind_buffer(&self.scratch.q, 0, 4);
                        batch.bind_buffer(&self.scratch.k, 0, 5);
                        batch.bind_buffer(&self.scratch.v, 0, 6);
                        batch.push(bytemuck::cast_slice(&qkv_params), 7);
                        batch.launch_groups((wg_q + wg_k + wg_v, 1, 1), (256, 1, 1));
                    }

                    // ── Attention biases (Qwen2: add bias to Q, K, V) ──
                    if let Some(ref qb) = layer.q_proj_bias {
                        let n = q_dim_val;
                        batch.bind(&p.add_f16);
                        batch.bind_buffer(&self.scratch.q, 0, 0);
                        batch.bind_buffer(qb, 0, 1);
                        batch.bind_buffer(&self.scratch.q, 0, 2);
                        batch.push(bytemuck::cast_slice(&[n]), 3);
                        batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
                    }
                    if let Some(ref kb) = layer.k_proj_bias {
                        let n = kv_dim_val;
                        batch.bind(&p.add_f16);
                        batch.bind_buffer(&self.scratch.k, 0, 0);
                        batch.bind_buffer(kb, 0, 1);
                        batch.bind_buffer(&self.scratch.k, 0, 2);
                        batch.push(bytemuck::cast_slice(&[n]), 3);
                        batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
                    }
                    if let Some(ref vb) = layer.v_proj_bias {
                        let n = kv_dim_val;
                        batch.bind(&p.add_f16);
                        batch.bind_buffer(&self.scratch.v, 0, 0);
                        batch.bind_buffer(vb, 0, 1);
                        batch.bind_buffer(&self.scratch.v, 0, 2);
                        batch.push(bytemuck::cast_slice(&[n]), 3);
                        batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
                    }

                    // ── QK-norm (Qwen3: per-head RMSNorm on Q and K) ──
                    // Uses rms_norm with head_dim, dispatches num_heads workgroups per call
                    if c.has_qk_norm {
                        let head_norm_params = {
                            let p = [c.head_dim as u32, 0u32];
                            let mut b = bytemuck::bytes_of(&p).to_vec();
                            b[4..8].copy_from_slice(&1e-6f32.to_le_bytes());
                            b
                        };
                        if let Some(ref qnw) = layer.q_norm_weight {
                            batch.bind(&p.rms_norm);
                            batch.bind_buffer(&self.scratch.q, 0, 0);
                            batch.bind_buffer(qnw, 0, 1);
                            batch.bind_buffer(&self.scratch.q, 0, 2);
                            batch.push(&head_norm_params, 3);
                            // One workgroup per head — each normalizes head_dim elements
                            batch.launch_groups((c.num_heads, 1, 1), (256, 1, 1));
                        }
                        if let Some(ref knw) = layer.k_norm_weight {
                            batch.bind(&p.rms_norm);
                            batch.bind_buffer(&self.scratch.k, 0, 0);
                            batch.bind_buffer(knw, 0, 1);
                            batch.bind_buffer(&self.scratch.k, 0, 2);
                            batch.push(&head_norm_params, 3);
                            batch.launch_groups((c.kv_num_heads, 1, 1), (256, 1, 1));
                        }
                    }

                    // ── Fused RoPE Q+K (1 dispatch instead of 2) ──
                    let rope_params = [half_dim as u32, c.head_dim as u32, c.num_heads as u32, c.kv_num_heads as u32];
                    let total_rope = half_dim * (c.num_heads + c.kv_num_heads);
                    batch.bind(&p.fused_rope_qk);
                    batch.bind_buffer(&self.scratch.q, 0, 0);
                    batch.bind_buffer(&self.scratch.k, 0, 1);
                    batch.bind_buffer(&self.cos_cache, pos * half_dim * 2, 2);
                    batch.bind_buffer(&self.sin_cache, pos * half_dim * 2, 3);
                    batch.push(bytemuck::cast_slice(&rope_params), 4);
                    batch.launch_groups((div_ceil(total_rope, 256), 1, 1), (256, 1, 1));

                    // ── Fused KV cache append K+V (1 dispatch instead of 2) ──
                    let append_params = [c.head_dim as u32, c.kv_num_heads as u32, pos as u32, c.max_seq as u32];
                    batch.bind(&p.fused_kv_append);
                    batch.bind_buffer(&self.scratch.k, 0, 0);
                    batch.bind_buffer(&self.scratch.v, 0, 1);
                    batch.bind_buffer(&kv.k, 0, 2);
                    batch.bind_buffer(&kv.v, 0, 3);
                    batch.push(bytemuck::cast_slice(&append_params), 4);
                    batch.launch_groups((div_ceil(c.kv_num_heads * c.head_dim, 256), 1, 1), (256, 1, 1));

                    // ── Attention decode (GQA-aware, KV cache stride = MAX_SEQ) ──
                    let attn_params = [c.head_dim as u32, total_seq as u32, c.num_heads as u32, scale.to_bits(), c.kv_num_heads as u32, c.max_seq as u32];
                    batch.bind(&p.attention_decode);
                    batch.bind_buffer(&self.scratch.q, 0, 0);
                    batch.bind_buffer(&kv.k, 0, 1);
                    batch.bind_buffer(&kv.v, 0, 2);
                    batch.bind_buffer(&self.scratch.attn_out, 0, 3);
                    batch.push(bytemuck::cast_slice(&attn_params), 4);
                    batch.launch_groups((c.num_heads, 1, 1), (256, 1, 1));

                    // ── Attention sub-norm (BitNet: RMSNorm before O_proj) ──
                    if let Some(ref asn) = layer.attn_sub_norm {
                        let sub_norm_params = {
                            let p = [(c.num_heads * c.head_dim) as u32, 0u32];
                            let mut b = bytemuck::bytes_of(&p).to_vec();
                            b[4..8].copy_from_slice(&1e-5f32.to_le_bytes());
                            b
                        };
                        batch.bind(&p.rms_norm);
                        batch.bind_buffer(&self.scratch.attn_out, 0, 0);
                        batch.bind_buffer(asn, 0, 1);
                        batch.bind_buffer(&self.scratch.attn_out, 0, 2);
                        batch.push(&sub_norm_params, 3);
                        batch.launch_groups((1, 1, 1), (256, 1, 1));
                    }

                    // ── O projection ──
                    {
                        let n = c.hidden_size as u32;
                        let k = (c.num_heads * c.head_dim) as u32;
                        let params = [n, k];
                        if c.is_ternary {
                            let ws = layer.weight_scales.unwrap_or([1.0; 7]);
                            let tp = [n, k, ws[3].to_bits()];
                            batch.bind(&p.matvec_ternary);
                            batch.bind_buffer(&self.scratch.attn_out, 0, 0);
                            batch.bind_buffer(&layer.o_proj, 0, 1);
                            batch.bind_buffer(&self.scratch.down, 0, 2);
                            batch.push(bytemuck::cast_slice(&tp), 3);
                            batch.launch_groups((div_ceil(n as usize, 8), 1, 1), (256, 1, 1));
                        } else if c.use_f16 {
                            batch.bind(&p.f16_matvec);
                            batch.bind_buffer(&self.scratch.attn_out, 0, 0);
                            batch.bind_buffer(&layer.o_proj, 0, 1);
                            batch.bind_buffer(&self.scratch.down, 0, 2);
                            batch.push(bytemuck::cast_slice(&params), 3);
                            batch.launch_groups((div_ceil(n as usize, 4), 1, 1), (256, 1, 1));
                        } else if layer.is_k_quant {
                            batch.bind(&p.matvec_q4k);
                            batch.bind_buffer(&self.scratch.attn_out, 0, 0);
                            batch.bind_buffer(&layer.o_proj, 0, 1);
                            batch.bind_buffer(&self.scratch.down, 0, 2);
                            batch.push(bytemuck::cast_slice(&params), 3);
                            batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
                        } else {
                            batch.bind(&p.matvec_q4_fast);
                            batch.bind_buffer(&self.scratch.attn_out, 0, 0);
                            batch.bind_buffer(&layer.o_proj, 0, 1);
                            batch.bind_buffer(&self.scratch.down, 0, 2);
                            batch.push(bytemuck::cast_slice(&params), 3);
                            batch.launch_groups((div_ceil(n as usize, 8), 1, 1), (256, 1, 1));
                        }
                    }

                    // ── Fused residual add + post RMS norm ──
                    batch.bind(&p.fused_add_norm);
                    batch.bind_buffer(&self.scratch.hidden, 0, 0);
                    batch.bind_buffer(&self.scratch.down, 0, 1);
                    batch.bind_buffer(&self.scratch.hidden, 0, 2);
                    batch.bind_buffer(&layer.post_norm_weight, 0, 3);
                    batch.bind_buffer(&self.scratch.hidden2, 0, 4);
                    batch.push(&norm_params_bytes, 5);
                    batch.launch_groups((1, 1, 1), (256, 1, 1));

                    // ── FFN projections ──
                    if c.is_ternary {
                        let ws = layer.weight_scales.unwrap_or([1.0; 7]);
                        // Gate projection
                        let params_g = [c.intermediate_size as u32, c.hidden_size as u32, ws[4].to_bits()];
                        batch.bind(&p.matvec_ternary);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(&layer.gate_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.gate, 0, 2);
                        batch.push(bytemuck::cast_slice(&params_g), 3);
                        batch.launch_groups((div_ceil(c.intermediate_size, 8), 1, 1), (256, 1, 1));
                        // Up projection
                        let params_u = [c.intermediate_size as u32, c.hidden_size as u32, ws[5].to_bits()];
                        batch.bind(&p.matvec_ternary);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(&layer.up_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.up, 0, 2);
                        batch.push(bytemuck::cast_slice(&params_u), 3);
                        batch.launch_groups((div_ceil(c.intermediate_size, 8), 1, 1), (256, 1, 1));
                    } else if c.use_f16 {
                        // f16 gate + up (separate dispatches)
                        let inter = c.intermediate_size as u32;
                        let hid = c.hidden_size as u32;
                        let f16_params = [inter, hid];
                        batch.bind(&p.f16_matvec);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(&layer.gate_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.gate, 0, 2);
                        batch.push(bytemuck::cast_slice(&f16_params), 3);
                        batch.launch_groups((div_ceil(inter as usize, 4), 1, 1), (256, 1, 1));

                        batch.bind(&p.f16_matvec);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(&layer.up_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.up, 0, 2);
                        batch.push(bytemuck::cast_slice(&f16_params), 3);
                        batch.launch_groups((div_ceil(inter as usize, 4), 1, 1), (256, 1, 1));
                    } else if layer.is_k_quant {
                        // Q4_K gate + up (separate dispatches)
                        let inter = c.intermediate_size as u32;
                        let hid = c.hidden_size as u32;
                        let q4k_params = [inter, hid];
                        batch.bind(&p.matvec_q4k);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(&layer.gate_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.gate, 0, 2);
                        batch.push(bytemuck::cast_slice(&q4k_params), 3);
                        batch.launch_groups((div_ceil(inter as usize, 256), 1, 1), (256, 1, 1));

                        batch.bind(&p.matvec_q4k);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(&layer.up_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.up, 0, 2);
                        batch.push(bytemuck::cast_slice(&q4k_params), 3);
                        batch.launch_groups((div_ceil(inter as usize, 256), 1, 1), (256, 1, 1));
                    } else {
                        let inter = c.intermediate_size as u32;
                        let wg_gate = div_ceil(c.intermediate_size, 8);
                        let gate_up_params = [inter, c.hidden_size as u32, wg_gate as u32];
                        batch.bind(&p.fused_gate_up);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(&layer.gate_proj, 0, 1);
                        batch.bind_buffer(&layer.up_proj, 0, 2);
                        batch.bind_buffer(&self.scratch.gate, 0, 3);
                        batch.bind_buffer(&self.scratch.up, 0, 4);
                        batch.push(bytemuck::cast_slice(&gate_up_params), 5);
                        batch.launch_groups((wg_gate * 2, 1, 1), (256, 1, 1));
                    }

                    // ── Activation: SiLU (default) or ReLU² (BitNet) ──
                    let act_params = [c.intermediate_size as u32];
                    if c.is_ternary {
                        batch.bind(&p.relu2_mul_f16);
                    } else {
                        batch.bind(&p.silu_mul_f16);
                    }
                    batch.bind_buffer(&self.scratch.gate, 0, 0);
                    batch.bind_buffer(&self.scratch.up, 0, 1);
                    batch.bind_buffer(&self.scratch.gate, 0, 2);
                    batch.push(bytemuck::cast_slice(&act_params), 3);
                    batch.launch_groups((div_ceil(c.intermediate_size, 256), 1, 1), (256, 1, 1));

                    // ── FFN sub-norm (BitNet: RMSNorm before down_proj) ──
                    if let Some(ref fsn) = layer.ffn_sub_norm {
                        let sub_norm_params = {
                            let p = [c.intermediate_size as u32, 0u32];
                            let mut b = bytemuck::bytes_of(&p).to_vec();
                            b[4..8].copy_from_slice(&1e-5f32.to_le_bytes());
                            b
                        };
                        batch.bind(&p.rms_norm);
                        batch.bind_buffer(&self.scratch.gate, 0, 0);
                        batch.bind_buffer(fsn, 0, 1);
                        batch.bind_buffer(&self.scratch.gate, 0, 2);
                        batch.push(&sub_norm_params, 3);
                        batch.launch_groups((1, 1, 1), (256, 1, 1));
                    }

                    // ── Down projection ──
                    if c.is_ternary {
                        let ws = layer.weight_scales.unwrap_or([1.0; 7]);
                        let params = [c.hidden_size as u32, c.intermediate_size as u32, ws[6].to_bits()];
                        batch.bind(&p.matvec_ternary);
                        batch.bind_buffer(&self.scratch.gate, 0, 0);
                        batch.bind_buffer(&layer.down_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.down, 0, 2);
                        batch.push(bytemuck::cast_slice(&params), 3);
                        batch.launch_groups((div_ceil(c.hidden_size, 8), 1, 1), (256, 1, 1));
                    } else if c.use_f16 {
                        let params = [c.hidden_size as u32, c.intermediate_size as u32];
                        batch.bind(&p.f16_matvec);
                        batch.bind_buffer(&self.scratch.gate, 0, 0);
                        batch.bind_buffer(&layer.down_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.down, 0, 2);
                        batch.push(bytemuck::cast_slice(&params), 3);
                        batch.launch_groups((div_ceil(c.hidden_size, 4), 1, 1), (256, 1, 1));
                    } else if layer.is_k_quant {
                        let params = [c.hidden_size as u32, c.intermediate_size as u32];
                        batch.bind(&p.matvec_q4k);
                        batch.bind_buffer(&self.scratch.gate, 0, 0);
                        batch.bind_buffer(&layer.down_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.down, 0, 2);
                        batch.push(bytemuck::cast_slice(&params), 3);
                        batch.launch_groups((div_ceil(c.hidden_size, 256), 1, 1), (256, 1, 1));
                    } else {
                        let params = [c.hidden_size as u32, c.intermediate_size as u32];
                        batch.bind(&p.matvec_q4_fast);
                        batch.bind_buffer(&self.scratch.gate, 0, 0);
                        batch.bind_buffer(&layer.down_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.down, 0, 2);
                        batch.push(bytemuck::cast_slice(&params), 3);
                        batch.launch_groups((div_ceil(c.hidden_size, 8), 1, 1), (256, 1, 1));
                    }

                    // ── FFN residual add + next layer input norm (fused) ──
                    let next_norm_weight = if i + 1 < c.num_layers {
                        &self.layers[i + 1].input_norm_weight
                    } else {
                        &self.final_norm_weight
                    };
                    batch.bind(&p.fused_add_norm);
                    batch.bind_buffer(&self.scratch.hidden, 0, 0);
                    batch.bind_buffer(&self.scratch.down, 0, 1);
                    batch.bind_buffer(&self.scratch.hidden, 0, 2);
                    batch.bind_buffer(next_norm_weight, 0, 3);
                    batch.bind_buffer(&self.scratch.hidden2, 0, 4);
                    batch.push(&norm_params_bytes, 5);
                    batch.launch_groups((1, 1, 1), (256, 1, 1));
                }

                // hidden2 now contains final normed hidden (from last layer's fused_add_norm)

                // ── LM Head ──
                let lm_params = [c.vocab_size as u32, c.hidden_size as u32];
                if c.use_f16 {
                    if let Some(ref lm_f16) = self.lm_head_f16 {
                        batch.bind(&p.f16_matvec);
                        batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                        batch.bind_buffer(lm_f16, 0, 1);
                        batch.bind_buffer(&self.scratch.logits, 0, 2);
                    }
                } else if self.lm_head_is_k_quant {
                    batch.bind(&p.matvec_q4k);
                    batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                    batch.bind_buffer(&self.lm_head_q4, 0, 1);
                    batch.bind_buffer(&self.scratch.logits, 0, 2);
                    batch.push(bytemuck::cast_slice(&lm_params), 3);
                    batch.launch_groups((div_ceil(c.vocab_size, 256), 1, 1), (256, 1, 1));
                } else {
                    batch.bind(&p.matvec_q4_fast);
                    batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                    batch.bind_buffer(&self.lm_head_q4, 0, 1);
                    batch.bind_buffer(&self.scratch.logits, 0, 2);
                }
                batch.push(bytemuck::cast_slice(&lm_params), 3);
                batch.launch_groups((div_ceil(c.vocab_size, 8), 1, 1), (256, 1, 1));

                // ── Argmax (two-pass: partial scan → reduce) ──
                let num_argmax_groups = 64u32.min((c.vocab_size as u32 + 255) / 256);
                let argmax_params = [c.vocab_size as u32, num_argmax_groups];

                // Pass 1: each workgroup finds local max
                batch.bind(&p.argmax_partial);
                batch.bind_buffer(&self.scratch.logits, 0, 0);
                batch.push(bytemuck::cast_slice(&argmax_params), 1);
                batch.bind_buffer(&self.scratch.argmax_partial_vals, 0, 2);
                batch.bind_buffer(&self.scratch.argmax_partial_idxs, 0, 3);
                batch.launch_groups((num_argmax_groups as usize, 1, 1), (256, 1, 1));

                // Pass 2: single workgroup reduces partials
                batch.bind(&p.argmax_reduce);
                batch.bind_buffer(&self.scratch.argmax_partial_vals, 0, 0);
                batch.bind_buffer(&self.scratch.argmax_partial_idxs, 0, 1);
                batch.bind_buffer(&self.scratch.argmax_result, 0, 2);
                batch.push(bytemuck::cast_slice(&argmax_params), 3);
                batch.launch_groups((1, 1, 1), (256, 1, 1));
            });
        });}

        self.past_seq_len = total_seq;

        // Debug: dump first few logits
        if pos < 2 {
            let logits_sample = self.scratch.logits.read(|d| {
                let f16s: &[u16] = bytemuck::cast_slice(&d[..20]);
                f16s.iter().map(|&v| aruminium::fp16_to_f32(v)).collect::<Vec<f32>>()
            });
            log::warn!("Metal logits[pos={pos}][0:10]: {:?}", logits_sample);
        }

        // Read argmax result from GPU (this blocks until dispatch_batch completes)
        let result = self.scratch.argmax_result.read(|d| {
            u32::from_le_bytes([d[0], d[1], d[2], d[3]])
        });

        // Debug: verify argmax by finding CPU max
        if pos < 2 {
            let (cpu_max_idx, cpu_max_val) = self.scratch.logits.read(|d| {
                let f16s: &[u16] = bytemuck::cast_slice(&d[..c.vocab_size * 2]);
                let mut max_idx = 0usize;
                let mut max_val = f32::NEG_INFINITY;
                for (i, &v) in f16s.iter().enumerate() {
                    let fv = aruminium::fp16_to_f32(v);
                    if fv > max_val { max_val = fv; max_idx = i; }
                }
                (max_idx, max_val)
            });
            log::warn!("Metal argmax[pos={pos}]: GPU={result}, CPU={cpu_max_idx} (val={cpu_max_val:.4})");
        }

        // Log timing every 20 steps
        if pos > 0 && pos % 50 == 0 {
            log::info!("Metal decode: pos={pos}, tok/s estimate from wall clock");
        }

        result
    }

    /// Debug: run embed only, return first 8 hidden values as f32
    pub fn debug_embed(&mut self, token_id: u32) -> Vec<f32> {
        let p = &self.pipelines;
        let c = &self.config;
        self.token_buf.write(|d| { d[..4].copy_from_slice(&token_id.to_le_bytes()); });
        unsafe { aruminium::autorelease_pool(|| {
            p.dispatcher.batch_raw(|batch| {
                let embed_params = [c.hidden_size as u32];
                batch.bind(&p.embed);
                batch.bind_buffer(&self.embed_table, 0, 0);
                batch.bind_buffer(&self.token_buf, 0, 1);
                batch.bind_buffer(&self.scratch.hidden, 0, 2);
                batch.push(bytemuck::cast_slice(&embed_params), 3);
                batch.launch_groups((div_ceil(c.hidden_size, 256), 1, 1), (256, 1, 1));
            });
        }); }
        // Read first 8 f16 values from scratch.hidden, convert to f32
        self.scratch.hidden.read(|d| {
            let f16s: &[u16] = bytemuck::cast_slice(&d[..16]);
            f16s.iter().map(|&v| aruminium::fp16_to_f32(v)).collect()
        })
    }

    /// Debug: run embed + norm + q_proj, return first 8 values from q output
    pub fn debug_q_proj(&mut self, token_id: u32) -> (Vec<f32>, Vec<f32>) {
        let p = &self.pipelines;
        let c = &self.config;
        self.token_buf.write(|d| { d[..4].copy_from_slice(&token_id.to_le_bytes()); });

        let norm_params = {
            let p = [c.hidden_size as u32, 0u32];
            let mut b = bytemuck::bytes_of(&p).to_vec();
            b[4..8].copy_from_slice(&1e-6f32.to_le_bytes());
            b
        };
        let q_dim = (c.num_heads * c.head_dim) as u32;

        unsafe { aruminium::autorelease_pool(|| {
            p.dispatcher.batch_raw(|batch| {
                // Embed
                let embed_params = [c.hidden_size as u32];
                batch.bind(&p.embed);
                batch.bind_buffer(&self.embed_table, 0, 0);
                batch.bind_buffer(&self.token_buf, 0, 1);
                batch.bind_buffer(&self.scratch.hidden, 0, 2);
                batch.push(bytemuck::cast_slice(&embed_params), 3);
                batch.launch_groups((div_ceil(c.hidden_size, 256), 1, 1), (256, 1, 1));

                // RMS norm
                batch.bind(&p.rms_norm);
                batch.bind_buffer(&self.scratch.hidden, 0, 0);
                batch.bind_buffer(&self.layers[0].input_norm_weight, 0, 1);
                batch.bind_buffer(&self.scratch.hidden2, 0, 2);
                batch.push(&norm_params, 3);
                batch.launch_groups((1, 1, 1), (256, 1, 1));

                // Q proj (matvec_q4)
                let params = [q_dim, c.hidden_size as u32];
                batch.bind(&p.matvec_q4);
                batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                batch.bind_buffer(&self.layers[0].q_proj, 0, 1);
                batch.bind_buffer(&self.scratch.q, 0, 2);
                batch.push(bytemuck::cast_slice(&params), 3);
                batch.launch_groups((div_ceil(q_dim as usize, 256), 1, 1), (256, 1, 1));
            });
        }); }

        // Read normed hidden (first 8 f16)
        let normed = self.scratch.hidden2.read(|d| {
            let f16s: &[u16] = bytemuck::cast_slice(&d[..16]);
            f16s.iter().map(|&v| aruminium::fp16_to_f32(v)).collect::<Vec<f32>>()
        });

        // Read q_proj output (first 8 f16)
        let q_out = self.scratch.q.read(|d| {
            let f16s: &[u16] = bytemuck::cast_slice(&d[..16]);
            f16s.iter().map(|&v| aruminium::fp16_to_f32(v)).collect::<Vec<f32>>()
        });

        (normed, q_out)
    }

    pub fn reset_kv_cache(&mut self) {
        self.past_seq_len = 0;
    }

    /// Profile forward pass — split into 3 phases, measure each
    pub fn forward_decode_profile(&mut self, token_id: u32, _warmup: bool) -> (u32, f64, f64, f64) {
        use std::time::Instant;
        let p = &self.pipelines;
        let c = &self.config;
        let pos = self.past_seq_len;
        let total_seq = pos + 1;
        let half_dim = c.head_dim / 2;
        let scale = 1.0 / (c.head_dim as f32).sqrt();

        self.token_buf.write(|d| {
            d[..4].copy_from_slice(&token_id.to_le_bytes());
        });

        // Phase 1: Embed + Layers
        let t0 = Instant::now();
        unsafe { aruminium::autorelease_pool(|| {
            p.dispatcher.batch_raw(|batch| {
                let embed_params = [c.hidden_size as u32];
                batch.bind(&p.embed);
                batch.bind_buffer(&self.embed_table, 0, 0);
                batch.bind_buffer(&self.token_buf, 0, 1);
                batch.bind_buffer(&self.scratch.hidden, 0, 2);
                batch.push(bytemuck::cast_slice(&embed_params), 3);
                batch.launch_groups((div_ceil(c.hidden_size, 256), 1, 1), (256, 1, 1));

                let norm_params_bytes = {
                    let p = [c.hidden_size as u32, 0u32];
                    let mut b = bytemuck::bytes_of(&p).to_vec();
                    b[4..8].copy_from_slice(&1e-6f32.to_le_bytes());
                    b
                };
                batch.bind(&p.rms_norm);
                batch.bind_buffer(&self.scratch.hidden, 0, 0);
                batch.bind_buffer(&self.layers[0].input_norm_weight, 0, 1);
                batch.bind_buffer(&self.scratch.hidden2, 0, 2);
                batch.push(&norm_params_bytes, 3);
                batch.launch_groups((1, 1, 1), (256, 1, 1));

                for i in 0..c.num_layers {
                    let layer = &self.layers[i];
                    let kv = &self.kv_cache[i];

                    let q_dim = c.num_heads * c.head_dim;
                    let kv_dim_val = c.kv_num_heads * c.head_dim;
                    let wg_q = div_ceil(q_dim, 8);
                    let wg_k = div_ceil(kv_dim_val, 8);
                    let wg_v = wg_k;
                    let qkv_params = [q_dim as u32, kv_dim_val as u32, c.hidden_size as u32, wg_q as u32, wg_k as u32];
                    batch.bind(&p.fused_qkv);
                    batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                    batch.bind_buffer(&layer.q_proj, 0, 1);
                    batch.bind_buffer(&layer.k_proj, 0, 2);
                    batch.bind_buffer(&layer.v_proj, 0, 3);
                    batch.bind_buffer(&self.scratch.q, 0, 4);
                    batch.bind_buffer(&self.scratch.k, 0, 5);
                    batch.bind_buffer(&self.scratch.v, 0, 6);
                    batch.push(bytemuck::cast_slice(&qkv_params), 7);
                    batch.launch_groups((wg_q + wg_k + wg_v, 1, 1), (256, 1, 1));

                    let rope_params = [half_dim as u32, c.head_dim as u32, c.num_heads as u32, c.kv_num_heads as u32];
                    let total_rope = half_dim * (c.num_heads + c.kv_num_heads);
                    batch.bind(&p.fused_rope_qk);
                    batch.bind_buffer(&self.scratch.q, 0, 0);
                    batch.bind_buffer(&self.scratch.k, 0, 1);
                    batch.bind_buffer(&self.cos_cache, pos * half_dim * 2, 2);
                    batch.bind_buffer(&self.sin_cache, pos * half_dim * 2, 3);
                    batch.push(bytemuck::cast_slice(&rope_params), 4);
                    batch.launch_groups((div_ceil(total_rope, 256), 1, 1), (256, 1, 1));

                    let append_params = [c.head_dim as u32, c.kv_num_heads as u32, pos as u32, c.max_seq as u32];
                    batch.bind(&p.fused_kv_append);
                    batch.bind_buffer(&self.scratch.k, 0, 0);
                    batch.bind_buffer(&self.scratch.v, 0, 1);
                    batch.bind_buffer(&kv.k, 0, 2);
                    batch.bind_buffer(&kv.v, 0, 3);
                    batch.push(bytemuck::cast_slice(&append_params), 4);
                    batch.launch_groups((div_ceil(c.kv_num_heads * c.head_dim, 256), 1, 1), (256, 1, 1));

                    let attn_params = [c.head_dim as u32, total_seq as u32, c.num_heads as u32, scale.to_bits(), c.kv_num_heads as u32, c.max_seq as u32];
                    batch.bind(&p.attention_decode);
                    batch.bind_buffer(&self.scratch.q, 0, 0);
                    batch.bind_buffer(&kv.k, 0, 1);
                    batch.bind_buffer(&kv.v, 0, 2);
                    batch.bind_buffer(&self.scratch.attn_out, 0, 3);
                    batch.push(bytemuck::cast_slice(&attn_params), 4);
                    batch.launch_groups((c.num_heads, 1, 1), (256, 1, 1));

                    {
                        let params = [c.hidden_size as u32, (c.num_heads * c.head_dim) as u32];
                        batch.bind(&p.matvec_q4_fast);
                        batch.bind_buffer(&self.scratch.attn_out, 0, 0);
                        batch.bind_buffer(&layer.o_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.down, 0, 2);
                        batch.push(bytemuck::cast_slice(&params), 3);
                        batch.launch_groups((div_ceil(c.hidden_size, 8), 1, 1), (256, 1, 1));
                    }

                    batch.bind(&p.fused_add_norm);
                    batch.bind_buffer(&self.scratch.hidden, 0, 0);
                    batch.bind_buffer(&self.scratch.down, 0, 1);
                    batch.bind_buffer(&self.scratch.hidden, 0, 2);
                    batch.bind_buffer(&layer.post_norm_weight, 0, 3);
                    batch.bind_buffer(&self.scratch.hidden2, 0, 4);
                    batch.push(&norm_params_bytes, 5);
                    batch.launch_groups((1, 1, 1), (256, 1, 1));

                    let inter = c.intermediate_size as u32;
                    let wg_gate = div_ceil(c.intermediate_size, 8);
                    let gate_up_params = [inter, c.hidden_size as u32, wg_gate as u32];
                    batch.bind(&p.fused_gate_up);
                    batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                    batch.bind_buffer(&layer.gate_proj, 0, 1);
                    batch.bind_buffer(&layer.up_proj, 0, 2);
                    batch.bind_buffer(&self.scratch.gate, 0, 3);
                    batch.bind_buffer(&self.scratch.up, 0, 4);
                    batch.push(bytemuck::cast_slice(&gate_up_params), 5);
                    batch.launch_groups((wg_gate * 2, 1, 1), (256, 1, 1));

                    let silu_params = [c.intermediate_size as u32];
                    batch.bind(&p.silu_mul_f16);
                    batch.bind_buffer(&self.scratch.gate, 0, 0);
                    batch.bind_buffer(&self.scratch.up, 0, 1);
                    batch.bind_buffer(&self.scratch.gate, 0, 2);
                    batch.push(bytemuck::cast_slice(&silu_params), 3);
                    batch.launch_groups((div_ceil(c.intermediate_size, 256), 1, 1), (256, 1, 1));

                    {
                        let params = [c.hidden_size as u32, c.intermediate_size as u32];
                        batch.bind(&p.matvec_q4_fast);
                        batch.bind_buffer(&self.scratch.gate, 0, 0);
                        batch.bind_buffer(&layer.down_proj, 0, 1);
                        batch.bind_buffer(&self.scratch.down, 0, 2);
                        batch.push(bytemuck::cast_slice(&params), 3);
                        batch.launch_groups((div_ceil(c.hidden_size, 8), 1, 1), (256, 1, 1));
                    }

                    let next_norm_weight = if i + 1 < c.num_layers {
                        &self.layers[i + 1].input_norm_weight
                    } else {
                        &self.final_norm_weight
                    };
                    batch.bind(&p.fused_add_norm);
                    batch.bind_buffer(&self.scratch.hidden, 0, 0);
                    batch.bind_buffer(&self.scratch.down, 0, 1);
                    batch.bind_buffer(&self.scratch.hidden, 0, 2);
                    batch.bind_buffer(next_norm_weight, 0, 3);
                    batch.bind_buffer(&self.scratch.hidden2, 0, 4);
                    batch.push(&norm_params_bytes, 5);
                    batch.launch_groups((1, 1, 1), (256, 1, 1));
                }
            });
        });}
        let layers_ms = t0.elapsed().as_secs_f64() * 1000.0;

        // Phase 2: LM Head
        let t1 = Instant::now();
        unsafe { aruminium::autorelease_pool(|| {
            p.dispatcher.batch_raw(|batch| {
                let lm_params = [c.vocab_size as u32, c.hidden_size as u32];
                batch.bind(&p.matvec_q4_fast);
                batch.bind_buffer(&self.scratch.hidden2, 0, 0);
                batch.bind_buffer(&self.lm_head_q4, 0, 1);
                batch.bind_buffer(&self.scratch.logits, 0, 2);
                batch.push(bytemuck::cast_slice(&lm_params), 3);
                batch.launch_groups((div_ceil(c.vocab_size, 8), 1, 1), (256, 1, 1));
            });
        });}
        let lm_head_ms = t1.elapsed().as_secs_f64() * 1000.0;

        // Phase 3: Argmax (two-pass)
        let t2 = Instant::now();
        unsafe { aruminium::autorelease_pool(|| {
            p.dispatcher.batch_raw(|batch| {
                let num_argmax_groups = 64u32.min((c.vocab_size as u32 + 255) / 256);
                let argmax_params = [c.vocab_size as u32, num_argmax_groups];

                batch.bind(&p.argmax_partial);
                batch.bind_buffer(&self.scratch.logits, 0, 0);
                batch.push(bytemuck::cast_slice(&argmax_params), 1);
                batch.bind_buffer(&self.scratch.argmax_partial_vals, 0, 2);
                batch.bind_buffer(&self.scratch.argmax_partial_idxs, 0, 3);
                batch.launch_groups((num_argmax_groups as usize, 1, 1), (256, 1, 1));

                batch.bind(&p.argmax_reduce);
                batch.bind_buffer(&self.scratch.argmax_partial_vals, 0, 0);
                batch.bind_buffer(&self.scratch.argmax_partial_idxs, 0, 1);
                batch.bind_buffer(&self.scratch.argmax_result, 0, 2);
                batch.push(bytemuck::cast_slice(&argmax_params), 3);
                batch.launch_groups((1, 1, 1), (256, 1, 1));
            });
        });}
        let argmax_ms = t2.elapsed().as_secs_f64() * 1000.0;

        self.past_seq_len = total_seq;

        let result = self.scratch.argmax_result.read(|d| {
            u32::from_le_bytes([d[0], d[1], d[2], d[3]])
        });

        (result, layers_ms, lm_head_ms, argmax_ms)
    }

    /// Read scratch.hidden as f32 (debug)
    pub fn debug_read_hidden(&self) -> Vec<f32> {
        let n = self.config.hidden_size;
        self.scratch.hidden.read(|d| {
            let f16s: &[u16] = bytemuck::cast_slice(&d[..n * 2]);
            f16s.iter().map(|&v| aruminium::fp16_to_f32(v)).collect()
        })
    }

    /// Get vocab size (for external callers)
    pub fn vocab_size(&self) -> usize {
        self.config.vocab_size
    }

    /// Read the full logits buffer as f32 (for speculative verification)
    pub fn read_logits(&self) -> Vec<f32> {
        self.debug_read_logits(self.config.vocab_size)
    }

    /// Read logits as f32 (debug) — first `count` elements
    pub fn debug_read_logits(&self, count: usize) -> Vec<f32> {
        self.scratch.logits.read(|d| {
            let n = count.min(d.len() / 2);
            let f16s: &[u16] = bytemuck::cast_slice(&d[..n * 2]);
            f16s.iter().map(|&v| aruminium::fp16_to_f32(v)).collect()
        })
    }
}

/// Transpose Q4 blocks from [N][K/32] (row-major, GGUF) to [K/32][N] (Metal layout)
/// AND repack nibbles from GGUF split order to Metal interleaved order.
///
/// GGUF Q4_0 nibble layout within a block (32 weights, 16 bytes of qs):
///   byte j: lo nibble = weight[j], hi nibble = weight[j+16]
///   (first 16 weights in lo nibbles, last 16 in hi nibbles)
///
/// Metal kernel expects interleaved nibbles:
///   byte i: lo nibble = weight[2*i], hi nibble = weight[2*i+1]
///   (consecutive weight pairs packed into each byte)
///
/// The repacking is lossless — just rearranging nibbles within each block.
/// Generic block transpose: [N][K/block_vals] → [K/block_vals][N]
/// Pure permutation, no data modification.
fn transpose_blocks(data: &[u8], n: usize, k: usize, block_vals: usize, block_bytes: usize) -> Vec<u8> {
    let blocks_per_row = k / block_vals;
    let total_blocks = blocks_per_row * n;
    let mut out = vec![0u8; total_blocks * block_bytes];
    for r in 0..n {
        for b in 0..blocks_per_row {
            let src = (r * blocks_per_row + b) * block_bytes;
            let dst = (b * n + r) * block_bytes;
            out[dst..dst + block_bytes].copy_from_slice(&data[src..src + block_bytes]);
        }
    }
    out
}

fn transpose_q4_blocks(data: &[u8], n: usize, k: usize) -> Vec<u8> {
    const BS: usize = 18; // bytes per Q4 block: 2 (scale) + 16 (qs)
    let blocks_per_row = k / 32;
    let total_blocks = blocks_per_row * n;
    let mut out = vec![0u8; total_blocks * BS];

    for r in 0..n {
        for b in 0..blocks_per_row {
            let src = (r * blocks_per_row + b) * BS;
            let dst = (b * n + r) * BS;

            // Copy scale (2 bytes) unchanged
            out[dst] = data[src];
            out[dst + 1] = data[src + 1];

            // Repack nibbles: GGUF split → Metal interleaved
            // GGUF: qs[j] has weight[j] in lo, weight[j+16] in hi  (j=0..15)
            // Metal: qs[i] has weight[2i] in lo, weight[2i+1] in hi (i=0..15)
            //
            // Build a 32-element nibble array, then repack:
            let qs_src = &data[src + 2..src + 18];
            let qs_dst = &mut out[dst + 2..dst + 18];

            // Extract all 32 nibbles in weight order (GGUF layout)
            let mut nibbles = [0u8; 32];
            for j in 0..16 {
                nibbles[j] = qs_src[j] & 0x0F;       // weight[j]
                nibbles[j + 16] = qs_src[j] >> 4;     // weight[j+16]
            }

            // Repack as interleaved pairs (Metal layout)
            for i in 0..16 {
                qs_dst[i] = nibbles[2 * i] | (nibbles[2 * i + 1] << 4);
            }
        }
    }
    out
}

fn quantize_f32_to_block_q4_0(weights: &[f32], n: usize, k: usize) -> Vec<u8> {
    const BS: usize = 32;
    let blocks_per_col = k / BS;
    let total_blocks = blocks_per_col * n;
    let mut out = vec![0u8; total_blocks * 18];

    // Metal kernel indexes: B[bk * N + col] where bk = block index, col = output neuron
    // So layout is [K/32][N] — block-major, then neuron-minor
    for col in 0..n {
        for bk in 0..blocks_per_col {
            let blk_start = col * k + bk * BS;
            let block = &weights[blk_start..blk_start + BS];

            let mut amax = 0.0f32;
            for &v in block {
                amax = amax.max(v.abs());
            }
            let scale = if amax > 0.0 { amax / 7.0 } else { 0.0 };
            let inv_scale = if scale > 0.0 { 1.0 / scale } else { 0.0 };

            let out_idx = (bk * n + col) * 18;
            let scale_fp16 = aruminium::f32_to_fp16(scale);
            out[out_idx] = (scale_fp16 & 0xFF) as u8;
            out[out_idx + 1] = (scale_fp16 >> 8) as u8;

            for j in 0..16 {
                let q0 = ((block[j * 2] * inv_scale).round() as i32 + 8).clamp(0, 15) as u8;
                let q1 = ((block[j * 2 + 1] * inv_scale).round() as i32 + 8).clamp(0, 15) as u8;
                out[out_idx + 2 + j] = q0 | (q1 << 4);
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpose_q4_nibble_repack() {
        // One block: scale=0.5 (as f16), 16 bytes of nibbles
        // GGUF layout: qs[j] has weight[j] in lo, weight[j+16] in hi
        let scale_f16 = half::f16::from_f32(0.5);
        let scale_bytes = scale_f16.to_le_bytes();
        
        // Set up: weight[0]=1, weight[1]=2, ..., weight[15]=15+1=0 (mod 16)
        // weight[16]=15, weight[17]=14, ..., weight[31]=1
        // GGUF qs[j] = weight[j] | (weight[j+16] << 4)
        let mut qs = [0u8; 16];
        for j in 0..16u8 {
            let lo = (j + 1) % 16; // weight[j]
            let hi = 15 - j;        // weight[j+16]
            qs[j as usize] = lo | (hi << 4);
        }
        
        let mut block = vec![0u8; 18];
        block[0..2].copy_from_slice(&scale_bytes);
        block[2..18].copy_from_slice(&qs);
        
        // n=1, k=32 → 1 row, 1 block, no spatial transpose needed
        let result = transpose_q4_blocks(&block, 1, 32);
        
        // After repack, Metal reads qs_new[i] as:
        // lo = weight[2i], hi = weight[2i+1]
        // So qs_new[0] should have weight[0]=1 in lo, weight[1]=2 in hi → 1 | (2<<4) = 33
        // qs_new[1] should have weight[2]=3 in lo, weight[3]=4 in hi → 3 | (4<<4) = 67
        
        // Verify
        let qs_out = &result[2..18];
        
        // Build expected weights in natural order
        let mut weights = [0u8; 32];
        for j in 0..16 {
            weights[j] = ((j + 1) % 16) as u8;  // GGUF lo → position j
            weights[j + 16] = (15 - j as u8);     // GGUF hi → position j+16
        }
        
        // Expected repacked bytes
        for i in 0..16 {
            let expected = weights[2*i] | (weights[2*i + 1] << 4);
            assert_eq!(qs_out[i], expected, 
                "byte {i}: got {}, expected {} (w[{}]={}, w[{}]={})", 
                qs_out[i], expected, 2*i, weights[2*i], 2*i+1, weights[2*i+1]);
        }
    }
}
