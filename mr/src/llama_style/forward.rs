//! LlamaStyle forward pass.
//!
//! Spec: reference/runtime/arch.md#llamastyle

use super::config::LlamaConfig;
use super::weights::{Weights, LayerWeights};
use crate::backend::{Backend, BackendError};
use crate::format::{FormatError, LoadedModel};
use crate::op::Op;
use crate::tensor::Tensor;
use std::path::Path;

pub struct LlamaModel {
    pub config: LlamaConfig,
    pub weights: Weights,
    /// KV cache per layer: K and V tensors shape [kv_heads, max_seq, head_dim].
    pub past_seq_len: usize,
    pub kv_cache: Vec<(Vec<f32>, Vec<f32>)>,
}

impl LlamaModel {
    pub fn load(path: &Path) -> Result<Self, FormatError> {
        let lm = LoadedModel::load(path)?;
        Self::from_loaded(&lm)
    }

    pub fn from_loaded(lm: &LoadedModel) -> Result<Self, FormatError> {
        let config = LlamaConfig::parse(&lm.file.config, &lm.tensors)?;
        let weights = Weights::load(lm, config.num_hidden_layers, config.tie_word_embeddings)?;
        let max_seq = config.max_position_embeddings.min(8192);
        let kv_size = config.num_key_value_heads * max_seq * config.head_dim;
        let kv_cache = (0..config.num_hidden_layers)
            .map(|_| (vec![0f32; kv_size], vec![0f32; kv_size]))
            .collect();
        Ok(Self {
            config,
            weights,
            past_seq_len: 0,
            kv_cache,
        })
    }

    /// Move weight tensors (except embed) to the backend once.
    /// Embed stays on host for efficient row lookup.
    /// Call after load. For CPU backend this is a no-op.
    pub fn to_backend(&mut self, backend: &dyn Backend) -> Result<(), BackendError> {
        // embed_tokens stays on host — we extract one row per forward.
        self.weights.final_norm = backend.to_backend(&self.weights.final_norm)?;
        if let Some(ref lm_head) = self.weights.lm_head {
            self.weights.lm_head = Some(backend.to_backend(lm_head)?);
        }
        for layer in &mut self.weights.layers {
            layer.input_norm = backend.to_backend(&layer.input_norm)?;
            layer.q_proj = backend.to_backend(&layer.q_proj)?;
            layer.k_proj = backend.to_backend(&layer.k_proj)?;
            layer.v_proj = backend.to_backend(&layer.v_proj)?;
            layer.o_proj = backend.to_backend(&layer.o_proj)?;
            if let Some(ref b) = layer.q_proj_bias {
                layer.q_proj_bias = Some(backend.to_backend(b)?);
            }
            if let Some(ref b) = layer.k_proj_bias {
                layer.k_proj_bias = Some(backend.to_backend(b)?);
            }
            if let Some(ref b) = layer.v_proj_bias {
                layer.v_proj_bias = Some(backend.to_backend(b)?);
            }
            if let Some(ref n) = layer.q_norm {
                layer.q_norm = Some(backend.to_backend(n)?);
            }
            if let Some(ref n) = layer.k_norm {
                layer.k_norm = Some(backend.to_backend(n)?);
            }
            layer.post_norm = backend.to_backend(&layer.post_norm)?;
            layer.gate_proj = backend.to_backend(&layer.gate_proj)?;
            layer.up_proj = backend.to_backend(&layer.up_proj)?;
            layer.down_proj = backend.to_backend(&layer.down_proj)?;
        }
        Ok(())
    }

    pub fn reset_kv_cache(&mut self) {
        self.past_seq_len = 0;
    }

    /// Single forward step: input one token, get logits for next.
    ///
    /// Uses the provided backend for every op. If backend reports
    /// unsupported, caller should fall back to CPU.
    pub fn forward(
        &mut self,
        token_id: u32,
        backend: &dyn Backend,
    ) -> Result<Vec<f32>, BackendError> {
        let c = &self.config;

        // Embed lookup: one row of embed_tokens.
        let embed_table = &self.weights.embed_tokens;
        let hidden_size = c.hidden_size;
        let row_start = (token_id as usize) * hidden_size;
        let embed_row: Vec<f32> = embed_table.as_f32()[row_start..row_start + hidden_size].to_vec();
        let mut hidden = Tensor::from_f32(vec![1, hidden_size], embed_row);

        let pos = self.past_seq_len as f32;
        let pos_tensor = Tensor::from_f32(vec![1], vec![pos]);

        for i in 0..c.num_hidden_layers {
            hidden = forward_layer(
                &hidden,
                &self.weights.layers[i],
                c,
                &pos_tensor,
                backend,
                &mut self.kv_cache[i],
                self.past_seq_len,
            )?;
        }

        // Final norm + lm_head
        let final_normed = backend
            .execute(
                &Op::RmsNorm {
                    eps: c.rms_norm_eps,
                },
                &[&hidden, &self.weights.final_norm],
            )?
            .remove(0);

        let lm_head = self
            .weights
            .lm_head
            .as_ref()
            .unwrap_or(&self.weights.embed_tokens);
        let logits = backend
            .execute(&Op::Matmul, &[&final_normed, lm_head])?
            .remove(0);

        self.past_seq_len += 1;
        let logits_vec = backend.download_f32(&logits)?;
        Ok(logits_vec)
    }
}

fn forward_layer(
    hidden: &Tensor,
    layer: &LayerWeights,
    config: &LlamaConfig,
    pos: &Tensor,
    backend: &dyn Backend,
    kv: &mut (Vec<f32>, Vec<f32>),
    past_seq_len: usize,
) -> Result<Tensor, BackendError> {
    let eps = config.rms_norm_eps;
    let hidden_size = config.hidden_size;
    let head_dim = config.head_dim;
    let num_heads = config.num_attention_heads;
    let kv_heads = config.num_key_value_heads;

    // 1. Input RmsNorm
    let normed = backend
        .execute(&Op::RmsNorm { eps }, &[hidden, &layer.input_norm])?
        .remove(0);

    // 2. QKV projections
    let mut q = backend
        .execute(&Op::Matmul, &[&normed, &layer.q_proj])?
        .remove(0);
    let mut k = backend
        .execute(&Op::Matmul, &[&normed, &layer.k_proj])?
        .remove(0);
    let mut v = backend
        .execute(&Op::Matmul, &[&normed, &layer.v_proj])?
        .remove(0);

    // Attention biases (Qwen2)
    if let Some(bias) = &layer.q_proj_bias {
        q = backend.execute(&Op::Add, &[&q, bias])?.remove(0);
    }
    if let Some(bias) = &layer.k_proj_bias {
        k = backend.execute(&Op::Add, &[&k, bias])?.remove(0);
    }
    if let Some(bias) = &layer.v_proj_bias {
        v = backend.execute(&Op::Add, &[&v, bias])?.remove(0);
    }

    // QK-norm (Qwen3) — per-head RmsNorm
    if let (Some(qn), Some(kn)) = (&layer.q_norm, &layer.k_norm) {
        // Reshape Q: [1, num_heads * head_dim] → [num_heads, head_dim]
        let q_reshaped = Tensor::from_f32(vec![num_heads, head_dim], q.to_f32_vec());
        let k_reshaped = Tensor::from_f32(vec![kv_heads, head_dim], k.to_f32_vec());
        q = backend
            .execute(&Op::RmsNorm { eps }, &[&q_reshaped, qn])?
            .remove(0);
        k = backend
            .execute(&Op::RmsNorm { eps }, &[&k_reshaped, kn])?
            .remove(0);
        q = Tensor::from_f32(vec![1, num_heads * head_dim], q.to_f32_vec());
        k = Tensor::from_f32(vec![1, kv_heads * head_dim], k.to_f32_vec());
    }

    // 3. RoPE on Q, K
    let q_shape = vec![num_heads, head_dim];
    let k_shape = vec![kv_heads, head_dim];
    let q_reshaped = Tensor::from_f32(q_shape.clone(), q.to_f32_vec());
    let k_reshaped = Tensor::from_f32(k_shape.clone(), k.to_f32_vec());
    let q_roped = backend
        .execute(
            &Op::Rope {
                head_dim: head_dim as u32,
                base: config.rope_theta,
            },
            &[&q_reshaped, pos],
        )?
        .remove(0);
    let k_roped = backend
        .execute(
            &Op::Rope {
                head_dim: head_dim as u32,
                base: config.rope_theta,
            },
            &[&k_reshaped, pos],
        )?
        .remove(0);

    // 4. Append to KV cache, build full K and V views for attention.
    let v_flat = v.to_f32_vec();
    let k_flat = k_roped.to_f32_vec();

    // Cache layout: [kv_heads, max_seq, head_dim] flat row-major.
    let max_seq = config.max_position_embeddings.min(8192);
    for h in 0..kv_heads {
        let src_base = h * head_dim;
        let dst_base = h * max_seq * head_dim + past_seq_len * head_dim;
        for d in 0..head_dim {
            kv.0[dst_base + d] = k_flat[src_base + d];
            kv.1[dst_base + d] = v_flat[src_base + d];
        }
    }
    let total_seq = past_seq_len + 1;

    // Build expanded K and V for GQA attention: [num_heads, total_seq, head_dim]
    let repeat = num_heads / kv_heads;
    let mut k_full = vec![0f32; num_heads * total_seq * head_dim];
    let mut v_full = vec![0f32; num_heads * total_seq * head_dim];
    for h in 0..num_heads {
        let kv_h = h / repeat;
        for s in 0..total_seq {
            for d in 0..head_dim {
                let src = kv_h * max_seq * head_dim + s * head_dim + d;
                let dst = h * total_seq * head_dim + s * head_dim + d;
                k_full[dst] = kv.0[src];
                v_full[dst] = kv.1[src];
            }
        }
    }

    // 5. Scaled dot-product attention (decode: single query)
    // Q: [num_heads, 1, head_dim]; K,V: [num_heads, total_seq, head_dim]
    let q_heads = q_roped.to_f32_vec();
    let mut attn_out = vec![0f32; num_heads * head_dim];
    let scale = 1.0 / (head_dim as f32).sqrt();
    for h in 0..num_heads {
        // scores[s] = sum_d Q[h,d] * K[h,s,d] * scale
        let mut scores = vec![0f32; total_seq];
        let q_off = h * head_dim;
        let kv_off = h * total_seq * head_dim;
        for s in 0..total_seq {
            let mut acc = 0f32;
            for d in 0..head_dim {
                acc += q_heads[q_off + d] * k_full[kv_off + s * head_dim + d];
            }
            scores[s] = acc * scale;
        }
        // Causal mask: scores[s] valid only for s <= past_seq_len
        // During decode (total_seq = past_seq_len+1), all positions ≤ current, no mask needed.
        // Softmax over scores
        let max_s = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0f32;
        for s in scores.iter_mut() {
            *s = (*s - max_s).exp();
            sum += *s;
        }
        for s in scores.iter_mut() {
            *s /= sum;
        }
        // attn_out[h,d] = sum_s scores[s] * V[h,s,d]
        let out_off = h * head_dim;
        for s in 0..total_seq {
            let v_row = &v_full[kv_off + s * head_dim..kv_off + (s + 1) * head_dim];
            for d in 0..head_dim {
                attn_out[out_off + d] += scores[s] * v_row[d];
            }
        }
    }

    // 6. Output projection
    let attn_tensor = Tensor::from_f32(vec![1, num_heads * head_dim], attn_out);
    let attn_proj = backend
        .execute(&Op::Matmul, &[&attn_tensor, &layer.o_proj])?
        .remove(0);

    // 7. Residual
    let hidden1 = backend
        .execute(&Op::Add, &[hidden, &attn_proj])?
        .remove(0);

    // 8. Post-attention RmsNorm + FFN (SwiGLU)
    let normed2 = backend
        .execute(&Op::RmsNorm { eps }, &[&hidden1, &layer.post_norm])?
        .remove(0);
    let ffn_out = backend
        .execute(
            &Op::SwiGlu,
            &[
                &normed2,
                &layer.gate_proj,
                &layer.up_proj,
                &layer.down_proj,
            ],
        )?
        .remove(0);

    // 9. Residual
    let _ = hidden_size;
    let out = backend
        .execute(&Op::Add, &[&hidden1, &ffn_out])?
        .remove(0);
    Ok(out)
}
