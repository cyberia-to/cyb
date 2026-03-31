//! Quantize safetensors models → GGUF Q4_0/Q8_0 format.
//!
//! Pure Rust, zero Python. Reads FP16/BF16/FP32 safetensors, block-quantizes
//! weights, writes standard GGUF v3 that cyb-llm (and llama.cpp) can load.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

// ── GGUF constants ───────────────────────────────────────────────

const GGUF_MAGIC: u32 = 0x4655_4747; // "GGUF" LE
const GGUF_VERSION: u32 = 3;

// GGML tensor types
const GGML_TYPE_F32: u32 = 0;
const GGML_TYPE_F16: u32 = 1;
const GGML_TYPE_Q4_0: u32 = 2;
const GGML_TYPE_Q8_0: u32 = 8;

// GGUF metadata value types
const GGUF_TYPE_UINT32: u32 = 4;
const GGUF_TYPE_INT32: u32 = 5;
const GGUF_TYPE_FLOAT32: u32 = 6;
const GGUF_TYPE_STRING: u32 = 8;
const GGUF_TYPE_UINT64: u32 = 10;

// Block sizes
const Q_BLOCK_SIZE: usize = 32;
const Q4_0_BLOCK_BYTES: usize = 2 + 16; // f16 scale + 32 nibbles
const Q8_0_BLOCK_BYTES: usize = 2 + 32; // f16 scale + 32 int8s

/// Quantization target
#[derive(Debug, Clone, Copy)]
pub enum QuantType {
    Q4_0,
    Q8_0,
    F16,
}

impl QuantType {
    pub fn ggml_type(&self) -> u32 {
        match self {
            Self::Q4_0 => GGML_TYPE_Q4_0,
            Self::Q8_0 => GGML_TYPE_Q8_0,
            Self::F16 => GGML_TYPE_F16,
        }
    }

    /// Bytes needed for `n` elements
    pub fn bytes_for(&self, n: usize) -> usize {
        match self {
            Self::Q4_0 => {
                let blocks = (n + Q_BLOCK_SIZE - 1) / Q_BLOCK_SIZE;
                blocks * Q4_0_BLOCK_BYTES
            }
            Self::Q8_0 => {
                let blocks = (n + Q_BLOCK_SIZE - 1) / Q_BLOCK_SIZE;
                blocks * Q8_0_BLOCK_BYTES
            }
            Self::F16 => n * 2,
        }
    }
}

// ── Quantization kernels ─────────────────────────────────────────

/// Quantize f32 slice → Q4_0 blocks. Returns raw bytes.
///
/// Q4_0 block layout (18 bytes per 32 weights):
///   d: f16 (max abs / 8, scale factor)
///   qs: [u8; 16] (32 x 4-bit signed ints packed as nibbles, offset by +8)
pub fn quantize_q4_0(data: &[f32]) -> Vec<u8> {
    let n_blocks = (data.len() + Q_BLOCK_SIZE - 1) / Q_BLOCK_SIZE;
    let mut out = Vec::with_capacity(n_blocks * Q4_0_BLOCK_BYTES);

    for block_idx in 0..n_blocks {
        let start = block_idx * Q_BLOCK_SIZE;
        let end = (start + Q_BLOCK_SIZE).min(data.len());
        let block = &data[start..end];

        // Find max absolute value
        let amax = block.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        let d = amax / 8.0; // scale: map [-8d, 7d] to [-8, 7]
        let id = if d != 0.0 { 1.0 / d } else { 0.0 };

        // Write scale as f16
        let d_f16 = half::f16::from_f32(d);
        out.extend_from_slice(&d_f16.to_le_bytes());

        // Quantize and pack nibbles
        let mut qs = [0u8; 16];
        for i in 0..16 {
            let lo_idx = i;
            let hi_idx = i + 16;
            let x0 = if lo_idx < block.len() {
                (block[lo_idx] * id + 8.5).clamp(0.0, 15.0) as u8
            } else {
                8 // zero point
            };
            let x1 = if hi_idx < block.len() {
                (block[hi_idx] * id + 8.5).clamp(0.0, 15.0) as u8
            } else {
                8
            };
            qs[i] = x0 | (x1 << 4);
        }
        out.extend_from_slice(&qs);
    }

    out
}

/// Quantize f32 slice → Q8_0 blocks. Returns raw bytes.
///
/// Q8_0 block layout (34 bytes per 32 weights):
///   d: f16 (max abs / 127, scale factor)
///   qs: [i8; 32] (32 x 8-bit signed ints)
pub fn quantize_q8_0(data: &[f32]) -> Vec<u8> {
    let n_blocks = (data.len() + Q_BLOCK_SIZE - 1) / Q_BLOCK_SIZE;
    let mut out = Vec::with_capacity(n_blocks * Q8_0_BLOCK_BYTES);

    for block_idx in 0..n_blocks {
        let start = block_idx * Q_BLOCK_SIZE;
        let end = (start + Q_BLOCK_SIZE).min(data.len());
        let block = &data[start..end];

        let amax = block.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        let d = amax / 127.0;
        let id = if d != 0.0 { 1.0 / d } else { 0.0 };

        let d_f16 = half::f16::from_f32(d);
        out.extend_from_slice(&d_f16.to_le_bytes());

        for i in 0..Q_BLOCK_SIZE {
            let val = if i < block.len() {
                (block[i] * id).round().clamp(-128.0, 127.0) as i8
            } else {
                0i8
            };
            out.push(val as u8);
        }
    }

    out
}

/// Convert f32 slice → f16 bytes
pub fn to_f16(data: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    for &x in data {
        let h = half::f16::from_f32(x);
        out.extend_from_slice(&h.to_le_bytes());
    }
    out
}

// ── Weight conversion helpers ────────────────────────────────────

/// Decode raw weight bytes to f32, given the source dtype
pub fn weights_to_f32(data: &[u8], dtype: &str) -> Vec<f32> {
    match dtype {
        "F32" => data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        "F16" => data
            .chunks_exact(2)
            .map(|c| half::f16::from_le_bytes([c[0], c[1]]).to_f32())
            .collect(),
        "BF16" => data
            .chunks_exact(2)
            .map(|c| {
                // BF16 → F32: just shift left 16 bits
                let bits = (c[1] as u32) << 24 | (c[0] as u32) << 16;
                f32::from_bits(bits)
            })
            .collect(),
        _ => {
            eprintln!("Unknown dtype: {dtype}, treating as F16");
            data.chunks_exact(2)
                .map(|c| half::f16::from_le_bytes([c[0], c[1]]).to_f32())
                .collect()
        }
    }
}

// ── GGUF writer ──────────────────────────────────────────────────

/// Information about a tensor to write
struct TensorEntry {
    name: String,
    shape: Vec<u64>,
    ggml_type: u32,
    data: Vec<u8>,
}

/// Write a complete GGUF file from safetensors model.
///
/// `model_dir`: directory containing *.safetensors + config.json
/// `output_path`: where to write the .gguf file
/// `quant`: quantization type for weight tensors
/// `skip_1d`: if true, keep 1D tensors (norms, biases) as F32 instead of quantizing
pub fn convert_safetensors_to_gguf(
    model_dir: &Path,
    output_path: &Path,
    quant: QuantType,
    skip_1d: bool,
) -> io::Result<ConvertStats> {
    let mut stats = ConvertStats::default();

    // 1. Read config.json for model metadata
    let config = read_config(model_dir)?;

    // 2. Find and read all safetensors files
    let mut st_files: Vec<_> = std::fs::read_dir(model_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "safetensors")
                .unwrap_or(false)
                && !e
                    .file_name()
                    .to_str()
                    .unwrap_or("")
                    .contains("index")
        })
        .map(|e| e.path())
        .collect();
    st_files.sort();

    if st_files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No safetensors files found",
        ));
    }

    // 3. Parse all tensors
    let mut tensors: Vec<TensorEntry> = Vec::new();

    for st_path in &st_files {
        let file_data = std::fs::read(st_path)?;
        if file_data.len() < 8 {
            continue;
        }

        // Safetensors header: u64 LE header_size, then JSON header, then data
        let header_size =
            u64::from_le_bytes(file_data[..8].try_into().unwrap()) as usize;
        if 8 + header_size > file_data.len() {
            continue;
        }

        let header_json = std::str::from_utf8(&file_data[8..8 + header_size])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let header: serde_json::Value = serde_json::from_str(header_json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let data_start = 8 + header_size;

        if let Some(obj) = header.as_object() {
            for (name, info) in obj {
                if name == "__metadata__" {
                    continue;
                }
                let dtype = info["dtype"].as_str().unwrap_or("F16");
                let shape: Vec<u64> = info["shape"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
                    .unwrap_or_default();
                let offsets = info["data_offsets"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_u64())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if offsets.len() < 2 {
                    continue;
                }

                let begin = data_start + offsets[0] as usize;
                let end = data_start + offsets[1] as usize;
                if end > file_data.len() {
                    eprintln!("  WARN: {name} data out of bounds, skipping");
                    continue;
                }

                let raw_data = &file_data[begin..end];
                let n_elements: u64 = shape.iter().product();
                let is_1d = shape.len() <= 1;

                // Decide: quantize or keep as-is
                let (ggml_type, quantized_data) =
                    if (is_1d && skip_1d) || n_elements < 32 {
                        // Keep small/1D tensors as F32 (norms, biases, embeddings)
                        let f32_data = weights_to_f32(raw_data, dtype);
                        let bytes: Vec<u8> = f32_data
                            .iter()
                            .flat_map(|x| x.to_le_bytes())
                            .collect();
                        stats.kept_f32 += 1;
                        (GGML_TYPE_F32, bytes)
                    } else {
                        let f32_data = weights_to_f32(raw_data, dtype);
                        match quant {
                            QuantType::Q4_0 => {
                                stats.quantized_q4 += 1;
                                (GGML_TYPE_Q4_0, quantize_q4_0(&f32_data))
                            }
                            QuantType::Q8_0 => {
                                stats.quantized_q8 += 1;
                                (GGML_TYPE_Q8_0, quantize_q8_0(&f32_data))
                            }
                            QuantType::F16 => {
                                stats.kept_f16 += 1;
                                (GGML_TYPE_F16, to_f16(&f32_data))
                            }
                        }
                    };

                stats.input_bytes += raw_data.len() as u64;
                stats.output_bytes += quantized_data.len() as u64;

                tensors.push(TensorEntry {
                    name: name.clone(),
                    shape,
                    ggml_type,
                    data: quantized_data,
                });
            }
        }
    }

    // Sort tensors by name for reproducibility
    tensors.sort_by(|a, b| a.name.cmp(&b.name));
    stats.n_tensors = tensors.len();

    // 4. Build metadata KV pairs
    let metadata = build_metadata(&config);

    // 5. Write GGUF file
    write_gguf(output_path, &metadata, &tensors)?;

    Ok(stats)
}

#[derive(Default)]
pub struct ConvertStats {
    pub n_tensors: usize,
    pub quantized_q4: usize,
    pub quantized_q8: usize,
    pub kept_f32: usize,
    pub kept_f16: usize,
    pub input_bytes: u64,
    pub output_bytes: u64,
}

impl ConvertStats {
    pub fn compression_ratio(&self) -> f64 {
        if self.output_bytes == 0 {
            0.0
        } else {
            self.input_bytes as f64 / self.output_bytes as f64
        }
    }
}

// ── Metadata extraction ──────────────────────────────────────────

#[derive(Debug)]
enum MetaValue {
    String(String),
    U32(u32),
    U64(u64),
    I32(i32),
    F32(f32),
}

fn read_config(model_dir: &Path) -> io::Result<serde_json::Value> {
    let config_path = model_dir.join("config.json");
    if !config_path.exists() {
        return Ok(serde_json::Value::Null);
    }
    let data = std::fs::read_to_string(config_path)?;
    serde_json::from_str(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn build_metadata(config: &serde_json::Value) -> Vec<(String, MetaValue)> {
    let mut kv = Vec::new();

    // Architecture name (required by GGUF)
    let arch = config["model_type"]
        .as_str()
        .unwrap_or("llama")
        .to_string();
    // Map HF model types to GGUF architecture names
    let gguf_arch = match arch.as_str() {
        "qwen2" | "qwen3" => "qwen2",
        "qwen2_vl" => "qwen2",
        "llama" => "llama",
        "phi3" => "phi3",
        "gemma" | "gemma2" => "gemma",
        "gpt2" => "gpt2",
        "starcoder2" => "starcoder2",
        "mistral" => "llama",
        "deepseek_v2" | "deepseek_v3" => "llama",
        other => other,
    };

    kv.push((
        "general.architecture".into(),
        MetaValue::String(gguf_arch.into()),
    ));

    if let Some(name) = config["_name_or_path"].as_str() {
        kv.push(("general.name".into(), MetaValue::String(name.into())));
    }

    // Standard transformer params
    let prefix = format!("{gguf_arch}.");
    if let Some(v) = config["vocab_size"].as_u64() {
        kv.push((format!("{prefix}vocab_size"), MetaValue::U64(v)));
    }
    if let Some(v) = config["hidden_size"].as_u64() {
        kv.push((
            format!("{prefix}embedding_length"),
            MetaValue::U64(v),
        ));
    }
    if let Some(v) = config["num_hidden_layers"].as_u64() {
        kv.push((format!("{prefix}block_count"), MetaValue::U64(v)));
    }
    if let Some(v) = config["num_attention_heads"].as_u64() {
        kv.push((
            format!("{prefix}attention.head_count"),
            MetaValue::U64(v),
        ));
    }
    if let Some(v) = config["num_key_value_heads"].as_u64() {
        kv.push((
            format!("{prefix}attention.head_count_kv"),
            MetaValue::U64(v),
        ));
    }
    if let Some(v) = config["intermediate_size"].as_u64() {
        kv.push((
            format!("{prefix}feed_forward_length"),
            MetaValue::U64(v),
        ));
    }
    if let Some(v) = config["max_position_embeddings"].as_u64() {
        kv.push((
            format!("{prefix}context_length"),
            MetaValue::U64(v),
        ));
    }
    if let Some(v) = config["rms_norm_eps"].as_f64() {
        kv.push((
            format!("{prefix}attention.layer_norm_rms_epsilon"),
            MetaValue::F32(v as f32),
        ));
    }
    if let Some(v) = config["rope_theta"].as_f64() {
        kv.push((
            format!("{prefix}rope.freq_base"),
            MetaValue::F32(v as f32),
        ));
    }

    kv
}

// ── GGUF binary writer ───────────────────────────────────────────

fn write_gguf(
    path: &Path,
    metadata: &[(String, MetaValue)],
    tensors: &[TensorEntry],
) -> io::Result<()> {
    let alignment: usize = 32;

    // Pre-compute header size to know where tensor data starts
    let mut header_size: usize = 0;
    header_size += 4 + 4 + 8 + 8; // magic + version + n_tensors + n_metadata

    for (key, value) in metadata {
        header_size += 8 + key.len(); // key string
        header_size += 4; // type tag
        match value {
            MetaValue::String(s) => header_size += 8 + s.len(),
            MetaValue::U32(_) => header_size += 4,
            MetaValue::U64(_) => header_size += 8,
            MetaValue::I32(_) => header_size += 4,
            MetaValue::F32(_) => header_size += 4,
        }
    }

    for t in tensors {
        header_size += 8 + t.name.len(); // name string
        header_size += 4; // n_dims
        header_size += 8 * t.shape.len(); // dims
        header_size += 4; // type
        header_size += 8; // offset
    }

    // Pad header to alignment
    let header_padded = (header_size + alignment - 1) / alignment * alignment;
    let header_padding = header_padded - header_size;

    // Compute tensor data offsets (relative to data start)
    let mut data_offset: u64 = 0;
    let mut offsets = Vec::with_capacity(tensors.len());
    for t in tensors {
        offsets.push(data_offset);
        let aligned = ((t.data.len() + alignment - 1) / alignment * alignment) as u64;
        data_offset += aligned;
    }

    // Write
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);

    // ── Header ──
    f.write_all(&GGUF_MAGIC.to_le_bytes())?;
    f.write_all(&GGUF_VERSION.to_le_bytes())?;
    f.write_all(&(tensors.len() as u64).to_le_bytes())?;
    f.write_all(&(metadata.len() as u64).to_le_bytes())?;

    // ── Metadata KV ──
    for (key, value) in metadata {
        write_gguf_string(&mut f, key)?;
        match value {
            MetaValue::String(s) => {
                f.write_all(&GGUF_TYPE_STRING.to_le_bytes())?;
                write_gguf_string(&mut f, s)?;
            }
            MetaValue::U32(v) => {
                f.write_all(&GGUF_TYPE_UINT32.to_le_bytes())?;
                f.write_all(&v.to_le_bytes())?;
            }
            MetaValue::U64(v) => {
                f.write_all(&GGUF_TYPE_UINT64.to_le_bytes())?;
                f.write_all(&v.to_le_bytes())?;
            }
            MetaValue::I32(v) => {
                f.write_all(&GGUF_TYPE_INT32.to_le_bytes())?;
                f.write_all(&v.to_le_bytes())?;
            }
            MetaValue::F32(v) => {
                f.write_all(&GGUF_TYPE_FLOAT32.to_le_bytes())?;
                f.write_all(&v.to_le_bytes())?;
            }
        }
    }

    // ── Tensor infos ──
    for (i, t) in tensors.iter().enumerate() {
        write_gguf_string(&mut f, &t.name)?;
        f.write_all(&(t.shape.len() as u32).to_le_bytes())?;
        for &dim in &t.shape {
            f.write_all(&dim.to_le_bytes())?;
        }
        f.write_all(&t.ggml_type.to_le_bytes())?;
        f.write_all(&offsets[i].to_le_bytes())?;
    }

    // ── Alignment padding ──
    if header_padding > 0 {
        f.write_all(&vec![0u8; header_padding])?;
    }

    // ── Tensor data ──
    for t in tensors {
        f.write_all(&t.data)?;
        let remainder = t.data.len() % alignment;
        if remainder != 0 {
            f.write_all(&vec![0u8; alignment - remainder])?;
        }
    }

    f.flush()?;
    Ok(())
}

fn write_gguf_string(w: &mut impl Write, s: &str) -> io::Result<()> {
    w.write_all(&(s.len() as u64).to_le_bytes())?;
    w.write_all(s.as_bytes())?;
    Ok(())
}
