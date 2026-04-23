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
        let weights = Weights::load(lm, &config)?;
        let max_seq = config.max_position_embeddings.min(8192);
        // Per-layer KV cache sizing: Gemma-4 full layers use different
        // (kv_heads, head_dim) than sliding layers.
        let kv_cache = (0..config.num_hidden_layers)
            .map(|i| {
                let sz = config.layer_kv_heads(i) * max_seq * config.layer_head_dim(i);
                (vec![0f32; sz], vec![0f32; sz])
            })
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
        let mut embed_row: Vec<f32> = embed_table.try_as_f32()?[row_start..row_start + hidden_size].to_vec();
        // Gemma family: scale embeddings by sqrt(hidden_size) before the first
        // layer. Llama/Qwen don't do this. Keyed off model_type prefix so we
        // catch gemma, gemma2, gemma3, gemma4 without per-version flags.
        if c.model_type.starts_with("gemma") {
            let scale = (hidden_size as f32).sqrt();
            for v in embed_row.iter_mut() {
                *v *= scale;
            }
        }
        let mut hidden = Tensor::try_from_f32(vec![1, hidden_size], embed_row)?;

        if prof_enabled {
            self.prof.embed_ms += t_embed.elapsed().as_secs_f64() * 1000.0;
        }

        let pos = self.past_seq_len as f32;
        let pos_tensor = Tensor::from_f32(vec![1], vec![pos]);

        let debug_layers = std::env::var("MR_DEBUG_LAYERS").is_ok();
        if debug_layers {
            let h = hidden.try_as_f32()?;
            let m = h.iter().map(|v| v.abs()).fold(0f32, f32::max);
            let s = (h.iter().map(|v| v * v).sum::<f32>() / h.len() as f32).sqrt();
            eprintln!("post-embed     abs_max={m:.4} rms={s:.4}");
        }
        for i in 0..c.num_hidden_layers {
            hidden = forward_layer(
                &hidden,
                i,
                &self.weights.layers[i],
                c,
                &pos_tensor,
                backend,
                &mut self.kv_cache[i],
                self.past_seq_len,
                if prof_enabled { Some(&mut self.prof) } else { None },
            )?;
            if debug_layers {
                let h = hidden.try_as_f32()?;
                let m = h.iter().map(|v| v.abs()).fold(0f32, f32::max);
                let s = (h.iter().map(|v| v * v).sum::<f32>() / h.len() as f32).sqrt();
                let kind = match c.layer_types.get(i).copied() {
                    Some(crate::llama_style::config::LayerKind::Full) => "full",
                    _ => "slid",
                };
                eprintln!("layer {i:>3} {kind} abs_max={m:>9.4} rms={s:>8.4}");
            }
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
        if debug_layers {
            let h = final_normed.try_as_f32()?;
            let m = h.iter().map(|v| v.abs()).fold(0f32, f32::max);
            let s = (h.iter().map(|v| v * v).sum::<f32>() / h.len() as f32).sqrt();
            eprintln!("post-final-norm abs_max={m:.4} rms={s:.4}");
        }
        let logits = qw_matmul(&final_normed, lm_head_qw)?;
        if debug_layers {
            let h = backend.download_f32(&logits)?;
            let m = h.iter().map(|v| v.abs()).fold(0f32, f32::max);
            let s = (h.iter().map(|v| v * v).sum::<f32>() / h.len() as f32).sqrt();
            eprintln!("post-lm-head    abs_max={m:.4} rms={s:.4}");
        }
        if prof_enabled {
            self.prof.lm_head_ms += t_lm.elapsed().as_secs_f64() * 1000.0;
        }

        if prof_enabled {
            self.prof.forwards += 1;
        }
        self.past_seq_len += 1;
        let mut logits_vec = backend.download_f32(&logits)?;

        // LlamaStyle+ (Gemma 3/4): final logit softcapping.
        // Spec: reference/runtime/arch.md §"Final logit softcapping"
        if let Some(cap) = c.final_logit_softcapping {
            for v in logits_vec.iter_mut() {
                *v = (*v / cap).tanh() * cap;
            }
        }

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
    layer_idx: usize,
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
    // LlamaStyle+ (Gemma-4) per-layer dims. LlamaStyle returns the global ones.
    let head_dim = config.layer_head_dim(layer_idx);
    let num_heads = config.num_attention_heads;
    let kv_heads = config.layer_kv_heads(layer_idx);
    let sliding_window = config.layer_window(layer_idx);

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

    // 3. RoPE on Q, K — per-layer base + rope_dim (Gemma-4 full layers
    // use rope_theta_full and partial rotary factor; sliding + LlamaStyle
    // use the regular rope_theta and full head_dim).
    let t = Instant::now();
    let layer_rope_base = config.layer_rope_theta(layer_idx);
    let layer_rope_dim = config.layer_rope_dim(layer_idx);
    let q_shape = vec![num_heads, head_dim];
    let k_shape = vec![kv_heads, head_dim];
    let q_reshaped = Tensor::from_f32(q_shape.clone(), q.to_f32_vec());
    let k_reshaped = Tensor::from_f32(k_shape.clone(), k.to_f32_vec());
    let q_roped = backend
        .execute(
            &Op::Rope {
                head_dim: head_dim as u32,
                rope_dim: layer_rope_dim as u32,
                base: layer_rope_base,
            },
            &[&q_reshaped, pos],
        )?
        .remove(0);
    let k_roped = backend
        .execute(
            &Op::Rope {
                head_dim: head_dim as u32,
                rope_dim: layer_rope_dim as u32,
                base: layer_rope_base,
            },
            &[&k_reshaped, pos],
        )?
        .remove(0);
    acc_rope += t.elapsed().as_secs_f64() * 1000.0;

    // 4. Append to KV cache, build full K and V views for attention.
    // Gemma 4: V additionally goes through RMSNorm-no-scale before caching
    // (per HF Gemma4TextAttention v_norm with_scale=False). Applied on
    // EVERY layer (sliding and full alike) — the `is_kv_shared_layer`
    // branch in HF only skips re-projecting K and V, but the norm is
    // unconditional for non-shared layers. Our num_kv_shared_layers is 0
    // for gemma-4-31b so we apply to all.
    let t = Instant::now();
    let v_flat = if config.model_type.starts_with("gemma4") {
        let mut v_data = v.to_f32_vec();
        // Per-head RMSNorm without learned scale: divide each head's vector
        // by sqrt(mean(x²) + eps).
        let inv_d = 1.0 / head_dim as f32;
        for h in 0..kv_heads {
            let off = h * head_dim;
            let mut sumsq = 0f32;
            for j in 0..head_dim {
                let val = v_data[off + j];
                sumsq += val * val;
            }
            let rms = (sumsq * inv_d + eps).sqrt();
            let scale = 1.0 / rms;
            for j in 0..head_dim {
                v_data[off + j] *= scale;
            }
        }
        v_data
    } else {
        v.to_f32_vec()
    };
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
    // Per-layer attention scale: Gemma uses query_pre_attn_scalar (default
    // 256) instead of head_dim, which matters for layers whose head_dim
    // differs from that scalar (Gemma-4 full layers: head_dim=512 but
    // scalar=256 → scaling = 1/sqrt(256), not 1/sqrt(512)).
    let q_heads = q_roped.to_f32_vec();
    let mut attn_out = vec![0f32; num_heads * head_dim];
    let scale = config.layer_attn_scale(layer_idx);
    // Sliding-window mask (LlamaStyle+, Gemma 3/4 sliding layers): position s
    // is valid iff s > current_pos - window. current_pos = total_seq - 1.
    // Spec: reference/runtime/arch.md §"Sliding window attention"
    let window_start: Option<usize> = sliding_window.map(|w| total_seq.saturating_sub(w));

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
        // Sliding-window mask: zero out positions outside the window.
        if let Some(start) = window_start {
            for s in 0..start {
                scores[s] = f32::NEG_INFINITY;
            }
        }
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
    let mut attn_proj = qw_matmul(&attn_tensor, &layer.o_proj)?;
    // Gemma 2/3/4: norm applied to attention output before residual.
    if let Some(ref n) = layer.post_attn_norm {
        attn_proj = backend
            .execute(&Op::RmsNorm { eps }, &[&attn_proj, n])?
            .remove(0);
    }
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

    // FFN: 3 quant matmuls + elementwise activation(gate) * up.
    // Activation per `config.hidden_activation` (LlamaStyle: SiLU; Gemma uses GELU).
    let t = Instant::now();
    let gate = qw_matmul(&normed2, &layer.gate_proj)?;
    let up = qw_matmul(&normed2, &layer.up_proj)?;
    let mut mid = gate.to_f32_vec();
    let up_vec = up.to_f32_vec();
    use crate::llama_style::config::HiddenActivation;
    let act: fn(f32) -> f32 = match config.hidden_activation {
        HiddenActivation::Silu => |x| x / (1.0 + (-x).exp()),
        HiddenActivation::GeluTanh => |x| {
            // 0.5 x (1 + tanh(sqrt(2/π) (x + 0.044715 x³)))
            let c = (2.0_f32 / std::f32::consts::PI).sqrt();
            0.5 * x * (1.0 + (c * (x + 0.044715 * x * x * x)).tanh())
        },
        HiddenActivation::GeluErf => |x| {
            // 0.5 x (1 + erf(x / sqrt(2)))
            let inv_sqrt2 = 1.0_f32 / std::f32::consts::SQRT_2;
            // libm erf via approximation matching cpu/activation.rs::erf_approx
            let z = x * inv_sqrt2;
            let sign = if z < 0.0 { -1.0 } else { 1.0 };
            let zabs = z.abs();
            let p = 0.327_591_1_f32;
            let a1 = 0.254_829_592_f32;
            let a2 = -0.284_496_736_f32;
            let a3 = 1.421_413_741_f32;
            let a4 = -1.453_152_027_f32;
            let a5 = 1.061_405_429_f32;
            let t_ = 1.0 / (1.0 + p * zabs);
            let y = 1.0
                - ((((a5 * t_ + a4) * t_ + a3) * t_ + a2) * t_ + a1)
                    * t_
                    * (-zabs * zabs).exp();
            0.5 * x * (1.0 + sign * y)
        },
    };
    for (m, u) in mid.iter_mut().zip(up_vec.iter()) {
        *m = act(*m) * *u;
    }
    let mid_t = Tensor::from_f32(gate.shape.clone(), mid);
    let mut ffn_out = qw_matmul(&mid_t, &layer.down_proj)?;
    // Gemma 2/3/4: norm applied to FFN output before residual.
    if let Some(ref n) = layer.post_ffw_norm {
        ffn_out = backend
            .execute(&Op::RmsNorm { eps }, &[&ffn_out, n])?
            .remove(0);
    }
    acc_ffn += t.elapsed().as_secs_f64() * 1000.0;

    // 9. Residual. Gemma-4 multiplies the final layer output by a SCALAR
    // `layer_scalar` (HF: `hidden_states *= self.layer_scalar`). The tensor
    // is shape [1] in our .model.
    let _ = hidden_size;
    let t = Instant::now();
    let mut out = backend
        .execute(&Op::Add, &[&hidden1, &ffn_out])?
        .remove(0);
    // Gemma-4 layer_scalar (HF: hidden_states *= self.layer_scalar). The
    // register_buffer is initialised to 1.0 but checkpoint values can differ
    // (per-layer trained scalar). Without this scale, activations explode
    // through the residual stream.
    if let Some(ref s) = layer.layer_output_scale {
        let scalar = s.as_f32()[0];
        let mut out_v = out.to_f32_vec();
        for v in out_v.iter_mut() {
            *v *= scalar;
        }
        out = Tensor::from_f32(out.shape.clone(), out_v);
    }
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
