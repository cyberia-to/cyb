//! LlamaStyle weight loading.

use crate::cpu::dequantize_to_f32;
use crate::format::{FormatError, LoadedModel, TensorMeta};
use crate::tensor::Tensor;

pub struct LayerWeights {
    pub input_norm: Tensor,
    pub q_proj: Tensor,
    pub k_proj: Tensor,
    pub v_proj: Tensor,
    pub o_proj: Tensor,
    pub q_proj_bias: Option<Tensor>,
    pub k_proj_bias: Option<Tensor>,
    pub v_proj_bias: Option<Tensor>,
    pub q_norm: Option<Tensor>,
    pub k_norm: Option<Tensor>,
    pub post_norm: Tensor,
    pub gate_proj: Tensor,
    pub up_proj: Tensor,
    pub down_proj: Tensor,
}

pub struct Weights {
    pub embed_tokens: Tensor,
    pub layers: Vec<LayerWeights>,
    pub final_norm: Tensor,
    /// None if tied to embed_tokens.
    pub lm_head: Option<Tensor>,
}

impl Weights {
    pub fn load(lm: &LoadedModel, num_layers: usize, tie_word_embeddings: bool) -> Result<Self, FormatError> {
        let embed_tokens = load_tensor_f32(lm, "model.embed_tokens.weight")?;
        let final_norm = load_tensor_f32(lm, "model.norm.weight")?;

        let lm_head = if tie_word_embeddings {
            None
        } else {
            Some(load_tensor_f32(lm, "lm_head.weight")?)
        };

        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            let lw = load_layer(lm, i)?;
            layers.push(lw);
        }

        Ok(Self {
            embed_tokens,
            layers,
            final_norm,
            lm_head,
        })
    }
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
    let f32s = dequantize_to_f32(bytes, meta.dtype);
    Ok(Tensor::from_f32(meta.shape.clone(), f32s))
}

fn load_layer(lm: &LoadedModel, i: usize) -> Result<LayerWeights, FormatError> {
    let prefix = format!("model.layers.{i}");
    let try_load = |name: &str| -> Option<Tensor> {
        let full = format!("{prefix}.{name}");
        lm.tensors.iter().find(|t| t.name == full)?;
        load_tensor_f32(lm, &full).ok()
    };
    let must_load = |name: &str| -> Result<Tensor, FormatError> {
        load_tensor_f32(lm, &format!("{prefix}.{name}"))
    };

    Ok(LayerWeights {
        input_norm: must_load("input_layernorm.weight")?,
        q_proj: must_load("self_attn.q_proj.weight")?,
        k_proj: must_load("self_attn.k_proj.weight")?,
        v_proj: must_load("self_attn.v_proj.weight")?,
        o_proj: must_load("self_attn.o_proj.weight")?,
        q_proj_bias: try_load("self_attn.q_proj.bias"),
        k_proj_bias: try_load("self_attn.k_proj.bias"),
        v_proj_bias: try_load("self_attn.v_proj.bias"),
        q_norm: try_load("self_attn.q_norm.weight"),
        k_norm: try_load("self_attn.k_norm.weight"),
        post_norm: must_load("post_attention_layernorm.weight")?,
        gate_proj: must_load("mlp.gate_proj.weight")?,
        up_proj: must_load("mlp.up_proj.weight")?,
        down_proj: must_load("mlp.down_proj.weight")?,
    })
}

/// Size of the meta — useful for debug.
pub fn tensor_meta_summary(lm: &LoadedModel) -> String {
    let total_bytes: u64 = lm.tensors.iter().map(|t| t.size).sum();
    let by_dtype: std::collections::HashMap<_, u64> = lm
        .tensors
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, t| {
            *acc.entry(t.dtype).or_insert(0) += t.size;
            acc
        });
    format!(
        "{} tensors, {:.1} MB total, by dtype: {:?}",
        lm.tensors.len(),
        total_bytes as f64 / 1e6,
        by_dtype
    )
}

/// Unused outside integration.
#[allow(dead_code)]
pub fn meta_for<'a>(lm: &'a LoadedModel, name: &str) -> Option<&'a TensorMeta> {
    lm.tensors.iter().find(|t| t.name == name)
}
