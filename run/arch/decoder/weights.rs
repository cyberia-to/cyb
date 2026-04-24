//! LlamaStyle weight loading.
//!
//! Matmul weights are kept QUANTIZED on host (not dequantized to f32).
//! This reduces memory 6-8× and enables fused dequant+matmul kernels
//! that read quant bytes directly.
//!
//! Non-matmul weights (norms, position caches, embed-for-lookup) are
//! dequantized at load.

use crate::backend::cpu::quant::try_dequantize_to_f32;
use crate::core::dtype::DType;
use crate::format::{FormatError, LoadedModel};
use crate::arch::decoder::config::LlamaConfig;
use crate::core::tensor::Tensor;

/// A weight that may be stored quantized (kept as raw bytes) or
/// dequantized (f32 tensor).
#[derive(Clone)]
pub struct QuantWeight {
    pub shape: Vec<usize>,
    pub dtype: DType,
    /// Raw bytes. For F32/F16 this is the natural layout; for quantized
    /// it's the packed block format described in reference/runtime/quant.md.
    pub bytes: std::sync::Arc<Vec<u8>>,
}

impl QuantWeight {
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }
    pub fn n(&self) -> usize {
        self.shape[0]
    }
    pub fn k(&self) -> usize {
        self.shape[1]
    }
}

pub struct LayerWeights {
    pub input_norm: Tensor,
    pub q_proj: QuantWeight,
    pub k_proj: QuantWeight,
    pub v_proj: QuantWeight,
    pub o_proj: QuantWeight,
    pub q_proj_bias: Option<Tensor>,
    pub k_proj_bias: Option<Tensor>,
    pub v_proj_bias: Option<Tensor>,
    pub q_norm: Option<Tensor>,
    pub k_norm: Option<Tensor>,
    /// Pre-FFN norm. (HF "post_attention_layernorm" / GGUF "ffn_norm".)
    pub post_norm: Tensor,
    pub gate_proj: QuantWeight,
    pub up_proj: QuantWeight,
    pub down_proj: QuantWeight,
    /// Gemma 2/3/4: norm applied to attention output before residual.
    pub post_attn_norm: Option<Tensor>,
    /// Gemma 2/3/4: norm applied to FFN output before residual.
    pub post_ffw_norm: Option<Tensor>,
    /// Gemma-4: per-channel scale applied to the residual layer output.
    pub layer_output_scale: Option<Tensor>,
}

pub struct Weights {
    /// Embed is dequantized to f32 for single-row lookup during forward.
    pub embed_tokens: Tensor,
    pub layers: Vec<LayerWeights>,
    pub final_norm: Tensor,
    /// LM head stays quantized. None = tied to embed_tokens (dequanted).
    pub lm_head: Option<QuantWeight>,
    /// Mirror of embed_tokens as quantized bytes, used for tied-weight matmul.
    /// Avoids re-dequanting on every lm_head call.
    pub embed_tokens_quant: QuantWeight,
}

impl Weights {
    /// Load all weights using per-layer attention dims from `config`.
    /// LlamaStyle has uniform dims; LlamaStyle+ (Gemma-4) varies by layer.
    ///
    /// Norm-offset families (Gemma 2 / 3) store RMSNorm weights as `w - 1`
    /// because the math is `(1 + w) * x / rms`. We add 1.0 at load so the
    /// runtime stays on one `Op::RmsNorm` codepath. The flag lives on the
    /// family profile.
    pub fn load(lm: &LoadedModel, config: &LlamaConfig) -> Result<Self, FormatError> {
        let hidden_size = config.hidden_size;
        let vocab_size = config.vocab_size;
        let intermediate_size = config.intermediate_size;
        let norm_offset = config.family.rmsnorm_plus_one;

        // Embed: some imports have [vocab, hidden] (HF-style), others
        // [hidden, vocab] (GGUF-native metadata). Physical byte layout is
        // always [vocab × hidden] values row-major, so we just force the
        // shape to [vocab, hidden] regardless of what the metadata says.
        let embed_tokens = load_tensor_f32_reshaped(
            lm,
            "model.embed_tokens.weight",
            vec![vocab_size, hidden_size],
        )?;
        let embed_tokens_quant = load_quant_weight_reshaped(
            lm,
            "model.embed_tokens.weight",
            vec![vocab_size, hidden_size],
        )?;

        let mut final_norm = load_tensor_f32(lm, "model.norm.weight")?;
        if norm_offset {
            offset_norm_by_one(&mut final_norm);
        }

        let lm_head = if config.tie_word_embeddings {
            None
        } else {
            Some(load_quant_weight_reshaped(
                lm,
                "lm_head.weight",
                vec![vocab_size, hidden_size],
            )?)
        };

        let mut layers = Vec::with_capacity(config.num_hidden_layers);
        for i in 0..config.num_hidden_layers {
            let q_dim = config.num_attention_heads * config.layer_head_dim(i);
            let kv_dim = config.layer_kv_heads(i) * config.layer_head_dim(i);
            let mut layer = load_layer(
                lm,
                i,
                hidden_size,
                q_dim,
                kv_dim,
                intermediate_size,
            )?;
            if norm_offset {
                offset_norm_by_one(&mut layer.input_norm);
                offset_norm_by_one(&mut layer.post_norm);
                if let Some(ref mut n) = layer.q_norm {
                    offset_norm_by_one(n);
                }
                if let Some(ref mut n) = layer.k_norm {
                    offset_norm_by_one(n);
                }
                if let Some(ref mut n) = layer.post_attn_norm {
                    offset_norm_by_one(n);
                }
                if let Some(ref mut n) = layer.post_ffw_norm {
                    offset_norm_by_one(n);
                }
            }
            layers.push(layer);
        }

        Ok(Self {
            embed_tokens,
            embed_tokens_quant,
            layers,
            final_norm,
            lm_head,
        })
    }
}

/// Add 1.0 to every element of a norm weight tensor in place.
/// Gemma family RMSNorm applies `(1 + w) * x / rms`; storing weights with
/// the +1 baked in lets the runtime use a single `Op::RmsNorm` codepath.
fn offset_norm_by_one(t: &mut Tensor) {
    let mut data = t.to_f32_vec();
    for v in data.iter_mut() {
        *v += 1.0;
    }
    *t = Tensor::from_f32(t.shape.clone(), data);
}

fn load_tensor_f32(lm: &LoadedModel, name: &str) -> Result<Tensor, FormatError> {
    let meta = lm
        .tensors
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| FormatError::Invalid(format!("missing tensor {name}")))?;
    let bytes = lm
        .tensor_bytes(name)
        .ok_or_else(|| FormatError::Invalid(format!("bytes missing for {name}")))?;
    let f32s = try_dequantize_to_f32(bytes, meta.dtype)
        .map_err(|e| FormatError::Invalid(format!("dequant {name}: {e}")))?;
    Tensor::try_from_f32(meta.shape.clone(), f32s)
        .map_err(|e| FormatError::Invalid(format!("tensor {name}: {e}")))
}

fn load_quant_weight(lm: &LoadedModel, name: &str) -> Result<QuantWeight, FormatError> {
    let meta = lm
        .tensors
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| FormatError::Invalid(format!("missing tensor {name}")))?;
    let bytes = lm
        .tensor_bytes(name)
        .ok_or_else(|| FormatError::Invalid(format!("bytes missing for {name}")))?;
    Ok(QuantWeight {
        shape: meta.shape.clone(),
        dtype: meta.dtype,
        bytes: std::sync::Arc::new(bytes.to_vec()),
    })
}

/// Load a tensor and override its shape to the expected canonical form.
/// Used for embed/lm_head where import may have reported GGUF-native [K, N].
/// Panics if element count would mismatch.
fn load_tensor_f32_reshaped(
    lm: &LoadedModel,
    name: &str,
    shape: Vec<usize>,
) -> Result<Tensor, FormatError> {
    let t = load_tensor_f32(lm, name)?;
    let declared: usize = t.shape.iter().product();
    let expected: usize = shape.iter().product();
    if declared != expected {
        return Err(FormatError::Invalid(format!(
            "{name}: declared shape {:?} ({} elems) != expected {:?} ({} elems)",
            t.shape, declared, shape, expected
        )));
    }
    Ok(Tensor::try_from_f32(shape, t.to_f32_vec())
        .map_err(|e| FormatError::Invalid(format!("{name}: {e}")))?)
}

fn load_quant_weight_reshaped(
    lm: &LoadedModel,
    name: &str,
    shape: Vec<usize>,
) -> Result<QuantWeight, FormatError> {
    let mut qw = load_quant_weight(lm, name)?;
    let declared: usize = qw.shape.iter().product();
    let expected: usize = shape.iter().product();
    if declared != expected {
        return Err(FormatError::Invalid(format!(
            "{name}: declared shape {:?} ({} elems) != expected {:?} ({} elems)",
            qw.shape, declared, shape, expected
        )));
    }
    qw.shape = shape;
    Ok(qw)
}

fn load_layer(
    lm: &LoadedModel,
    i: usize,
    hidden: usize,
    q_dim: usize,
    kv_dim: usize,
    intermediate: usize,
) -> Result<LayerWeights, FormatError> {
    let prefix = format!("model.layers.{i}");
    let try_load_f32 = |name: &str| -> Option<Tensor> {
        let full = format!("{prefix}.{name}");
        lm.tensors.iter().find(|t| t.name == full)?;
        load_tensor_f32(lm, &full).ok()
    };
    let must_f32 = |name: &str| -> Result<Tensor, FormatError> {
        load_tensor_f32(lm, &format!("{prefix}.{name}"))
    };
    // Matmul weight with expected [N, K]. If metadata stores [K, N],
    // we force the canonical shape (physical byte layout is the same —
    // the whole array is just N*K values row-major).
    let quant_nk = |name: &str, n: usize, k: usize| -> Result<QuantWeight, FormatError> {
        load_quant_weight_reshaped(lm, &format!("{prefix}.{name}"), vec![n, k])
    };

    Ok(LayerWeights {
        input_norm: must_f32("input_layernorm.weight")?,
        q_proj: quant_nk("self_attn.q_proj.weight", q_dim, hidden)?,
        k_proj: quant_nk("self_attn.k_proj.weight", kv_dim, hidden)?,
        v_proj: quant_nk("self_attn.v_proj.weight", kv_dim, hidden)?,
        o_proj: quant_nk("self_attn.o_proj.weight", hidden, q_dim)?,
        q_proj_bias: try_load_f32("self_attn.q_proj.bias"),
        k_proj_bias: try_load_f32("self_attn.k_proj.bias"),
        v_proj_bias: try_load_f32("self_attn.v_proj.bias"),
        q_norm: try_load_f32("self_attn.q_norm.weight"),
        k_norm: try_load_f32("self_attn.k_norm.weight"),
        post_norm: must_f32("post_attention_layernorm.weight")?,
        gate_proj: quant_nk("mlp.gate_proj.weight", intermediate, hidden)?,
        up_proj: quant_nk("mlp.up_proj.weight", intermediate, hidden)?,
        down_proj: quant_nk("mlp.down_proj.weight", hidden, intermediate)?,
        post_attn_norm: try_load_f32("post_attention_norm.weight"),
        post_ffw_norm: try_load_f32("post_ffw_norm.weight"),
        layer_output_scale: try_load_f32("layer_output_scale.weight"),
    })
}
