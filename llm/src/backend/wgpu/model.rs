//! NativeModel — transformer forward pass on pure wgpu
//! Loads ONNX Q4 or safetensors f32/bf16 weights, runs transformer layers via WGSL compute shaders

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::onnx_proto::onnx::ModelProto;
use prost::Message;

use crate::cyb_format::toml_to_json_value as toml_to_json;

use super::dispatch;
use super::pipelines::{ComputeShader, Pipelines};

/// Quantization format for weight projections
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum QuantFormat {
    F32,
    F16,
    Q4,
    Q8,
    Ternary,
}

/// Transformer model config detected from weight shapes
struct ModelConfig {
    hidden_size: usize,
    num_heads: usize,
    kv_num_heads: usize,
    head_dim: usize,
    num_layers: usize,
    vocab_size: usize,
    block_size: usize,
    has_qk_norm: bool,
}

/// Pre-computed uniform parameter buffers for constant-per-step ops
struct LayerParamBuffers {
    input_norm_params: wgpu::Buffer,
    input_norm_wg: (u32, u32, u32),
    q_norm_params: Option<wgpu::Buffer>,
    q_norm_wg: (u32, u32, u32),
    k_norm_params: Option<wgpu::Buffer>,
    k_norm_wg: (u32, u32, u32),
    post_norm_params: wgpu::Buffer,
    post_norm_wg: (u32, u32, u32),
    q_matmul_params: wgpu::Buffer,
    q_matmul_wg: (u32, u32, u32),
    k_matmul_params: wgpu::Buffer,
    k_matmul_wg: (u32, u32, u32),
    v_matmul_params: wgpu::Buffer,
    v_matmul_wg: (u32, u32, u32),
    o_matmul_params: wgpu::Buffer,
    o_matmul_wg: (u32, u32, u32),
    gate_matmul_params: wgpu::Buffer,
    gate_matmul_wg: (u32, u32, u32),
    up_matmul_params: wgpu::Buffer,
    up_matmul_wg: (u32, u32, u32),
    down_matmul_params: wgpu::Buffer,
    down_matmul_wg: (u32, u32, u32),
    q_rope_params: wgpu::Buffer,
    q_rope_wg: (u32, u32, u32),
    k_rope_params: wgpu::Buffer,
    k_rope_wg: (u32, u32, u32),
    fused_q_params: Option<wgpu::Buffer>,
    fused_q_wg: (u32, u32, u32),
    fused_k_params: Option<wgpu::Buffer>,
    fused_k_wg: (u32, u32, u32),
    fused_v_params: Option<wgpu::Buffer>,
    fused_v_wg: (u32, u32, u32),
}

/// Per-layer weights, all on GPU.
/// Each projection stores its own quant format for per-tensor mixed quantization.
#[allow(dead_code)]
struct LayerWeights {
    input_norm_weight: wgpu::Buffer,
    q_proj_packed: wgpu::Buffer,
    q_proj_scales: wgpu::Buffer,
    q_proj_quant: QuantFormat,
    q_n: u32,
    k_proj_packed: wgpu::Buffer,
    k_proj_scales: wgpu::Buffer,
    k_proj_quant: QuantFormat,
    k_n: u32,
    v_proj_packed: wgpu::Buffer,
    v_proj_scales: wgpu::Buffer,
    v_proj_quant: QuantFormat,
    v_n: u32,
    q_proj_bias: Option<wgpu::Buffer>,
    k_proj_bias: Option<wgpu::Buffer>,
    v_proj_bias: Option<wgpu::Buffer>,
    q_norm_weight: Option<wgpu::Buffer>,
    k_norm_weight: Option<wgpu::Buffer>,
    o_proj_packed: wgpu::Buffer,
    o_proj_scales: wgpu::Buffer,
    o_proj_quant: QuantFormat,
    o_n: u32,
    post_norm_weight: wgpu::Buffer,
    gate_proj_packed: wgpu::Buffer,
    gate_proj_scales: wgpu::Buffer,
    gate_proj_quant: QuantFormat,
    gate_n: u32,
    up_proj_packed: wgpu::Buffer,
    up_proj_scales: wgpu::Buffer,
    up_proj_quant: QuantFormat,
    up_n: u32,
    down_proj_packed: wgpu::Buffer,
    down_proj_scales: wgpu::Buffer,
    down_proj_quant: QuantFormat,
    down_n: u32,
    params: LayerParamBuffers,
}

/// KV cache for one layer
struct KVCache {
    key: Option<wgpu::Buffer>,
    value: Option<wgpu::Buffer>,
}

/// Pre-computed param buffers for model-level ops
struct ModelParamBuffers {
    final_norm_params: wgpu::Buffer,
    final_norm_wg: (u32, u32, u32),
    f32_matmul_params: wgpu::Buffer,
    f32_matmul_wg: (u32, u32, u32),
    argmax_params: wgpu::Buffer,
    argmax_wg: (u32, u32, u32),
}

/// Native wgpu model — holds all weights on GPU and runs forward pass
pub struct NativeModel {
    config: ModelConfig,
    pipelines: Arc<Pipelines>,
    embed_table: wgpu::Buffer,
    /// Separate LM head weights (None = tied to embed_table)
    lm_head: Option<wgpu::Buffer>,
    layers: Vec<LayerWeights>,
    final_norm_weight: wgpu::Buffer,
    cos_cache: Vec<f32>,
    sin_cache: Vec<f32>,
    kv_cache: Vec<KVCache>,
    past_seq_len: usize,
    /// When true, argmax is computed on GPU
    pub greedy_mode: bool,
    /// Quantization format for projections (Q4, Q8, or F32)
    quant_format: QuantFormat,
    model_params: ModelParamBuffers,
    /// TurboQuant KV cache compressor (None = standard uncompressed cache)
    pub kv_compressor: Option<crate::kv_compress::KvCompressor>,
}

/// Convert model-level QuantFormat to dispatch::QuantFormat
fn quant_fmt_to_dispatch(qf: QuantFormat) -> dispatch::QuantFormat {
    match qf {
        QuantFormat::F32 => dispatch::QuantFormat::F32,
        QuantFormat::F16 => dispatch::QuantFormat::F16,
        QuantFormat::Q4 => dispatch::QuantFormat::Q4,
        QuantFormat::Q8 => dispatch::QuantFormat::Q8,
        QuantFormat::Ternary => dispatch::QuantFormat::Ternary,
    }
}

/// Load and parse an ONNX model protobuf
fn load_model_proto(path: &Path) -> Result<ModelProto, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buf)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    ModelProto::decode(&*buf).map_err(|e| format!("Failed to decode ONNX protobuf: {e}"))
}

/// Read raw bytes from a TensorProto, supporting external data
fn read_tensor_raw(
    tp: &crate::onnx_proto::onnx::TensorProto,
    model_dir: &Path,
) -> Result<Vec<u8>, String> {
    if tp.data_location == 1 {
        let mut location = String::new();
        let mut offset: u64 = 0;
        let mut length: u64 = 0;

        for entry in &tp.external_data {
            match entry.key.as_str() {
                "location" => location = entry.value.clone(),
                "offset" => offset = entry.value.parse().unwrap_or(0),
                "length" => length = entry.value.parse().unwrap_or(0),
                _ => {}
            }
        }

        if location.is_empty() || length == 0 {
            return Err(format!(
                "External tensor {} has no location/length",
                tp.name
            ));
        }

        let data_path = model_dir.join(&location);
        let file = std::fs::File::open(&data_path)
            .map_err(|e| format!("Cannot open external data {}: {e}", data_path.display()))?;
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(|e| format!("Cannot mmap {}: {e}", data_path.display()))?
        };
        let end = (offset + length) as usize;
        if end > mmap.len() {
            return Err(format!(
                "External data {} out of bounds: offset={offset} length={length} file_size={}",
                tp.name,
                mmap.len()
            ));
        }

        Ok(mmap[offset as usize..end].to_vec())
    } else if !tp.raw_data.is_empty() {
        Ok(tp.raw_data.clone())
    } else if !tp.float_data.is_empty() {
        Ok(bytemuck::cast_slice(&tp.float_data).to_vec())
    } else {
        Err(format!("Tensor {} has no data", tp.name))
    }
}

/// Convert raw bytes to f32 based on data_type
fn raw_to_f32(raw: &[u8], data_type: i32) -> Vec<f32> {
    match data_type {
        1 => raw
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        10 => raw
            .chunks_exact(2)
            .map(|c| half::f16::from_le_bytes([c[0], c[1]]).to_f32())
            .collect(),
        2 => raw.iter().map(|&b| b as f32).collect(),
        _ => {
            log::warn!("Unsupported data type {data_type}, treating as f32 bytes");
            raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        }
    }
}

/// Convert safetensors weight bytes to f32, based on DType from IR
/// Transpose a 2D f32 matrix from [rows, cols] to [cols, rows]
pub fn transpose_f32(data: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            out[c * rows + r] = data[r * cols + c];
        }
    }
    out
}

pub fn safetensors_to_f32(data: &[u8], dtype: crate::ir::DType) -> Vec<f32> {
    use crate::ir::DType;
    match dtype {
        DType::F32 => data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        DType::BF16 => data
            .chunks_exact(2)
            .map(|c| {
                let bits = u16::from_le_bytes([c[0], c[1]]);
                f32::from_bits((bits as u32) << 16)
            })
            .collect(),
        DType::F16 => data
            .chunks_exact(2)
            .map(|c| half::f16::from_le_bytes([c[0], c[1]]).to_f32())
            .collect(),
        DType::Q4 => {
            // GGUF Q4_0: blocks of {half scale, uint8_t qs[16]} = 18 bytes = 32 weights
            // llama.cpp order: first all lo nibbles (j=0..15), then all hi nibbles (j=16..31)
            let block_size = 18;
            data.chunks_exact(block_size).flat_map(|block| {
                let scale_bits = u16::from_le_bytes([block[0], block[1]]);
                let scale = half::f16::from_bits(scale_bits).to_f32();
                let qs = &block[2..18];
                let mut vals = [0.0f32; 32];
                for j in 0..16 {
                    let byte = qs[j];
                    vals[j] = scale * ((byte & 0x0F) as i32 - 8) as f32;      // lo → first half
                    vals[j + 16] = scale * ((byte >> 4) as i32 - 8) as f32;    // hi → second half
                }
                vals
            }).collect()
        }
        DType::Q8 => {
            // GGUF Q8_0: blocks of {half scale, int8_t qs[32]} = 34 bytes = 32 weights
            let block_size = 34;
            data.chunks_exact(block_size).flat_map(|block| {
                let scale_bits = u16::from_le_bytes([block[0], block[1]]);
                let scale = half::f16::from_bits(scale_bits).to_f32();
                let qs = &block[2..34];
                qs.iter().map(move |&q| scale * (q as i8) as f32)
            }).collect()
        }
        DType::U8 | DType::Ternary => {
            // Ternary weights: 4 values packed per byte (2 bits each)
            // Encoding: 0b00 = -1, 0b01 = 0, 0b10 = +1
            data.iter().flat_map(|&byte| {
                (0..4).map(move |i| {
                    match (byte >> (i * 2)) & 0x3 {
                        0 => -1.0f32,
                        1 =>  0.0f32,
                        2 =>  1.0f32,
                        _ =>  0.0f32,
                    }
                })
            }).collect()
        }
        _ => {
            log::warn!("Unsupported safetensors dtype {:?}, treating as f32", dtype);
            data.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        }
    }
}

/// Quantize f32 weight matrix [N, K] to Q4 format (4-bit unsigned, block_size=32).
/// Returns (packed_u32s, scales) where:
///   packed: each u32 holds 8 nibbles (8 weights), row-major [N, K/8]
///   scales: one f32 per block per row, [N, K/block_size]
fn quantize_f32_to_q4(weights: &[f32], n: usize, k: usize) -> (Vec<u32>, Vec<f32>) {
    const BLOCK_SIZE: usize = 32;
    let num_blocks = k / BLOCK_SIZE;
    let u32s_per_row = k / 8; // 8 nibbles per u32 (4 bits * 8 = 32 bits)
    let mut packed = vec![0u32; n * u32s_per_row];
    let mut scales = vec![0.0f32; n * num_blocks];

    for row in 0..n {
        let row_offset = row * k;
        for blk in 0..num_blocks {
            let blk_start = row_offset + blk * BLOCK_SIZE;
            let block = &weights[blk_start..blk_start + BLOCK_SIZE];

            // Find absmax for symmetric quantization
            let mut amax = 0.0f32;
            for &v in block {
                amax = amax.max(v.abs());
            }

            // Scale: map [-amax, amax] to [0, 15] with zero_point = 8
            let scale = if amax > 0.0 { amax / 7.0 } else { 0.0 };
            scales[row * num_blocks + blk] = scale;

            let inv_scale = if scale > 0.0 { 1.0 / scale } else { 0.0 };

            // Quantize each weight to 4-bit nibble [0, 15]
            for j in 0..BLOCK_SIZE {
                let q = ((block[j] * inv_scale).round() as i32 + 8).clamp(0, 15) as u8;
                let col = blk * BLOCK_SIZE + j;
                let u32_idx = col / 8;
                let nibble_pos = col % 8;
                // Pack: first nibble in low bits of first byte, etc.
                // Each byte holds 2 nibbles: lo = even, hi = odd
                let byte_pos = nibble_pos / 2;
                let is_hi = nibble_pos % 2;
                let shift = byte_pos * 8 + is_hi * 4;
                packed[row * u32s_per_row + u32_idx] |= (q as u32) << shift;
            }
        }
    }

    (packed, scales)
}

/// Pack raw UINT8 bytes into u32
fn pack_bytes_to_u32(raw: &[u8]) -> Vec<u32> {
    raw.chunks(4)
        .map(|chunk| {
            let mut val = 0u32;
            for (i, &b) in chunk.iter().enumerate() {
                val |= (b as u32) << (i * 8);
            }
            val
        })
        .collect()
}

// ---- Precompute helpers ----

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q4MatmulParams {
    n: u32,
    k: u32,
    num_blocks: u32,
    u32s_per_row: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RmsNormParams {
    hidden: u32,
    eps: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FusedNormQ4Params {
    n: u32,
    k: u32,
    num_blocks: u32,
    u32s_per_row: u32,
    eps: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

fn precompute_fused_norm_q4(
    p: &Pipelines,
    n: u32,
    k: u32,
    block_size: u32,
    eps: f32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let num_blocks = k / block_size;
    let params = FusedNormQ4Params {
        n,
        k,
        num_blocks,
        u32s_per_row: num_blocks * (block_size / 2) / 4,
        eps,
        _pad0: 0,
        _pad1: 0,
        _pad2: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (buf, (x, y, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RopeParams {
    half_dim: u32,
    head_dim: u32,
    seq_len: u32,
    total_elements: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct F32MatmulParams {
    n: u32,
    k: u32,
    p: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ArgmaxParams {
    n: u32,
}

fn precompute_q4_matmul(
    p: &Pipelines,
    n: u32,
    k: u32,
    block_size: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let num_blocks = k / block_size;
    let params = Q4MatmulParams {
        n,
        k,
        num_blocks,
        u32s_per_row: num_blocks * (block_size / 2) / 4,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (buf, (x, y, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q8MatmulParams {
    n: u32,
    k: u32,
    num_blocks: u32,
    u32s_per_row: u32,
}

fn precompute_q8_matmul(
    p: &Pipelines,
    n: u32,
    k: u32,
    block_size: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let num_blocks = k / block_size;
    let params = Q8MatmulParams {
        n,
        k,
        num_blocks,
        u32s_per_row: k / 4, // 4 int8 per u32
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (buf, (x, y, 1))
}

fn precompute_rms_norm(
    p: &Pipelines,
    positions: u32,
    hidden: u32,
    eps: f32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = RmsNormParams { hidden, eps };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    (buf, (positions, 1, 1))
}

fn precompute_rope(
    p: &Pipelines,
    total_elements: u32,
    head_dim: u32,
    seq_len: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = RopeParams {
        half_dim: head_dim / 2,
        head_dim,
        seq_len,
        total_elements,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    (buf, ((total_elements + 255) / 256, 1, 1))
}

fn precompute_f32_matmul(p: &Pipelines, n: u32, k: u32) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = F32MatmulParams { n, k, p: 1 };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    if n <= 65535 {
        (buf, (n, 1, 1))
    } else {
        let x = 65535u32.min(n);
        let z = (n + x - 1) / x;
        (buf, (x, 1, z))
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct F16MatmulParams {
    n: u32,
    k: u32,
    k_half: u32,
    _pad: u32,
}

fn precompute_f16_matmul(p: &Pipelines, n: u32, k: u32) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = F16MatmulParams {
        n,
        k,
        k_half: k / 2,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let x = n.min(65535);
    let y = (n + x - 1) / x;
    (buf, (x, y, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TernaryMatmulParams {
    n: u32,
    k: u32,
    u32s_per_row: u32,
    _pad: u32,
}

fn precompute_ternary_matmul(p: &Pipelines, n: u32, k: u32) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = TernaryMatmulParams {
        n,
        k,
        u32s_per_row: (k + 15) / 16,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let x = n.min(65535);
    let y = (n + x - 1) / x;
    (buf, (x, y, 1))
}

fn precompute_argmax(p: &Pipelines, n: u32) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = ArgmaxParams { n };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    (buf, (1, 1, 1))
}

impl NativeModel {
    /// Load model from .model file — the only runtime entry point.
    pub fn load(path: &Path, pipelines: Arc<Pipelines>) -> Result<(Self, String), String> {
        use crate::cyb_format;

        let (config_str, weights, vocab_str) = cyb_format::load_weights_from_model(path)
            .map_err(|e| format!("Cannot read .model file: {e}"))?;

        let toml_val: toml::Value = toml::from_str(&config_str)
            .map_err(|e| format!("Invalid config in .model: {e}"))?;
        let config_json = toml_to_json(&toml_val);
        let arch = config_json.get("architecture").unwrap_or(&config_json);

        let hidden_size = arch.get("hidden_size").and_then(|v| v.as_u64())
            .ok_or("Missing hidden_size in config")? as usize;
        let num_heads = arch.get("num_attention_heads").and_then(|v| v.as_u64())
            .ok_or("Missing num_attention_heads in config")? as usize;
        let kv_num_heads = arch.get("num_key_value_heads").and_then(|v| v.as_u64())
            .unwrap_or(num_heads as u64) as usize;
        let num_layers = arch.get("num_hidden_layers").and_then(|v| v.as_u64())
            .ok_or("Missing num_hidden_layers in config")? as usize;
        let vocab_size = arch.get("vocab_size").and_then(|v| v.as_u64())
            .ok_or("Missing vocab_size in config")? as usize;
        let intermediate_size = arch.get("intermediate_size").and_then(|v| v.as_u64())
            .unwrap_or((hidden_size * 4) as u64) as usize;

        let rope_theta = arch.get("rope_theta")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(10000.0) as f32;

        // rms_norm_eps: stored as 1/ε in .model spec (1000000 → 1e-6)
        let rms_norm_eps_raw = arch.get("rms_norm_eps")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(1e-6);
        let rms_norm_eps = if rms_norm_eps_raw >= 1.0 {
            (1.0 / rms_norm_eps_raw) as f32
        } else {
            rms_norm_eps_raw as f32
        };

        let tie_word_embeddings = config_json.get("tie_word_embeddings")
            .or_else(|| arch.get("tie_word_embeddings"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let head_dim = arch.get("head_dim").and_then(|v| v.as_u64()).map(|v| v as usize)
            .unwrap_or_else(|| {
                if let Some(w) = weights.get("model.layers.0.self_attn.q_proj.weight") {
                    w.shape[0] / num_heads
                } else {
                    hidden_size / num_heads
                }
            });

        let has_qk_norm = weights.contains_key("model.layers.0.self_attn.q_norm.weight");
        let has_attn_bias = weights.contains_key("model.layers.0.self_attn.q_proj.bias");

        let config = ModelConfig {
            hidden_size, num_heads, kv_num_heads, head_dim, num_layers, vocab_size,
            block_size: 32, has_qk_norm,
        };

        log::info!(
            ".model: hidden={}, heads={}/{}, head_dim={}, layers={}, vocab={}, ffn={}, rope={}, eps={}",
            hidden_size, num_heads, kv_num_heads, head_dim, num_layers, vocab_size,
            intermediate_size, rope_theta, rms_norm_eps,
        );

        let weight_to_f32 = |name: &str| -> Result<Vec<f32>, String> {
            let w = weights.get(name).ok_or_else(|| format!("Missing weight: {name}"))?;
            Ok(safetensors_to_f32(&w.data, w.dtype))
        };

        let embed_f32 = weight_to_f32("model.embed_tokens.weight")?;
        let embed_table = pipelines.upload_f32(&embed_f32);

        let lm_head = if !tie_word_embeddings {
            weights.get("lm_head.weight").map(|lm_w| {
                pipelines.upload_f32(&safetensors_to_f32(&lm_w.data, lm_w.dtype))
            })
        } else { None };

        let q_dim = num_heads * head_dim;
        let kv_dim = kv_num_heads * head_dim;

        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            log::info!("Loading layer {i}/{num_layers}...");

            let input_norm_f32 = weight_to_f32(&format!("model.layers.{i}.input_layernorm.weight"))?;
            let input_norm_weight = pipelines.upload_f32(&input_norm_f32);

            let quantize_upload = |name: &str, n: usize, k: usize| -> Result<(wgpu::Buffer, wgpu::Buffer), String> {
                let f32_data = weight_to_f32(name)?;
                let (packed, scales) = quantize_f32_to_q4(&f32_data, n, k);
                Ok((pipelines.upload_u32(&packed), pipelines.upload_f32(&scales)))
            };

            let (q_proj_packed, q_proj_scales) = quantize_upload(
                &format!("model.layers.{i}.self_attn.q_proj.weight"), q_dim, hidden_size)?;
            let (k_proj_packed, k_proj_scales) = quantize_upload(
                &format!("model.layers.{i}.self_attn.k_proj.weight"), kv_dim, hidden_size)?;
            let (v_proj_packed, v_proj_scales) = quantize_upload(
                &format!("model.layers.{i}.self_attn.v_proj.weight"), kv_dim, hidden_size)?;
            let (o_proj_packed, o_proj_scales) = quantize_upload(
                &format!("model.layers.{i}.self_attn.o_proj.weight"), hidden_size, q_dim)?;

            let post_norm_f32 = weight_to_f32(&format!("model.layers.{i}.post_attention_layernorm.weight"))?;
            let post_norm_weight = pipelines.upload_f32(&post_norm_f32);

            let (gate_proj_packed, gate_proj_scales) = quantize_upload(
                &format!("model.layers.{i}.mlp.gate_proj.weight"), intermediate_size, hidden_size)?;
            let (up_proj_packed, up_proj_scales) = quantize_upload(
                &format!("model.layers.{i}.mlp.up_proj.weight"), intermediate_size, hidden_size)?;
            let (down_proj_packed, down_proj_scales) = quantize_upload(
                &format!("model.layers.{i}.mlp.down_proj.weight"), hidden_size, intermediate_size)?;

            let q_proj_bias = if has_attn_bias {
                Some(pipelines.upload_f32(&weight_to_f32(&format!("model.layers.{i}.self_attn.q_proj.bias"))?))
            } else { None };
            let k_proj_bias = if has_attn_bias {
                Some(pipelines.upload_f32(&weight_to_f32(&format!("model.layers.{i}.self_attn.k_proj.bias"))?))
            } else { None };
            let v_proj_bias = if has_attn_bias {
                Some(pipelines.upload_f32(&weight_to_f32(&format!("model.layers.{i}.self_attn.v_proj.bias"))?))
            } else { None };

            let q_norm_w = if has_qk_norm {
                Some(pipelines.upload_f32(&weight_to_f32(&format!("model.layers.{i}.self_attn.q_norm.weight"))?))
            } else { None };
            let k_norm_w = if has_qk_norm {
                Some(pipelines.upload_f32(&weight_to_f32(&format!("model.layers.{i}.self_attn.k_norm.weight"))?))
            } else { None };

            let hs = hidden_size as u32;
            let hd = head_dim as u32;
            let bs = 32u32;
            let (input_norm_params, input_norm_wg) = precompute_rms_norm(&pipelines, 1, hs, rms_norm_eps);
            let (q_matmul_params, q_matmul_wg) = precompute_q4_matmul(&pipelines, q_dim as u32, hs, bs);
            let (k_matmul_params, k_matmul_wg) = precompute_q4_matmul(&pipelines, kv_dim as u32, hs, bs);
            let (v_matmul_params, v_matmul_wg) = precompute_q4_matmul(&pipelines, kv_dim as u32, hs, bs);
            let (q_rope_params, q_rope_wg) = precompute_rope(&pipelines, q_dim as u32, hd, 1);
            let (k_rope_params, k_rope_wg) = precompute_rope(&pipelines, kv_dim as u32, hd, 1);
            let (o_matmul_params, o_matmul_wg) = precompute_q4_matmul(&pipelines, hs, q_dim as u32, bs);
            let (post_norm_params, post_norm_wg) = precompute_rms_norm(&pipelines, 1, hs, rms_norm_eps);
            let (gate_matmul_params, gate_matmul_wg) = precompute_q4_matmul(&pipelines, intermediate_size as u32, hs, bs);
            let (up_matmul_params, up_matmul_wg) = precompute_q4_matmul(&pipelines, intermediate_size as u32, hs, bs);
            let (down_matmul_params, down_matmul_wg) = precompute_q4_matmul(&pipelines, hs, intermediate_size as u32, bs);

            let (fused_q_params, fused_q_wg) = {
                let (b, w) = precompute_fused_norm_q4(&pipelines, q_dim as u32, hs, bs, rms_norm_eps);
                (Some(b), w)
            };
            let (fused_k_params, fused_k_wg) = {
                let (b, w) = precompute_fused_norm_q4(&pipelines, kv_dim as u32, hs, bs, rms_norm_eps);
                (Some(b), w)
            };
            let (fused_v_params, fused_v_wg) = {
                let (b, w) = precompute_fused_norm_q4(&pipelines, kv_dim as u32, hs, bs, rms_norm_eps);
                (Some(b), w)
            };

            layers.push(LayerWeights {
                input_norm_weight,
                q_proj_packed, q_proj_scales, q_proj_quant: QuantFormat::Q4, q_n: q_dim as u32,
                k_proj_packed, k_proj_scales, k_proj_quant: QuantFormat::Q4, k_n: kv_dim as u32,
                v_proj_packed, v_proj_scales, v_proj_quant: QuantFormat::Q4, v_n: kv_dim as u32,
                q_proj_bias, k_proj_bias, v_proj_bias,
                q_norm_weight: q_norm_w, k_norm_weight: k_norm_w,
                o_proj_packed, o_proj_scales, o_proj_quant: QuantFormat::Q4, o_n: hidden_size as u32,
                post_norm_weight,
                gate_proj_packed, gate_proj_scales, gate_proj_quant: QuantFormat::Q4, gate_n: intermediate_size as u32,
                up_proj_packed, up_proj_scales, up_proj_quant: QuantFormat::Q4, up_n: intermediate_size as u32,
                down_proj_packed, down_proj_scales, down_proj_quant: QuantFormat::Q4, down_n: hidden_size as u32,
                params: LayerParamBuffers {
                    input_norm_params, input_norm_wg,
                    q_norm_params: if has_qk_norm { Some(precompute_rms_norm(&pipelines, num_heads as u32, hd, rms_norm_eps).0) } else { None },
                    q_norm_wg: if has_qk_norm { precompute_rms_norm(&pipelines, num_heads as u32, hd, rms_norm_eps).1 } else { (0,0,0) },
                    k_norm_params: if has_qk_norm { Some(precompute_rms_norm(&pipelines, kv_num_heads as u32, hd, rms_norm_eps).0) } else { None },
                    k_norm_wg: if has_qk_norm { precompute_rms_norm(&pipelines, kv_num_heads as u32, hd, rms_norm_eps).1 } else { (0,0,0) },
                    post_norm_params, post_norm_wg,
                    q_matmul_params, q_matmul_wg, k_matmul_params, k_matmul_wg,
                    v_matmul_params, v_matmul_wg, o_matmul_params, o_matmul_wg,
                    gate_matmul_params, gate_matmul_wg, up_matmul_params, up_matmul_wg,
                    down_matmul_params, down_matmul_wg,
                    q_rope_params, q_rope_wg, k_rope_params, k_rope_wg,
                    fused_q_params, fused_q_wg, fused_k_params, fused_k_wg, fused_v_params, fused_v_wg,
                },
            });
        }

        let final_norm_f32 = weight_to_f32("model.norm.weight")?;
        let final_norm_weight = pipelines.upload_f32(&final_norm_f32);

        let max_seq_len = 2048;
        let half_dim = head_dim / 2;
        let mut cos_cache = vec![0.0f32; max_seq_len * half_dim];
        let mut sin_cache = vec![0.0f32; max_seq_len * half_dim];
        for pos in 0..max_seq_len {
            for j in 0..half_dim {
                let theta = (pos as f32) / rope_theta.powf(2.0 * j as f32 / head_dim as f32);
                cos_cache[pos * half_dim + j] = theta.cos();
                sin_cache[pos * half_dim + j] = theta.sin();
            }
        }

        let kv_cache = (0..num_layers).map(|_| KVCache { key: None, value: None }).collect();

        let (final_norm_params, final_norm_wg) = precompute_rms_norm(&pipelines, 1, hidden_size as u32, rms_norm_eps);
        let (f32_matmul_params, f32_matmul_wg) = precompute_f32_matmul(&pipelines, vocab_size as u32, hidden_size as u32);
        let (argmax_params, argmax_wg) = precompute_argmax(&pipelines, vocab_size as u32);

        log::info!(".model loaded: {} layers, Q4 quantize-on-load", num_layers);

        let model = Self {
            config, pipelines, embed_table, lm_head, layers, final_norm_weight,
            cos_cache, sin_cache, kv_cache, past_seq_len: 0, greedy_mode: false,
            quant_format: QuantFormat::Q4,
            model_params: ModelParamBuffers {
                final_norm_params, final_norm_wg,
                f32_matmul_params, f32_matmul_wg,
                argmax_params, argmax_wg,
            },
            kv_compressor: None,
        };
        Ok((model, vocab_str))
    }

    /// Enable TurboQuant KV cache compression.
    /// Call after model load, before inference. Reduces KV cache memory ~4x.
    pub fn enable_kv_compression(&mut self) {
        let config = crate::kv_compress::KvCompressConfig {
            head_dim: self.config.head_dim,
            kv_heads: self.config.kv_num_heads,
            num_layers: self.config.num_layers,
            enabled: true,
        };
        self.kv_compressor = Some(crate::kv_compress::KvCompressor::new(config));
        log::info!(
            "TurboQuant KV compression enabled: head_dim={}, kv_heads={}, layers={}",
            self.config.head_dim, self.config.kv_num_heads, self.config.num_layers
        );
    }

    /// Reset KV cache
    pub fn reset_kv_cache(&mut self) {
        self.past_seq_len = 0;
        for cache in &mut self.kv_cache {
            cache.key = None;
            cache.value = None;
        }
        if let Some(ref mut compressor) = self.kv_compressor {
            compressor.reset();
        }
    }

    /// Run one forward pass: token_ids -> logits as Vec<f32>
    pub fn forward(&mut self, token_ids: &[u32]) -> Vec<f32> {
        let p = self.pipelines.clone();
        p.begin_frame();

        let seq_len = token_ids.len();
        let pos_offset = self.past_seq_len;
        let total_seq = pos_offset + seq_len;
        let hidden_size = self.config.hidden_size as u32;
        let head_dim = self.config.head_dim as u32;
        let half_dim = head_dim / 2;
        let num_heads = self.config.num_heads as u32;
        let kv_num_heads = self.config.kv_num_heads as u32;
        let block_size = self.config.block_size as u32;
        let quant_fmt = self.quant_format;

        let mut enc = p.device.create_command_encoder(&Default::default());

        let ids_f32: Vec<f32> = token_ids.iter().map(|&id| id as f32).collect();
        let ids_buf = p.upload_f32(&ids_f32);

        let half = half_dim as usize;
        let start = pos_offset * half;
        let end = start + seq_len * half;
        let cos_buf = p.upload_f32(&self.cos_cache[start..end]);
        let sin_buf = p.upload_f32(&self.sin_cache[start..end]);

        let num_layers = self.layers.len();

        if seq_len == 1 {
            // ================================================================
            // DECODE PATH — ONE compute pass for ALL layers
            // ================================================================
            struct DispatchCmd<'a> {
                shader: &'a ComputeShader,
                bg: wgpu::BindGroup,
                wg: (u32, u32, u32),
            }

            let mut all_dispatches: Vec<DispatchCmd> = Vec::with_capacity(num_layers * 21 + 5);

            let (embed_out, embed_bg, embed_wg) =
                dispatch::embed_prepare(&p, &self.embed_table, &ids_buf, hidden_size, 1);
            all_dispatches.push(DispatchCmd {
                shader: &p.embed,
                bg: embed_bg,
                wg: embed_wg,
            });
            let mut hidden = embed_out;

            for i in 0..num_layers {
                let lp = &self.layers[i].params;

                // Attention input norm + QKV projections
                // Use fused norm+q4 when available (saves 1 dispatch per layer)
                let (mut q_buf, mut k_buf, mut v_buf);
                if let (Some(fq), Some(fk), Some(fv)) =
                    (&lp.fused_q_params, &lp.fused_k_params, &lp.fused_v_params)
                {
                    let (q, q_bg, q_wg) = dispatch::fused_norm_q4_prepare_precomputed(
                        &p, &hidden, &self.layers[i].input_norm_weight,
                        &self.layers[i].q_proj_packed, &self.layers[i].q_proj_scales,
                        fq, self.layers[i].q_n, lp.fused_q_wg,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.fused_norm_q4, bg: q_bg, wg: q_wg });
                    let (k, k_bg, k_wg) = dispatch::fused_norm_q4_prepare_precomputed(
                        &p, &hidden, &self.layers[i].input_norm_weight,
                        &self.layers[i].k_proj_packed, &self.layers[i].k_proj_scales,
                        fk, self.layers[i].k_n, lp.fused_k_wg,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.fused_norm_q4, bg: k_bg, wg: k_wg });
                    let (v, v_bg, v_wg) = dispatch::fused_norm_q4_prepare_precomputed(
                        &p, &hidden, &self.layers[i].input_norm_weight,
                        &self.layers[i].v_proj_packed, &self.layers[i].v_proj_scales,
                        fv, self.layers[i].v_n, lp.fused_v_wg,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.fused_norm_q4, bg: v_bg, wg: v_wg });
                    q_buf = q; k_buf = k; v_buf = v;
                } else {
                    let (normed, norm_bg, norm_wg) = dispatch::rms_norm_prepare_precomputed(
                        &p, &hidden, &self.layers[i].input_norm_weight,
                        &lp.input_norm_params, 1, hidden_size, lp.input_norm_wg,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.rms_norm, bg: norm_bg, wg: norm_wg });
                    let (q, q_bg, q_wg, q_shader) = dispatch::prepare_matmul_for_quant(
                        &p, &normed, &self.layers[i].q_proj_packed, &self.layers[i].q_proj_scales,
                        &lp.q_matmul_params, self.layers[i].q_n, lp.q_matmul_wg,
                        quant_fmt_to_dispatch(quant_fmt),
                    );
                    all_dispatches.push(DispatchCmd { shader: q_shader, bg: q_bg, wg: q_wg });
                    let (k, k_bg, k_wg, k_shader) = dispatch::prepare_matmul_for_quant(
                        &p, &normed, &self.layers[i].k_proj_packed, &self.layers[i].k_proj_scales,
                        &lp.k_matmul_params, self.layers[i].k_n, lp.k_matmul_wg,
                        quant_fmt_to_dispatch(quant_fmt),
                    );
                    all_dispatches.push(DispatchCmd { shader: k_shader, bg: k_bg, wg: k_wg });
                    let (v, v_bg, v_wg, v_shader) = dispatch::prepare_matmul_for_quant(
                        &p, &normed, &self.layers[i].v_proj_packed, &self.layers[i].v_proj_scales,
                        &lp.v_matmul_params, self.layers[i].v_n, lp.v_matmul_wg,
                        quant_fmt_to_dispatch(quant_fmt),
                    );
                    all_dispatches.push(DispatchCmd { shader: v_shader, bg: v_bg, wg: v_wg });
                    q_buf = q; k_buf = k; v_buf = v;
                };

                // Add attention biases (Qwen2-style)
                if let Some(ref bias) = self.layers[i].q_proj_bias {
                    let (qb, qb_bg, qb_wg) = dispatch::add_prepare(&p, &q_buf, bias, self.layers[i].q_n);
                    all_dispatches.push(DispatchCmd { shader: &p.add, bg: qb_bg, wg: qb_wg });
                    q_buf = qb;
                }
                if let Some(ref bias) = self.layers[i].k_proj_bias {
                    let (kb, kb_bg, kb_wg) = dispatch::add_prepare(&p, &k_buf, bias, self.layers[i].k_n);
                    all_dispatches.push(DispatchCmd { shader: &p.add, bg: kb_bg, wg: kb_wg });
                    k_buf = kb;
                }
                if let Some(ref bias) = self.layers[i].v_proj_bias {
                    let (vb, vb_bg, vb_wg) = dispatch::add_prepare(&p, &v_buf, bias, self.layers[i].v_n);
                    all_dispatches.push(DispatchCmd { shader: &p.add, bg: vb_bg, wg: vb_wg });
                    v_buf = vb;
                }

                // Q/K norm (optional)
                let q_for_rope;
                let k_for_rope;
                if self.config.has_qk_norm {
                    let (qn, qn_bg, qn_wg) = dispatch::rms_norm_prepare_precomputed(
                        &p, &q_buf, self.layers[i].q_norm_weight.as_ref().unwrap(),
                        lp.q_norm_params.as_ref().unwrap(), num_heads, head_dim, lp.q_norm_wg,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.rms_norm, bg: qn_bg, wg: qn_wg });
                    let (kn, kn_bg, kn_wg) = dispatch::rms_norm_prepare_precomputed(
                        &p, &k_buf, self.layers[i].k_norm_weight.as_ref().unwrap(),
                        lp.k_norm_params.as_ref().unwrap(), kv_num_heads, head_dim, lp.k_norm_wg,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.rms_norm, bg: kn_bg, wg: kn_wg });
                    q_for_rope = qn;
                    k_for_rope = kn;
                } else {
                    q_for_rope = q_buf;
                    k_for_rope = k_buf;
                }

                // RoPE
                let (q_roped, qr_bg, qr_wg) = dispatch::rope_prepare_precomputed(
                    &p, &q_for_rope, &cos_buf, &sin_buf,
                    &lp.q_rope_params, self.layers[i].q_n, lp.q_rope_wg,
                );
                all_dispatches.push(DispatchCmd { shader: &p.rope, bg: qr_bg, wg: qr_wg });

                let (k_roped, kr_bg, kr_wg) = dispatch::rope_prepare_precomputed(
                    &p, &k_for_rope, &cos_buf, &sin_buf,
                    &lp.k_rope_params, self.layers[i].k_n, lp.k_rope_wg,
                );
                all_dispatches.push(DispatchCmd { shader: &p.rope, bg: kr_bg, wg: kr_wg });

                // KV cache
                let past_seq_u32 = self.past_seq_len as u32;
                let empty_buf = p.alloc(4);
                let past_k_ref = self.kv_cache[i].key.as_ref().unwrap_or(&empty_buf);
                let past_v_ref = self.kv_cache[i].value.as_ref().unwrap_or(&empty_buf);

                let (full_k, kv_k_bg, kv_k_wg) = dispatch::kv_append_prepare_permanent(
                    &p, past_k_ref, &k_roped, kv_num_heads, head_dim, past_seq_u32, seq_len as u32,
                );
                all_dispatches.push(DispatchCmd { shader: &p.kv_append, bg: kv_k_bg, wg: kv_k_wg });

                let (full_v, kv_v_bg, kv_v_wg) = dispatch::kv_append_prepare_permanent(
                    &p, past_v_ref, &v_buf, kv_num_heads, head_dim, past_seq_u32, seq_len as u32,
                );
                all_dispatches.push(DispatchCmd { shader: &p.kv_append, bg: kv_v_bg, wg: kv_v_wg });

                self.kv_cache[i].key = Some(full_k.clone());
                self.kv_cache[i].value = Some(full_v.clone());

                // GQA head expansion
                let (attn_k, attn_v) = if num_heads != kv_num_heads {
                    let (ek, ekb, ekw) = dispatch::kv_expand_prepare(
                        &p, &full_k, kv_num_heads, num_heads, head_dim, total_seq as u32,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.kv_expand, bg: ekb, wg: ekw });
                    let (ev, evb, evw) = dispatch::kv_expand_prepare(
                        &p, &full_v, kv_num_heads, num_heads, head_dim, total_seq as u32,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.kv_expand, bg: evb, wg: evw });
                    (ek, ev)
                } else {
                    (full_k, full_v)
                };

                let scale = 1.0 / (head_dim as f32).sqrt();

                let (attn_out, attn_bg, attn_wg) = dispatch::attention_decode_prepare(
                    &p, &q_roped, &attn_k, &attn_v,
                    num_heads, head_dim, total_seq as u32, scale,
                );
                all_dispatches.push(DispatchCmd { shader: &p.attention, bg: attn_bg, wg: attn_wg });

                let (attn_proj, oproj_bg, oproj_wg, oproj_shader) = dispatch::prepare_matmul_for_quant(
                    &p, &attn_out, &self.layers[i].o_proj_packed, &self.layers[i].o_proj_scales,
                    &lp.o_matmul_params, self.layers[i].o_n, lp.o_matmul_wg,
                    quant_fmt_to_dispatch(quant_fmt),
                );
                all_dispatches.push(DispatchCmd { shader: oproj_shader, bg: oproj_bg, wg: oproj_wg });

                // Post-attention: fused skip+norm merges add+norm into 1 dispatch
                let (residual1, gate, up);
                {
                    let (normed2, res1, fsn_bg, fsn_wg) = dispatch::fused_skip_norm_prepare_precomputed(
                        &p, &attn_proj, &hidden, &self.layers[i].post_norm_weight,
                        &lp.post_norm_params, 1, hidden_size, lp.post_norm_wg,
                    );
                    all_dispatches.push(DispatchCmd { shader: &p.fused_skip_norm, bg: fsn_bg, wg: fsn_wg });
                    residual1 = res1;
                    let (g, gate_bg, gate_wg, gate_shader) = dispatch::prepare_matmul_for_quant(
                        &p, &normed2, &self.layers[i].gate_proj_packed, &self.layers[i].gate_proj_scales,
                        &lp.gate_matmul_params, self.layers[i].gate_n, lp.gate_matmul_wg,
                        quant_fmt_to_dispatch(quant_fmt),
                    );
                    all_dispatches.push(DispatchCmd { shader: gate_shader, bg: gate_bg, wg: gate_wg });
                    let (u, up_bg, up_wg, up_shader) = dispatch::prepare_matmul_for_quant(
                        &p, &normed2, &self.layers[i].up_proj_packed, &self.layers[i].up_proj_scales,
                        &lp.up_matmul_params, self.layers[i].up_n, lp.up_matmul_wg,
                        quant_fmt_to_dispatch(quant_fmt),
                    );
                    all_dispatches.push(DispatchCmd { shader: up_shader, bg: up_bg, wg: up_wg });
                    gate = g; up = u;
                };

                let (ffn, silu_bg, silu_wg) = dispatch::silu_mul_prepare(&p, &gate, &up, self.layers[i].gate_n);
                all_dispatches.push(DispatchCmd { shader: &p.silu_mul, bg: silu_bg, wg: silu_wg });

                let (ffn_out, down_bg, down_wg, down_shader) = dispatch::prepare_matmul_for_quant(
                    &p, &ffn, &self.layers[i].down_proj_packed, &self.layers[i].down_proj_scales,
                    &lp.down_matmul_params, self.layers[i].down_n, lp.down_matmul_wg,
                    quant_fmt_to_dispatch(quant_fmt),
                );
                all_dispatches.push(DispatchCmd { shader: down_shader, bg: down_bg, wg: down_wg });

                let (new_hidden, res2_bg, res2_wg) = dispatch::add_prepare(&p, &residual1, &ffn_out, hidden_size);
                all_dispatches.push(DispatchCmd { shader: &p.add, bg: res2_bg, wg: res2_wg });

                hidden = new_hidden;
            }

            // Final norm + LM head
            let vocab = self.config.vocab_size as u32;
            let mp = &self.model_params;

            let (final_normed, fn_bg, fn_wg) = dispatch::rms_norm_prepare_precomputed(
                &p, &hidden, &self.final_norm_weight,
                &mp.final_norm_params, 1, hidden_size, mp.final_norm_wg,
            );
            all_dispatches.push(DispatchCmd { shader: &p.rms_norm, bg: fn_bg, wg: fn_wg });

            let lm_head_buf = self.lm_head.as_ref().unwrap_or(&self.embed_table);
            let (logits_buf, lm_bg, lm_wg) = dispatch::f32_matmul_prepare_precomputed(
                &p, &final_normed, lm_head_buf,
                &mp.f32_matmul_params, vocab, mp.f32_matmul_wg,
            );
            all_dispatches.push(DispatchCmd { shader: &p.f32_matmul, bg: lm_bg, wg: lm_wg });

            if self.greedy_mode {
                let (argmax_buf, argmax_bg, argmax_wg) =
                    dispatch::argmax_gpu_prepare_precomputed(&p, &logits_buf, &mp.argmax_params, mp.argmax_wg);
                all_dispatches.push(DispatchCmd { shader: &p.argmax, bg: argmax_bg, wg: argmax_wg });

                {
                    let mut pass = enc.begin_compute_pass(&Default::default());
                    for cmd in &all_dispatches {
                        p.dispatch_in_pass(&mut pass, cmd.shader, &cmd.bg, cmd.wg);
                    }
                }
                p.queue.submit(std::iter::once(enc.finish()));
                self.past_seq_len = total_seq;

                let bytes = p.read_f32(&argmax_buf, 1);
                let token_id = bytes[0].to_bits();

                let mut logits = vec![0.0f32; vocab as usize];
                logits[token_id as usize] = 1.0;
                return logits;
            }

            // Non-greedy
            {
                let mut pass = enc.begin_compute_pass(&Default::default());
                for cmd in &all_dispatches {
                    p.dispatch_in_pass(&mut pass, cmd.shader, &cmd.bg, cmd.wg);
                }
            }
            p.queue.submit(std::iter::once(enc.finish()));
            self.past_seq_len = total_seq;

            return p.read_f32(&logits_buf, vocab as usize);
        }

        // ================================================================
        // PREFILL PATH — legacy one-pass-per-op
        // ================================================================
        let mut hidden = dispatch::embed(&p, &mut enc, &self.embed_table, &ids_buf, hidden_size, seq_len as u32);

        for i in 0..num_layers {
            let normed = dispatch::rms_norm(
                &p, &mut enc, &hidden, &self.layers[i].input_norm_weight,
                seq_len as u32, hidden_size, 1e-6,
            );

            let layer_q_n = self.layers[i].q_n;
            let layer_k_n = self.layers[i].k_n;
            let layer_v_n = self.layers[i].v_n;
            let q_size = (layer_q_n as u64) * (seq_len as u64) * 4;
            let k_size = (layer_k_n as u64) * (seq_len as u64) * 4;
            let v_size = (layer_v_n as u64) * (seq_len as u64) * 4;
            let q_combined = p.alloc(q_size);
            let k_combined = p.alloc(k_size);
            let v_combined = p.alloc(v_size);

            for s in 0..seq_len {
                let normed_slice = slice_buffer(&p, &mut enc, &normed, s * self.config.hidden_size, self.config.hidden_size);
                let mut q_pos = dispatch_matmul_enc(&p, &mut enc, quant_fmt, &normed_slice, &self.layers[i].q_proj_packed, &self.layers[i].q_proj_scales, layer_q_n, hidden_size, block_size);
                let mut k_pos = dispatch_matmul_enc(&p, &mut enc, quant_fmt, &normed_slice, &self.layers[i].k_proj_packed, &self.layers[i].k_proj_scales, layer_k_n, hidden_size, block_size);
                let mut v_pos = dispatch_matmul_enc(&p, &mut enc, quant_fmt, &normed_slice, &self.layers[i].v_proj_packed, &self.layers[i].v_proj_scales, layer_v_n, hidden_size, block_size);
                // Add attention biases
                if let Some(ref bias) = self.layers[i].q_proj_bias {
                    q_pos = dispatch::add(&p, &mut enc, &q_pos, bias, layer_q_n);
                }
                if let Some(ref bias) = self.layers[i].k_proj_bias {
                    k_pos = dispatch::add(&p, &mut enc, &k_pos, bias, layer_k_n);
                }
                if let Some(ref bias) = self.layers[i].v_proj_bias {
                    v_pos = dispatch::add(&p, &mut enc, &v_pos, bias, layer_v_n);
                }
                copy_into(&mut enc, &q_pos, &q_combined, s * layer_q_n as usize, layer_q_n as usize);
                copy_into(&mut enc, &k_pos, &k_combined, s * layer_k_n as usize, layer_k_n as usize);
                copy_into(&mut enc, &v_pos, &v_combined, s * layer_v_n as usize, layer_v_n as usize);
            }

            let (q_buf, k_buf, v_buf) = (q_combined, k_combined, v_combined);

            let q_positions = seq_len as u32 * num_heads;
            let k_positions = seq_len as u32 * kv_num_heads;

            let (q_for_rope_pf, k_for_rope_pf) = if self.config.has_qk_norm {
                let qn = dispatch::rms_norm(&p, &mut enc, &q_buf, self.layers[i].q_norm_weight.as_ref().unwrap(), q_positions, head_dim, 1e-6);
                let kn = dispatch::rms_norm(&p, &mut enc, &k_buf, self.layers[i].k_norm_weight.as_ref().unwrap(), k_positions, head_dim, 1e-6);
                (qn, kn)
            } else {
                (q_buf, k_buf)
            };

            let q_elements = seq_len as u32 * self.layers[i].q_n;
            let k_elements = seq_len as u32 * self.layers[i].k_n;

            let q_roped = dispatch::rope(&p, &mut enc, &q_for_rope_pf, &cos_buf, &sin_buf, q_elements, head_dim, seq_len as u32);
            let k_roped = dispatch::rope(&p, &mut enc, &k_for_rope_pf, &cos_buf, &sin_buf, k_elements, head_dim, seq_len as u32);

            // KV cache (prefill path)
            let kv_heads_usize = self.config.kv_num_heads;
            let head_dim_usize = self.config.head_dim;
            let total_kv_elements = total_seq * kv_heads_usize * head_dim_usize;
            let full_k = p.alloc((total_kv_elements as u64) * 4);
            let full_v = p.alloc((total_kv_elements as u64) * 4);

            {
                let past_seq = self.past_seq_len;
                for h in 0..kv_heads_usize {
                    let new_head_offset = (total_seq * head_dim_usize * h) as u64 * 4;
                    if past_seq > 0 {
                        if let Some(ref past_k) = self.kv_cache[i].key {
                            let past_head_src = (h * past_seq * head_dim_usize) as u64 * 4;
                            let past_bytes = (past_seq * head_dim_usize) as u64 * 4;
                            if past_head_src + past_bytes <= past_k.size() {
                                enc.copy_buffer_to_buffer(past_k, past_head_src, &full_k, new_head_offset, past_bytes);
                            }
                        }
                        if let Some(ref past_v) = self.kv_cache[i].value {
                            let past_head_src = (h * past_seq * head_dim_usize) as u64 * 4;
                            let past_bytes = (past_seq * head_dim_usize) as u64 * 4;
                            if past_head_src + past_bytes <= past_v.size() {
                                enc.copy_buffer_to_buffer(past_v, past_head_src, &full_v, new_head_offset, past_bytes);
                            }
                        }
                    }
                    for s in 0..seq_len {
                        let new_src = ((s * kv_heads_usize + h) * head_dim_usize) as u64 * 4;
                        let new_dst = new_head_offset + ((past_seq + s) * head_dim_usize) as u64 * 4;
                        let dim_bytes = (head_dim_usize as u64) * 4;
                        let k_buf_size = (seq_len * kv_heads_usize * head_dim_usize) as u64 * 4;
                        if new_src + dim_bytes <= k_buf_size {
                            enc.copy_buffer_to_buffer(&k_roped, new_src, &full_k, new_dst, dim_bytes);
                            enc.copy_buffer_to_buffer(&v_buf, new_src, &full_v, new_dst, dim_bytes);
                        }
                    }
                }
            }

            let cache_k = p.alloc_permanent((total_kv_elements as u64) * 4);
            let cache_v = p.alloc_permanent((total_kv_elements as u64) * 4);
            enc.copy_buffer_to_buffer(&full_k, 0, &cache_k, 0, (total_kv_elements as u64) * 4);
            enc.copy_buffer_to_buffer(&full_v, 0, &cache_v, 0, (total_kv_elements as u64) * 4);
            self.kv_cache[i].key = Some(cache_k);
            self.kv_cache[i].value = Some(cache_v);

            // Attention
            let (attn_k, attn_v) = if num_heads != kv_num_heads {
                let expanded_k = expand_kv_heads(&p, &mut enc, &full_k, kv_num_heads, num_heads, head_dim, total_seq as u32);
                let expanded_v = expand_kv_heads(&p, &mut enc, &full_v, kv_num_heads, num_heads, head_dim, total_seq as u32);
                (expanded_k, expanded_v)
            } else {
                (full_k, full_v)
            };

            let scale = 1.0 / (head_dim as f32).sqrt();
            let last_q_offset = (seq_len - 1) * (self.layers[i].q_n as usize);
            let last_q = slice_buffer(&p, &mut enc, &q_roped, last_q_offset, self.layers[i].q_n as usize);
            let attn_out = dispatch::attention_decode(&p, &mut enc, &last_q, &attn_k, &attn_v, num_heads, head_dim, total_seq as u32, scale);

            let attn_proj = dispatch_matmul_enc(&p, &mut enc, quant_fmt, &attn_out, &self.layers[i].o_proj_packed, &self.layers[i].o_proj_scales, self.layers[i].o_n, self.layers[i].q_n, block_size);

            let residual_hidden = slice_buffer(&p, &mut enc, &hidden, (seq_len - 1) * self.config.hidden_size, self.config.hidden_size);
            hidden = dispatch::add(&p, &mut enc, &residual_hidden, &attn_proj, hidden_size);

            let normed2 = dispatch::rms_norm(&p, &mut enc, &hidden, &self.layers[i].post_norm_weight, 1, hidden_size, 1e-6);

            let gate = dispatch_matmul_enc(&p, &mut enc, quant_fmt, &normed2, &self.layers[i].gate_proj_packed, &self.layers[i].gate_proj_scales, self.layers[i].gate_n, hidden_size, block_size);
            let up = dispatch_matmul_enc(&p, &mut enc, quant_fmt, &normed2, &self.layers[i].up_proj_packed, &self.layers[i].up_proj_scales, self.layers[i].up_n, hidden_size, block_size);
            let ffn = dispatch::silu_mul(&p, &mut enc, &gate, &up, self.layers[i].gate_n);
            let ffn_out = dispatch_matmul_enc(&p, &mut enc, quant_fmt, &ffn, &self.layers[i].down_proj_packed, &self.layers[i].down_proj_scales, self.layers[i].down_n, self.layers[i].gate_n, block_size);

            hidden = dispatch::add(&p, &mut enc, &hidden, &ffn_out, hidden_size);
        }

        // Final norm + LM head (PREFILL PATH)
        let vocab = self.config.vocab_size as u32;
        let normed = dispatch::rms_norm(&p, &mut enc, &hidden, &self.final_norm_weight, 1, hidden_size, 1e-6);
        let lm_head_buf_pf = self.lm_head.as_ref().unwrap_or(&self.embed_table);
        let logits_buf = dispatch::f32_matmul(&p, &mut enc, &normed, lm_head_buf_pf, vocab, hidden_size);

        if self.greedy_mode {
            let argmax_buf = dispatch::argmax_gpu(&p, &mut enc, &logits_buf, vocab);
            p.queue.submit(std::iter::once(enc.finish()));
            self.past_seq_len = total_seq;

            let bytes = p.read_f32(&argmax_buf, 1);
            let token_id = bytes[0].to_bits();
            let mut logits = vec![0.0f32; vocab as usize];
            logits[token_id as usize] = 1.0;
            return logits;
        }

        p.queue.submit(std::iter::once(enc.finish()));
        self.past_seq_len = total_seq;

        p.read_f32(&logits_buf, vocab as usize)
    }
}

// --- Free helper functions ---

/// Dispatch a matmul using command encoder (prefill path) for any quant format
fn dispatch_matmul_enc(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    quant_fmt: QuantFormat,
    activation: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    scales: &wgpu::Buffer,
    n: u32,
    k: u32,
    block_size: u32,
) -> wgpu::Buffer {
    match quant_fmt {
        QuantFormat::F32 => dispatch::f32_matmul(p, enc, activation, weight, n, k),
        QuantFormat::F16 => dispatch::f16_matmul(p, enc, activation, weight, n, k),
        QuantFormat::Q8 => dispatch::q8_matmul(p, enc, activation, weight, scales, n, k, block_size),
        QuantFormat::Q4 => dispatch::q4_matmul(p, enc, activation, weight, scales, n, k, block_size),
        QuantFormat::Ternary => dispatch::ternary_matmul(p, enc, activation, weight, scales, n, k),
    }
}

fn slice_buffer(p: &Pipelines, enc: &mut wgpu::CommandEncoder, src: &wgpu::Buffer, offset_elements: usize, count_elements: usize) -> wgpu::Buffer {
    let dst = p.alloc((count_elements as u64) * 4);
    enc.copy_buffer_to_buffer(src, (offset_elements as u64) * 4, &dst, 0, (count_elements as u64) * 4);
    dst
}

fn copy_into(enc: &mut wgpu::CommandEncoder, src: &wgpu::Buffer, dst: &wgpu::Buffer, offset_elements: usize, count_elements: usize) {
    enc.copy_buffer_to_buffer(src, 0, dst, (offset_elements as u64) * 4, (count_elements as u64) * 4);
}

fn expand_kv_heads(p: &Pipelines, enc: &mut wgpu::CommandEncoder, kv: &wgpu::Buffer, kv_heads: u32, num_heads: u32, head_dim: u32, total_seq: u32) -> wgpu::Buffer {
    let repeats = num_heads / kv_heads;
    let output_elements = num_heads * total_seq * head_dim;
    let output = p.alloc((output_elements as u64) * 4);
    let kv_head_size = (total_seq * head_dim) as u64 * 4;

    for kv_h in 0..kv_heads {
        for r in 0..repeats {
            let dst_head = kv_h * repeats + r;
            enc.copy_buffer_to_buffer(kv, (kv_h as u64) * kv_head_size, &output, (dst_head as u64) * kv_head_size, kv_head_size);
        }
    }

    output
}

/// Greedy argmax on CPU
pub fn argmax(logits: &[f32]) -> u32 {
    logits.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i as u32).unwrap_or(0)
}

/// Top-p sampling on CPU
pub fn sample_top_p(logits: &[f32], temperature: f32, top_p: f32) -> u32 {
    if temperature <= 0.0 {
        return argmax(logits);
    }

    let scaled: Vec<f32> = logits.iter().map(|&v| v / temperature).collect();
    let max_val = scaled.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp_vals: Vec<f32> = scaled.iter().map(|&v| (v - max_val).exp()).collect();
    let sum: f32 = exp_vals.iter().sum();
    let probs: Vec<f32> = exp_vals.iter().map(|&v| v / sum).collect();

    let mut indexed: Vec<(usize, f32)> = probs.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let mut cumulative = 0.0;
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for (idx, prob) in &indexed {
        cumulative += prob;
        candidates.push((*idx, *prob));
        if cumulative >= top_p {
            break;
        }
    }

    let total: f32 = candidates.iter().map(|(_, p)| p).sum();

    let r = {
        use std::time::SystemTime;
        let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().subsec_nanos();
        let x = seed.wrapping_mul(1103515245).wrapping_add(12345);
        (x as f32 / u32::MAX as f32).abs()
    };

    let mut acc = 0.0;
    for (idx, prob) in &candidates {
        acc += prob / total;
        if acc >= r {
            return *idx as u32;
        }
    }

    candidates.last().map(|(i, _)| *i as u32).unwrap_or(0)
}
