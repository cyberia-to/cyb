//! Tensor name normalization. Source-format names → canonical HuggingFace
//! naming as defined in `specs/import.md`.
//!
//! Also: canonical-encoding policy — which of the five canonical on-disk
//! encodings to use for each canonical tensor name.

/// Choose the canonical encoding for a tensor by its canonical (HF) name.
///
/// Per `cyb/cyb-model`:
///   `u32` (16.16 fixed-point) — norms, biases, full-precision scalars
///   `q8`  (8-bit block)       — every matmul-shaped weight by default
///
/// `q4` is also part of the canonical set but produces garbage output on
/// medium-sized models (verified: qwen2.5-coder-14b at hidden=5120 with
/// matmul→q4 gave coherent loading but token-level gibberish). q8 holds
/// up empirically at moderate cost (~2× storage of q4). q4 is reserved
/// for explicit smaller-storage targets, not the default.
///
/// Caller is responsible for tensor-shape constraints (q8 requires the
/// element count to be a multiple of 32).
pub fn canonical_encoding_for(canonical_name: &str) -> &'static str {
    if canonical_name.contains("norm") {
        return "u32";
    }
    if canonical_name.ends_with(".bias") {
        return "u32";
    }
    // Embeddings, lm_head, attention projections, MLP projections.
    // EXPERIMENT: q4 for half storage + bandwidth (faster GPU compute).
    "q4"
}

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
