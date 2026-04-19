//! LlamaStyle forward pass.
//!
//! Spec: reference/runtime/arch.md#llamastyle

use super::config::LlamaConfig;
use super::weights::{LayerWeights, QuantWeight, Weights};
use crate::backend::{Backend, BackendError};
use crate::cpu::matmul_quant_f32;
use crate::format::{FormatError, LoadedModel};
use crate::op::Op;
use crate::tensor::Tensor;
use std::path::Path;

/// Fused quant matmul via CPU kernel. Returns f32 Tensor.
fn qw_matmul(x: &Tensor, w: &QuantWeight) -> Result<Tensor, BackendError> {
    matmul_quant_f32(x, w.bytes.as_slice(), w.dtype, w.n(), w.k())
}

pub struct LlamaModel {
    pub config: LlamaConfig,
    pub weights: Weights,
    /// KV cache per layer: K and V tensors shape [kv_heads, max_seq, head_dim].
    pub past_seq_len: usize,
    pub kv_cache: Vec<(Vec<f32>, Vec<f32>)>,
    /// Per-op timing accumulator. Reset via `reset_prof`, read via `prof`.
    pub prof: ForwardProf,
}

#[derive(Default, Clone, Debug)]
pub struct ForwardProf {
    pub enabled: bool,
    pub embed_ms: f64,
    pub input_norm_ms: f64,
    pub qkv_proj_ms: f64,
    pub qk_norm_ms: f64,
    pub rope_ms: f64,
    pub kv_append_ms: f64,
    pub attention_ms: f64,
    pub o_proj_ms: f64,
    pub post_norm_ms: f64,
    pub ffn_ms: f64,
    pub residual_ms: f64,
    pub final_norm_ms: f64,
    pub lm_head_ms: f64,
    pub forwards: usize,
}

impl ForwardProf {
    pub fn total_ms(&self) -> f64 {
        self.embed_ms
            + self.input_norm_ms
            + self.qkv_proj_ms
            + self.qk_norm_ms
            + self.rope_ms
            + self.kv_append_ms
            + self.attention_ms
            + self.o_proj_ms
            + self.post_norm_ms
            + self.ffn_ms
            + self.residual_ms
            + self.final_norm_ms
            + self.lm_head_ms
    }
    pub fn summary(&self) -> String {
        let total = self.total_ms().max(0.001);
        let pct = |ms: f64| (ms / total) * 100.0;
        format!(
            "  embed        {:>7.1} ms  ({:>5.1}%)\n\
             \x20 input_norm   {:>7.1} ms  ({:>5.1}%)\n\
             \x20 qkv_proj     {:>7.1} ms  ({:>5.1}%)\n\
             \x20 qk_norm      {:>7.1} ms  ({:>5.1}%)\n\
             \x20 rope         {:>7.1} ms  ({:>5.1}%)\n\
             \x20 kv_append    {:>7.1} ms  ({:>5.1}%)\n\
             \x20 attention    {:>7.1} ms  ({:>5.1}%)\n\
             \x20 o_proj       {:>7.1} ms  ({:>5.1}%)\n\
             \x20 post_norm    {:>7.1} ms  ({:>5.1}%)\n\
             \x20 ffn          {:>7.1} ms  ({:>5.1}%)\n\
             \x20 residual     {:>7.1} ms  ({:>5.1}%)\n\
             \x20 final_norm   {:>7.1} ms  ({:>5.1}%)\n\
             \x20 lm_head      {:>7.1} ms  ({:>5.1}%)\n\
             \x20 ─────────────────────────────\n\
             \x20 TOTAL        {:>7.1} ms  ({} forwards)",
            self.embed_ms, pct(self.embed_ms),
            self.input_norm_ms, pct(self.input_norm_ms),
            self.qkv_proj_ms, pct(self.qkv_proj_ms),
            self.qk_norm_ms, pct(self.qk_norm_ms),
            self.rope_ms, pct(self.rope_ms),
            self.kv_append_ms, pct(self.kv_append_ms),
            self.attention_ms, pct(self.attention_ms),
            self.o_proj_ms, pct(self.o_proj_ms),
            self.post_norm_ms, pct(self.post_norm_ms),
            self.ffn_ms, pct(self.ffn_ms),
            self.residual_ms, pct(self.residual_ms),
            self.final_norm_ms, pct(self.final_norm_ms),
            self.lm_head_ms, pct(self.lm_head_ms),
            total,
            self.forwards,
        )
    }
}

impl LlamaModel {
    pub fn load(path: &Path) -> Result<Self, FormatError> {
        let lm = LoadedModel::load(path)?;
        Self::from_loaded(&lm)
    }

    pub fn from_loaded(lm: &LoadedModel) -> Result<Self, FormatError> {
        let config = LlamaConfig::parse(&lm.file.config, &lm.tensors)?;
        let q_dim = config.num_attention_heads * config.head_dim;
        let kv_dim = config.num_key_value_heads * config.head_dim;
        let weights = Weights::load(
            lm,
            config.num_hidden_layers,
            config.tie_word_embeddings,
            config.vocab_size,
            config.hidden_size,
            q_dim,
            kv_dim,
            config.intermediate_size,
        )?;
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
            prof: ForwardProf::default(),
        })
    }

    /// Enable per-op timing. Reset counters first.
    pub fn enable_prof(&mut self) {
        self.prof = ForwardProf {
            enabled: true,
            ..Default::default()
        };
    }

    /// Move norm/embed tensors to the backend (if it supports persistent
    /// upload). Matmul weights are QuantWeight (raw bytes on host) and
    /// dispatch to CPU quant matmul regardless of "backend" selected;
    /// GPU speedup requires per-backend quant matmul kernel (future).
    pub fn to_backend(&mut self, backend: &dyn Backend) -> Result<(), BackendError> {
        self.weights.final_norm = backend.to_backend(&self.weights.final_norm)?;
        for layer in &mut self.weights.layers {
            layer.input_norm = backend.to_backend(&layer.input_norm)?;
            layer.post_norm = backend.to_backend(&layer.post_norm)?;
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

        // Context overflow check.
        let max_seq = c.max_position_embeddings.min(8192);
        if self.past_seq_len >= max_seq {
            return Err(BackendError::ContextOverflow {
                pos: self.past_seq_len,
                max: max_seq,
            });
        }

        // Token id bounds check.
        if (token_id as usize) >= c.vocab_size {
            return Err(BackendError::InvalidInput {
                op: "TokenEmbed",
                reason: format!(
                    "token_id {token_id} out of vocab range {}",
                    c.vocab_size
                ),
            });
        }

        use std::time::Instant;
        let prof_enabled = self.prof.enabled;
        let t_embed = Instant::now();

        // Embed lookup: one row of embed_tokens.
        let embed_table = &self.weights.embed_tokens;
        let hidden_size = c.hidden_size;
        let row_start = (token_id as usize) * hidden_size;
        let embed_row: Vec<f32> = embed_table.try_as_f32()?[row_start..row_start + hidden_size].to_vec();
        let mut hidden = Tensor::try_from_f32(vec![1, hidden_size], embed_row)?;

        if prof_enabled {
            self.prof.embed_ms += t_embed.elapsed().as_secs_f64() * 1000.0;
        }

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
                if prof_enabled { Some(&mut self.prof) } else { None },
            )?;
        }

        let t_final = Instant::now();
        // Final norm + lm_head
        let final_normed = backend
            .execute(
                &Op::RmsNorm {
                    eps: c.rms_norm_eps,
                },
                &[&hidden, &self.weights.final_norm],
            )?
            .remove(0);
        if prof_enabled {
            self.prof.final_norm_ms += t_final.elapsed().as_secs_f64() * 1000.0;
        }

        let t_lm = Instant::now();
        let lm_head_qw = self
            .weights
            .lm_head
            .as_ref()
            .unwrap_or(&self.weights.embed_tokens_quant);
        let logits = qw_matmul(&final_normed, lm_head_qw)?;
        if prof_enabled {
            self.prof.lm_head_ms += t_lm.elapsed().as_secs_f64() * 1000.0;
        }

        if prof_enabled {
            self.prof.forwards += 1;
        }
        self.past_seq_len += 1;
        let logits_vec = backend.download_f32(&logits)?;

        // NaN/Inf detection at the forward boundary.
        if logits_vec.iter().any(|v| !v.is_finite()) {
            return Err(BackendError::NonFiniteOutput {
                op: "forward",
                layer: c.num_hidden_layers,
                pos: self.past_seq_len - 1,
            });
        }

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
    prof: Option<&mut ForwardProf>,
) -> Result<Tensor, BackendError> {
    use std::time::Instant;
    let eps = config.rms_norm_eps;
    let hidden_size = config.hidden_size;
    let head_dim = config.head_dim;
    let num_heads = config.num_attention_heads;
    let kv_heads = config.num_key_value_heads;

    // accumulator helpers
    let mut acc_input_norm = 0f64;
    let mut acc_qkv_proj = 0f64;
    let mut acc_qk_norm = 0f64;
    let mut acc_rope = 0f64;
    let mut acc_kv_append = 0f64;
    let mut acc_attention = 0f64;
    let mut acc_o_proj = 0f64;
    let mut acc_post_norm = 0f64;
    let mut acc_ffn = 0f64;
    let mut acc_residual = 0f64;

    // 1. Input RmsNorm
    let t = Instant::now();
    let normed = backend
        .execute(&Op::RmsNorm { eps }, &[hidden, &layer.input_norm])?
        .remove(0);
    acc_input_norm += t.elapsed().as_secs_f64() * 1000.0;

    // 2. QKV projections — fused dequant+matmul on CPU.
    let t = Instant::now();
    let mut q = qw_matmul(&normed, &layer.q_proj)?;
    let mut k = qw_matmul(&normed, &layer.k_proj)?;
    let mut v = qw_matmul(&normed, &layer.v_proj)?;

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
    acc_qkv_proj += t.elapsed().as_secs_f64() * 1000.0;

    // QK-norm (Qwen3) — per-head RmsNorm
    let t = Instant::now();
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
    acc_qk_norm += t.elapsed().as_secs_f64() * 1000.0;

    // 3. RoPE on Q, K
    let t = Instant::now();
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
    acc_rope += t.elapsed().as_secs_f64() * 1000.0;

    // 4. Append to KV cache, build full K and V views for attention.
    let t = Instant::now();
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
    acc_kv_append += t.elapsed().as_secs_f64() * 1000.0;

    // Build expanded K and V for GQA attention: [num_heads, total_seq, head_dim]
    let t = Instant::now();
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

    acc_attention += t.elapsed().as_secs_f64() * 1000.0;

    // 6. Output projection — quant matmul.
    let t = Instant::now();
    let attn_tensor = Tensor::from_f32(vec![1, num_heads * head_dim], attn_out);
    let attn_proj = qw_matmul(&attn_tensor, &layer.o_proj)?;
    acc_o_proj += t.elapsed().as_secs_f64() * 1000.0;

    // 7. Residual
    let t = Instant::now();
    let hidden1 = backend
        .execute(&Op::Add, &[hidden, &attn_proj])?
        .remove(0);
    acc_residual += t.elapsed().as_secs_f64() * 1000.0;

    // 8. Post-attention RmsNorm + FFN (SwiGLU)
    let t = Instant::now();
    let normed2 = backend
        .execute(&Op::RmsNorm { eps }, &[&hidden1, &layer.post_norm])?
        .remove(0);
    acc_post_norm += t.elapsed().as_secs_f64() * 1000.0;

    // SwiGLU FFN as 3 quant matmuls + elementwise silu*up.
    let t = Instant::now();
    let gate = qw_matmul(&normed2, &layer.gate_proj)?;
    let up = qw_matmul(&normed2, &layer.up_proj)?;
    // Fused silu(gate) * up in-place.
    let mut mid = gate.to_f32_vec();
    let up_vec = up.to_f32_vec();
    for (m, u) in mid.iter_mut().zip(up_vec.iter()) {
        let s = *m / (1.0 + (-*m).exp());
        *m = s * u;
    }
    let mid_t = Tensor::from_f32(gate.shape.clone(), mid);
    let ffn_out = qw_matmul(&mid_t, &layer.down_proj)?;
    acc_ffn += t.elapsed().as_secs_f64() * 1000.0;

    // 9. Residual
    let _ = hidden_size;
    let t = Instant::now();
    let out = backend
        .execute(&Op::Add, &[&hidden1, &ffn_out])?
        .remove(0);
    acc_residual += t.elapsed().as_secs_f64() * 1000.0;

    if let Some(p) = prof {
        p.input_norm_ms += acc_input_norm;
        p.qkv_proj_ms += acc_qkv_proj;
        p.qk_norm_ms += acc_qk_norm;
        p.rope_ms += acc_rope;
        p.kv_append_ms += acc_kv_append;
        p.attention_ms += acc_attention;
        p.o_proj_ms += acc_o_proj;
        p.post_norm_ms += acc_post_norm;
        p.ffn_ms += acc_ffn;
        p.residual_ms += acc_residual;
    }
    Ok(out)
}
