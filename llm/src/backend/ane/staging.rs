//! IOSurface weight and activation staging helpers for ANE kernels.
//!
//! These functions write transposed weight matrices and activation data
//! into IOSurface buffers in the fp16 layout expected by ANE programs.

use crate::config::ModelConfig;
use rane::{cvt_f16_f32, f32_to_fp16, AneSurface};

// ── Weight staging ──

/// Stage transposed Q, K, V weight matrices into a QKV projection surface.
pub fn stage_qkv_weights(
    surface: &AneSurface,
    wqt: &[f32],
    wkt: &[f32],
    wvt: &[f32],
    cfg: &ModelConfig,
) {
    let sp = cfg.qkv_proj_sp();
    let seq = cfg.seq;
    surface.with_data_mut(|buf| {
        for d in 0..cfg.dim {
            for i in 0..cfg.q_dim {
                buf[d * sp + seq + i] = f32_to_fp16(wqt[d * cfg.q_dim + i]);
            }
            for i in 0..cfg.kv_dim {
                buf[d * sp + seq + cfg.q_dim + i] = f32_to_fp16(wkt[d * cfg.kv_dim + i]);
            }
            for i in 0..cfg.kv_dim {
                buf[d * sp + seq + cfg.q_dim + cfg.kv_dim + i] =
                    f32_to_fp16(wvt[d * cfg.kv_dim + i]);
            }
        }
    });
}

/// Stage transposed Wo weight matrix into a Wo projection surface.
pub fn stage_wo_weights(surface: &AneSurface, wot: &[f32], cfg: &ModelConfig) {
    let sp = cfg.wo_fwd_sp();
    surface.with_data_mut(|buf| {
        for d in 0..cfg.q_dim {
            for i in 0..cfg.dim {
                buf[d * sp + cfg.seq + i] = f32_to_fp16(wot[d * cfg.dim + i]);
            }
        }
    });
}

/// Stage transposed W1, W3, and W2 weight matrices into an FFN surface.
pub fn stage_ffn_weights(
    surface: &AneSurface,
    w1t: &[f32],
    w3t: &[f32],
    w2: &[f32],
    cfg: &ModelConfig,
) {
    let sp = cfg.ffn_fused_sp();
    let (seq, hidden, dim) = (cfg.seq, cfg.hidden, cfg.dim);
    surface.with_data_mut(|buf| {
        for d in 0..dim {
            for i in 0..hidden {
                buf[d * sp + 2 * seq + i] = f32_to_fp16(w1t[d * hidden + i]);
            }
            for i in 0..hidden {
                buf[d * sp + 2 * seq + hidden + i] = f32_to_fp16(w3t[d * hidden + i]);
            }
            for i in 0..hidden {
                buf[d * sp + 2 * seq + 2 * hidden + i] = f32_to_fp16(w2[d * hidden + i]);
            }
        }
    });
}

// ── Activation staging ──

/// Write normalized activations into a QKV projection input surface.
pub fn write_qkv_acts(surface: &AneSurface, xnorm: &[f32], cfg: &ModelConfig) {
    let sp = cfg.qkv_proj_sp();
    surface.with_data_mut(|buf| {
        for d in 0..cfg.dim {
            for s in 0..cfg.seq {
                buf[d * sp + s] = f32_to_fp16(xnorm[d * cfg.seq + s]);
            }
        }
    });
}

/// Write attention output into a Wo projection input surface.
pub fn write_wo_acts(surface: &AneSurface, attn_out: &[f32], cfg: &ModelConfig) {
    let sp = cfg.wo_fwd_sp();
    surface.with_data_mut(|buf| {
        for d in 0..cfg.q_dim {
            for s in 0..cfg.seq {
                buf[d * sp + s] = f32_to_fp16(attn_out[d * cfg.seq + s]);
            }
        }
    });
}

/// Write post-norm and residual activations into an FFN input surface.
pub fn write_ffn_acts(
    surface: &AneSurface,
    x2norm: &[f32],
    x2: &[f32],
    cfg: &ModelConfig,
) {
    let sp = cfg.ffn_fused_sp();
    let seq = cfg.seq;
    surface.with_data_mut(|buf| {
        for d in 0..cfg.dim {
            for s in 0..seq {
                buf[d * sp + s] = f32_to_fp16(x2norm[d * seq + s]);
            }
            for s in 0..seq {
                buf[d * sp + seq + s] = f32_to_fp16(x2[d * seq + s]);
            }
        }
    });
}

// ── Output reading ──

/// Read Q, K, V outputs from a QKV projection output surface.
pub fn read_qkv_output(
    surface: &AneSurface,
    q: &mut [f32],
    k: &mut [f32],
    v: &mut [f32],
    cfg: &ModelConfig,
) {
    let seq = cfg.seq;
    let q_dim = cfg.q_dim;
    let kv_dim = cfg.kv_dim;
    surface.with_data(|buf| {
        cvt_f16_f32(q, &buf[..q_dim * seq]);
        cvt_f16_f32(k, &buf[q_dim * seq..(q_dim + kv_dim) * seq]);
        cvt_f16_f32(v, &buf[(q_dim + kv_dim) * seq..(q_dim + 2 * kv_dim) * seq]);
    });
}

/// Read f32 values from a surface (converting from fp16).
pub fn read_surface_f32(surface: &AneSurface, out: &mut [f32], n: usize) {
    surface.with_data(|buf| {
        cvt_f16_f32(out, &buf[..n]);
    });
}
