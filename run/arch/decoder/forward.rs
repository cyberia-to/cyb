//! LlamaStyle forward pass.
//!
//! Spec: specs/arch.md#llamastyle

use super::config::LlamaConfig;
use super::weights::{LayerWeights, QuantWeight, Weights};
use crate::backend::{Backend, BackendError};
use crate::format::{FormatError, LoadedModel};
use crate::core::op::Op;
use crate::core::tensor::Tensor;
use std::path::Path;
use std::sync::Arc;

/// Fused quant matmul — dispatches to backend (Metal on honeycrisp, CPU otherwise).
/// Uses w.tensor which is GPU-resident after to_backend(), avoiding per-call uploads.
fn qw_matmul(x: &Tensor, w: &QuantWeight, backend: &dyn Backend) -> Result<Tensor, BackendError> {
    backend.quant_matmul(x, &w.tensor)
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
        // (kv_heads, head_dim) than sliding layers. Sliding layers cap their
        // cache to the window size to avoid 10× RAM bloat on large-context models.
        let kv_cache = (0..config.num_hidden_layers)
            .map(|i| {
                let cache_seq = config.layer_kv_cache_seq(i, max_seq);
                let sz = config.layer_kv_heads(i) * cache_seq * config.layer_head_dim(i);
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

    /// Upload all weights to the backend. Norm/embed tensors go as f32;
    /// quant weights upload their raw bytes so the backend can dispatch
    /// fused dequant+matmul kernels without per-call re-uploads.
    pub fn to_backend(&mut self, backend: &dyn Backend) -> Result<(), BackendError> {
        self.weights.final_norm = backend.to_backend(&self.weights.final_norm)?;
        let upload_quant = backend.uploads_quant_weights();
        if upload_quant {
            self.weights.embed_tokens_quant.tensor =
                backend.to_backend(&self.weights.embed_tokens_quant.tensor)?;
            self.weights.embed_tokens_quant.bytes = Arc::new(Vec::new());
            if let Some(ref mut lm) = self.weights.lm_head {
                lm.tensor = backend.to_backend(&lm.tensor)?;
                lm.bytes = Arc::new(Vec::new());
            }
        }
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
            if let Some(ref n) = layer.post_attn_norm {
                layer.post_attn_norm = Some(backend.to_backend(n)?);
            }
            if let Some(ref n) = layer.post_ffw_norm {
                layer.post_ffw_norm = Some(backend.to_backend(n)?);
            }
            if upload_quant {
                layer.q_proj.tensor    = backend.to_backend(&layer.q_proj.tensor)?;
                layer.q_proj.bytes     = Arc::new(Vec::new());
                layer.k_proj.tensor    = backend.to_backend(&layer.k_proj.tensor)?;
                layer.k_proj.bytes     = Arc::new(Vec::new());
                layer.v_proj.tensor    = backend.to_backend(&layer.v_proj.tensor)?;
                layer.v_proj.bytes     = Arc::new(Vec::new());
                layer.o_proj.tensor    = backend.to_backend(&layer.o_proj.tensor)?;
                layer.o_proj.bytes     = Arc::new(Vec::new());
                layer.gate_proj.tensor = backend.to_backend(&layer.gate_proj.tensor)?;
                layer.gate_proj.bytes  = Arc::new(Vec::new());
                layer.up_proj.tensor   = backend.to_backend(&layer.up_proj.tensor)?;
                layer.up_proj.bytes    = Arc::new(Vec::new());
                layer.down_proj.tensor = backend.to_backend(&layer.down_proj.tensor)?;
                layer.down_proj.bytes  = Arc::new(Vec::new());
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
        // One-shot diagnostic: dump stats for specific rows on first call.
        if std::env::var("RUN_DEBUG_EMBED_ROWS").is_ok() && self.past_seq_len == 0 {
            let table = embed_table.try_as_f32()?;
            let rows_to_check: Vec<usize> = std::env::var("RUN_DEBUG_EMBED_ROWS")
                .ok()
                .map(|s| {
                    s.split(',')
                        .filter_map(|x| x.trim().parse::<usize>().ok())
                        .collect()
                })
                .unwrap_or_default();
            for r in &rows_to_check {
                let s = r * hidden_size;
                let row = &table[s..s + hidden_size];
                let m = row.iter().map(|v| v.abs()).fold(0f32, f32::max);
                let rms = (row.iter().map(|v| v * v).sum::<f32>() / hidden_size as f32).sqrt();
                let mean = row.iter().sum::<f32>() / hidden_size as f32;
                eprintln!("embed row {r:>6}: abs_max={m:>8.4} rms={rms:>7.4} mean={mean:>9.5}");
            }
        }
        let row_start = (token_id as usize) * hidden_size;
        let mut embed_row: Vec<f32> = embed_table.try_as_f32()?[row_start..row_start + hidden_size].to_vec();
        // Families that scale embeddings by sqrt(hidden_size) on lookup
        // (Gemma 1/2/3/4). The flag is set once on the family profile.
        if c.family.scaled_embeddings {
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

        let debug_layers = std::env::var("RUN_DEBUG_LAYERS").is_ok();
        if debug_layers {
            let h = hidden.try_as_f32()?;
            let m = h.iter().map(|v| v.abs()).fold(0f32, f32::max);
            let s = (h.iter().map(|v| v * v).sum::<f32>() / h.len() as f32).sqrt();
            eprintln!("post-embed     abs_max={m:.4} rms={s:.4}");
        }

        // Attempt single-batch cross-layer GPU forward (saves ~N × 3 waits).
        // For mixed-geometry models (e.g. Gemma-4 sliding vs global layers),
        // groups consecutive uniform layers and calls fused per group.
        // Falls back to per-layer loop when any group returns None.
        use crate::backend::LayerFusedInput;
        use crate::arch::decoder::config::HiddenActivation;
        let fused_inputs: Vec<LayerFusedInput<'_>> = self.weights.layers.iter().enumerate()
            .map(|(i, l)| LayerFusedInput {
                input_norm:     &l.input_norm,
                q_proj:         &l.q_proj.tensor,
                k_proj:         &l.k_proj.tensor,
                v_proj:         &l.v_proj.tensor,
                q_bias:         l.q_proj_bias.as_ref(),
                k_bias:         l.k_proj_bias.as_ref(),
                v_bias:         l.v_proj_bias.as_ref(),
                q_norm:         l.q_norm.as_ref(),
                k_norm:         l.k_norm.as_ref(),
                o_proj:         &l.o_proj.tensor,
                post_norm:      &l.post_norm,
                gate_proj:      &l.gate_proj.tensor,
                up_proj:        &l.up_proj.tensor,
                down_proj:      &l.down_proj.tensor,
                num_heads:      c.num_attention_heads as u32,
                kv_heads:       c.layer_kv_heads(i) as u32,
                head_dim:       c.layer_head_dim(i) as u32,
                rope_dim:       c.layer_rope_dim(i) as u32,
                rope_theta:     c.layer_rope_theta(i),
                attn_scale:     c.layer_attn_scale(i),
                window:         c.layer_window(i).map(|w| w as u32).unwrap_or(0),
                layer_idx:      i,
                post_attn_norm: l.post_attn_norm.as_ref(),
                post_ffw_norm:  l.post_ffw_norm.as_ref(),
                use_gelu_tanh:  c.hidden_activation == HiddenActivation::GeluTanh,
                layer_output_scale: l.layer_output_scale.as_ref()
                    .and_then(|t| t.try_as_f32().ok().and_then(|s| s.first().copied()))
                    .unwrap_or(1.0_f32),
            })
            .collect();

        let hc_timing = std::env::var("HC_TIMING").is_ok();
        let t_fused = Instant::now();

        // Try grouped fused forward: split into uniform-geometry groups.
        // FUSED_MAX_N=N limits group size for diagnostics (default: unlimited).
        let fused_max_n: usize = std::env::var("FUSED_MAX_N")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
        let fused_debug = std::env::var("FUSED_DEBUG").is_ok();
        let used_fused;
        if !debug_layers && !prof_enabled {
            let mut li = 0;  // next layer index to process
            let mut any_fused = false;
            loop {
                if li >= c.num_hidden_layers { break; }
                let l0 = &fused_inputs[li];
                // Find end of uniform group (same head_dim, kv_heads, window).
                let mut gi = li + 1;
                while gi < c.num_hidden_layers {
                    let lj = &fused_inputs[gi];
                    if lj.head_dim != l0.head_dim || lj.kv_heads != l0.kv_heads
                        || lj.num_heads != l0.num_heads || lj.window != l0.window
                    { break; }
                    if gi - li >= fused_max_n { break; }
                    gi += 1;
                }
                // For sliding layers, cap GPU KV cache at window size.
                let group_max_seq = if l0.window > 0 { l0.window } else { max_seq as u32 };
                match backend.forward_decode_fused_layers(
                    &hidden, &fused_inputs[li..gi],
                    self.past_seq_len, group_max_seq, c.rms_norm_eps,
                )? {
                    Some(h) => {
                        if fused_debug {
                            let vals = backend.download_f32(&h)?;
                            let nan_c = vals.iter().filter(|v| !v.is_finite()).count();
                            eprintln!("[FUSED_DEBUG] group li={li}..{gi} n={} nan={nan_c}/{}", gi-li, vals.len());
                            if nan_c > 0 {
                                let first8: Vec<_> = vals.iter().take(8).collect();
                                eprintln!("  first8: {first8:.4?}");
                            }
                        }
                        hidden = h; li = gi; any_fused = true;
                    }
                    None => {
                        // Group can't run fused (geometry mismatch, etc.) — fall back
                        // per-layer for this group only and continue with the next group.
                        if fused_debug {
                            eprintln!("[FUSED_DEBUG] group li={li}..{gi} n={} → per-layer fallback", gi-li);
                        }
                        for i in li..gi {
                            hidden = forward_layer(
                                &hidden, i, &self.weights.layers[i], c, &pos_tensor, backend,
                                &mut self.kv_cache[i], self.past_seq_len, None,
                            )?;
                        }
                        li = gi;
                    }
                }
            }
            used_fused = any_fused;
        } else {
            used_fused = false;
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
                        Some(crate::arch::decoder::config::LayerKind::Full) => "full",
                        _ => "slid",
                    };
                    eprintln!("layer {i:>3} {kind} abs_max={m:>9.4} rms={s:>8.4}");
                }
            }
        }

        let fused_ms = t_fused.elapsed().as_secs_f64() * 1000.0;
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
        let final_ms = t_final.elapsed().as_secs_f64() * 1000.0;
        if prof_enabled {
            self.prof.final_norm_ms += final_ms;
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
        let logits = qw_matmul(&final_normed, lm_head_qw, backend)?;
        if debug_layers {
            let h = backend.download_f32(&logits)?;
            let m = h.iter().map(|v| v.abs()).fold(0f32, f32::max);
            let s = (h.iter().map(|v| v * v).sum::<f32>() / h.len() as f32).sqrt();
            eprintln!("post-lm-head    abs_max={m:.4} rms={s:.4}");
        }
        let lm_ms = t_lm.elapsed().as_secs_f64() * 1000.0;
        if prof_enabled {
            self.prof.lm_head_ms += lm_ms;
        }

        if prof_enabled {
            self.prof.forwards += 1;
        }
        self.past_seq_len += 1;
        let t_dl = Instant::now();
        let mut logits_vec = backend.download_f32(&logits)?;
        let dl_ms = t_dl.elapsed().as_secs_f64() * 1000.0;
        if hc_timing {
            eprintln!(
                "  fused={fused_ms:>6.2}ms({})  fn={final_ms:>5.2}ms  lm={lm_ms:>6.2}ms  dl={dl_ms:>5.2}ms  tot={:.2}ms",
                if used_fused { "Y" } else { "N" },
                fused_ms + final_ms + lm_ms + dl_ms
            );
        }

        // LlamaStyle+ (Gemma 3/4): final logit softcapping.
        // Spec: specs/arch.md §"Final logit softcapping"
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

        if std::env::var("RUN_DEBUG_LOGITS").is_ok() {
            let mut idx: Vec<usize> = (0..logits_vec.len()).collect();
            idx.sort_unstable_by(|&a, &b| {
                logits_vec[b].partial_cmp(&logits_vec[a]).unwrap_or(std::cmp::Ordering::Equal)
            });
            eprintln!("top-10 logits:");
            for &i in idx.iter().take(10) {
                eprintln!("  id={i:>6}  logit={:>8.3}", logits_vec[i]);
            }
        }

        Ok(logits_vec)
    }
}

fn dbg_stats_l(layer: usize, label: &str, v: &[f32]) {
    let m = v.iter().map(|x| x.abs()).fold(0f32, f32::max);
    let rms = (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
    eprintln!("  L{layer} {:20} abs_max={:>12.4} rms={:>10.4} len={}", label, m, rms, v.len());
}

fn dbg_stats(label: &str, v: &[f32]) { dbg_stats_l(0, label, v); }

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
    let debug_layer = std::env::var("RUN_DEBUG_LAYER_IDX")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let debug_l0 = std::env::var("RUN_DEBUG_LAYERS").is_ok() && layer_idx == debug_layer && past_seq_len == 0;
    let eps = config.rms_norm_eps;
    if debug_l0 {
        dbg_stats_l(layer_idx, "hidden_in (embed)", &backend.download_f32(hidden)?);
    }
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

    // 1+2(+optionally qk_norm). For qwen3 (qk_norm + no bias), fuse the WHOLE
    // chain (input_norm + qkv + qk_norm) into ONE command buffer.
    let t = Instant::now();
    let no_bias = layer.q_proj_bias.is_none()
        && layer.k_proj_bias.is_none()
        && layer.v_proj_bias.is_none();
    let (mut q, mut k, mut v, qk_norm_done) = if no_bias {
        if let (Some(qn), Some(kn)) = (&layer.q_norm, &layer.k_norm) {
            let (qq, kk, vv) = backend.fused_norm_qkv_qknorm(
                hidden,
                &layer.input_norm,
                &layer.q_proj.tensor,
                &layer.k_proj.tensor,
                &layer.v_proj.tensor,
                qn, kn,
                eps,
                num_heads, kv_heads, head_dim,
            )?;
            (qq, kk, vv, true)
        } else {
            let qkv_outs = backend.fused_norm_quant_matmul_multi(
                hidden, &layer.input_norm, eps,
                &[&layer.q_proj.tensor, &layer.k_proj.tensor, &layer.v_proj.tensor],
            )?;
            let mut it = qkv_outs.into_iter();
            (it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), false)
        }
    } else {
        let qkv_outs = backend.fused_norm_quant_matmul_multi(
            hidden, &layer.input_norm, eps,
            &[&layer.q_proj.tensor, &layer.k_proj.tensor, &layer.v_proj.tensor],
        )?;
        let mut it = qkv_outs.into_iter();
        (it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), false)
    };
    acc_input_norm += t.elapsed().as_secs_f64() * 1000.0;

    if debug_l0 || (std::env::var("RUN_DEBUG_LAYERS").is_ok() && layer_idx == 1 && past_seq_len == 0) {
        dbg_stats_l(layer_idx, "q (post-qkv)", &backend.download_f32(&q)?);
        dbg_stats_l(layer_idx, "k (post-qkv)", &backend.download_f32(&k)?);
        dbg_stats_l(layer_idx, "v (post-qkv)", &backend.download_f32(&v)?);
    }

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

    // QK-norm (Qwen3) — per-head RmsNorm. Batched: ONE command buffer for both.
    // Skipped if already done by fused_norm_qkv_qknorm above.
    let t = Instant::now();
    if !qk_norm_done {
        if let (Some(qn), Some(kn)) = (&layer.q_norm, &layer.k_norm) {
            let q_reshaped = Tensor::from_f32(vec![num_heads, head_dim], q.to_f32_vec());
            let k_reshaped = Tensor::from_f32(vec![kv_heads, head_dim], k.to_f32_vec());
            let normed = backend.rms_norm_multi(&[(&q_reshaped, qn), (&k_reshaped, kn)], eps)?;
            let mut it = normed.into_iter();
            let q_n = it.next().unwrap();
            let k_n = it.next().unwrap();
            q = Tensor::from_f32(vec![1, num_heads * head_dim], q_n.to_f32_vec());
            k = Tensor::from_f32(vec![1, kv_heads * head_dim], k_n.to_f32_vec());
        }
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

    if debug_l0 {
        dbg_stats_l(layer_idx, "q_roped", &backend.download_f32(&q_roped)?);
        dbg_stats_l(layer_idx, "k_roped", &backend.download_f32(&k_roped)?);
    }

    // 4-5. Attention. Use GPU path when backend supports it (KV cache stays
    // GPU-resident, all of (kv_append + score + softmax + output) lives in
    // ONE Metal command buffer). Falls back to per-token CPU pipeline.
    let max_seq = config.max_position_embeddings.min(8192);
    let cache_seq = config.layer_kv_cache_seq(layer_idx, max_seq);
    let scale = config.layer_attn_scale(layer_idx);
    let window: u32 = sliding_window.map(|w| w as u32).unwrap_or(0);
    let total_seq = past_seq_len + 1;
    let _ = (total_seq, max_seq);

    // GPU attention path is correct but currently slower in wall clock due to
    // 1 extra batch wait per layer. It's the foundation for cross-layer batching
    // (next step). Off by default — enable with MR_GPU_ATTN=1.
    let use_gpu_attn = backend.supports_gpu_attention()
        && !config.family.v_norm_per_head
        && layer.post_attn_norm.is_none()
        && std::env::var("MR_GPU_ATTN").is_ok();
    // Fused attention path returns hidden1 (post-residual) directly,
    // bypassing the separate o_proj/residual blocks below.
    let mut hidden1_gpu: Option<Tensor> = None;
    let attn_tensor = if use_gpu_attn {
        let t = Instant::now();
        let q_for_attn = Tensor { shape: vec![num_heads, head_dim], dtype: q_roped.dtype, data: q_roped.data.clone() };
        let k_for_attn = Tensor { shape: vec![kv_heads,  head_dim], dtype: k_roped.dtype, data: k_roped.data.clone() };
        let v_for_attn = Tensor { shape: vec![kv_heads,  head_dim], dtype: v.dtype,       data: v.data.clone() };
        let h1 = backend.fused_attn_oproj_residual(
            &q_for_attn, &k_for_attn, &v_for_attn,
            hidden, &layer.o_proj.tensor,
            layer_idx, past_seq_len,
            num_heads as u32, kv_heads as u32, head_dim as u32, cache_seq as u32,
            scale, window,
        )?;
        acc_kv_append += t.elapsed().as_secs_f64() * 1000.0;
        acc_attention += 0.0;
        hidden1_gpu = Some(h1);
        // attn_tensor placeholder — unused on GPU path
        Tensor::from_f32(vec![1, num_heads * head_dim], vec![0.0; num_heads * head_dim])
    } else {
        // CPU path (legacy)
        let t = Instant::now();
        let v_flat = if config.family.v_norm_per_head {
            let mut v_data = v.to_f32_vec();
            let inv_d = 1.0 / head_dim as f32;
            for h in 0..kv_heads {
                let off = h * head_dim;
                let mut sumsq = 0f32;
                for j in 0..head_dim {
                    let val = v_data[off + j];
                    sumsq += val * val;
                }
                let rms = (sumsq * inv_d + eps).sqrt();
                let scale_v = 1.0 / rms;
                for j in 0..head_dim {
                    v_data[off + j] *= scale_v;
                }
            }
            v_data
        } else {
            v.to_f32_vec()
        };
        let k_flat = k_roped.to_f32_vec();
        let kv_slot = past_seq_len % cache_seq;
        for h in 0..kv_heads {
            let src_base = h * head_dim;
            let dst_base = h * cache_seq * head_dim + kv_slot * head_dim;
            for d in 0..head_dim {
                kv.0[dst_base + d] = k_flat[src_base + d];
                kv.1[dst_base + d] = v_flat[src_base + d];
            }
        }
        acc_kv_append += t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        let repeat = num_heads / kv_heads;
        // Read at most cache_seq tokens (ring buffer wraps for long contexts).
        let read_seq = total_seq.min(cache_seq);
        let read_start = if total_seq > cache_seq { total_seq - cache_seq } else { 0 };
        let mut k_full = vec![0f32; num_heads * read_seq * head_dim];
        let mut v_full = vec![0f32; num_heads * read_seq * head_dim];
        for h in 0..num_heads {
            let kv_h = h / repeat;
            for si in 0..read_seq {
                let s = read_start + si;
                let slot = s % cache_seq;
                for d in 0..head_dim {
                    let src = kv_h * cache_seq * head_dim + slot * head_dim + d;
                    let dst = h * read_seq * head_dim + si * head_dim + d;
                    k_full[dst] = kv.0[src];
                    v_full[dst] = kv.1[src];
                }
            }
        }
        let q_heads = q_roped.to_f32_vec();
        let mut attn_out = vec![0f32; num_heads * head_dim];
        for h in 0..num_heads {
            let mut scores = vec![0f32; read_seq];
            let q_off = h * head_dim;
            let kv_off = h * read_seq * head_dim;
            for s in 0..read_seq {
                let mut acc = 0f32;
                for d in 0..head_dim {
                    acc += q_heads[q_off + d] * k_full[kv_off + s * head_dim + d];
                }
                scores[s] = acc * scale;
            }
            let max_s = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let mut sum = 0f32;
            for s in scores.iter_mut() { *s = (*s - max_s).exp(); sum += *s; }
            for s in scores.iter_mut() { *s /= sum; }
            let out_off = h * head_dim;
            for s in 0..read_seq {
                let v_row = &v_full[kv_off + s * head_dim..kv_off + (s + 1) * head_dim];
                for d in 0..head_dim {
                    attn_out[out_off + d] += scores[s] * v_row[d];
                }
            }
        }
        acc_attention += t.elapsed().as_secs_f64() * 1000.0;
        Tensor::from_f32(vec![1, num_heads * head_dim], attn_out)
    };

    let debug_l1 = std::env::var("RUN_DEBUG_LAYERS").is_ok() && layer_idx == 1 && past_seq_len == 0;
    if debug_l0 || debug_l1 {
        dbg_stats_l(layer_idx, "attn_out", &backend.download_f32(&attn_tensor)?);
    }

    // 6+7. Output projection + residual. Skipped if hidden1_gpu was produced
    // by the fused attention path above.
    let hidden1 = if let Some(h1) = hidden1_gpu {
        h1
    } else {
        let t = Instant::now();
        let mut attn_proj = qw_matmul(&attn_tensor, &layer.o_proj, backend)?;
        if debug_l0 || debug_l1 {
            dbg_stats_l(layer_idx, "o_proj_out", &backend.download_f32(&attn_proj)?);
        }
        if let Some(ref n) = layer.post_attn_norm {
            attn_proj = backend
                .execute(&Op::RmsNorm { eps }, &[&attn_proj, n])?
                .remove(0);
        }
        acc_o_proj += t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        let h1 = backend.execute(&Op::Add, &[hidden, &attn_proj])?.remove(0);
        if debug_l0 || debug_l1 {
            dbg_stats_l(layer_idx, "hidden1 (attn+res)", &backend.download_f32(&h1)?);
        }
        acc_residual += t.elapsed().as_secs_f64() * 1000.0;
        h1
    };

    // 8+9. FFN. Try fully fused FFN (norm + gate + up + silu*up + down + residual)
    // for SiLU; falls back to per-op for other activations.
    use crate::arch::decoder::config::HiddenActivation;
    let mut out_gpu: Option<Tensor> = None;
    let mut ffn_out = match config.hidden_activation {
        HiddenActivation::Silu if layer.post_ffw_norm.is_none() && layer.layer_output_scale.is_none() && !debug_l0 => {
            if std::env::var("RUN_DEBUG_LAYERS").is_ok() && past_seq_len == 0 && layer_idx <= 2 {
                dbg_stats_l(layer_idx, "hidden1 (fused input)", &backend.download_f32(&hidden1)?);
            }
            let t = Instant::now();
            let out = backend.fused_ffn_residual(
                &hidden1,
                &layer.post_norm,
                &layer.gate_proj.tensor,
                &layer.up_proj.tensor,
                &layer.down_proj.tensor,
                eps,
            )?;
            acc_post_norm += t.elapsed().as_secs_f64() * 1000.0;
            out_gpu = Some(out);
            if std::env::var("RUN_DEBUG_LAYERS").is_ok() && past_seq_len == 0 && layer_idx <= 2 {
                let fused = out_gpu.as_ref().unwrap();
                dbg_stats_l(layer_idx, "ffn+res (fused)", &backend.download_f32(fused)?);
            }
            // Placeholder ffn_out — unused on GPU path
            Tensor::from_f32(hidden1.shape.clone(), vec![0.0; hidden_size])
        }
        _ => {
            let t = Instant::now();
            let gate_up = backend.fused_norm_quant_matmul_multi(
                &hidden1,
                &layer.post_norm,
                eps,
                &[&layer.gate_proj.tensor, &layer.up_proj.tensor],
            )?;
            acc_post_norm += t.elapsed().as_secs_f64() * 1000.0;

            let t = Instant::now();
            let mut gate_up_iter = gate_up.into_iter();
            let gate = gate_up_iter.next().unwrap();
            let up = gate_up_iter.next().unwrap();
            if debug_l0 {
                dbg_stats_l(layer_idx, "gate (post_norm+proj)", &backend.download_f32(&gate)?);
                dbg_stats_l(layer_idx, "up (post_norm+proj)", &backend.download_f32(&up)?);
            }
            let act: fn(f32) -> f32 = match config.hidden_activation {
                HiddenActivation::Silu => |x| x / (1.0 + (-x).exp()),
                HiddenActivation::GeluTanh => |x| {
                    let c = (2.0_f32 / std::f32::consts::PI).sqrt();
                    0.5 * x * (1.0 + (c * (x + 0.044715 * x * x * x)).tanh())
                },
                HiddenActivation::GeluErf => |x| {
                    let inv_sqrt2 = 1.0_f32 / std::f32::consts::SQRT_2;
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
            let mut mid = gate.to_f32_vec();
            let up_vec = up.to_f32_vec();
            for (m, u) in mid.iter_mut().zip(up_vec.iter()) { *m = act(*m) * *u; }
            let mid_t = Tensor::from_f32(gate.shape.clone(), mid);
            if debug_l0 {
                dbg_stats_l(layer_idx, "silu(gate)*up (mid)", &backend.download_f32(&mid_t)?);
            }
            let _ = t;
            let down_out = qw_matmul(&mid_t, &layer.down_proj, backend)?;
            if debug_l0 {
                dbg_stats_l(layer_idx, "down_proj_out", &backend.download_f32(&down_out)?);
            }
            down_out
        }
    };
    // Gemma 2/3/4: norm applied to FFN output before residual.
    if let Some(ref n) = layer.post_ffw_norm {
        ffn_out = backend
            .execute(&Op::RmsNorm { eps }, &[&ffn_out, n])?
            .remove(0);
    }
    acc_ffn += t.elapsed().as_secs_f64() * 1000.0;

    if debug_l0 {
        if let Some(ref o) = out_gpu {
            dbg_stats_l(layer_idx, "ffn+res (fused)", &backend.download_f32(o)?);
        } else {
            dbg_stats_l(layer_idx, "ffn_out", &backend.download_f32(&ffn_out)?);
        }
    }

    // 9. Residual. Gemma-4 multiplies the final layer output by a SCALAR
    // `layer_scalar` (HF: `hidden_states *= self.layer_scalar`). The tensor
    // is shape [1] in our .model.
    let _ = hidden_size;
    let t = Instant::now();
    let mut out = if let Some(o) = out_gpu {
        // Fused FFN already added the residual on GPU.
        o
    } else {
        backend.execute(&Op::Add, &[&hidden1, &ffn_out])?.remove(0)
    };
    // Gemma-4 layer_scalar (HF: hidden_states *= self.layer_scalar). The
    // register_buffer is initialised to 1.0 but checkpoint values can differ
    // (per-layer trained scalar). Without this scale, activations explode
    // through the residual stream.
    if let Some(ref s) = layer.layer_output_scale {
        let scalar = s.as_f32()[0];
        if std::env::var("RUN_DEBUG_LAYERS").is_ok() {
            eprintln!("  layer {layer_idx:>3} layer_output_scale = {scalar:.6}");
        }
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
