//! MetalModel — full transformer inference via aruminium
//!
//! All ops in MSL, all fp16, single dispatch_batch per decode step.
//! Weights: safetensors f32 → Q4 quantize-on-load (block_q4_0 format).

use std::path::Path;
use aruminium::MtlBuffer;
use super::pipelines::MetalPipelines;

const MAX_SEQ: usize = 2048;

pub struct MetalModelConfig {
    pub hidden_size: usize,
    pub num_heads: usize,
    pub kv_num_heads: usize,
    pub head_dim: usize,
    pub num_layers: usize,
    pub vocab_size: usize,
    pub intermediate_size: usize,
    pub has_qk_norm: bool,
}

struct MetalLayerWeights {
    input_norm_weight: MtlBuffer,
    q_proj: MtlBuffer,   // block_q4_0 [K/32][N]
    k_proj: MtlBuffer,
    v_proj: MtlBuffer,
    o_proj: MtlBuffer,
    q_proj_bias: Option<MtlBuffer>,
    k_proj_bias: Option<MtlBuffer>,
    v_proj_bias: Option<MtlBuffer>,
    q_norm_weight: Option<MtlBuffer>,
    k_norm_weight: Option<MtlBuffer>,
    post_norm_weight: MtlBuffer,
    gate_proj: MtlBuffer,
    up_proj: MtlBuffer,
    down_proj: MtlBuffer,
}

struct MetalKVCache {
    k: MtlBuffer, // [kv_heads, MAX_SEQ, head_dim] fp16
    v: MtlBuffer,
}

struct Scratch {
    hidden: MtlBuffer,
    hidden2: MtlBuffer,
    q: MtlBuffer,
    k: MtlBuffer,
    v: MtlBuffer,
    attn_out: MtlBuffer,
    gate: MtlBuffer,
    up: MtlBuffer,
    down: MtlBuffer,
    logits: MtlBuffer,
    argmax_result: MtlBuffer,
}

pub struct MetalModel {
    pipelines: MetalPipelines,
    config: MetalModelConfig,
    embed_table: MtlBuffer,
    lm_head_q4: MtlBuffer,    // Q4 quantized LM head (or embed_table quantized)
    final_norm_weight: MtlBuffer,
    token_buf: MtlBuffer,      // pre-allocated, rewritten each step
    layers: Vec<MetalLayerWeights>,
    cos_cache: MtlBuffer,
    sin_cache: MtlBuffer,
    kv_cache: Vec<MetalKVCache>,
    past_seq_len: usize,
    scratch: Scratch,
}

fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

impl MetalModel {
    /// Load model from safetensors, quantize projections to Q4, all buffers in fp16.
    pub fn load_from_safetensors(path: &Path) -> Result<Self, String> {
        let pipelines = MetalPipelines::new().map_err(|e| format!("Metal init: {e}"))?;

        let model_dir = path.parent().unwrap_or(Path::new("."));
        let config_str = std::fs::read_to_string(model_dir.join("config.json"))
            .map_err(|e| format!("config.json: {e}"))?;
        let cj: serde_json::Value = serde_json::from_str(&config_str)
            .map_err(|e| format!("config parse: {e}"))?;

        let hidden_size = cj["hidden_size"].as_u64().ok_or("missing hidden_size")? as usize;
        let num_heads = cj["num_attention_heads"].as_u64().ok_or("missing num_attention_heads")? as usize;
        let kv_num_heads = cj["num_key_value_heads"].as_u64().unwrap_or(num_heads as u64) as usize;
        let num_layers = cj["num_hidden_layers"].as_u64().ok_or("missing num_hidden_layers")? as usize;
        let vocab_size = cj["vocab_size"].as_u64().ok_or("missing vocab_size")? as usize;
        let head_dim = cj["head_dim"].as_u64().map(|v| v as usize).unwrap_or(hidden_size / num_heads);
        let intermediate_size = cj["intermediate_size"].as_u64().unwrap_or((hidden_size * 4) as u64) as usize;
        let rope_theta = cj["rope_theta"].as_f64().unwrap_or(10000.0) as f32;
        let rms_norm_eps = cj["rms_norm_eps"].as_f64().unwrap_or(1e-6) as f32;
        let tie_word_embeddings = cj["tie_word_embeddings"].as_bool().unwrap_or(true);

        let graph = crate::loader::safetensors::load_safetensors(path)?;
        let has_qk_norm = graph.get_weight("model.layers.0.self_attn.q_norm.weight").is_some();
        let has_attn_bias = graph.get_weight("model.layers.0.self_attn.q_proj.bias").is_some();

        let config = MetalModelConfig {
            hidden_size, num_heads, kv_num_heads, head_dim,
            num_layers, vocab_size, intermediate_size, has_qk_norm,
        };

        log::info!("Metal model: hidden={hidden_size}, heads={num_heads}, kv_heads={kv_num_heads}, layers={num_layers}, vocab={vocab_size}");

        // Helper: load weight as f32
        let weight_to_f32 = |name: &str| -> Result<Vec<f32>, String> {
            let w = graph.get_weight(name).ok_or_else(|| format!("Missing: {name}"))?;
            Ok(crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype))
        };

        // Helper: load weight as f32 then convert to fp16
        let weight_to_f16 = |name: &str| -> Result<Vec<u16>, String> {
            let w = graph.get_weight(name).ok_or_else(|| format!("Missing: {name}"))?;
            let f32s = crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
            Ok(f32s.iter().map(|&v| aruminium::f32_to_fp16(v)).collect())
        };

        // Embed table (fp16)
        let embed_f16 = weight_to_f16("model.embed_tokens.weight")?;
        let embed_table = pipelines.upload_f16(&embed_f16).map_err(|e| format!("{e}"))?;

        // LM head — Q4 quantized for speed (vocab is huge, memory-bound)
        let lm_head_q4 = if !tie_word_embeddings {
            if let Some(w) = graph.get_weight("lm_head.weight") {
                let f32s = crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
                let packed = quantize_f32_to_block_q4_0(&f32s, vocab_size, hidden_size);
                pipelines.upload_bytes(&packed).map_err(|e| format!("{e}"))?
            } else {
                // Fallback: quantize embed_table
                let embed_f32 = weight_to_f32("model.embed_tokens.weight")?;
                let packed = quantize_f32_to_block_q4_0(&embed_f32, vocab_size, hidden_size);
                pipelines.upload_bytes(&packed).map_err(|e| format!("{e}"))?
            }
        } else {
            // Tied weights — quantize embed_table for LM head
            let embed_f32 = weight_to_f32("model.embed_tokens.weight")?;
            let packed = quantize_f32_to_block_q4_0(&embed_f32, vocab_size, hidden_size);
            pipelines.upload_bytes(&packed).map_err(|e| format!("{e}"))?
        };

        // Final norm
        let final_norm_f16 = weight_to_f16("model.norm.weight")?;
        let final_norm_weight = pipelines.upload_f16(&final_norm_f16).map_err(|e| format!("{e}"))?;

        let q_dim = num_heads * head_dim;
        let kv_dim = kv_num_heads * head_dim;

        // Quantize projection weights to block_q4_0 and upload
        let quantize_upload = |name: &str, n: usize, k: usize| -> Result<MtlBuffer, String> {
            let w = graph.get_weight(name).ok_or_else(|| format!("Missing: {name}"))?;
            let f32s = crate::backend::wgpu::model::safetensors_to_f32(&w.data, w.dtype);
            let packed = quantize_f32_to_block_q4_0(&f32s, n, k);
            pipelines.upload_bytes(&packed).map_err(|e| format!("{e}"))
        };

        // Load layers
        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            log::info!("Metal: loading layer {i}/{num_layers}");

            let input_norm_weight = pipelines.upload_f16(
                &weight_to_f16(&format!("model.layers.{i}.input_layernorm.weight"))?
            ).map_err(|e| format!("{e}"))?;

            let q_proj = quantize_upload(&format!("model.layers.{i}.self_attn.q_proj.weight"), q_dim, hidden_size)?;
            let k_proj = quantize_upload(&format!("model.layers.{i}.self_attn.k_proj.weight"), kv_dim, hidden_size)?;
            let v_proj = quantize_upload(&format!("model.layers.{i}.self_attn.v_proj.weight"), kv_dim, hidden_size)?;
            let o_proj = quantize_upload(&format!("model.layers.{i}.self_attn.o_proj.weight"), hidden_size, q_dim)?;

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

            let gate_proj = quantize_upload(&format!("model.layers.{i}.mlp.gate_proj.weight"), intermediate_size, hidden_size)?;
            let up_proj = quantize_upload(&format!("model.layers.{i}.mlp.up_proj.weight"), intermediate_size, hidden_size)?;
            let down_proj = quantize_upload(&format!("model.layers.{i}.mlp.down_proj.weight"), hidden_size, intermediate_size)?;

            layers.push(MetalLayerWeights {
                input_norm_weight, q_proj, k_proj, v_proj, o_proj,
                q_proj_bias, k_proj_bias, v_proj_bias,
                q_norm_weight, k_norm_weight,
                post_norm_weight, gate_proj, up_proj, down_proj,
            });
        }

        // RoPE cache (fp16)
        let half_dim = head_dim / 2;
        let mut cos_f16 = vec![0u16; MAX_SEQ * half_dim];
        let mut sin_f16 = vec![0u16; MAX_SEQ * half_dim];
        for pos in 0..MAX_SEQ {
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
        let kv_buf_size = kv_num_heads * MAX_SEQ * head_dim * 2; // fp16
        let mut kv_cache = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            kv_cache.push(MetalKVCache {
                k: pipelines.alloc(kv_buf_size).map_err(|e| format!("{e}"))?,
                v: pipelines.alloc(kv_buf_size).map_err(|e| format!("{e}"))?,
            });
        }

        // Scratch buffers (fp16)
        let alloc_f16 = |n: usize| -> Result<MtlBuffer, String> {
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
        };

        log::info!("Metal model loaded: {} layers, Q4 weights, fp16 activations", num_layers);

        // Pre-allocate token buffer (rewritten each step, avoids allocation)
        let token_buf = pipelines.alloc(4).map_err(|e| format!("{e}"))?;

        Ok(MetalModel {
            pipelines, config, embed_table, lm_head_q4, final_norm_weight,
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
        self.token_buf.with_data_mut(|d| {
            d[..4].copy_from_slice(&token_id.to_le_bytes());
        });

        unsafe { aruminium::autorelease_pool(|| {
            p.dispatcher.dispatch_batch_raw(|batch| {
                // ── Embed ──
                let embed_params = [c.hidden_size as u32];
                batch.set_pipeline(&p.embed);
                batch.set_buffer(&self.embed_table, 0, 0);
                batch.set_buffer(&self.token_buf, 0, 1);
                batch.set_buffer(&self.scratch.hidden, 0, 2);
                batch.set_bytes(bytemuck::cast_slice(&embed_params), 3);
                batch.dispatch_threadgroups((div_ceil(c.hidden_size, 256), 1, 1), (256, 1, 1));

                let norm_params_bytes = {
                    let p = [c.hidden_size as u32, 0u32]; // hidden + padding for eps
                    let mut b = bytemuck::bytes_of(&p).to_vec();
                    // Replace last 4 bytes with eps as f32
                    let eps_bytes = 1e-6f32.to_le_bytes();
                    b[4..8].copy_from_slice(&eps_bytes);
                    b
                };

                for i in 0..c.num_layers {
                    let layer = &self.layers[i];
                    let kv = &self.kv_cache[i];

                    // ── Input RMS Norm ──
                    batch.set_pipeline(&p.rms_norm);
                    batch.set_buffer(&self.scratch.hidden, 0, 0);
                    batch.set_buffer(&layer.input_norm_weight, 0, 1);
                    batch.set_buffer(&self.scratch.hidden2, 0, 2);
                    batch.set_bytes(&norm_params_bytes, 3);
                    batch.dispatch_threadgroups((1, 1, 1), (256, 1, 1));

                    // ── Q/K/V projections (matvec_q4_fast — 8 rows/WG, simd cooperative) ──
                    let matvec_q4 = |batch: &aruminium::BatchEncoder, input: &MtlBuffer, proj: &MtlBuffer, out: &MtlBuffer, n: u32, k: u32| {
                        let params = [n, k];
                        batch.set_pipeline(&p.matvec_q4_fast);
                        batch.set_buffer(input, 0, 0);
                        batch.set_buffer(proj, 0, 1);
                        batch.set_buffer(out, 0, 2);
                        batch.set_bytes(bytemuck::cast_slice(&params), 3);
                        batch.dispatch_threadgroups((div_ceil(n as usize, 8), 1, 1), (256, 1, 1));
                    };

                    matvec_q4(batch, &self.scratch.hidden2, &layer.q_proj, &self.scratch.q, (c.num_heads * c.head_dim) as u32, c.hidden_size as u32);
                    matvec_q4(batch, &self.scratch.hidden2, &layer.k_proj, &self.scratch.k, (c.kv_num_heads * c.head_dim) as u32, c.hidden_size as u32);
                    matvec_q4(batch, &self.scratch.hidden2, &layer.v_proj, &self.scratch.v, (c.kv_num_heads * c.head_dim) as u32, c.hidden_size as u32);

                    // ── RoPE (per-head rotation) ──
                    let rope_params_q = [half_dim as u32, c.head_dim as u32, c.num_heads as u32];
                    batch.set_pipeline(&p.rope);
                    batch.set_buffer(&self.scratch.q, 0, 0);
                    batch.set_buffer(&self.cos_cache, pos * half_dim * 2, 1);
                    batch.set_buffer(&self.sin_cache, pos * half_dim * 2, 2);
                    batch.set_buffer(&self.scratch.q, 0, 3); // in-place
                    batch.set_bytes(bytemuck::cast_slice(&rope_params_q), 4);
                    batch.dispatch_threadgroups((div_ceil(half_dim * c.num_heads, 256), 1, 1), (256, 1, 1));

                    let rope_params_k = [half_dim as u32, c.head_dim as u32, c.kv_num_heads as u32];
                    batch.set_pipeline(&p.rope);
                    batch.set_buffer(&self.scratch.k, 0, 0);
                    batch.set_buffer(&self.cos_cache, pos * half_dim * 2, 1);
                    batch.set_buffer(&self.sin_cache, pos * half_dim * 2, 2);
                    batch.set_buffer(&self.scratch.k, 0, 3);
                    batch.set_bytes(bytemuck::cast_slice(&rope_params_k), 4);
                    batch.dispatch_threadgroups((div_ceil(half_dim * c.kv_num_heads, 256), 1, 1), (256, 1, 1));

                    // ── KV cache append ──
                    let append_params = [c.head_dim as u32, c.kv_num_heads as u32, pos as u32, MAX_SEQ as u32];
                    batch.set_pipeline(&p.kv_append);
                    batch.set_buffer(&self.scratch.k, 0, 0);
                    batch.set_buffer(&kv.k, 0, 1);
                    batch.set_bytes(bytemuck::cast_slice(&append_params), 2);
                    batch.dispatch_threadgroups((div_ceil(c.kv_num_heads * c.head_dim, 256), 1, 1), (256, 1, 1));

                    batch.set_pipeline(&p.kv_append);
                    batch.set_buffer(&self.scratch.v, 0, 0);
                    batch.set_buffer(&kv.v, 0, 1);
                    batch.set_bytes(bytemuck::cast_slice(&append_params), 2);
                    batch.dispatch_threadgroups((div_ceil(c.kv_num_heads * c.head_dim, 256), 1, 1), (256, 1, 1));

                    // ── Attention decode (GQA-aware, KV cache stride = MAX_SEQ) ──
                    let attn_params = [c.head_dim as u32, total_seq as u32, c.num_heads as u32, scale.to_bits(), c.kv_num_heads as u32, MAX_SEQ as u32];
                    batch.set_pipeline(&p.attention_decode);
                    batch.set_buffer(&self.scratch.q, 0, 0);
                    batch.set_buffer(&kv.k, 0, 1);
                    batch.set_buffer(&kv.v, 0, 2);
                    batch.set_buffer(&self.scratch.attn_out, 0, 3);
                    batch.set_bytes(bytemuck::cast_slice(&attn_params), 4);
                    batch.dispatch_threadgroups((c.num_heads, 1, 1), (256, 1, 1));

                    // ── O projection (input = attention output) ──
                    matvec_q4(batch, &self.scratch.attn_out, &layer.o_proj, &self.scratch.down, c.hidden_size as u32, (c.num_heads * c.head_dim) as u32);

                    // ── Residual add ──
                    let add_params = [c.hidden_size as u32];
                    batch.set_pipeline(&p.add_f16);
                    batch.set_buffer(&self.scratch.hidden, 0, 0);
                    batch.set_buffer(&self.scratch.down, 0, 1);
                    batch.set_buffer(&self.scratch.hidden, 0, 2); // in-place
                    batch.set_bytes(bytemuck::cast_slice(&add_params), 3);
                    batch.dispatch_threadgroups((div_ceil(c.hidden_size, 256), 1, 1), (256, 1, 1));

                    // ── Post RMS Norm ──
                    batch.set_pipeline(&p.rms_norm);
                    batch.set_buffer(&self.scratch.hidden, 0, 0);
                    batch.set_buffer(&layer.post_norm_weight, 0, 1);
                    batch.set_buffer(&self.scratch.hidden2, 0, 2);
                    batch.set_bytes(&norm_params_bytes, 3);
                    batch.dispatch_threadgroups((1, 1, 1), (256, 1, 1));

                    // ── Gate/Up projections (input = post-normed hidden2) ──
                    matvec_q4(batch, &self.scratch.hidden2, &layer.gate_proj, &self.scratch.gate, c.intermediate_size as u32, c.hidden_size as u32);
                    matvec_q4(batch, &self.scratch.hidden2, &layer.up_proj, &self.scratch.up, c.intermediate_size as u32, c.hidden_size as u32);

                    // ── SwiGLU ──
                    let silu_params = [c.intermediate_size as u32];
                    batch.set_pipeline(&p.silu_mul_f16);
                    batch.set_buffer(&self.scratch.gate, 0, 0);
                    batch.set_buffer(&self.scratch.up, 0, 1);
                    batch.set_buffer(&self.scratch.gate, 0, 2); // reuse gate as output
                    batch.set_bytes(bytemuck::cast_slice(&silu_params), 3);
                    batch.dispatch_threadgroups((div_ceil(c.intermediate_size, 256), 1, 1), (256, 1, 1));

                    // ── Down projection (input = SwiGLU output in gate scratch) ──
                    matvec_q4(batch, &self.scratch.gate, &layer.down_proj, &self.scratch.down, c.hidden_size as u32, c.intermediate_size as u32);

                    // ── Residual add ──
                    batch.set_pipeline(&p.add_f16);
                    batch.set_buffer(&self.scratch.hidden, 0, 0);
                    batch.set_buffer(&self.scratch.down, 0, 1);
                    batch.set_buffer(&self.scratch.hidden, 0, 2);
                    batch.set_bytes(bytemuck::cast_slice(&add_params), 3);
                    batch.dispatch_threadgroups((div_ceil(c.hidden_size, 256), 1, 1), (256, 1, 1));
                }

                // ── Final RMS Norm ──
                batch.set_pipeline(&p.rms_norm);
                batch.set_buffer(&self.scratch.hidden, 0, 0);
                batch.set_buffer(&self.final_norm_weight, 0, 1);
                batch.set_buffer(&self.scratch.hidden2, 0, 2);
                batch.set_bytes(&norm_params_bytes, 3);
                batch.dispatch_threadgroups((1, 1, 1), (256, 1, 1));

                // ── LM Head (Q4 fast matvec — vocab=151k, biggest single op) ──
                let lm_params = [c.vocab_size as u32, c.hidden_size as u32];
                batch.set_pipeline(&p.matvec_q4_fast);
                batch.set_buffer(&self.scratch.hidden2, 0, 0);
                batch.set_buffer(&self.lm_head_q4, 0, 1);
                batch.set_buffer(&self.scratch.logits, 0, 2);
                batch.set_bytes(bytemuck::cast_slice(&lm_params), 3);
                batch.dispatch_threadgroups((div_ceil(c.vocab_size, 8), 1, 1), (256, 1, 1));

                // ── Argmax ──
                let argmax_params = [c.vocab_size as u32];
                batch.set_pipeline(&p.argmax);
                batch.set_buffer(&self.scratch.logits, 0, 0);
                batch.set_buffer(&self.scratch.argmax_result, 0, 1);
                batch.set_bytes(bytemuck::cast_slice(&argmax_params), 2);
                batch.dispatch_threadgroups((1, 1, 1), (256, 1, 1));
            });
        });}

        self.past_seq_len = total_seq;

        // Read argmax result from GPU (this blocks until dispatch_batch completes)
        let result = self.scratch.argmax_result.with_data(|d| {
            u32::from_le_bytes([d[0], d[1], d[2], d[3]])
        });

        // Log timing every 20 steps
        if pos > 0 && pos % 50 == 0 {
            log::info!("Metal decode: pos={pos}, tok/s estimate from wall clock");
        }

        result
    }

    pub fn reset_kv_cache(&mut self) {
        self.past_seq_len = 0;
    }

    /// Read scratch.hidden as f32 (debug)
    pub fn debug_read_hidden(&self) -> Vec<f32> {
        let n = self.config.hidden_size;
        self.scratch.hidden.with_data(|d| {
            let f16s: &[u16] = bytemuck::cast_slice(&d[..n * 2]);
            f16s.iter().map(|&v| aruminium::fp16_to_f32(v)).collect()
        })
    }

    /// Read logits as f32 (debug) — first `count` elements
    pub fn debug_read_logits(&self, count: usize) -> Vec<f32> {
        self.scratch.logits.with_data(|d| {
            let n = count.min(d.len() / 2);
            let f16s: &[u16] = bytemuck::cast_slice(&d[..n * 2]);
            f16s.iter().map(|&v| aruminium::fp16_to_f32(v)).collect()
        })
    }
}

/// Quantize f32 weight matrix [N, K] to block_q4_0 format for Metal matvec_q4 kernel.
/// Layout: [K/32][N] blocks, each block = { half scale; uint8_t qs[16]; } = 18 bytes.
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
