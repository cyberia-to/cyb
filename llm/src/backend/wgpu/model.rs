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
    Q4K,
    Q5K,
    Q6K,
    Q3K,
    Q2K,
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
    f16_matmul_params: wgpu::Buffer,
    f16_matmul_wg: (u32, u32, u32),
    q4k_lm_params: wgpu::Buffer,
    q4k_lm_wg: (u32, u32, u32),
    q5k_lm_params: wgpu::Buffer,
    q5k_lm_wg: (u32, u32, u32),
    q6k_lm_params: wgpu::Buffer,
    q6k_lm_wg: (u32, u32, u32),
    q3k_lm_params: wgpu::Buffer,
    q3k_lm_wg: (u32, u32, u32),
    q2k_lm_params: wgpu::Buffer,
    q2k_lm_wg: (u32, u32, u32),
    argmax_params: wgpu::Buffer,
    argmax_wg: (u32, u32, u32),
}

/// Native wgpu model — holds all weights on GPU and runs forward pass
pub struct NativeModel {
    config: ModelConfig,
    pipelines: Arc<Pipelines>,
    embed_table: wgpu::Buffer,
    /// CPU-side f32 embed table for reliable embedding lookup (small models).
    /// Empty when using GPU Q4_K embed path for large vocab models.
    embed_f32: Vec<f32>,
    /// Raw Q4_K embed table on GPU for large-vocab models (gemma 262k vocab).
    /// When Some, decode/prefill use q4k_embed shader instead of CPU lookup.
    embed_q4k_gpu: Option<wgpu::Buffer>,
    /// Separate LM head weights (None = tied to embed_table)
    lm_head: Option<wgpu::Buffer>,
    lm_head_quant: QuantFormat,
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
    /// Persistent staging buffer for greedy argmax readback (avoids alloc per token)
    argmax_staging: wgpu::Buffer,
    /// Persistent decode buffers (reused every token, no allocation)
    decode_ids_buf: wgpu::Buffer,
    decode_cos_buf: wgpu::Buffer,
    decode_sin_buf: wgpu::Buffer,
    /// Persistent embedding buffer for decode path (avoids create_buffer_init)
    decode_embed_buf: wgpu::Buffer,
}

/// Convert model-level QuantFormat to dispatch::QuantFormat
fn quant_fmt_to_dispatch(qf: QuantFormat) -> dispatch::QuantFormat {
    match qf {
        QuantFormat::F32 => dispatch::QuantFormat::F32,
        QuantFormat::F16 => dispatch::QuantFormat::F16,
        QuantFormat::Q4 => dispatch::QuantFormat::Q4,
        QuantFormat::Q4K => dispatch::QuantFormat::Q4K,
        QuantFormat::Q5K => dispatch::QuantFormat::Q5K,
        QuantFormat::Q6K => dispatch::QuantFormat::Q6K,
        QuantFormat::Q3K => dispatch::QuantFormat::Q3K,
        QuantFormat::Q2K => dispatch::QuantFormat::Q2K,
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
        DType::Q4_1 => {
            // GGUF Q4_1: blocks of {half scale, half min, uint8_t qs[16]} = 20 bytes = 32 weights
            let block_size = 20;
            data.chunks_exact(block_size).flat_map(|block| {
                let scale = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
                let min = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
                let qs = &block[4..20];
                let mut vals = [0.0f32; 32];
                for j in 0..16 {
                    let byte = qs[j];
                    vals[j] = scale * (byte & 0x0F) as f32 + min;
                    vals[j + 16] = scale * (byte >> 4) as f32 + min;
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
        DType::Q4_K => {
            // Q4_K: super blocks of 256 values = 144 bytes
            // Layout: d(f16=2) + dmin(f16=2) + scales(12) + qs(128) = 144 bytes
            // Reference: llama.cpp dequantize_row_q4_K / get_scale_min_k4
            let block_size = 144;
            data.chunks_exact(block_size).flat_map(|block| {
                let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
                let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
                let scales_raw = &block[4..16]; // 12 bytes of packed 6-bit scales+mins
                let qs = &block[16..144];       // 128 bytes = 256 nibbles

                // get_scale_min_k4: unpack 6-bit scale and min for sub-block j (j=0..7, stepped by 2)
                // j < 4: sc = q[j] & 63, m = q[j+4] & 63
                // j >= 4: sc = (q[j+4] & 0xF) | ((q[j-4] >> 6) << 4)
                //         m  = (q[j+4] >>  4) | ((q[j  ] >> 6) << 4)
                let get_scale_min = |j: usize| -> (u8, u8) {
                    if j < 4 {
                        (scales_raw[j] & 63, scales_raw[j + 4] & 63)
                    } else {
                        let sc = (scales_raw[j + 4] & 0xF) | ((scales_raw[j - 4] >> 6) << 4);
                        let mn = (scales_raw[j + 4] >> 4) | ((scales_raw[j] >> 6) << 4);
                        (sc, mn)
                    }
                };

                let mut vals = [0.0f32; 256];
                let mut q_ptr = 0usize; // pointer into qs
                let mut is = 0usize;    // scale index (0..7)

                // 4 groups of 64 values each (2 sub-blocks per group)
                for grp in 0..4 {
                    let (sc1, m1) = get_scale_min(is);
                    is += 1;
                    let d1 = d * sc1 as f32;
                    let m1_val = dmin * m1 as f32;

                    let (sc2, m2) = get_scale_min(is);
                    is += 1;
                    let d2 = d * sc2 as f32;
                    let m2_val = dmin * m2 as f32;

                    let base = grp * 64;
                    // First 32: low nibbles with scale 1
                    for l in 0..32 {
                        vals[base + l] = d1 * (qs[q_ptr + l] & 0xF) as f32 - m1_val;
                    }
                    // Next 32: high nibbles with scale 2
                    for l in 0..32 {
                        vals[base + 32 + l] = d2 * (qs[q_ptr + l] >> 4) as f32 - m2_val;
                    }
                    q_ptr += 32;
                }

                vals
            }).collect()
        }
        DType::Q6_K => {
            // Q6_K: super blocks of 256 values = 210 bytes
            // Layout: ql(128) + qh(64) + scales(16) + d(f16=2) = 210 bytes
            // Reference: llama.cpp dequantize_row_q6_K
            let block_size = 210;
            data.chunks_exact(block_size).flat_map(|block| {
                let d = half::f16::from_le_bytes([block[208], block[209]]).to_f32();
                let ql = &block[0..128];
                let qh = &block[128..192];
                let scales = &block[192..208]; // 16 x int8 scales for 16 sub-blocks of 16

                let mut vals = [0.0f32; 256];
                for j in 0..256 {
                    let ql_byte = ql[j / 2];
                    let ql_nib = if j % 2 == 0 { ql_byte & 0x0F } else { ql_byte >> 4 };
                    let qh_bit = (qh[j / 4] >> ((j % 4) * 2)) & 0x03;
                    let q = ((qh_bit as u8) << 4) | ql_nib;
                    let sc = scales[j / 16] as i8;
                    vals[j] = d * sc as f32 * (q as f32 - 32.0);
                }
                vals
            }).collect()
        }
        DType::Q5_K => {
            // Q5_K: super blocks of 256 values = 176 bytes
            // Layout: d(f16=2) + dmin(f16=2) + scales(12) + qh(32) + qs(128) = 176 bytes
            let block_size = 176;
            data.chunks_exact(block_size).flat_map(|block| {
                let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
                let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
                let scales_raw = &block[4..16];
                let qh = &block[16..48];
                let qs = &block[48..176];

                let get_scale_min = |j: usize| -> (u8, u8) {
                    if j < 4 {
                        (scales_raw[j] & 63, scales_raw[j + 4] & 63)
                    } else {
                        let sc = (scales_raw[j + 4] & 0xF) | ((scales_raw[j - 4] >> 6) << 4);
                        let mn = (scales_raw[j + 4] >> 4) | ((scales_raw[j] >> 6) << 4);
                        (sc, mn)
                    }
                };

                let mut vals = [0.0f32; 256];
                let mut q_ptr = 0usize;
                let mut is = 0usize;

                for grp in 0..4 {
                    let (sc1, m1) = get_scale_min(is);
                    is += 1;
                    let d1 = d * sc1 as f32;
                    let m1_val = dmin * m1 as f32;

                    let (sc2, m2) = get_scale_min(is);
                    is += 1;
                    let d2 = d * sc2 as f32;
                    let m2_val = dmin * m2 as f32;

                    let base = grp * 64;
                    for l in 0..32 {
                        let lo_nib = qs[q_ptr + l] & 0xF;
                        let lo_idx = grp * 64 + l;
                        let lo_qh = (qh[lo_idx / 8] >> (lo_idx % 8)) & 1;
                        let lo_q = lo_nib | (lo_qh << 4);
                        vals[base + l] = d1 * lo_q as f32 - m1_val;
                    }
                    for l in 0..32 {
                        let hi_nib = qs[q_ptr + l] >> 4;
                        let hi_idx = grp * 64 + 32 + l;
                        let hi_qh = (qh[hi_idx / 8] >> (hi_idx % 8)) & 1;
                        let hi_q = hi_nib | (hi_qh << 4);
                        vals[base + 32 + l] = d2 * hi_q as f32 - m2_val;
                    }
                    q_ptr += 32;
                }

                vals
            }).collect()
        }
        DType::Q3_K => {
            // Q3_K: super blocks of 256 values = 110 bytes
            // Layout: hmask(32) + qs(64) + scales(12) + d(f16=2) = 110 bytes
            let block_size = 110;
            data.chunks_exact(block_size).flat_map(|block| {
                let hmask = &block[0..32];
                let qs = &block[32..96];
                let scales_raw = &block[96..108];
                let d = half::f16::from_le_bytes([block[108], block[109]]).to_f32();

                // Unpack 16 6-bit scales from 12 bytes
                let get_scale = |j: usize| -> i32 {
                    let us = if j < 4 {
                        (scales_raw[j] & 0xF) as u32
                            | ((((scales_raw[8 + (j >> 1)] >> (4 * (j & 1))) & 3) as u32) << 4)
                    } else if j < 8 {
                        let jj = j - 4;
                        (scales_raw[4 + jj] & 0xF) as u32
                            | ((((scales_raw[10 + (jj >> 1)] >> (4 * (jj & 1))) & 3) as u32) << 4)
                    } else if j < 12 {
                        let jj = j - 8;
                        (scales_raw[jj] >> 4) as u32
                            | ((((scales_raw[8 + (jj >> 1)] >> (4 * (jj & 1) + 2)) & 3) as u32) << 4)
                    } else {
                        let jj = j - 12;
                        (scales_raw[4 + jj] >> 4) as u32
                            | ((((scales_raw[10 + (jj >> 1)] >> (4 * (jj & 1) + 2)) & 3) as u32) << 4)
                    };
                    us as i32 - 32
                };

                let mut vals = [0.0f32; 256];
                for sb in 0..16 {
                    let sc = get_scale(sb) as f32;
                    for l in 0..16 {
                        let j = sb * 16 + l;
                        let ql = (qs[j / 4] >> ((j % 4) * 2)) & 3;
                        let hm = (hmask[j / 8] >> (j % 8)) & 1;
                        let q3 = (ql | (hm << 2)) as i32 - 4;
                        vals[j] = d * sc * q3 as f32;
                    }
                }
                vals
            }).collect()
        }
        DType::Q2_K => {
            // Q2_K: super blocks of 256 values = 84 bytes
            // Layout: scales(16) + qs(64) + d(f16=2) + dmin(f16=2) = 84 bytes
            let block_size = 84;
            data.chunks_exact(block_size).flat_map(|block| {
                let scales = &block[0..16];
                let qs = &block[16..80];
                let d = half::f16::from_le_bytes([block[80], block[81]]).to_f32();
                let dmin = half::f16::from_le_bytes([block[82], block[83]]).to_f32();

                let mut vals = [0.0f32; 256];
                for sb in 0..16 {
                    let sc = (scales[sb] & 0xF) as f32;
                    let m = (scales[sb] >> 4) as f32;
                    let ds = d * sc;
                    let dm = dmin * m;
                    for l in 0..16 {
                        let j = sb * 16 + l;
                        let q2 = (qs[j / 4] >> ((j % 4) * 2)) & 3;
                        vals[j] = ds * q2 as f32 - dm;
                    }
                }
                vals
            }).collect()
        }
        _ => {
            log::error!("Unsupported dtype {:?} — cannot dequantize. Data will be interpreted as f32 (likely wrong).", dtype);
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

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q4KMatmulParams {
    n: u32,
    k: u32,
    blocks_per_row: u32,
    _pad: u32,
}

fn precompute_q4k_matmul(
    p: &Pipelines,
    n: u32,
    k: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let blocks_per_row = k / 256;
    let params = Q4KMatmulParams {
        n,
        k,
        blocks_per_row,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (buf, (x, y, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q6KMatmulParams {
    n: u32,
    k: u32,
    blocks_per_row: u32,
    _pad: u32,
}

fn precompute_q6k_matmul(
    p: &Pipelines,
    n: u32,
    k: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let blocks_per_row = k / 256;
    let params = Q6KMatmulParams {
        n,
        k,
        blocks_per_row,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (buf, (x, y, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q5KMatmulParams {
    n: u32,
    k: u32,
    blocks_per_row: u32,
    _pad: u32,
}

fn precompute_q5k_matmul(
    p: &Pipelines,
    n: u32,
    k: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let blocks_per_row = k / 256;
    let params = Q5KMatmulParams {
        n,
        k,
        blocks_per_row,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (buf, (x, y, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q3KMatmulParams {
    n: u32,
    k: u32,
    blocks_per_row: u32,
    _pad: u32,
}

fn precompute_q3k_matmul(
    p: &Pipelines,
    n: u32,
    k: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let blocks_per_row = k / 256;
    let params = Q3KMatmulParams {
        n,
        k,
        blocks_per_row,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (buf, (x, y, 1))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q2KMatmulParams {
    n: u32,
    k: u32,
    blocks_per_row: u32,
    _pad: u32,
}

fn precompute_q2k_matmul(
    p: &Pipelines,
    n: u32,
    k: u32,
) -> (wgpu::Buffer, (u32, u32, u32)) {
    let blocks_per_row = k / 256;
    let params = Q2KMatmulParams {
        n,
        k,
        blocks_per_row,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (buf, (x, y, 1))
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
    /// Load from .model file path (convenience)
    pub fn load(path: &Path, pipelines: Arc<Pipelines>) -> Result<(Self, String), String> {
        use crate::cyb_format::LoadedModel;
        let lm = LoadedModel::load(path).map_err(|e| format!("Cannot read .model: {e}"))?;
        let vocab = lm.vocab.clone();
        let model = Self::from_loaded(lm, pipelines)?;
        Ok((model, vocab))
    }

    /// Build from pre-parsed LoadedModel — shared loader, any backend can use
    pub fn from_loaded(lm: crate::cyb_format::LoadedModel, pipelines: Arc<Pipelines>) -> Result<Self, String> {
        let config_json = lm.config_json();
        let weights = lm.weights;
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

        // Pre-allocate decode embedding buffer EARLY — before heavy weight loading
        // which can exhaust wgpu's internal staging pools.
        let decode_embed_buf = pipelines.upload_f32(&vec![0.0f32; hidden_size]);

        // Detect large Q4_K embed tables that would exceed GPU buffer limits if dequanted to f32.
        // Threshold: vocab * hidden * 4 bytes > 2GB → keep raw Q4_K on GPU, dequant per-token.
        let embed_w = weights.get("model.embed_tokens.weight")
            .ok_or("Missing weight: model.embed_tokens.weight")?;
        let embed_f32_size = vocab_size * hidden_size * 4;
        let use_gpu_q4k_embed = matches!(embed_w.dtype, crate::ir::DType::Q4_K)
            && embed_f32_size > 2_000_000_000;

        let (embed_f32, embed_table, embed_q4k_gpu) = if use_gpu_q4k_embed {
            // Large Q4_K embed: upload raw bytes to GPU, dequant per-token via shader
            let raw_size_mb = embed_w.data.len() as f64 / 1e6;
            let f32_size_gb = embed_f32_size as f64 / 1e9;
            log::info!(
                "Large embed: vocab={vocab_size} hidden={hidden_size} → {f32_size_gb:.1}GB f32 exceeds 2GB limit. \
                 Using GPU Q4_K embed ({raw_size_mb:.0}MB raw)."
            );
            let q4k_buf = pipelines.upload_bytes(&embed_w.data);
            // For tied weights, the lm_head also needs Q4_K buffer, so upload as embed_table too
            let embed_table = if tie_word_embeddings {
                // The lm_head will use the Q4_K buffer via q4k_matmul — store it as embed_table
                pipelines.upload_bytes(&embed_w.data)
            } else {
                pipelines.upload_f32(&[0.0f32])
            };
            (Vec::new(), embed_table, Some(q4k_buf))
        } else {
            // Small embed: dequant to f32 on CPU (existing path)
            let f32_data = safetensors_to_f32(&embed_w.data, embed_w.dtype);
            let embed_table = if tie_word_embeddings {
                pipelines.upload_f32(&f32_data)
            } else {
                pipelines.upload_f32(&[0.0f32])
            };
            (f32_data, embed_table, None)
        };

        let (lm_head, lm_head_quant) = if !tie_word_embeddings {
            if let Some(lm_w) = weights.get("lm_head.weight") {
                match lm_w.dtype {
                    crate::ir::DType::Q4_K => {
                        log::info!("Uploading lm_head as Q4_K ({:.1}MB)", lm_w.data.len() as f64 / 1e6);
                        (Some(pipelines.upload_bytes(&lm_w.data)), QuantFormat::Q4K)
                    }
                    crate::ir::DType::Q5_K => {
                        log::info!("Uploading lm_head as Q5_K ({:.1}MB)", lm_w.data.len() as f64 / 1e6);
                        (Some(pipelines.upload_bytes(&lm_w.data)), QuantFormat::Q5K)
                    }
                    crate::ir::DType::Q6_K => {
                        log::info!("Uploading lm_head as Q6_K ({:.1}MB)", lm_w.data.len() as f64 / 1e6);
                        (Some(pipelines.upload_bytes(&lm_w.data)), QuantFormat::Q6K)
                    }
                    crate::ir::DType::Q3_K => {
                        log::info!("Uploading lm_head as Q3_K ({:.1}MB)", lm_w.data.len() as f64 / 1e6);
                        (Some(pipelines.upload_bytes(&lm_w.data)), QuantFormat::Q3K)
                    }
                    crate::ir::DType::Q2_K => {
                        log::info!("Uploading lm_head as Q2_K ({:.1}MB)", lm_w.data.len() as f64 / 1e6);
                        (Some(pipelines.upload_bytes(&lm_w.data)), QuantFormat::Q2K)
                    }
                    crate::ir::DType::F32 | crate::ir::DType::F16 | crate::ir::DType::Q4 | crate::ir::DType::Q8 => {
                        let f32_data = safetensors_to_f32(&lm_w.data, lm_w.dtype);
                        log::info!("Uploading lm_head as f32 ({:.1}MB)", f32_data.len() as f64 * 4.0 / 1e6);
                        (Some(pipelines.upload_f32(&f32_data)), QuantFormat::F32)
                    }
                    _ => {
                        return Err(format!(
                            "Unsupported dtype {:?} for lm_head.weight. Re-import the model.", lm_w.dtype
                        ));
                    }
                }
            } else { (None, QuantFormat::F32) }
        } else if use_gpu_q4k_embed {
            // Tied weights + Q4_K embed: lm_head uses embed_table which is Q4_K raw bytes
            (None, QuantFormat::Q4K)
        } else { (None, QuantFormat::F32) };

        // Note: for small embeds, lookup is done on CPU using embed_f32.
        // For large Q4_K embeds, lookup is done on GPU via q4k_embed shader.
        // The lm_head is used in matmul which runs after all layer dispatches.

        let q_dim = num_heads * head_dim;
        let kv_dim = kv_num_heads * head_dim;

        // Detect K-quant format — check first layer's q_proj
        let first_proj_dtype = weights.get("model.layers.0.self_attn.q_proj.weight")
            .map(|w| w.dtype);
        let is_q4k = matches!(first_proj_dtype, Some(crate::ir::DType::Q4_K));
        let is_q5k = matches!(first_proj_dtype, Some(crate::ir::DType::Q5_K));
        let is_q6k = matches!(first_proj_dtype, Some(crate::ir::DType::Q6_K));
        let is_q3k = matches!(first_proj_dtype, Some(crate::ir::DType::Q3_K));
        let is_q2k = matches!(first_proj_dtype, Some(crate::ir::DType::Q2_K));
        let model_quant_format = if is_q6k { QuantFormat::Q6K }
            else if is_q5k { QuantFormat::Q5K }
            else if is_q4k { QuantFormat::Q4K }
            else if is_q3k { QuantFormat::Q3K }
            else if is_q2k { QuantFormat::Q2K }
            else { QuantFormat::Q4 };

        if is_q6k {
            log::info!("Detected Q6_K weights — using native Q6_K matmul (no requant)");
        } else if is_q5k {
            log::info!("Detected Q5_K weights — using native Q5_K matmul (no requant)");
        } else if is_q4k {
            log::info!("Detected Q4_K weights — using native Q4_K matmul (no requant)");
        } else if is_q3k {
            log::info!("Detected Q3_K weights — using native Q3_K matmul (no requant)");
        } else if is_q2k {
            log::info!("Detected Q2_K weights — using native Q2_K matmul (no requant)");
        }

        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            log::info!("Loading layer {i}/{num_layers}...");

            let input_norm_f32 = weight_to_f32(&format!("model.layers.{i}.input_layernorm.weight"))?;
            let input_norm_weight = pipelines.upload_f32(&input_norm_f32);

            // K-quant: upload raw bytes directly; Q4_0: dequant→requant to Q4_0
            let quantize_upload = |name: &str, n: usize, k: usize| -> Result<(wgpu::Buffer, wgpu::Buffer, QuantFormat), String> {
                let w = weights.get(name).ok_or_else(|| format!("Missing weight: {name}"))?;
                match w.dtype {
                    crate::ir::DType::Q4_K => {
                        let buf = pipelines.upload_bytes(&w.data);
                        let dummy_scales = pipelines.upload_f32(&[0.0f32]);
                        Ok((buf, dummy_scales, QuantFormat::Q4K))
                    }
                    crate::ir::DType::Q5_K => {
                        let buf = pipelines.upload_bytes(&w.data);
                        let dummy_scales = pipelines.upload_f32(&[0.0f32]);
                        Ok((buf, dummy_scales, QuantFormat::Q5K))
                    }
                    crate::ir::DType::Q6_K => {
                        let buf = pipelines.upload_bytes(&w.data);
                        let dummy_scales = pipelines.upload_f32(&[0.0f32]);
                        Ok((buf, dummy_scales, QuantFormat::Q6K))
                    }
                    crate::ir::DType::Q3_K => {
                        let buf = pipelines.upload_bytes(&w.data);
                        let dummy_scales = pipelines.upload_f32(&[0.0f32]);
                        Ok((buf, dummy_scales, QuantFormat::Q3K))
                    }
                    crate::ir::DType::Q2_K => {
                        let buf = pipelines.upload_bytes(&w.data);
                        let dummy_scales = pipelines.upload_f32(&[0.0f32]);
                        Ok((buf, dummy_scales, QuantFormat::Q2K))
                    }
                    crate::ir::DType::Q4 => {
                        // Legacy Q4_0 — dequant and requant (backwards compat for old .model files)
                        let f32_data = safetensors_to_f32(&w.data, w.dtype);
                        let (packed, scales) = quantize_f32_to_q4(&f32_data, n, k);
                        Ok((pipelines.upload_u32(&packed), pipelines.upload_f32(&scales), QuantFormat::Q4))
                    }
                    crate::ir::DType::F32 | crate::ir::DType::F16 => {
                        // Float weights — dequant and requant to Q4_0
                        let f32_data = safetensors_to_f32(&w.data, w.dtype);
                        let (packed, scales) = quantize_f32_to_q4(&f32_data, n, k);
                        Ok((pipelines.upload_u32(&packed), pipelines.upload_f32(&scales), QuantFormat::Q4))
                    }
                    _ => Err(format!(
                        "Unsupported dtype {:?} for weight '{}'. Re-import the model.", w.dtype, name
                    )),
                }
            };

            let (q_proj_packed, q_proj_scales, q_proj_qf) = quantize_upload(
                &format!("model.layers.{i}.self_attn.q_proj.weight"), q_dim, hidden_size)?;
            let (k_proj_packed, k_proj_scales, k_proj_qf) = quantize_upload(
                &format!("model.layers.{i}.self_attn.k_proj.weight"), kv_dim, hidden_size)?;
            let (v_proj_packed, v_proj_scales, v_proj_qf) = quantize_upload(
                &format!("model.layers.{i}.self_attn.v_proj.weight"), kv_dim, hidden_size)?;
            let (o_proj_packed, o_proj_scales, o_proj_qf) = quantize_upload(
                &format!("model.layers.{i}.self_attn.o_proj.weight"), hidden_size, q_dim)?;

            let post_norm_f32 = weight_to_f32(&format!("model.layers.{i}.post_attention_layernorm.weight"))?;
            let post_norm_weight = pipelines.upload_f32(&post_norm_f32);

            let (gate_proj_packed, gate_proj_scales, gate_proj_qf) = quantize_upload(
                &format!("model.layers.{i}.mlp.gate_proj.weight"), intermediate_size, hidden_size)?;
            let (up_proj_packed, up_proj_scales, up_proj_qf) = quantize_upload(
                &format!("model.layers.{i}.mlp.up_proj.weight"), intermediate_size, hidden_size)?;
            let (down_proj_packed, down_proj_scales, down_proj_qf) = quantize_upload(
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

            // Precompute matmul params: K-quants use different params structs than Q4_0
            let precompute_matmul = |qf: QuantFormat, n: u32, k: u32| -> (wgpu::Buffer, (u32, u32, u32)) {
                match qf {
                    QuantFormat::Q4K => precompute_q4k_matmul(&pipelines, n, k),
                    QuantFormat::Q5K => precompute_q5k_matmul(&pipelines, n, k),
                    QuantFormat::Q6K => precompute_q6k_matmul(&pipelines, n, k),
                    QuantFormat::Q3K => precompute_q3k_matmul(&pipelines, n, k),
                    QuantFormat::Q2K => precompute_q2k_matmul(&pipelines, n, k),
                    QuantFormat::F16 => precompute_f16_matmul(&pipelines, n, k),
                    QuantFormat::F32 => precompute_f32_matmul(&pipelines, n, k),
                    _ => precompute_q4_matmul(&pipelines, n, k, bs),
                }
            };

            let (q_matmul_params, q_matmul_wg) = precompute_matmul(q_proj_qf, q_dim as u32, hs);
            let (k_matmul_params, k_matmul_wg) = precompute_matmul(k_proj_qf, kv_dim as u32, hs);
            let (v_matmul_params, v_matmul_wg) = precompute_matmul(v_proj_qf, kv_dim as u32, hs);
            let (q_rope_params, q_rope_wg) = precompute_rope(&pipelines, q_dim as u32, hd, 1);
            let (k_rope_params, k_rope_wg) = precompute_rope(&pipelines, kv_dim as u32, hd, 1);
            let (o_matmul_params, o_matmul_wg) = precompute_matmul(o_proj_qf, hs, q_dim as u32);
            let (post_norm_params, post_norm_wg) = precompute_rms_norm(&pipelines, 1, hs, rms_norm_eps);
            let (gate_matmul_params, gate_matmul_wg) = precompute_matmul(gate_proj_qf, intermediate_size as u32, hs);
            let (up_matmul_params, up_matmul_wg) = precompute_matmul(up_proj_qf, intermediate_size as u32, hs);
            let (down_matmul_params, down_matmul_wg) = precompute_matmul(down_proj_qf, hs, intermediate_size as u32);

            // Fused norm+q4 uses workgroup shared memory for the full hidden state.
            // Max shared_normed is 4096 floats (16 KB) — disable fused path for
            // larger hidden sizes to avoid OOB in workgroup memory.
            // Also disable for K-quants since fused_norm_q4 is Q4_0-specific.
            let use_fused = hidden_size <= 4096 && !is_q4k && !is_q5k && !is_q6k && !is_q3k && !is_q2k;
            let (fused_q_params, fused_q_wg) = if use_fused {
                let (b, w) = precompute_fused_norm_q4(&pipelines, q_dim as u32, hs, bs, rms_norm_eps);
                (Some(b), w)
            } else {
                (None, (0, 0, 0))
            };
            let (fused_k_params, fused_k_wg) = if use_fused {
                let (b, w) = precompute_fused_norm_q4(&pipelines, kv_dim as u32, hs, bs, rms_norm_eps);
                (Some(b), w)
            } else {
                (None, (0, 0, 0))
            };
            let (fused_v_params, fused_v_wg) = if use_fused {
                let (b, w) = precompute_fused_norm_q4(&pipelines, kv_dim as u32, hs, bs, rms_norm_eps);
                (Some(b), w)
            } else {
                (None, (0, 0, 0))
            };

            layers.push(LayerWeights {
                input_norm_weight,
                q_proj_packed, q_proj_scales, q_proj_quant: q_proj_qf, q_n: q_dim as u32,
                k_proj_packed, k_proj_scales, k_proj_quant: k_proj_qf, k_n: kv_dim as u32,
                v_proj_packed, v_proj_scales, v_proj_quant: v_proj_qf, v_n: kv_dim as u32,
                q_proj_bias, k_proj_bias, v_proj_bias,
                q_norm_weight: q_norm_w, k_norm_weight: k_norm_w,
                o_proj_packed, o_proj_scales, o_proj_quant: o_proj_qf, o_n: hidden_size as u32,
                post_norm_weight,
                gate_proj_packed, gate_proj_scales, gate_proj_quant: gate_proj_qf, gate_n: intermediate_size as u32,
                up_proj_packed, up_proj_scales, up_proj_quant: up_proj_qf, up_n: intermediate_size as u32,
                down_proj_packed, down_proj_scales, down_proj_quant: down_proj_qf, down_n: hidden_size as u32,
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
        let (f16_matmul_params, f16_matmul_wg) = precompute_f16_matmul(&pipelines, vocab_size as u32, hidden_size as u32);
        let (q4k_lm_params, q4k_lm_wg) = precompute_q4k_matmul(&pipelines, vocab_size as u32, hidden_size as u32);
        let (q5k_lm_params, q5k_lm_wg) = precompute_q5k_matmul(&pipelines, vocab_size as u32, hidden_size as u32);
        let (q6k_lm_params, q6k_lm_wg) = precompute_q6k_matmul(&pipelines, vocab_size as u32, hidden_size as u32);
        let (q3k_lm_params, q3k_lm_wg) = precompute_q3k_matmul(&pipelines, vocab_size as u32, hidden_size as u32);
        let (q2k_lm_params, q2k_lm_wg) = precompute_q2k_matmul(&pipelines, vocab_size as u32, hidden_size as u32);
        let (argmax_params, argmax_wg) = precompute_argmax(&pipelines, vocab_size as u32);

        // Persistent staging buffer for greedy argmax readback (4 bytes = 1 f32)
        let argmax_staging = pipelines.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("argmax_staging"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Persistent decode buffers (1 token, reused every step)
        let half = head_dim / 2;
        let decode_ids_buf = pipelines.upload_f32(&[0.0f32]);
        let decode_cos_buf = pipelines.upload_f32(&vec![0.0f32; half]);
        let decode_sin_buf = pipelines.upload_f32(&vec![0.0f32; half]);
        // decode_embed_buf was pre-allocated above, before weight loading

        log::info!(".model loaded: {} layers, {:?} weights", num_layers, model_quant_format);

        Ok(Self {
            config, pipelines, embed_table, embed_f32, embed_q4k_gpu, lm_head, lm_head_quant, layers, final_norm_weight,
            cos_cache, sin_cache, kv_cache, past_seq_len: 0, greedy_mode: false,
            quant_format: model_quant_format,
            model_params: ModelParamBuffers {
                final_norm_params, final_norm_wg,
                f32_matmul_params, f32_matmul_wg,
                f16_matmul_params, f16_matmul_wg,
                q4k_lm_params, q4k_lm_wg,
                q5k_lm_params, q5k_lm_wg,
                q6k_lm_params, q6k_lm_wg,
                q3k_lm_params, q3k_lm_wg,
                q2k_lm_params, q2k_lm_wg,
                argmax_params, argmax_wg,
            },
            kv_compressor: None,
            argmax_staging,
            decode_ids_buf, decode_cos_buf, decode_sin_buf, decode_embed_buf,
        })
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

        let half = half_dim as usize;
        let start = pos_offset * half;
        let end = start + seq_len * half;

        // For decode (seq_len=1): reuse persistent buffers, no allocation
        // For prefill: allocate fresh (variable length)
        let prefill_ids;
        let prefill_cos;
        let prefill_sin;
        let (ids_buf, cos_buf, sin_buf) = if seq_len == 1 {
            p.queue.write_buffer(&self.decode_ids_buf, 0, bytemuck::cast_slice(&[token_ids[0] as f32]));
            p.queue.write_buffer(&self.decode_cos_buf, 0, bytemuck::cast_slice(&self.cos_cache[start..end]));
            p.queue.write_buffer(&self.decode_sin_buf, 0, bytemuck::cast_slice(&self.sin_cache[start..end]));
            (&self.decode_ids_buf, &self.decode_cos_buf, &self.decode_sin_buf)
        } else {
            let ids_f32: Vec<f32> = token_ids.iter().map(|&id| id as f32).collect();
            prefill_ids = p.upload_f32(&ids_f32);
            prefill_cos = p.upload_f32(&self.cos_cache[start..end]);
            prefill_sin = p.upload_f32(&self.sin_cache[start..end]);
            (&prefill_ids, &prefill_cos, &prefill_sin)
        };

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

            let token_id = token_ids[0] as usize;
            let hs = hidden_size as usize;

            let mut hidden = if let Some(ref q4k_buf) = self.embed_q4k_gpu {
                // GPU Q4_K embed: dequant one row on GPU (avoids 5.6GB f32 dequant)
                if pos_offset < 2 {
                    log::warn!("WGPU q4k_embed[token={token_id}] (GPU path)");
                }
                let (out, bg, wg) = dispatch::q4k_embed_prepare(
                    &p, q4k_buf, token_id as u32, hidden_size,
                );
                all_dispatches.push(DispatchCmd { shader: &p.q4k_embed, bg, wg });
                out
            } else {
                // CPU embedding lookup: avoids GPU-side read of the multi-GB embed table
                // which can return zeros if the upload hasn't fully resolved on Metal.
                let embed_start = token_id * hs;
                let embed_slice = &self.embed_f32[embed_start..embed_start + hs];
                if pos_offset < 2 {
                    log::warn!("WGPU embed[token={token_id}][0:4]: {:?}", &embed_slice[..4]);
                }
                // Use fresh buffer for embed (write_buffer may fail under GPU memory pressure)
                p.upload_f32(embed_slice)
            };

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
                        quant_fmt_to_dispatch(self.layers[i].q_proj_quant),
                    );
                    all_dispatches.push(DispatchCmd { shader: q_shader, bg: q_bg, wg: q_wg });
                    let (k, k_bg, k_wg, k_shader) = dispatch::prepare_matmul_for_quant(
                        &p, &normed, &self.layers[i].k_proj_packed, &self.layers[i].k_proj_scales,
                        &lp.k_matmul_params, self.layers[i].k_n, lp.k_matmul_wg,
                        quant_fmt_to_dispatch(self.layers[i].k_proj_quant),
                    );
                    all_dispatches.push(DispatchCmd { shader: k_shader, bg: k_bg, wg: k_wg });
                    let (v, v_bg, v_wg, v_shader) = dispatch::prepare_matmul_for_quant(
                        &p, &normed, &self.layers[i].v_proj_packed, &self.layers[i].v_proj_scales,
                        &lp.v_matmul_params, self.layers[i].v_n, lp.v_matmul_wg,
                        quant_fmt_to_dispatch(self.layers[i].v_proj_quant),
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
                    quant_fmt_to_dispatch(self.layers[i].o_proj_quant),
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
                        quant_fmt_to_dispatch(self.layers[i].gate_proj_quant),
                    );
                    all_dispatches.push(DispatchCmd { shader: gate_shader, bg: gate_bg, wg: gate_wg });
                    let (u, up_bg, up_wg, up_shader) = dispatch::prepare_matmul_for_quant(
                        &p, &normed2, &self.layers[i].up_proj_packed, &self.layers[i].up_proj_scales,
                        &lp.up_matmul_params, self.layers[i].up_n, lp.up_matmul_wg,
                        quant_fmt_to_dispatch(self.layers[i].up_proj_quant),
                    );
                    all_dispatches.push(DispatchCmd { shader: up_shader, bg: up_bg, wg: up_wg });
                    gate = g; up = u;
                };

                let (ffn, silu_bg, silu_wg) = dispatch::silu_mul_prepare(&p, &gate, &up, self.layers[i].gate_n);
                all_dispatches.push(DispatchCmd { shader: &p.silu_mul, bg: silu_bg, wg: silu_wg });

                let (ffn_out, down_bg, down_wg, down_shader) = dispatch::prepare_matmul_for_quant(
                    &p, &ffn, &self.layers[i].down_proj_packed, &self.layers[i].down_proj_scales,
                    &lp.down_matmul_params, self.layers[i].down_n, lp.down_matmul_wg,
                    quant_fmt_to_dispatch(self.layers[i].down_proj_quant),
                );
                all_dispatches.push(DispatchCmd { shader: down_shader, bg: down_bg, wg: down_wg });

                let (new_hidden, res2_bg, res2_wg) = dispatch::add_prepare(&p, &residual1, &ffn_out, hidden_size);
                all_dispatches.push(DispatchCmd { shader: &p.add, bg: res2_bg, wg: res2_wg });

                hidden = new_hidden;

                // DEBUG: dump hidden state after each layer on first token
                // Enable with DEBUG_LAYERS=1 env var
                if pos_offset == 0 && std::env::var("DEBUG_LAYERS").is_ok()
                    && (i < 3 || i >= num_layers - 2)
                {
                    let mut enc = p.device.create_command_encoder(&Default::default());
                    for cmd in all_dispatches.drain(..) {
                        let mut pass = enc.begin_compute_pass(&Default::default());
                        pass.set_pipeline(&cmd.shader.pipeline);
                        pass.set_bind_group(0, Some(&cmd.bg), &[]);
                        pass.dispatch_workgroups(cmd.wg.0, cmd.wg.1, cmd.wg.2);
                    }
                    let idx = p.queue.submit(std::iter::once(enc.finish()));
                    p.device.poll(wgpu::Maintain::WaitForSubmissionIndex(idx));
                    let vals = p.read_f32(&hidden, 8);
                    let status = if vals.iter().any(|v| v.is_nan()) { "NaN!" }
                        else if vals.iter().any(|v| v.is_infinite()) { "INF!" }
                        else { "ok" };
                    log::warn!("LAYER {i} hidden[0:8]: {:?} {}", vals, status);
                }
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
            let (logits_buf, lm_bg, lm_wg, lm_shader) = if self.lm_head_quant == QuantFormat::Q4K {
                let (buf, bg, wg) = dispatch::q4k_matmul_prepare_precomputed(
                    &p, &final_normed, lm_head_buf,
                    &mp.q4k_lm_params, vocab, mp.q4k_lm_wg,
                );
                (buf, bg, wg, &p.q4k_matmul)
            } else if self.lm_head_quant == QuantFormat::Q5K {
                let (buf, bg, wg) = dispatch::q5k_matmul_prepare_precomputed(
                    &p, &final_normed, lm_head_buf,
                    &mp.q5k_lm_params, vocab, mp.q5k_lm_wg,
                );
                (buf, bg, wg, &p.q5k_matmul)
            } else if self.lm_head_quant == QuantFormat::Q6K {
                let (buf, bg, wg) = dispatch::q6k_matmul_prepare_precomputed(
                    &p, &final_normed, lm_head_buf,
                    &mp.q6k_lm_params, vocab, mp.q6k_lm_wg,
                );
                (buf, bg, wg, &p.q6k_matmul)
            } else if self.lm_head_quant == QuantFormat::Q3K {
                let (buf, bg, wg) = dispatch::q3k_matmul_prepare_precomputed(
                    &p, &final_normed, lm_head_buf,
                    &mp.q3k_lm_params, vocab, mp.q3k_lm_wg,
                );
                (buf, bg, wg, &p.q3k_matmul)
            } else if self.lm_head_quant == QuantFormat::Q2K {
                let (buf, bg, wg) = dispatch::q2k_matmul_prepare_precomputed(
                    &p, &final_normed, lm_head_buf,
                    &mp.q2k_lm_params, vocab, mp.q2k_lm_wg,
                );
                (buf, bg, wg, &p.q2k_matmul)
            } else {
                let (buf, bg, wg) = dispatch::f32_matmul_prepare_precomputed(
                    &p, &final_normed, lm_head_buf,
                    &mp.f32_matmul_params, vocab, mp.f32_matmul_wg,
                );
                (buf, bg, wg, &p.f32_matmul)
            };
            all_dispatches.push(DispatchCmd { shader: lm_shader, bg: lm_bg, wg: lm_wg });

            {
                let mut pass = enc.begin_compute_pass(&Default::default());
                for cmd in &all_dispatches {
                    p.dispatch_in_pass(&mut pass, cmd.shader, &cmd.bg, cmd.wg);
                }
            }
            p.queue.submit(std::iter::once(enc.finish()));
            self.past_seq_len = total_seq;

            let logits = p.read_f32(&logits_buf, vocab as usize);
            if total_seq <= 2 {
                let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let min_logit = logits.iter().cloned().fold(f32::INFINITY, f32::min);
                let argmax = logits.iter().enumerate().max_by(|a,b| a.1.partial_cmp(b.1).unwrap()).map(|(i,_)| i).unwrap_or(0);
                // Top-5 tokens
                let mut indexed: Vec<(usize, f32)> = logits.iter().enumerate().map(|(i,&v)| (i,v)).collect();
                indexed.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap());
                let top5: Vec<_> = indexed[..5.min(indexed.len())].iter().map(|(i,v)| format!("{}:{:.2}", i, v)).collect();
                log::warn!("WGPU pos={}: logits range=[{:.2}..{:.2}] argmax={} top5=[{}]", total_seq-1, min_logit, max_logit, argmax, top5.join(", "));
            }
            return logits;
        }

        // ================================================================
        // PREFILL PATH — legacy one-pass-per-op
        // ================================================================
        let mut hidden = if let Some(ref q4k_buf) = self.embed_q4k_gpu {
            // Q4_K embed prefill: dequant each token's row on GPU via individual dispatches
            let out = p.alloc((seq_len as u64) * (hidden_size as u64) * 4);
            let blocks_per_row = hidden_size / 256;
            for s in 0..seq_len {
                let tid = token_ids[s] as u32;
                #[repr(C)]
                #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
                struct EParams { token_id: u32, hidden_size: u32, blocks_per_row: u32, _pad: u32 }
                let ep = EParams { token_id: tid, hidden_size, blocks_per_row, _pad: 0 };
                let ep_buf = p.upload_uniform(bytemuck::bytes_of(&ep));
                // Write to output[s * hidden_size .. (s+1) * hidden_size]
                // We need a per-position output buffer, then copy
                let row_out = p.alloc((hidden_size as u64) * 4);
                p.encode(
                    &mut enc, &p.q4k_embed,
                    &[
                        q4k_buf.as_entire_binding(),
                        row_out.as_entire_binding(),
                        ep_buf.as_entire_binding(),
                    ],
                    ((hidden_size + 255) / 256, 1, 1),
                );
                let offset = (s as u64) * (hidden_size as u64) * 4;
                enc.copy_buffer_to_buffer(&row_out, 0, &out, offset, (hidden_size as u64) * 4);
            }
            out
        } else {
            dispatch::embed(&p, &mut enc, &self.embed_table, &ids_buf, hidden_size, seq_len as u32)
        };

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
                // Use per-layer quant format (not model-level) — models like Q4_K_M have mixed Q4_K/Q6_K
                let mut q_pos = dispatch_matmul_enc(&p, &mut enc, self.layers[i].q_proj_quant, &normed_slice, &self.layers[i].q_proj_packed, &self.layers[i].q_proj_scales, layer_q_n, hidden_size, block_size);
                let mut k_pos = dispatch_matmul_enc(&p, &mut enc, self.layers[i].k_proj_quant, &normed_slice, &self.layers[i].k_proj_packed, &self.layers[i].k_proj_scales, layer_k_n, hidden_size, block_size);
                let mut v_pos = dispatch_matmul_enc(&p, &mut enc, self.layers[i].v_proj_quant, &normed_slice, &self.layers[i].v_proj_packed, &self.layers[i].v_proj_scales, layer_v_n, hidden_size, block_size);
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

            let attn_proj = dispatch_matmul_enc(&p, &mut enc, self.layers[i].o_proj_quant, &attn_out, &self.layers[i].o_proj_packed, &self.layers[i].o_proj_scales, self.layers[i].o_n, self.layers[i].q_n, block_size);

            let residual_hidden = slice_buffer(&p, &mut enc, &hidden, (seq_len - 1) * self.config.hidden_size, self.config.hidden_size);
            hidden = dispatch::add(&p, &mut enc, &residual_hidden, &attn_proj, hidden_size);

            let normed2 = dispatch::rms_norm(&p, &mut enc, &hidden, &self.layers[i].post_norm_weight, 1, hidden_size, 1e-6);

            let gate = dispatch_matmul_enc(&p, &mut enc, self.layers[i].gate_proj_quant, &normed2, &self.layers[i].gate_proj_packed, &self.layers[i].gate_proj_scales, self.layers[i].gate_n, hidden_size, block_size);
            let up = dispatch_matmul_enc(&p, &mut enc, self.layers[i].up_proj_quant, &normed2, &self.layers[i].up_proj_packed, &self.layers[i].up_proj_scales, self.layers[i].up_n, hidden_size, block_size);
            let ffn = dispatch::silu_mul(&p, &mut enc, &gate, &up, self.layers[i].gate_n);
            let ffn_out = dispatch_matmul_enc(&p, &mut enc, self.layers[i].down_proj_quant, &ffn, &self.layers[i].down_proj_packed, &self.layers[i].down_proj_scales, self.layers[i].down_n, self.layers[i].gate_n, block_size);

            hidden = dispatch::add(&p, &mut enc, &hidden, &ffn_out, hidden_size);
        }

        // Final norm + LM head (PREFILL PATH)
        let vocab = self.config.vocab_size as u32;
        let normed = dispatch::rms_norm(&p, &mut enc, &hidden, &self.final_norm_weight, 1, hidden_size, 1e-6);
        let lm_head_buf_pf = self.lm_head.as_ref().unwrap_or(&self.embed_table);
        // Dispatch LM head with its actual quant format (Q4_K/Q5_K/Q6_K for K-quant models)
        let logits_buf = dispatch_matmul_enc(&p, &mut enc, self.lm_head_quant, &normed, lm_head_buf_pf, &self.embed_table, vocab, hidden_size, block_size);

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
        QuantFormat::Q4K => dispatch::q4k_matmul(p, enc, activation, weight, n, k),
        QuantFormat::Q5K => dispatch::q5k_matmul(p, enc, activation, weight, n, k),
        QuantFormat::Q6K => dispatch::q6k_matmul(p, enc, activation, weight, n, k),
        QuantFormat::Q3K => dispatch::q3k_matmul(p, enc, activation, weight, n, k),
        QuantFormat::Q2K => dispatch::q2k_matmul(p, enc, activation, weight, n, k),
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

#[cfg(test)]
mod tests {
    use super::safetensors_to_f32;
    use crate::ir::DType;

    fn from_hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn test_q4k_dequant() {
        // First Q4_K superblock from blk.0.attn_q.weight in Qwen2.5-Coder-14B GGUF
        let raw = from_hex(concat!(
            "7b0cb91253ce5952a294ffaa618f1265",
            "92054f3584280455a48353b03a5413e3",
            "7463367854ac33492588944a80f54879",
            "67476a6888e7f748ab99eb4c985d57f6",
            "984c383178aa05ba5c685556e03b6aaf",
            "15373a1d1836312a482823080c1a0237",
            "392a28472713390f36100a23f8272e06",
            "467c6af67c6b559a12262625162b0477",
            "908e57538a1989285636485a6a597b58",
        ));

        assert_eq!(raw.len(), 144);
        let result = safetensors_to_f32(&raw, DType::Q4_K);
        assert_eq!(result.len(), 256);

        // Reference from Python llama.cpp-style dequant
        let expected: [f32; 32] = [
            -0.017509937, -0.001922369, 0.050036192, -0.001922369,
            -0.007118225, 0.013665199, -0.007118225, -0.001922369,
            -0.007118225, -0.012314081, -0.012314081, -0.027901649,
            0.024056911, -0.007118225, -0.012314081, -0.012314081,
            -0.007118225, -0.012314081, 0.003273487, 0.013665199,
            -0.007118225, 0.034448624, -0.012314081, 0.018861055,
            -0.001922369, 0.013665199, -0.007118225, 0.024056911,
            -0.027901649, -0.001922369, 0.013665199, 0.018861055,
        ];

        for (i, (&got, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let diff = (got - exp).abs();
            assert!(
                diff < 1e-4,
                "val[{i}]: got={got:.6}, expected={exp:.6}, diff={diff:.2e}"
            );
        }
    }

    /// End-to-end test: embed → RMS norm → Q4_K matmul for layer 0 k_proj
    #[test]
    fn test_q4k_gpu_e2e_layer0() {
        use std::sync::Arc;
        use std::path::Path;
        use crate::backend::wgpu::pipelines::Pipelines;
        use crate::backend::wgpu::dispatch;
        use crate::cyb_format::LoadedModel;

        let model_path = Path::new("/Users/mastercyb/llm/qwen2.5-coder-14b-abl.model");
        if !model_path.exists() {
            eprintln!("Model file not found, skipping test");
            return;
        }

        let lm = LoadedModel::load(model_path).expect("Failed to load model");

        // Get config
        let config_json = lm.config_json();
        let arch = config_json.get("architecture").unwrap_or(&config_json);
        let hidden_size = arch.get("hidden_size").and_then(|v| v.as_u64()).unwrap() as usize;
        let kv_num_heads = arch.get("num_key_value_heads").and_then(|v| v.as_u64()).unwrap() as usize;
        let head_dim = 128usize;
        let kv_dim = kv_num_heads * head_dim;
        eprintln!("hidden_size={hidden_size}, kv_dim={kv_dim}, head_dim={head_dim}");

        // Embedding for token 27 ("<|im_start|>")
        let embed_w = lm.weights.get("model.embed_tokens.weight").expect("no embed");
        let embed_f32 = safetensors_to_f32(&embed_w.data, embed_w.dtype);
        let token_id = 27usize;
        let embed = &embed_f32[token_id * hidden_size..(token_id + 1) * hidden_size];
        eprintln!("embed[0..4] = {:?}", &embed[..4]);

        // CPU RMS norm
        let input_norm_w = lm.weights.get("model.layers.0.input_layernorm.weight").expect("no norm");
        let norm_w = safetensors_to_f32(&input_norm_w.data, input_norm_w.dtype);
        let eps = 1e-6f32;
        let rms = (embed.iter().map(|x| x * x).sum::<f32>() / hidden_size as f32 + eps).sqrt();
        let normed: Vec<f32> = embed.iter().zip(norm_w.iter()).map(|(&x, &w)| (x / rms) * w).collect();
        eprintln!("normed[0..4] = {:?}", &normed[..4]);

        // CPU Q4_K matmul for k_proj
        let k_proj = lm.weights.get("model.layers.0.self_attn.k_proj.weight").expect("no k_proj");
        let k_proj_f32_all = safetensors_to_f32(&k_proj.data, k_proj.dtype);
        let n = kv_dim;  // output dim
        let k = hidden_size;  // input dim
        let total_vals = k_proj_f32_all.len();
        eprintln!("k_proj: {} total f32 vals, expected {}", total_vals, n * k);
        assert_eq!(total_vals, n * k);

        let cpu_result: Vec<f32> = (0..n).map(|r| {
            let row = &k_proj_f32_all[r * k..(r + 1) * k];
            row.iter().zip(normed.iter()).map(|(&w, &a)| w * a).sum()
        }).collect();
        eprintln!("CPU k_proj[0..4] = {:?}", &cpu_result[..4]);

        // GPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(), ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default()
        })).expect("No GPU adapter");

        let mut limits = wgpu::Limits::default();
        limits.max_buffer_size = 1u64 << 30;
        limits.max_storage_buffer_binding_size = 1u32 << 30;
        let mut features = wgpu::Features::empty();
        if adapter.features().contains(wgpu::Features::SUBGROUP) {
            features |= wgpu::Features::SUBGROUP;
        }
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("e2e-test"), required_features: features,
                required_limits: limits, memory_hints: Default::default(),
            }, None,
        )).expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let p = Pipelines::new(device.clone(), queue.clone());

        // GPU: upload normed activation
        let normed_buf = p.upload_f32(&normed);
        // GPU: upload raw Q4_K weights
        let weight_buf = p.upload_bytes(&k_proj.data);

        // GPU: Q4_K matmul
        let mut enc = p.device.create_command_encoder(&Default::default());
        let gpu_result_buf = dispatch::q4k_matmul(&p, &mut enc, &normed_buf, &weight_buf, n as u32, k as u32);
        p.queue.submit(std::iter::once(enc.finish()));

        let gpu_result = p.read_f32(&gpu_result_buf, n);

        // Compare
        let mut max_diff = 0.0f32;
        let mut max_rel = 0.0f32;
        for r in 0..n {
            let diff = (gpu_result[r] - cpu_result[r]).abs();
            let rel = if cpu_result[r].abs() > 1e-8 { diff / cpu_result[r].abs() } else { diff };
            if diff > max_diff { max_diff = diff; }
            if rel > max_rel { max_rel = rel; }
        }
        eprintln!("GPU k_proj[0..4] = {:?}", &gpu_result[..4]);
        eprintln!("Max diff={max_diff:.2e}, max_rel={max_rel:.2e} across {n} rows");

        assert!(max_diff < 0.5 || max_rel < 0.05,
            "Q4_K e2e mismatch: max_diff={max_diff:.2e}, max_rel={max_rel:.2e}");
    }

    /// GPU test with real model data: loads k_proj from qwen2.5-coder-14b model,
    /// computes Q4_K matmul for first 4 rows, compares GPU vs CPU reference.
    #[test]
    fn test_q4k_gpu_real_model() {
        use std::sync::Arc;
        use std::path::Path;
        use crate::backend::wgpu::pipelines::Pipelines;
        use crate::cyb_format::LoadedModel;

        let model_path = Path::new("/Users/mastercyb/llm/qwen2.5-coder-14b-abl.model");
        if !model_path.exists() {
            eprintln!("Model file not found, skipping test");
            return;
        }

        let lm = LoadedModel::load(model_path).expect("Failed to load model");
        let k_proj = lm.weights.get("model.layers.0.self_attn.k_proj.weight")
            .expect("Missing k_proj weight");

        assert!(matches!(k_proj.dtype, DType::Q4_K), "k_proj should be Q4_K");

        // k_proj: N=1024, K=5120
        let n: u32 = 4; // Test first 4 rows only
        let k: u32 = 5120;
        let blocks_per_row: u32 = k / 256;

        // Extract first 4 rows of raw Q4_K data
        let bytes_per_row = (blocks_per_row as usize) * 144;
        let weight_data = &k_proj.data[..n as usize * bytes_per_row];

        // CPU dequant of first 4 rows
        let all_cpu_vals: Vec<f32> = (0..n as usize).map(|r| {
            let row_bytes = &weight_data[r * bytes_per_row..(r + 1) * bytes_per_row];
            safetensors_to_f32(row_bytes, DType::Q4_K)
        }).collect::<Vec<Vec<f32>>>().concat();

        // Activation: simple ramp [0/K, 1/K, ..., (K-1)/K]
        let activation: Vec<f32> = (0..k).map(|i| (i as f32) / (k as f32)).collect();

        // CPU matmul
        let cpu_dots: Vec<f32> = (0..n as usize).map(|r| {
            let row = &all_cpu_vals[r * k as usize..(r + 1) * k as usize];
            row.iter().zip(activation.iter()).map(|(&w, &a)| w * a).sum()
        }).collect();

        // GPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            }),
        ).expect("No GPU adapter");

        let mut limits = wgpu::Limits::default();
        limits.max_buffer_size = 1u64 << 30;
        limits.max_storage_buffer_binding_size = 1u32 << 30;

        let mut features = wgpu::Features::empty();
        if adapter.features().contains(wgpu::Features::SUBGROUP) {
            features |= wgpu::Features::SUBGROUP;
        }

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("q4k-real-test"),
                required_features: features,
                required_limits: limits,
                memory_hints: Default::default(),
            },
            None,
        )).expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let p = Pipelines::new(device.clone(), queue.clone());

        let act_buf = p.upload_f32(&activation);
        let weight_buf = p.upload_bytes(weight_data);

        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params { n: u32, k: u32, blocks_per_row: u32, _pad: u32 }
        let params = Params { n, k, blocks_per_row, _pad: 0 };
        let params_buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));

        let output_buf = p.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (n as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bg = p.create_bind_group(
            &p.q4k_matmul,
            &[
                act_buf.as_entire_binding(),
                weight_buf.as_entire_binding(),
                output_buf.as_entire_binding(),
                params_buf.as_entire_binding(),
            ],
        );

        let num_wg = (n + 3) / 4;
        let mut encoder = p.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            p.dispatch_in_pass(&mut pass, &p.q4k_matmul, &bg, (num_wg, 1, 1));
        }
        p.queue.submit(std::iter::once(encoder.finish()));

        let result = p.read_f32(&output_buf, n as usize);

        for r in 0..n as usize {
            let gpu = result[r];
            let cpu = cpu_dots[r];
            let diff = (gpu - cpu).abs();
            let rel = if cpu.abs() > 1e-8 { diff / cpu.abs() } else { diff };
            eprintln!("Row {r}: CPU={cpu:.6}, GPU={gpu:.6}, diff={diff:.2e}, rel={rel:.2e}");
            assert!(
                diff < 0.1 || rel < 0.02,
                "Row {r}: Q4_K GPU vs CPU mismatch: gpu={gpu:.6}, cpu={cpu:.6}, diff={diff:.2e}"
            );
        }
    }

    /// GPU integration test: run the Q4_K WGSL shader on a single superblock
    /// and compare the dot product with the CPU reference dequant.
    #[test]
    fn test_q4k_gpu() {
        use std::sync::Arc;
        use crate::backend::wgpu::pipelines::Pipelines;

        // Same Q4_K block as test_q4k_dequant (verified correct on CPU)
        let raw = from_hex(concat!(
            "7b0cb91253ce5952a294ffaa618f1265",
            "92054f3584280455a48353b03a5413e3",
            "7463367854ac33492588944a80f54879",
            "67476a6888e7f748ab99eb4c985d57f6",
            "984c383178aa05ba5c685556e03b6aaf",
            "15373a1d1836312a482823080c1a0237",
            "392a28472713390f36100a23f8272e06",
            "467c6af67c6b559a12262625162b0477",
            "908e57538a1989285636485a6a597b58",
        ));
        // Pad to 144 if needed (the concat above should be exactly 144 bytes)
        assert_eq!(raw.len(), 144, "Q4_K block must be 144 bytes");

        // CPU reference: dequant and dot product with all-ones
        let cpu_vals = safetensors_to_f32(&raw, DType::Q4_K);
        assert_eq!(cpu_vals.len(), 256);

        // Activation: all ones (strongest single-block test)
        let activation: Vec<f32> = vec![1.0f32; 256];
        let cpu_dot: f32 = cpu_vals.iter().sum();

        // -- GPU --
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            }),
        )
        .expect("No GPU adapter for test");

        let mut limits = wgpu::Limits::default();
        limits.max_buffer_size = 1u64 << 30;
        limits.max_storage_buffer_binding_size = 1u32 << 30;

        let mut features = wgpu::Features::empty();
        if adapter.features().contains(wgpu::Features::SUBGROUP) {
            features |= wgpu::Features::SUBGROUP;
        }

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("q4k-test"),
                required_features: features,
                required_limits: limits,
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let p = Pipelines::new(device.clone(), queue.clone());

        // Upload activation (256 f32s)
        let act_buf = p.upload_f32(&activation);

        // Upload weights: 1 row of 1 Q4_K block = 36 u32s = 144 bytes
        let weight_buf = p.upload_bytes(&raw);

        // Output: 1 row
        let n: u32 = 1;
        let k: u32 = 256;
        let blocks_per_row: u32 = 1;

        // Params uniform
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            n: u32,
            k: u32,
            blocks_per_row: u32,
            _pad: u32,
        }
        let params = Params { n, k, blocks_per_row, _pad: 0 };
        let params_buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));

        // Output buffer
        let output_buf = p.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 4, // 1 f32
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Dispatch
        let bg = p.create_bind_group(
            &p.q4k_matmul,
            &[
                act_buf.as_entire_binding(),
                weight_buf.as_entire_binding(),
                output_buf.as_entire_binding(),
                params_buf.as_entire_binding(),
            ],
        );

        // NR=4, so 1 workgroup covers rows 0..3 (we only have row 0)
        let mut encoder = p.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            p.dispatch_in_pass(&mut pass, &p.q4k_matmul, &bg, (1, 1, 1));
        }
        p.queue.submit(std::iter::once(encoder.finish()));

        // Read back
        let result = p.read_f32(&output_buf, 1);
        let gpu_dot = result[0];

        let diff = (gpu_dot - cpu_dot).abs();
        let rel = if cpu_dot.abs() > 1e-8 { diff / cpu_dot.abs() } else { diff };

        eprintln!("CPU dot = {cpu_dot:.6}, GPU dot = {gpu_dot:.6}, diff = {diff:.2e}, rel = {rel:.2e}");

        // Allow small floating-point divergence (f16 intermediates on GPU)
        assert!(
            diff < 0.01 || rel < 0.02,
            "Q4_K GPU vs CPU mismatch: gpu={gpu_dot:.6}, cpu={cpu_dot:.6}, diff={diff:.2e}"
        );
    }

    /// Multi-block GPU test: N=4 rows, K=512 (2 blocks per row), random activation.
    /// This tests the cross-thread accumulation that single-block test misses.
    #[test]
    fn test_q4k_gpu_multiblock() {
        use std::sync::Arc;
        use crate::backend::wgpu::pipelines::Pipelines;

        // Reuse the verified Q4_K block from test_q4k_dequant
        let block_hex = concat!(
            "7b0cb91253ce5952a294ffaa618f1265",
            "92054f3584280455a48353b03a5413e3",
            "7463367854ac33492588944a80f54879",
            "67476a6888e7f748ab99eb4c985d57f6",
            "984c383178aa05ba5c685556e03b6aaf",
            "15373a1d1836312a482823080c1a0237",
            "392a28472713390f36100a23f8272e06",
            "467c6af67c6b559a12262625162b0477",
            "908e57538a1989285636485a6a597b58",
        );
        let one_block = from_hex(block_hex);
        assert_eq!(one_block.len(), 144);

        let n: u32 = 4;
        let blocks_per_row: u32 = 2;
        let k: u32 = blocks_per_row * 256;

        // Build weight buffer: N rows, each with blocks_per_row copies of same block
        let mut weight_bytes: Vec<u8> = Vec::new();
        for _row in 0..n {
            for _blk in 0..blocks_per_row {
                weight_bytes.extend_from_slice(&one_block);
            }
        }

        // CPU reference: dequant one block, tile it
        let one_block_f32 = safetensors_to_f32(&one_block, DType::Q4_K);
        assert_eq!(one_block_f32.len(), 256);

        // Activation: simple ramp pattern
        let activation: Vec<f32> = (0..k).map(|i| (i as f32) / (k as f32)).collect();

        // CPU dot product per row (all rows identical since all use same blocks)
        let cpu_dot: f32 = (0..blocks_per_row).map(|b| {
            let col_base = (b * 256) as usize;
            one_block_f32.iter().enumerate()
                .map(|(j, &w)| w * activation[col_base + j])
                .sum::<f32>()
        }).sum();

        // -- GPU --
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            }),
        )
        .expect("No GPU adapter");

        let mut limits = wgpu::Limits::default();
        limits.max_buffer_size = 1u64 << 30;
        limits.max_storage_buffer_binding_size = 1u32 << 30;

        let mut features = wgpu::Features::empty();
        if adapter.features().contains(wgpu::Features::SUBGROUP) {
            features |= wgpu::Features::SUBGROUP;
        }

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("q4k-multi-test"),
                required_features: features,
                required_limits: limits,
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let p = Pipelines::new(device.clone(), queue.clone());

        let act_buf = p.upload_f32(&activation);
        let weight_buf = p.upload_bytes(&weight_bytes);

        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params { n: u32, k: u32, blocks_per_row: u32, _pad: u32 }
        let params = Params { n, k, blocks_per_row, _pad: 0 };
        let params_buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));

        let output_buf = p.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (n as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bg = p.create_bind_group(
            &p.q4k_matmul,
            &[
                act_buf.as_entire_binding(),
                weight_buf.as_entire_binding(),
                output_buf.as_entire_binding(),
                params_buf.as_entire_binding(),
            ],
        );

        let num_wg = (n + 3) / 4; // = 1 (4 rows, NR=4)
        let mut encoder = p.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            p.dispatch_in_pass(&mut pass, &p.q4k_matmul, &bg, (num_wg, 1, 1));
        }
        p.queue.submit(std::iter::once(encoder.finish()));

        let result = p.read_f32(&output_buf, n as usize);

        for r in 0..n as usize {
            let gpu_dot = result[r];
            let diff = (gpu_dot - cpu_dot).abs();
            let rel = if cpu_dot.abs() > 1e-8 { diff / cpu_dot.abs() } else { diff };
            eprintln!("Row {r}: CPU={cpu_dot:.6}, GPU={gpu_dot:.6}, diff={diff:.2e}, rel={rel:.2e}");
            assert!(
                diff < 0.05 || rel < 0.02,
                "Row {r}: Q4_K GPU vs CPU mismatch: gpu={gpu_dot:.6}, cpu={cpu_dot:.6}, diff={diff:.2e}"
            );
        }
    }

    #[test]
    fn test_q6k_dequant() {
        // First Q6_K superblock from output.weight in Qwen2.5-Coder-14B GGUF
        let raw = from_hex(concat!(
            "162af4414e1521f6ac0ebf0105983baf",
            "1e679e3157ad69dbd8a68f83900bb800",
            "51f05484432b61c6ebcba8d40a8e791c",
            "b234daa1f17e2611459d05e5c736789329",
            "e65bbf85b3f5c2102356bef34721f731",
            "1e013093f45a32de38fa2226a352a115",
            "1e50b209daa6eda17edaed546910e4a4",
            "e133e13a0dc232da331f70ab72b9546d",
            "659199ac40205d6d775656ac88a11a15",
            "e5959a12d19b198926a66e98899b4aad",
            "c62665abaf554a40a766618a623e456b",
            "8ab668990965a962a991a262aad563c0",
            "56c6486153805b99a752425a949ca5cd",
            "00",
        ));

        assert_eq!(raw.len(), 210, "Q6_K block is 210 bytes");
        let result = safetensors_to_f32(&raw, DType::Q6_K);
        assert_eq!(result.len(), 256);

        // Reference from Python dequant (first 16 values)
        let expected: [f32; 16] = [
            0.00782013, -0.01329422, -0.00782013, 0.01094818,
            0.00938416, 0.00078201, -0.00078201, 0.00938416,
            0.00156403, 0.02189636, 0.00860214, -0.00078201,
            0.01173019, -0.00156403, 0.00782013, -0.01173019,
        ];

        for (i, (&got, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let diff = (got - exp).abs();
            assert!(
                diff < 1e-4,
                "Q6K val[{i}]: got={got:.6}, expected={exp:.6}, diff={diff:.2e}"
            );
        }
    }

    #[test]
    fn test_q4k_gpu_matmul() {
        // Q4_K block from qwen2.5-coder-14b q_proj layer 0, row 0
        let block_hex = "7b0cb91253ce5952a294ffaa618f12659205\
4f3584280455a48353b03a5413e37463367854ac33492588944a80f5487967476a\
6888e7f748ab99eb4c985d57f6984c383178aa05ba5c685556e03b6aaf1537\
3a1d1836312a482823080c1a0237392a28472713390f36100a23f8272e06467c6a\
f67c6b559a12262625162b0477908e57538a1989285636485a6a597b58";
        let block = from_hex(block_hex);
        assert_eq!(block.len(), 144, "Q4_K block = 144 bytes");

        // CPU dequant reference (same algorithm as shader)
        let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
        let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
        let scales = &block[4..16];
        let qs = &block[16..144];

        fn gsm(j: usize, sc: &[u8]) -> (f32, f32) {
            if j < 4 {
                ((sc[j] & 63) as f32, (sc[j + 4] & 63) as f32)
            } else {
                let s = ((sc[j + 4] & 0xF) | ((sc[j - 4] >> 6) << 4)) as f32;
                let m = ((sc[j + 4] >> 4) | ((sc[j] >> 6) << 4)) as f32;
                (s, m)
            }
        }

        let mut ref_vals = [0.0f32; 256];
        for grp in 0..4 {
            let (sc1, m1) = gsm(grp * 2, scales);
            let (sc2, m2) = gsm(grp * 2 + 1, scales);
            for l in 0..32 {
                let qb = qs[grp * 32 + l];
                ref_vals[grp * 64 + l] = d * sc1 * (qb & 0xF) as f32 - dmin * m1;
                ref_vals[grp * 64 + l + 32] = d * sc2 * (qb >> 4) as f32 - dmin * m2;
            }
        }

        // Test: activation = [1,1,1,...,1] → dot = sum of all dequanted values
        let expected_sum: f32 = ref_vals.iter().sum();

        // Run on GPU
        let backend = crate::backend::create_wgpu_backend();
        let p = &backend.pipelines;

        // Upload activation: 256 f32 ones
        let act_data = vec![1.0f32; 256];
        let act_buf = p.upload_f32(&act_data);

        // Upload Q4_K block as raw bytes (padded to u32 alignment — already is)
        let weight_buf = p.upload_bytes(&block);

        // Create params: n=1 (1 output row), k=256, blocks_per_row=1
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Q4KParams { n: u32, k: u32, blocks_per_row: u32, _pad: u32 }
        let params = Q4KParams { n: 1, k: 256, blocks_per_row: 1, _pad: 0 };
        let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

        // Dispatch
        let mut enc = p.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = crate::backend::wgpu::dispatch::q4k_matmul(p, &mut enc, &act_buf, &weight_buf, 1, 256);
        p.queue.submit(std::iter::once(enc.finish()));

        // Read back
        let result = p.read_f32(&output, 1);

        let diff = (result[0] - expected_sum).abs();
        eprintln!("Q4K GPU test: got={:.6}, expected={:.6}, diff={:.2e}", result[0], expected_sum, diff);
        assert!(diff < 0.01, "Q4K GPU matmul mismatch: got={}, expected={}, diff={}", result[0], expected_sum, diff);
    }

    #[test]
    fn test_q4k_gpu_matmul_multiblock() {
        // Test N=2, K=512 (2 superblocks per row) from real model data
        let model_path = std::path::Path::new("/Users/mastercyb/llm/qwen2.5-coder-14b-abl.model");
        if !model_path.exists() { eprintln!("SKIP: model not found"); return; }

        let data = std::fs::read(model_path).unwrap();
        let ws = data.windows(11).position(|w| w == b"~~~weights\n").unwrap() + 11;
        let offset = 17739776usize; // q_proj offset
        let bpr = 20usize; // 5120/256

        // Read 2 blocks from row 0, 2 blocks from row 1
        let mut test_weights = Vec::with_capacity(576);
        test_weights.extend_from_slice(&data[ws+offset..ws+offset+288]); // row 0
        test_weights.extend_from_slice(&data[ws+offset+bpr*144..ws+offset+bpr*144+288]); // row 1

        // CPU reference dequant
        fn dequant_block(block: &[u8]) -> Vec<f32> {
            let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
            let dm = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
            let sc = &block[4..16]; let qs = &block[16..144];
            fn gsm(j: usize, sc: &[u8]) -> (f32, f32) {
                if j < 4 { ((sc[j]&63) as f32, (sc[j+4]&63) as f32) }
                else { (((sc[j+4]&0xF)|((sc[j-4]>>6)<<4)) as f32, ((sc[j+4]>>4)|((sc[j]>>6)<<4)) as f32) }
            }
            let mut v = vec![0.0f32; 256];
            for g in 0..4 {
                let (s1,m1) = gsm(g*2,sc); let (s2,m2) = gsm(g*2+1,sc);
                for l in 0..32 { let q=qs[g*32+l]; v[g*64+l]=d*s1*(q&0xF) as f32-dm*m1; v[g*64+l+32]=d*s2*(q>>4) as f32-dm*m2; }
            }
            v
        }

        let mut row0_vals = dequant_block(&test_weights[0..144]);
        row0_vals.extend(dequant_block(&test_weights[144..288]));
        let mut row1_vals = dequant_block(&test_weights[288..432]);
        row1_vals.extend(dequant_block(&test_weights[432..576]));

        let act = vec![1.0f32; 512];
        let exp0: f32 = act.iter().zip(row0_vals.iter()).map(|(a,w)| a*w).sum();
        let exp1: f32 = act.iter().zip(row1_vals.iter()).map(|(a,w)| a*w).sum();

        // GPU
        let backend = crate::backend::create_wgpu_backend();
        let p = &backend.pipelines;
        let act_buf = p.upload_f32(&act);
        let weight_buf = p.upload_bytes(&test_weights);

        let mut enc = p.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = crate::backend::wgpu::dispatch::q4k_matmul(p, &mut enc, &act_buf, &weight_buf, 2, 512);
        p.queue.submit(std::iter::once(enc.finish()));
        let result = p.read_f32(&output, 2);

        eprintln!("Q4K multi: row0 got={:.6} exp={:.6} diff={:.2e}", result[0], exp0, (result[0]-exp0).abs());
        eprintln!("Q4K multi: row1 got={:.6} exp={:.6} diff={:.2e}", result[1], exp1, (result[1]-exp1).abs());
        assert!((result[0]-exp0).abs() < 0.01, "row0 mismatch");
        assert!((result[1]-exp1).abs() < 0.01, "row1 mismatch");
    }

    #[test]
    fn test_q4k_gpu_fullsize() {
        // Test with real model row: N=1, K=5120 (20 superblocks)
        let model_path = std::path::Path::new("/Users/mastercyb/llm/qwen2.5-coder-14b-abl.model");
        if !model_path.exists() { eprintln!("SKIP"); return; }

        let data = std::fs::read(model_path).unwrap();
        let ws = data.windows(11).position(|w| w == b"~~~weights\n").unwrap() + 11;
        let offset = 17739776usize;
        let bpr = 20usize;

        let row_data = data[ws+offset..ws+offset+bpr*144].to_vec();

        // CPU dequant
        fn dequant_block(block: &[u8]) -> Vec<f32> {
            let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
            let dm = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
            let sc = &block[4..16]; let qs = &block[16..144];
            fn gsm(j: usize, sc: &[u8]) -> (f32, f32) {
                if j < 4 { ((sc[j]&63) as f32, (sc[j+4]&63) as f32) }
                else { (((sc[j+4]&0xF)|((sc[j-4]>>6)<<4)) as f32, ((sc[j+4]>>4)|((sc[j]>>6)<<4)) as f32) }
            }
            let mut v = vec![0.0f32; 256];
            for g in 0..4 {
                let (s1,m1) = gsm(g*2,sc); let (s2,m2) = gsm(g*2+1,sc);
                for l in 0..32 { let q=qs[g*32+l]; v[g*64+l]=d*s1*(q&0xF) as f32-dm*m1; v[g*64+l+32]=d*s2*(q>>4) as f32-dm*m2; }
            }
            v
        }

        let mut all_vals = Vec::with_capacity(5120);
        for b in 0..bpr { all_vals.extend(dequant_block(&row_data[b*144..(b+1)*144])); }

        let act = vec![1.0f32; 5120];
        let expected: f32 = act.iter().zip(all_vals.iter()).map(|(a,w)| a*w).sum();

        let backend = crate::backend::create_wgpu_backend();
        let p = &backend.pipelines;
        let act_buf = p.upload_f32(&act);
        let weight_buf = p.upload_bytes(&row_data);

        let mut enc = p.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = crate::backend::wgpu::dispatch::q4k_matmul(p, &mut enc, &act_buf, &weight_buf, 1, 5120);
        p.queue.submit(std::iter::once(enc.finish()));
        let result = p.read_f32(&output, 1);

        eprintln!("Q4K fullsize: got={:.6} exp={:.6} diff={:.2e}", result[0], expected, (result[0]-expected).abs());
        assert!((result[0]-expected).abs() < 0.1, "fullsize mismatch: got={} exp={}", result[0], expected);
    }

    #[test]
    fn test_q6k_gpu_matmul() {
        // Q6_K GPU dequant+matmul roundtrip test
        // Uses the same superblock from test_q6k_dequant
        let raw = from_hex(concat!(
            "162af4414e1521f6ac0ebf0105983baf",
            "1e679e3157ad69dbd8a68f83900bb800",
            "51f05484432b61c6ebcba8d40a8e791c",
            "b234daa1f17e2611459d05e5c736789329",
            "e65bbf85b3f5c2102356bef34721f731",
            "1e013093f45a32de38fa2226a352a115",
            "1e50b209daa6eda17edaed546910e4a4",
            "e133e13a0dc232da331f70ab72b9546d",
            "659199ac40205d6d775656ac88a11a15",
            "e5959a12d19b198926a66e98899b4aad",
            "c62665abaf554a40a766618a623e456b",
            "8ab668990965a962a991a262aad563c0",
            "56c6486153805b99a752425a949ca5cd",
            "00",
        ));
        assert_eq!(raw.len(), 210, "Q6_K block is 210 bytes");

        // CPU reference: dequant and sum (activation = all 1s)
        let cpu_vals = safetensors_to_f32(&raw, DType::Q6_K);
        assert_eq!(cpu_vals.len(), 256);
        let expected_sum: f32 = cpu_vals.iter().sum();

        // Run on GPU
        let backend = crate::backend::create_wgpu_backend();
        let p = &backend.pipelines;

        let act_data = vec![1.0f32; 256];
        let act_buf = p.upload_f32(&act_data);
        let weight_buf = p.upload_bytes(&raw);

        let mut enc = p.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = crate::backend::wgpu::dispatch::q6k_matmul(p, &mut enc, &act_buf, &weight_buf, 1, 256);
        p.queue.submit(std::iter::once(enc.finish()));

        let result = p.read_f32(&output, 1);
        let diff = (result[0] - expected_sum).abs();
        eprintln!("Q6K GPU test: got={:.6}, expected={:.6}, diff={:.2e}", result[0], expected_sum, diff);
        assert!(diff < 0.01, "Q6K GPU matmul mismatch: got={}, expected={}, diff={}", result[0], expected_sum, diff);
    }

    #[test]
    fn test_q5k_dequant() {
        // Construct a synthetic Q5_K block (176 bytes = 256 values)
        // Layout: d(f16=2) + dmin(f16=2) + scales(12) + qh(32) + qs(128) = 176
        let mut block = vec![0u8; 176];

        // d = 1.0 as f16
        let d_bits = half::f16::from_f32(1.0).to_bits();
        block[0] = (d_bits & 0xFF) as u8;
        block[1] = (d_bits >> 8) as u8;
        // dmin = 0.5 as f16
        let dmin_bits = half::f16::from_f32(0.5).to_bits();
        block[2] = (dmin_bits & 0xFF) as u8;
        block[3] = (dmin_bits >> 8) as u8;

        // scales[12]: set sub-block 0 scale=2, min=1; rest zero
        // For j=0: sc = scales[0] & 63, m = scales[4] & 63
        block[4] = 2; // scale for sub-block 0
        block[8] = 1; // min for sub-block 0

        // qs[128] at offset 48: set first byte to 0x32 (lo=2, hi=3)
        block[48] = 0x32;
        // qh[32] at offset 16: set bit 0 for value 0 (5th bit = 1)
        block[16] = 0x01; // bit 0 = value index 0

        let result = safetensors_to_f32(&block, DType::Q5_K);
        assert_eq!(result.len(), 256);

        // Value 0: lo_nib=2, qh=1, q=2|(1<<4)=18, val = d*sc*q - dmin*m = 1.0*2*18 - 0.5*1 = 35.5
        let expected_0 = 1.0f32 * 2.0 * 18.0 - 0.5 * 1.0;
        assert!((result[0] - expected_0).abs() < 1e-3, "Q5K val[0]: got={}, expected={}", result[0], expected_0);

        // Value 1: qs[1]=0, lo_nib=0, qh bit 1 not set, q=0, val = 1.0*2*0 - 0.5*1 = -0.5
        let expected_1 = 1.0f32 * 2.0 * 0.0 - 0.5 * 1.0;
        assert!((result[1] - expected_1).abs() < 1e-3, "Q5K val[1]: got={}, expected={}", result[1], expected_1);

        // Value 32: hi_nib of qs[0]=0x32 is 3, q=3|(0<<4)=3, sub-block 1 with scale=0, min=0
        // Sub-block 1 has scale=scales[5]&63=0 and min=scales[9]&63=0 → val = 0
        let expected_32 = 0.0f32;
        assert!((result[32] - expected_32).abs() < 1e-3, "Q5K val[32]: got={}, expected={}", result[32], expected_32);
    }

    #[test]
    fn test_q5k_gpu_matmul() {
        // Synthetic Q5_K block test on GPU
        let mut block = vec![0u8; 176];
        let d_bits = half::f16::from_f32(0.1).to_bits();
        block[0] = (d_bits & 0xFF) as u8;
        block[1] = (d_bits >> 8) as u8;
        let dmin_bits = half::f16::from_f32(0.05).to_bits();
        block[2] = (dmin_bits & 0xFF) as u8;
        block[3] = (dmin_bits >> 8) as u8;

        // Set scale=3, min=2 for sub-block 0
        block[4] = 3;
        block[8] = 2;
        // Set scale=1, min=1 for sub-block 1
        block[5] = 1;
        block[9] = 1;

        // Fill qs with pattern
        for i in 0..128 {
            block[48 + i] = ((i * 7 + 3) % 256) as u8;
        }
        // Fill qh with pattern
        for i in 0..32 {
            block[16 + i] = ((i * 13 + 5) % 256) as u8;
        }

        // CPU reference
        let cpu_vals = safetensors_to_f32(&block, DType::Q5_K);
        assert_eq!(cpu_vals.len(), 256);
        let expected_sum: f32 = cpu_vals.iter().sum();

        // GPU
        let backend = crate::backend::create_wgpu_backend();
        let p = &backend.pipelines;
        let act_data = vec![1.0f32; 256];
        let act_buf = p.upload_f32(&act_data);
        let weight_buf = p.upload_bytes(&block);

        let mut enc = p.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = crate::backend::wgpu::dispatch::q5k_matmul(p, &mut enc, &act_buf, &weight_buf, 1, 256);
        p.queue.submit(std::iter::once(enc.finish()));

        let result = p.read_f32(&output, 1);
        let diff = (result[0] - expected_sum).abs();
        eprintln!("Q5K GPU test: got={:.6}, expected={:.6}, diff={:.2e}", result[0], expected_sum, diff);
        assert!(diff < 0.1, "Q5K GPU matmul mismatch: got={}, expected={}, diff={}", result[0], expected_sum, diff);
    }

    #[test]
    fn test_q3k_dequant() {
        // Synthetic Q3_K block (110 bytes = 256 values)
        // Layout: hmask(32) + qs(64) + scales(12) + d(f16=2) = 110
        let mut block = vec![0u8; 110];

        // d = 1.0 as f16 at offset 108
        let d_bits = half::f16::from_f32(1.0).to_bits();
        block[108] = (d_bits & 0xFF) as u8;
        block[109] = (d_bits >> 8) as u8;

        // scales[12] at offset 96: set scale for sub-block 0 to raw value 34 (34-32=2)
        // For j=0: us = (scales[0] & 0xF) | (((scales[8] >> 0) & 3) << 4)
        // We want us=34: low4=2, high2=2 → scales[0]=(scales[0]&0xF0)|2=2, scales[8]=(scales[8]&~3)|2=2
        block[96] = 0x02; // low 4 bits = 2
        block[104] = 0x02; // bits [1:0] = 2 → us = 2 | (2<<4) = 34 → scale = 34-32 = 2

        // qs[64] at offset 32: value 0 → 2 bits at qs[0] bits [1:0]
        // Set qs[0] = 0x03 → q_lo = 3 for value 0
        block[32] = 0x03;

        // hmask[32] at offset 0: set bit 0 for value 0 → qh = 1
        block[0] = 0x01;

        let result = safetensors_to_f32(&block, DType::Q3_K);
        assert_eq!(result.len(), 256);

        // Value 0: ql=3, hm=1, q3=(3|(1<<2))-4 = 7-4 = 3, scale=2, val = 1.0 * 2 * 3 = 6.0
        let expected_0 = 6.0f32;
        assert!((result[0] - expected_0).abs() < 1e-3, "Q3K val[0]: got={}, expected={}", result[0], expected_0);
    }

    #[test]
    fn test_q3k_gpu_matmul() {
        // Synthetic Q3_K GPU test
        let mut block = vec![0u8; 110];
        let d_bits = half::f16::from_f32(0.1).to_bits();
        block[108] = (d_bits & 0xFF) as u8;
        block[109] = (d_bits >> 8) as u8;

        // Fill scales with pattern that gives known values after unpacking
        // Use raw value 33 for all sub-blocks (scale = 33-32 = 1)
        // For j=0: us = (scales[0]&0xF) | (((scales[8]>>(4*(0&1))) & 3)<<4)
        // Want 33: low4=1, high2=2 → raw[0]|=1, raw[8]|=2
        for i in 0..4 {
            block[96 + i] = 0x11; // low nib = 1 for j and j+8
            block[96 + 4 + i] = 0x11; // for j+4 and j+12
        }
        block[96 + 8] = 0x22; // high 2 bits for j=0,1
        block[96 + 9] = 0x22; // high 2 bits for j=2,3
        block[96 + 10] = 0x22; // high 2 bits for j=4,5
        block[96 + 11] = 0x22; // high 2 bits for j=6,7

        // Fill qs and hmask with deterministic pattern
        for i in 0..64 {
            block[32 + i] = ((i * 17 + 7) % 256) as u8;
        }
        for i in 0..32 {
            block[i] = ((i * 11 + 3) % 256) as u8;
        }

        let cpu_vals = safetensors_to_f32(&block, DType::Q3_K);
        assert_eq!(cpu_vals.len(), 256);
        let expected_sum: f32 = cpu_vals.iter().sum();

        let backend = crate::backend::create_wgpu_backend();
        let p = &backend.pipelines;
        let act_data = vec![1.0f32; 256];
        let act_buf = p.upload_f32(&act_data);
        let weight_buf = p.upload_bytes(&block);

        let mut enc = p.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = crate::backend::wgpu::dispatch::q3k_matmul(p, &mut enc, &act_buf, &weight_buf, 1, 256);
        p.queue.submit(std::iter::once(enc.finish()));

        let result = p.read_f32(&output, 1);
        let diff = (result[0] - expected_sum).abs();
        eprintln!("Q3K GPU test: got={:.6}, expected={:.6}, diff={:.2e}", result[0], expected_sum, diff);
        assert!(diff < 0.1, "Q3K GPU matmul mismatch: got={}, expected={}, diff={}", result[0], expected_sum, diff);
    }

    #[test]
    fn test_q2k_dequant() {
        // Synthetic Q2_K block (84 bytes = 256 values)
        // Layout: scales(16) + qs(64) + d(f16=2) + dmin(f16=2) = 84
        let mut block = vec![0u8; 84];

        // d = 1.0 as f16 at offset 80
        let d_bits = half::f16::from_f32(1.0).to_bits();
        block[80] = (d_bits & 0xFF) as u8;
        block[81] = (d_bits >> 8) as u8;
        // dmin = 0.5 as f16 at offset 82
        let dmin_bits = half::f16::from_f32(0.5).to_bits();
        block[82] = (dmin_bits & 0xFF) as u8;
        block[83] = (dmin_bits >> 8) as u8;

        // scales[16] at offset 0: sub-block 0 scale=3, min=2 → byte = (2<<4)|3 = 0x23
        block[0] = 0x23;

        // qs[64] at offset 16: value 0 at qs[0] bits [1:0]
        // Set qs[0] = 0x03 → q2 = 3 for value 0
        block[16] = 0x03;

        let result = safetensors_to_f32(&block, DType::Q2_K);
        assert_eq!(result.len(), 256);

        // Value 0: sc=3, m=2, q2=3, val = d*sc*q2 - dmin*m = 1.0*3*3 - 0.5*2 = 8.0
        let expected_0 = 8.0f32;
        assert!((result[0] - expected_0).abs() < 1e-3, "Q2K val[0]: got={}, expected={}", result[0], expected_0);

        // Value 1: at qs[0] bits [3:2] = 0, so q2=0, val = 1.0*3*0 - 0.5*2 = -1.0
        let expected_1 = -1.0f32;
        assert!((result[1] - expected_1).abs() < 1e-3, "Q2K val[1]: got={}, expected={}", result[1], expected_1);
    }

    #[test]
    fn test_q2k_gpu_matmul() {
        // Synthetic Q2_K GPU test
        let mut block = vec![0u8; 84];
        let d_bits = half::f16::from_f32(0.1).to_bits();
        block[80] = (d_bits & 0xFF) as u8;
        block[81] = (d_bits >> 8) as u8;
        let dmin_bits = half::f16::from_f32(0.05).to_bits();
        block[82] = (dmin_bits & 0xFF) as u8;
        block[83] = (dmin_bits >> 8) as u8;

        // Fill scales: each sub-block gets scale=2, min=1 → byte = (1<<4)|2 = 0x12
        for i in 0..16 {
            block[i] = 0x12;
        }

        // Fill qs with pattern
        for i in 0..64 {
            block[16 + i] = ((i * 19 + 11) % 256) as u8;
        }

        let cpu_vals = safetensors_to_f32(&block, DType::Q2_K);
        assert_eq!(cpu_vals.len(), 256);
        let expected_sum: f32 = cpu_vals.iter().sum();

        let backend = crate::backend::create_wgpu_backend();
        let p = &backend.pipelines;
        let act_data = vec![1.0f32; 256];
        let act_buf = p.upload_f32(&act_data);
        let weight_buf = p.upload_bytes(&block);

        let mut enc = p.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = crate::backend::wgpu::dispatch::q2k_matmul(p, &mut enc, &act_buf, &weight_buf, 1, 256);
        p.queue.submit(std::iter::once(enc.finish()));

        let result = p.read_f32(&output, 1);
        let diff = (result[0] - expected_sum).abs();
        eprintln!("Q2K GPU test: got={:.6}, expected={:.6}, diff={:.2e}", result[0], expected_sum, diff);
        assert!(diff < 0.1, "Q2K GPU matmul mismatch: got={}, expected={}, diff={}", result[0], expected_sum, diff);
    }

    /// Test Q6_K lm_head matmul with real coder-14b weights at full scale (152064×5120)
    /// Compare GPU output rows 0, 1000, 50000, 100000, 150000 with CPU reference
    #[test]
    fn test_q6k_lm_head_real() {
        use std::sync::Arc;
        use std::path::Path;
        use crate::cyb_format::LoadedModel;

        let model_path = Path::new("/Users/mastercyb/llm/qwen2.5-coder-14b-abl.model");
        if !model_path.exists() { eprintln!("Skipping: model not found"); return; }

        let lm = LoadedModel::load(model_path).expect("Failed to load");
        let lm_w = lm.weights.get("lm_head.weight").expect("no lm_head");
        let hidden_size = 5120usize;
        let vocab_size = 152064usize;

        // CPU reference: dequant lm_head fully
        eprintln!("Dequanting lm_head Q6_K to f32 ({}×{})...", vocab_size, hidden_size);
        let lm_f32 = safetensors_to_f32(&lm_w.data, lm_w.dtype);
        assert_eq!(lm_f32.len(), vocab_size * hidden_size, "lm_head f32 size mismatch");
        eprintln!("Dequanted {} values", lm_f32.len());

        // Random-ish activation: use first embed row as activation
        let embed_w = lm.weights.get("model.embed_tokens.weight").expect("no embed");
        let embed_f32 = safetensors_to_f32(&embed_w.data, embed_w.dtype);
        let activation = &embed_f32[27 * hidden_size..28 * hidden_size]; // token 27

        // CPU matmul for specific rows
        let test_rows = [0usize, 1000, 50000, 100000, 150000];
        let cpu_vals: Vec<f32> = test_rows.iter().map(|&r| {
            let row = &lm_f32[r * hidden_size..(r + 1) * hidden_size];
            row.iter().zip(activation.iter()).map(|(&w, &a)| w * a).sum()
        }).collect();

        // GPU
        let backend = crate::backend::create_wgpu_backend();
        let p = &backend.pipelines;

        let act_buf = p.upload_f32(activation);
        let weight_buf = p.upload_bytes(&lm_w.data);

        let mut enc = p.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = crate::backend::wgpu::dispatch::q6k_matmul(
            p, &mut enc, &act_buf, &weight_buf, vocab_size as u32, hidden_size as u32,
        );
        p.queue.submit(std::iter::once(enc.finish()));

        let gpu_all = p.read_f32(&output, vocab_size);

        // Compare
        let mut max_diff = 0.0f32;
        for (i, &row) in test_rows.iter().enumerate() {
            let cpu = cpu_vals[i];
            let gpu = gpu_all[row];
            let diff = (cpu - gpu).abs();
            let rel = if cpu.abs() > 1e-6 { diff / cpu.abs() } else { diff };
            max_diff = max_diff.max(diff);
            eprintln!("row {row:>6}: CPU={cpu:>10.4} GPU={gpu:>10.4} diff={diff:.2e} rel={rel:.2e}");
        }
        // Also check top-5 GPU argmax
        let mut indexed: Vec<(usize, f32)> = gpu_all.iter().enumerate().map(|(i,&v)| (i,v)).collect();
        indexed.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap());
        eprintln!("GPU top-5: {:?}", &indexed[..5]);
        // CPU top-5
        let cpu_full: Vec<f32> = (0..vocab_size).map(|r| {
            lm_f32[r * hidden_size..(r + 1) * hidden_size].iter()
                .zip(activation.iter()).map(|(&w, &a)| w * a).sum()
        }).collect();
        let mut cpu_indexed: Vec<(usize, f32)> = cpu_full.iter().enumerate().map(|(i,&v)| (i,v)).collect();
        cpu_indexed.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap());
        eprintln!("CPU top-5: {:?}", &cpu_indexed[..5]);

        assert!(max_diff < 0.1, "lm_head GPU/CPU divergence too large: {max_diff}");
    }
}
