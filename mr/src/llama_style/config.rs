//! LlamaStyle configuration parsed from .model config section.

use crate::format::FormatError;

/// Per-layer attention kind. LlamaStyle has all `Sliding` (single shape).
/// LlamaStyle+ (Gemma 3/4) interleaves `Sliding` and `Full`; full layers
/// in Gemma 4 use `global_head_dim` / `num_global_key_value_heads`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    Sliding,
    Full,
}

/// Activation function for the FFN gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenActivation {
    Silu,
    GeluTanh,
    GeluErf,
}

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

    // ── LlamaStyle+ (Gemma 3/4) optional fields ──
    /// One entry per layer. LlamaStyle defaults to all `Sliding`.
    pub layer_types: Vec<LayerKind>,
    /// Window size for `Sliding` layers (Gemma 3/4 typical: 1024).
    pub sliding_window: Option<usize>,
    /// FFN activation. Default `Silu`.
    pub hidden_activation: HiddenActivation,
    /// Logit softcapping value applied after lm_head. None or 0.0 = skip.
    pub final_logit_softcapping: Option<f32>,
    /// K and V projections share weights (mi materialises both names).
    /// Informational; the runtime always sees both tensors.
    pub attention_k_eq_v: bool,
    /// Gemma-4: head_dim used by `Full` layers (per arch.md §LlamaStyle+).
    /// None = `Full` layers use the regular `head_dim`.
    pub global_head_dim: Option<usize>,
    /// Gemma-4: kv_heads used by `Full` layers.
    /// None = `Full` layers use the regular `num_key_value_heads`.
    pub num_global_key_value_heads: Option<usize>,
    /// Gemma-4: per-layer-kind rope_theta. `Full` layers use `rope_theta_full`
    /// (typically 1e6), `Sliding` layers use the regular `rope_theta` (1e4).
    /// None = both kinds use the same `rope_theta`.
    pub rope_theta_full: Option<f32>,
    /// Gemma-4: fraction of head_dim that gets rotated for `Full` layers
    /// (`partial_rotary_factor`, e.g. 0.25). Sliding layers always rotate
    /// the full head_dim. None = no partial rotary.
    pub partial_rotary_factor_full: Option<f32>,
    /// Gemma family: divisor for attention scaling. Default per HF Gemma 3
    /// is 256 regardless of head_dim. LlamaStyle defaults to head_dim
    /// (standard 1/sqrt(head_dim)). Affects full layers most because their
    /// head_dim differs from sliding's.
    pub query_pre_attn_scalar: Option<usize>,
}

impl LlamaConfig {
    /// Per-layer head_dim. Gemma-4 full layers use `global_head_dim`.
    pub fn layer_head_dim(&self, layer: usize) -> usize {
        match self.layer_types.get(layer).copied() {
            Some(LayerKind::Full) => self.global_head_dim.unwrap_or(self.head_dim),
            _ => self.head_dim,
        }
    }

    /// Per-layer kv_heads. Gemma-4 full layers use `num_global_key_value_heads`.
    pub fn layer_kv_heads(&self, layer: usize) -> usize {
        match self.layer_types.get(layer).copied() {
            Some(LayerKind::Full) => self
                .num_global_key_value_heads
                .unwrap_or(self.num_key_value_heads),
            _ => self.num_key_value_heads,
        }
    }

    /// Returns the sliding-window mask boundary for a layer, or None for full attention.
    pub fn layer_window(&self, layer: usize) -> Option<usize> {
        match self.layer_types.get(layer).copied() {
            Some(LayerKind::Sliding) => self.sliding_window,
            _ => None,
        }
    }

    /// Per-layer RoPE base. Gemma-4 full layers use `rope_theta_full` (1e6);
    /// sliding layers and LlamaStyle use `rope_theta` (1e4 default).
    pub fn layer_rope_theta(&self, layer: usize) -> f32 {
        match self.layer_types.get(layer).copied() {
            Some(LayerKind::Full) => self.rope_theta_full.unwrap_or(self.rope_theta),
            _ => self.rope_theta,
        }
    }

    /// Per-layer rotated dim. Gemma-4 full layers use partial_rotary_factor.
    /// Returns even count ≤ head_dim.
    pub fn layer_rope_dim(&self, layer: usize) -> usize {
        let head_dim = self.layer_head_dim(layer);
        match self.layer_types.get(layer).copied() {
            Some(LayerKind::Full) => match self.partial_rotary_factor_full {
                Some(f) if f > 0.0 && f < 1.0 => {
                    let d = (head_dim as f32 * f) as usize;
                    // round down to even
                    d & !1
                }
                _ => head_dim,
            },
            _ => head_dim,
        }
    }

    /// Attention scaling. Three regimes:
    ///   - Gemma 4: scaling = 1.0 (Q and K already pre-normalised by q_norm
    ///     and k_norm, so the dot product is bounded; no extra sqrt scale).
    ///   - Gemma 3 with `query_pre_attn_scalar`: 1/sqrt(scalar).
    ///   - LlamaStyle (everything else): standard 1/sqrt(head_dim).
    pub fn layer_attn_scale(&self, layer: usize) -> f32 {
        // Gemma 4 only: identified by the presence of attention_k_eq_v
        // alongside the model_type starting with "gemma".
        if self.attention_k_eq_v && self.model_type.starts_with("gemma") {
            return 1.0;
        }
        let scalar = self
            .query_pre_attn_scalar
            .unwrap_or(self.layer_head_dim(layer));
        1.0 / (scalar as f32).sqrt()
    }
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

        // ── LlamaStyle+ (Gemma 3/4) parsing ──
        // Spec: reference/runtime/format.md §"LlamaStyle+ extra fields"
        let layer_types: Vec<LayerKind> = arch
            .get("layer_types")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| match s {
                        "full_attention" | "full" => LayerKind::Full,
                        _ => LayerKind::Sliding,
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![LayerKind::Sliding; num_hidden_layers]);
        if layer_types.len() != num_hidden_layers {
            return Err(FormatError::Invalid(format!(
                "layer_types length {} != num_hidden_layers {}",
                layer_types.len(),
                num_hidden_layers
            )));
        }
        let sliding_window = arch
            .get("sliding_window")
            .and_then(|v| v.as_integer())
            .map(|i| i as usize);
        let hidden_activation = arch
            .get("hidden_activation")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "gelu_pytorch_tanh" | "gelu_tanh" => HiddenActivation::GeluTanh,
                "gelu" | "gelu_erf" => HiddenActivation::GeluErf,
                _ => HiddenActivation::Silu,
            })
            .unwrap_or(HiddenActivation::Silu);
        let final_logit_softcapping = arch
            .get("final_logit_softcapping")
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .map(|f| f as f32)
            .filter(|&f| f > 0.0);
        let attention_k_eq_v = arch
            .get("attention_k_eq_v")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let global_head_dim = arch
            .get("global_head_dim")
            .and_then(|v| v.as_integer())
            .map(|i| i as usize);
        let num_global_key_value_heads = arch
            .get("num_global_key_value_heads")
            .and_then(|v| v.as_integer())
            .map(|i| i as usize);
        let rope_theta_full = arch
            .get("rope_theta_full")
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .map(|f| f as f32);
        let partial_rotary_factor_full = arch
            .get("partial_rotary_factor_full")
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .map(|f| f as f32);
        // Gemma family uses query_pre_attn_scalar=256 by default per HF
        // Gemma 3 (independent of head_dim). LlamaStyle leaves it None →
        // attention scaling falls back to 1/sqrt(head_dim).
        let query_pre_attn_scalar = arch
            .get("query_pre_attn_scalar")
            .and_then(|v| v.as_integer())
            .map(|i| i as usize)
            .or_else(|| {
                if model_type.starts_with("gemma") {
                    Some(256)
                } else {
                    None
                }
            });

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
            layer_types,
            sliding_window,
            hidden_activation,
            final_logit_softcapping,
            attention_k_eq_v,
            global_head_dim,
            num_global_key_value_heads,
            rope_theta_full,
            partial_rotary_factor_full,
            query_pre_attn_scalar,
        })
    }
}
