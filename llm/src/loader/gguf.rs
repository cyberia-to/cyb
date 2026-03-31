//! GGUF loader — parse header, metadata, and tensor data
//!
//! Format (GGUF v2/v3):
//! [4 bytes: magic "GGUF"]
//! [4 bytes: version (u32 LE)]
//! [8 bytes: tensor_count (u64 LE for v3, u32 LE for v2)]
//! [8 bytes: metadata_kv_count (u64 LE for v3, u32 LE for v2)]
//! [metadata key-value pairs...]
//! [tensor info entries...]
//! [alignment padding to `general.alignment` or 32]
//! [tensor data...]
//!
//! Metadata value types:
//!   0=u8, 1=i8, 2=u16, 3=i16, 4=u32, 5=i32, 6=f32,
//!   7=bool(1 byte), 8=string(u64 len + bytes), 9=array(u32 type + u64 len + elements),
//!   10=u64, 11=i64, 12=f64

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

use crate::ir::{DType, Graph, TensorMeta, WeightData};

/// GGUF metadata value types
#[derive(Debug)]
#[allow(dead_code)]
pub enum GgufValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    F32(f32),
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    Str(String),
    Array(Vec<GgufValue>),
}

impl GgufValue {
    /// Extract as u32 (from any integer type)
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            GgufValue::U8(v) => Some(*v as u32),
            GgufValue::I8(v) => Some(*v as u32),
            GgufValue::U16(v) => Some(*v as u32),
            GgufValue::I16(v) => Some(*v as u32),
            GgufValue::U32(v) => Some(*v),
            GgufValue::I32(v) => Some(*v as u32),
            GgufValue::U64(v) => Some(*v as u32),
            GgufValue::I64(v) => Some(*v as u32),
            _ => None,
        }
    }

    /// Extract as u64 (from any integer type)
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            GgufValue::U8(v) => Some(*v as u64),
            GgufValue::I8(v) => Some(*v as u64),
            GgufValue::U16(v) => Some(*v as u64),
            GgufValue::I16(v) => Some(*v as u64),
            GgufValue::U32(v) => Some(*v as u64),
            GgufValue::I32(v) => Some(*v as u64),
            GgufValue::U64(v) => Some(*v),
            GgufValue::I64(v) => Some(*v as u64),
            _ => None,
        }
    }

    /// Extract as f32 (from any float type)
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            GgufValue::F32(v) => Some(*v),
            GgufValue::F64(v) => Some(*v as f32),
            GgufValue::U32(v) => Some(*v as f32),
            GgufValue::I32(v) => Some(*v as f32),
            _ => None,
        }
    }

    /// Extract as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            GgufValue::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// GGUF tensor type IDs
fn gguf_type_to_dtype(type_id: u32) -> DType {
    match type_id {
        0 => DType::F32,    // GGML_TYPE_F32
        1 => DType::F16,    // GGML_TYPE_F16
        2 => DType::Q4,     // GGML_TYPE_Q4_0
        3 => DType::Q4_1,   // GGML_TYPE_Q4_1
        8 => DType::Q8,     // GGML_TYPE_Q8_0
        10 => DType::Q3_K,  // GGML_TYPE_Q3_K
        11 => DType::Q2_K,  // GGML_TYPE_Q2_K
        12 => DType::Q4_K,  // GGML_TYPE_Q4_K
        13 => DType::Q5_K,  // GGML_TYPE_Q5_K
        14 => DType::Q6_K,  // GGML_TYPE_Q6_K
        _ => {
            log::warn!("Unknown GGUF type {type_id}, defaulting to F32");
            DType::F32
        }
    }
}

/// Size of one block for quantized types (elements per block)
fn gguf_type_block_size(type_id: u32) -> usize {
    match type_id {
        0 => 1,     // F32: 1 element = 4 bytes
        1 => 1,     // F16: 1 element = 2 bytes
        2 => 32,    // Q4_0: 32 elements per block
        3 => 32,    // Q4_1: 32 elements per block
        8 => 32,    // Q8_0: 32 elements per block
        10 => 256,  // Q3_K: 256 elements per super block
        11 => 256,  // Q2_K: 256 elements per super block
        12 => 256,  // Q4_K: 256 elements per super block
        13 => 256,  // Q5_K: 256 elements per super block
        14 => 256,  // Q6_K: 256 elements per super block
        _ => 1,
    }
}

/// Bytes per block for quantized types
/// Verified against llama.cpp sizeof(block_q*):
///   Q4_0:  2 (f16 scale) + 16 (nibbles) = 18
///   Q4_1:  2 (f16 scale) + 2 (f16 min) + 16 (nibbles) = 20
///   Q8_0:  2 (f16 scale) + 32 (int8) = 34
///   Q2_K:  16 (scales) + 64 (qs) + 2 (d) + 2 (dmin) = 84
///   Q3_K:  32 (hmask) + 64 (qs) + 12 (scales) + 2 (d) = 110
///   Q4_K:  2 (d) + 2 (dmin) + 12 (scales) + 128 (qs) = 144
///   Q5_K:  2 (d) + 2 (dmin) + 12 (scales) + 32 (qh) + 128 (qs) = 176
///   Q6_K:  128 (ql) + 64 (qh) + 16 (scales) + 2 (d) = 210
fn gguf_type_bytes_per_block(type_id: u32) -> usize {
    match type_id {
        0 => 4,     // F32
        1 => 2,     // F16
        2 => 18,    // Q4_0
        3 => 20,    // Q4_1
        8 => 34,    // Q8_0
        10 => 110,  // Q3_K
        11 => 84,   // Q2_K
        12 => 144,  // Q4_K
        13 => 176,  // Q5_K
        14 => 210,  // Q6_K
        _ => 4,
    }
}

/// Internal struct for tensor info parsed from GGUF header
struct TensorInfo {
    name: String,
    dims: Vec<u64>,
    type_id: u32,
    offset: u64,
}

/// Parse the common GGUF header and return (metadata, tensor_infos, header_end_pos)
fn parse_gguf_header(
    mmap: &[u8],
) -> Result<(HashMap<String, GgufValue>, Vec<TensorInfo>, usize), String> {
    if mmap.len() < 24 {
        return Err("File too small for GGUF".to_string());
    }

    let mut cursor = Cursor::new(mmap);

    // Magic
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != b"GGUF" {
        return Err(format!("Invalid GGUF magic: {:?}", magic));
    }

    // Version
    let version = read_u32(&mut cursor)?;
    log::info!("GGUF version: {version}");

    // Tensor count and metadata count
    let tensor_count = if version >= 3 {
        read_u64(&mut cursor)?
    } else {
        read_u32(&mut cursor)? as u64
    };

    let metadata_kv_count = if version >= 3 {
        read_u64(&mut cursor)?
    } else {
        read_u32(&mut cursor)? as u64
    };

    log::info!(
        "GGUF: {tensor_count} tensors, {metadata_kv_count} metadata entries"
    );

    // Read metadata key-value pairs
    // This correctly handles ALL GGUF value types including:
    //   - Arrays of strings (tokenizer.ggml.tokens — can be 32000+ strings)
    //   - Arrays of floats (tokenizer.ggml.scores)
    //   - Arrays of ints (tokenizer.ggml.token_type)
    //   - Nested value types
    let mut metadata: HashMap<String, GgufValue> = HashMap::new();
    for i in 0..metadata_kv_count {
        let pos_before = cursor.position();
        let key = read_gguf_string(&mut cursor)?;
        let value = read_gguf_value(&mut cursor)?;
        let pos_after = cursor.position();
        log::debug!(
            "GGUF metadata [{}/{}]: {} ({} bytes)",
            i + 1,
            metadata_kv_count,
            key,
            pos_after - pos_before,
        );
        metadata.insert(key, value);
    }

    // Log useful metadata
    if let Some(GgufValue::Str(arch)) = metadata.get("general.architecture") {
        log::info!("Architecture: {arch}");
    }
    if let Some(GgufValue::Str(name)) = metadata.get("general.name") {
        log::info!("Model name: {name}");
    }

    // Read tensor info entries
    let mut tensor_infos = Vec::with_capacity(tensor_count as usize);
    for _ in 0..tensor_count {
        let name = read_gguf_string(&mut cursor)?;
        let n_dims = read_u32(&mut cursor)?;
        let mut dims = Vec::with_capacity(n_dims as usize);
        for _ in 0..n_dims {
            dims.push(read_u64(&mut cursor)?);
        }
        let type_id = read_u32(&mut cursor)?;
        let offset = read_u64(&mut cursor)?;
        tensor_infos.push(TensorInfo {
            name,
            dims,
            type_id,
            offset,
        });
    }

    let header_end = cursor.position() as usize;
    Ok((metadata, tensor_infos, header_end))
}

/// Extract alignment from metadata (GGUF v3 supports `general.alignment`)
/// Falls back to 32 if not specified.
fn get_alignment(metadata: &HashMap<String, GgufValue>) -> usize {
    metadata
        .get("general.alignment")
        .and_then(|v| v.as_u32())
        .map(|v| v as usize)
        .unwrap_or(32)
}

/// Build the graph from parsed header + mmap data
fn build_graph_from_gguf(
    mmap: &[u8],
    tensor_infos: &[TensorInfo],
    data_start: usize,
) -> Graph {
    let mut graph = Graph::new();

    for info in tensor_infos {
        // Our converter stores dims in HF convention: [N, K] = [output_features, input_features]
        // (Standard GGUF from llama.cpp uses [ne0, ne1] = [K, N], but our format uses HF order)
        let shape: Vec<usize> = info.dims.iter().map(|&d| d as usize).collect();
        let dtype = gguf_type_to_dtype(info.type_id);

        // Calculate tensor size in bytes
        let num_elements: usize = shape.iter().product();
        let block_size = gguf_type_block_size(info.type_id);
        let bytes_per_block = gguf_type_bytes_per_block(info.type_id);
        let num_blocks = (num_elements + block_size - 1) / block_size;
        let byte_size = num_blocks * bytes_per_block;

        let abs_offset = data_start + info.offset as usize;
        let abs_end = abs_offset + byte_size;

        if abs_end > mmap.len() {
            log::warn!(
                "Tensor {} data out of bounds: offset={} size={} end={} > file_size={} (data_start={})",
                info.name,
                abs_offset,
                byte_size,
                abs_end,
                mmap.len(),
                data_start,
            );
            continue;
        }

        let raw_data = mmap[abs_offset..abs_end].to_vec();

        graph.add_tensor(
            info.name.clone(),
            TensorMeta::weight(shape.clone(), dtype),
        );

        graph.add_weight(
            info.name.clone(),
            WeightData {
                data: raw_data,
                // Our converter stores HF-order (row-major) data, not standard GGUF column-major
                needs_transpose: false,
                shape,
                dtype,
            },
        );
    }

    // Debug: log weight shapes for layer 0
    for suffix in ["q_proj.weight", "k_proj.weight", "v_proj.weight", "o_proj.weight", "mlp.gate_proj.weight", "mlp.up_proj.weight", "mlp.down_proj.weight"] {
        let name = format!("model.layers.0.self_attn.{suffix}");
        let name2 = format!("model.layers.0.{suffix}");
        let key = if graph.weights.contains_key(&name) { &name } else { &name2 };
        if let Some(w) = graph.weights.get(key) {
            log::info!("GGUF {}: shape={:?} dtype={:?} bytes={}", key, w.shape, w.dtype, w.data.len());
        }
    }
    // Check for fused QKV or gate_up
    let l0_keys: Vec<&String> = graph.weights.keys().filter(|k| k.contains("layers.0")).collect();
    log::info!("GGUF layer 0 tensors: {:?}", l0_keys);

    graph
}

/// Load a GGUF file into Graph IR
pub fn load_gguf(path: &Path) -> Result<Graph, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mmap = unsafe {
        memmap2::Mmap::map(&file)
            .map_err(|e| format!("Cannot mmap {}: {e}", path.display()))?
    };

    let (metadata, tensor_infos, header_end) = parse_gguf_header(&mmap)?;

    // Use alignment from metadata (GGUF v3 `general.alignment`), default 32
    let alignment = get_alignment(&metadata);
    let data_start = (header_end + alignment - 1) & !(alignment - 1);
    log::info!(
        "GGUF data_start: {} (header_end={}, alignment={})",
        data_start,
        header_end,
        alignment,
    );

    let graph = build_graph_from_gguf(&mmap, &tensor_infos, data_start);

    log::info!(
        "GGUF loaded: {} tensors from {}",
        graph.weights.len(),
        path.display()
    );

    Ok(graph)
}

/// Load GGUF file and return (Graph, metadata HashMap)
/// Used by the native model loader to extract config from metadata
pub fn load_gguf_with_metadata(path: &Path) -> Result<(Graph, HashMap<String, GgufValue>), String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mmap = unsafe {
        memmap2::Mmap::map(&file)
            .map_err(|e| format!("Cannot mmap {}: {e}", path.display()))?
    };

    let (metadata, tensor_infos, header_end) = parse_gguf_header(&mmap)?;

    // Use alignment from metadata (GGUF v3 `general.alignment`), default 32
    let alignment = get_alignment(&metadata);
    let data_start = (header_end + alignment - 1) & !(alignment - 1);
    log::info!(
        "GGUF data_start: {} (header_end={}, alignment={})",
        data_start,
        header_end,
        alignment,
    );

    let graph = build_graph_from_gguf(&mmap, &tensor_infos, data_start);

    log::info!(
        "GGUF loaded: {} tensors from {}",
        graph.weights.len(),
        path.display()
    );

    Ok((graph, metadata))
}

// ---- Binary reading helpers ----

fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, String> {
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_u8: {e}"))?;
    Ok(buf[0])
}

fn read_i8(cursor: &mut Cursor<&[u8]>) -> Result<i8, String> {
    Ok(read_u8(cursor)? as i8)
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, String> {
    let mut buf = [0u8; 2];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_u16: {e}"))?;
    Ok(u16::from_le_bytes(buf))
}

fn read_i16(cursor: &mut Cursor<&[u8]>) -> Result<i16, String> {
    let mut buf = [0u8; 2];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_i16: {e}"))?;
    Ok(i16::from_le_bytes(buf))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, String> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_u32: {e}"))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i32(cursor: &mut Cursor<&[u8]>) -> Result<i32, String> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_i32: {e}"))?;
    Ok(i32::from_le_bytes(buf))
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, String> {
    let mut buf = [0u8; 8];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_u64: {e}"))?;
    Ok(u64::from_le_bytes(buf))
}

fn read_i64(cursor: &mut Cursor<&[u8]>) -> Result<i64, String> {
    let mut buf = [0u8; 8];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_i64: {e}"))?;
    Ok(i64::from_le_bytes(buf))
}

fn read_f32_val(cursor: &mut Cursor<&[u8]>) -> Result<f32, String> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_f32: {e}"))?;
    Ok(f32::from_le_bytes(buf))
}

fn read_f64_val(cursor: &mut Cursor<&[u8]>) -> Result<f64, String> {
    let mut buf = [0u8; 8];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_f64: {e}"))?;
    Ok(f64::from_le_bytes(buf))
}

/// Read a GGUF string: u64 length prefix + UTF-8 bytes
fn read_gguf_string(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
    let len = read_u64(cursor)? as usize;
    if len > 1024 * 1024 * 64 {
        return Err(format!("GGUF string too large: {len} bytes"));
    }
    let mut buf = vec![0u8; len];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_string({len}): {e}"))?;
    String::from_utf8(buf).map_err(|e| format!("Invalid UTF-8 in GGUF string: {e}"))
}

/// Read a GGUF value with type tag
/// Handles ALL GGUF value types (0-12) including nested arrays.
fn read_gguf_value(cursor: &mut Cursor<&[u8]>) -> Result<GgufValue, String> {
    let value_type = read_u32(cursor)?;
    read_gguf_value_typed(cursor, value_type)
}

/// Read a GGUF value of known type (no type tag prefix).
/// Used both for top-level values and array elements.
///
/// Type mapping:
///   0=u8(1B), 1=i8(1B), 2=u16(2B), 3=i16(2B), 4=u32(4B), 5=i32(4B),
///   6=f32(4B), 7=bool(1B), 8=string(u64 len + bytes),
///   9=array(u32 elem_type + u64 count + elements),
///   10=u64(8B), 11=i64(8B), 12=f64(8B)
fn read_gguf_value_typed(cursor: &mut Cursor<&[u8]>, value_type: u32) -> Result<GgufValue, String> {
    match value_type {
        0 => Ok(GgufValue::U8(read_u8(cursor)?)),
        1 => Ok(GgufValue::I8(read_i8(cursor)?)),
        2 => Ok(GgufValue::U16(read_u16(cursor)?)),
        3 => Ok(GgufValue::I16(read_i16(cursor)?)),
        4 => Ok(GgufValue::U32(read_u32(cursor)?)),
        5 => Ok(GgufValue::I32(read_i32(cursor)?)),
        6 => Ok(GgufValue::F32(read_f32_val(cursor)?)),
        7 => Ok(GgufValue::Bool(read_u8(cursor)? != 0)),
        8 => Ok(GgufValue::Str(read_gguf_string(cursor)?)),
        9 => {
            // Array: u32 element_type + u64 count + count * element
            let arr_type = read_u32(cursor)?;
            let arr_len = read_u64(cursor)? as usize;
            if arr_len > 100_000_000 {
                return Err(format!("GGUF array too large: {arr_len} elements"));
            }
            let mut values = Vec::with_capacity(arr_len.min(1024 * 1024));
            for _ in 0..arr_len {
                let val = read_gguf_value_typed(cursor, arr_type)?;
                values.push(val);
            }
            Ok(GgufValue::Array(values))
        }
        10 => Ok(GgufValue::U64(read_u64(cursor)?)),
        11 => Ok(GgufValue::I64(read_i64(cursor)?)),
        12 => Ok(GgufValue::F64(read_f64_val(cursor)?)),
        _ => {
            // Skip unknown types gracefully: log warning and return a placeholder
            log::warn!("Unknown GGUF value type: {value_type} at position {}", cursor.position());
            Err(format!("Unknown GGUF value type: {value_type}"))
        }
    }
}
