//! Small helpers used by the importer CLI:
//! - [`gguf_to_hf`]: translate GGUF tensor names to HuggingFace conventions.
//! - [`quantize_f32_to_q4k`]: bulk re-quantizer used when the source is
//!   Q4_0/Q4_1 (legacy) and we want Q4_K in the output.

/// Map a GGUF tensor name to its HuggingFace canonical equivalent.
///
/// Examples:
///   `token_embd.weight`            → `model.embed_tokens.weight`
///   `output_norm.weight`           → `model.norm.weight`
///   `output.weight`                → `lm_head.weight`
///   `blk.5.attn_q.weight`          → `model.layers.5.self_attn.q_proj.weight`
///   `blk.12.ffn_gate.weight`       → `model.layers.12.mlp.gate_proj.weight`
pub fn gguf_to_hf(name: &str) -> String {
    if name == "token_embd.weight" {
        return "model.embed_tokens.weight".into();
    }
    if name == "output_norm.weight" {
        return "model.norm.weight".into();
    }
    if name == "output.weight" {
        return "lm_head.weight".into();
    }

    if let Some(rest) = name.strip_prefix("blk.") {
        if let Some(dot) = rest.find('.') {
            let layer_num = &rest[..dot];
            let suffix = &rest[dot + 1..];
            let mapped = match suffix {
                "attn_norm.weight" => "input_layernorm.weight",
                "attn_q.weight" => "self_attn.q_proj.weight",
                "attn_k.weight" => "self_attn.k_proj.weight",
                "attn_v.weight" => "self_attn.v_proj.weight",
                "attn_output.weight" => "self_attn.o_proj.weight",
                "attn_q.bias" => "self_attn.q_proj.bias",
                "attn_k.bias" => "self_attn.k_proj.bias",
                "attn_v.bias" => "self_attn.v_proj.bias",
                "attn_q_norm.weight" => "self_attn.q_norm.weight",
                "attn_k_norm.weight" => "self_attn.k_norm.weight",
                "ffn_norm.weight" => "post_attention_layernorm.weight",
                "ffn_gate.weight" => "mlp.gate_proj.weight",
                "ffn_up.weight" => "mlp.up_proj.weight",
                "ffn_down.weight" => "mlp.down_proj.weight",
                other => other,
            };
            return format!("model.layers.{layer_num}.{mapped}");
        }
    }
    name.to_string()
}

/// Quantize `weights` (an `[N, K]` matrix in row-major f32) into Q4_K.
///
/// Q4_K layout per 256-value superblock (144 bytes):
///   f16 d + f16 dmin + 12 bytes packed scales/mins + 128 bytes of nibbles.
/// See `reference/runtime/quant.md` for the full format.
pub fn quantize_f32_to_q4k(weights: &[f32], n: usize, k: usize) -> Vec<u8> {
    let super_blocks = k / 256;
    let mut out = vec![0u8; n * super_blocks * 144];
    for row in 0..n {
        for sb in 0..super_blocks {
            let base = row * k + sb * 256;
            let dst = (row * super_blocks + sb) * 144;
            let block_vals = &weights[base..base + 256];
            let mut scales_arr = [0u8; 12];
            let mut qs = [0u8; 128];
            let mut sub_scales = [0.0f32; 8];
            let mut sub_mins = [0.0f32; 8];
            for j in 0..8 {
                let sub = &block_vals[j * 32..(j + 1) * 32];
                let max_val = sub.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let min_val = sub.iter().cloned().fold(f32::INFINITY, f32::min);
                let eff_min = min_val.min(0.0);
                sub_mins[j] = -eff_min;
                sub_scales[j] = if max_val > eff_min {
                    (max_val - eff_min) / 15.0
                } else {
                    0.0
                };
            }
            let max_scale = sub_scales.iter().cloned().fold(0.0f32, f32::max);
            let max_min = sub_mins.iter().cloned().fold(0.0f32, f32::max);
            let d = max_scale / 63.0;
            let dmin = max_min / 63.0;
            let inv_d = if max_scale > 0.0 { 1.0 / d } else { 0.0 };
            let inv_dmin = if max_min > 0.0 { 1.0 / dmin } else { 0.0 };
            out[dst..dst + 2].copy_from_slice(&half::f16::from_f32(d).to_le_bytes());
            out[dst + 2..dst + 4].copy_from_slice(&half::f16::from_f32(dmin).to_le_bytes());
            for j in 0..8 {
                let sc = (sub_scales[j] * inv_d).round().min(63.0).max(0.0) as u8;
                let m = (sub_mins[j] * inv_dmin).round().min(63.0).max(0.0) as u8;
                if j < 4 {
                    scales_arr[j] = (scales_arr[j] & 0xC0) | (sc & 63);
                    scales_arr[j + 4] = (scales_arr[j + 4] & 0xC0) | (m & 63);
                } else {
                    scales_arr[j + 4] = (scales_arr[j + 4] & 0xF0) | (sc & 0xF);
                    scales_arr[j - 4] = (scales_arr[j - 4] & 0x3F) | ((sc >> 4) << 6);
                    scales_arr[j + 4] = (scales_arr[j + 4] & 0x0F) | ((m & 0xF) << 4);
                    scales_arr[j] = (scales_arr[j] & 0x3F) | ((m >> 4) << 6);
                }
            }
            out[dst + 4..dst + 16].copy_from_slice(&scales_arr);
            for grp in 0..4 {
                let inv_sc1 = if sub_scales[grp * 2] > 0.0 {
                    1.0 / sub_scales[grp * 2]
                } else {
                    0.0
                };
                let inv_sc2 = if sub_scales[grp * 2 + 1] > 0.0 {
                    1.0 / sub_scales[grp * 2 + 1]
                } else {
                    0.0
                };
                for l in 0..32 {
                    let q1 = ((block_vals[grp * 64 + l] + sub_mins[grp * 2]) * inv_sc1)
                        .round()
                        .min(15.0)
                        .max(0.0) as u8;
                    let q2 = ((block_vals[grp * 64 + 32 + l] + sub_mins[grp * 2 + 1]) * inv_sc2)
                        .round()
                        .min(15.0)
                        .max(0.0) as u8;
                    qs[grp * 32 + l] = (q1 & 0xF) | ((q2 & 0xF) << 4);
                }
            }
            out[dst + 16..dst + 144].copy_from_slice(&qs);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gguf_to_hf_core_tensors() {
        assert_eq!(gguf_to_hf("token_embd.weight"), "model.embed_tokens.weight");
        assert_eq!(gguf_to_hf("output_norm.weight"), "model.norm.weight");
        assert_eq!(gguf_to_hf("output.weight"), "lm_head.weight");
    }

    #[test]
    fn gguf_to_hf_layer_tensors() {
        assert_eq!(
            gguf_to_hf("blk.5.attn_q.weight"),
            "model.layers.5.self_attn.q_proj.weight"
        );
        assert_eq!(
            gguf_to_hf("blk.12.ffn_gate.weight"),
            "model.layers.12.mlp.gate_proj.weight"
        );
        assert_eq!(
            gguf_to_hf("blk.0.attn_q_norm.weight"),
            "model.layers.0.self_attn.q_norm.weight"
        );
    }

    #[test]
    fn gguf_to_hf_passes_through_unknown() {
        assert_eq!(gguf_to_hf("some.other.weight"), "some.other.weight");
    }
}
