//! LlamaStyle configuration parsed from .model config section.

use crate::format::FormatError;

#[derive(Clone, Debug)]
pub struct LlamaConfig {
    pub model_type: String,
    pub hidden_size: usize,
    pub num_attention_heads: usize,
    pub num_key_value_heads: usize,
    pub num_hidden_layers: usize,
    pub intermediate_size: usize,
    pub vocab_size: usize,
    pub max_position_embeddings: usize,
    pub rope_theta: f32,
    pub rms_norm_eps: f32,
    pub tie_word_embeddings: bool,
    pub head_dim: usize,
    /// Detected from tensor presence.
    pub has_qk_norm: bool,
    pub has_attn_bias: bool,
    pub eos_token_ids: Vec<u32>,
}

impl LlamaConfig {
    pub fn parse(config_toml: &str, tensors: &[crate::format::TensorMeta]) -> Result<Self, FormatError> {
        let value: toml::Value = toml::from_str(config_toml)
            .map_err(|e| FormatError::Invalid(format!("config.toml: {e}")))?;

        let model_type = value
            .get("model_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let arch = value.get("architecture").ok_or_else(|| {
            FormatError::Invalid("config.toml: missing [architecture]".into())
        })?;

        let get_usize = |key: &str| -> Result<usize, FormatError> {
            arch.get(key)
                .and_then(|v| v.as_integer())
                .map(|i| i as usize)
                .ok_or_else(|| FormatError::Invalid(format!("missing/invalid {key}")))
        };
        let get_usize_default = |key: &str, default: usize| -> usize {
            arch.get(key)
                .and_then(|v| v.as_integer())
                .map(|i| i as usize)
                .unwrap_or(default)
        };

        let hidden_size = get_usize("hidden_size")?;
        let num_attention_heads = get_usize("num_attention_heads")?;
        let num_key_value_heads =
            get_usize_default("num_key_value_heads", num_attention_heads);
        let num_hidden_layers = get_usize("num_hidden_layers")?;
        let intermediate_size = get_usize("intermediate_size")?;
        let vocab_size = get_usize("vocab_size")?;
        let max_position_embeddings = get_usize_default("max_position_embeddings", 2048);

        let rope_theta = arch
            .get("rope_theta")
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .unwrap_or(10000.0) as f32;

        // rms_norm_eps is stored as direct (0.000001) or inverse (1000000) — canonicalize.
        let eps_raw = arch
            .get("rms_norm_eps")
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .unwrap_or(1e-6);
        let rms_norm_eps = if eps_raw >= 1.0 {
            (1.0 / eps_raw) as f32
        } else {
            eps_raw as f32
        };

        let tie_word_embeddings = arch
            .get("tie_word_embeddings")
            .and_then(|v| v.as_bool())
            .or_else(|| value.get("tie_word_embeddings").and_then(|v| v.as_bool()))
            .unwrap_or(true);

        // head_dim: config if present, else derive from q_proj shape.
        // Qwen3 uses head_dim=128 independent of hidden_size/num_heads.
        let head_dim = arch
            .get("head_dim")
            .and_then(|v| v.as_integer())
            .map(|i| i as usize)
            .or_else(|| {
                tensors
                    .iter()
                    .find(|t| t.name == "model.layers.0.self_attn.q_proj.weight")
                    .map(|t| t.shape[0] / num_attention_heads)
            })
            .unwrap_or(hidden_size / num_attention_heads);

        // Spec validation per arch.md LlamaStyle.
        if head_dim == 0 || head_dim % 2 != 0 {
            return Err(FormatError::Invalid(format!(
                "head_dim must be positive and even, got {head_dim}"
            )));
        }
        if num_attention_heads == 0 {
            return Err(FormatError::Invalid("num_attention_heads must be > 0".into()));
        }
        if num_key_value_heads == 0 || num_attention_heads % num_key_value_heads != 0 {
            return Err(FormatError::Invalid(format!(
                "GQA requires num_heads ({num_attention_heads}) divisible by kv_heads ({num_key_value_heads})"
            )));
        }
        if num_hidden_layers == 0 {
            return Err(FormatError::Invalid("num_hidden_layers must be > 0".into()));
        }
        if vocab_size == 0 {
            return Err(FormatError::Invalid("vocab_size must be > 0".into()));
        }
        if rope_theta <= 0.0 {
            return Err(FormatError::Invalid(format!(
                "rope_theta must be positive, got {rope_theta}"
            )));
        }
        if !(rms_norm_eps > 0.0 && rms_norm_eps < 1.0) {
            return Err(FormatError::Invalid(format!(
                "rms_norm_eps outside sane range (0, 1): {rms_norm_eps}"
            )));
        }

        // Detect variants by tensor presence
        let has_qk_norm = tensors
            .iter()
            .any(|t| t.name == "model.layers.0.self_attn.q_norm.weight");
        let has_attn_bias = tensors
            .iter()
            .any(|t| t.name == "model.layers.0.self_attn.q_proj.bias");

        // EOS tokens from [tokenizer].eos_token_ids
        let eos_token_ids = value
            .get("tokenizer")
            .and_then(|t| t.get("eos_token_ids"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_integer().map(|i| i as u32))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            model_type,
            hidden_size,
            num_attention_heads,
            num_key_value_heads,
            num_hidden_layers,
            intermediate_size,
            vocab_size,
            max_position_embeddings,
            rope_theta,
            rms_norm_eps,
            tie_word_embeddings,
            head_dim,
            has_qk_norm,
            has_attn_bias,
            eos_token_ids,
        })
    }
}
