//! GGUF loader — parse header, metadata, and tensor data
//!
//! Format:
//! [4 bytes: magic "GGUF"]
//! [4 bytes: version (u32 LE)]
//! [8 bytes: tensor_count (u64 LE)]
//! [8 bytes: metadata_kv_count (u64 LE)]
//! [metadata key-value pairs...]
//! [tensor info entries...]
//! [alignment padding]
//! [tensor data...]

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

/// Size of one block for quantized types
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
fn gguf_type_bytes_per_block(type_id: u32) -> usize {
    match type_id {
        0 => 4,     // F32
        1 => 2,     // F16
        2 => 18,    // Q4_0
        3 => 20,    // Q4_1
        8 => 34,    // Q8_0
        10 => 110,  // Q3_K: 256 elements in 110 bytes
        11 => 84,   // Q2_K: 256 elements in 84 bytes
        12 => 144,  // Q4_K: 256 elements in 144 bytes
        13 => 176,  // Q5_K: 256 elements in 176 bytes
        14 => 210,  // Q6_K: 256 elements in 210 bytes
        _ => 4,
    }
}

/// Load a GGUF file into Graph IR
pub fn load_gguf(path: &Path) -> Result<Graph, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mmap = unsafe {
        memmap2::Mmap::map(&file)
            .map_err(|e| format!("Cannot mmap {}: {e}", path.display()))?
    };

    if mmap.len() < 24 {
        return Err("File too small for GGUF".to_string());
    }

    let mut cursor = Cursor::new(&mmap[..]);

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
    let mut metadata: HashMap<String, GgufValue> = HashMap::new();
    for _ in 0..metadata_kv_count {
        let key = read_gguf_string(&mut cursor)?;
        let value = read_gguf_value(&mut cursor)?;
        log::debug!("GGUF metadata: {key}");
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
    struct TensorInfo {
        name: String,
        dims: Vec<u64>,
        type_id: u32,
        offset: u64,
    }

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

    // Calculate tensor data start (aligned to 32 bytes)
    let header_end = cursor.position() as usize;
    let alignment = 32;
    let data_start = (header_end + alignment - 1) & !(alignment - 1);

    // Build graph from tensor data
    let mut graph = Graph::new();

    for info in &tensor_infos {
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
                "Tensor {} data out of bounds: {abs_end} > {}",
                info.name,
                mmap.len()
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
                shape,
                dtype,
            },
        );
    }

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

    if mmap.len() < 24 {
        return Err("File too small for GGUF".to_string());
    }

    let mut cursor = Cursor::new(&mmap[..]);

    // Magic
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != b"GGUF" {
        return Err(format!("Invalid GGUF magic: {:?}", magic));
    }

    let version = read_u32(&mut cursor)?;
    log::info!("GGUF version: {version}");

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

    let mut metadata: HashMap<String, GgufValue> = HashMap::new();
    for _ in 0..metadata_kv_count {
        let key = read_gguf_string(&mut cursor)?;
        let value = read_gguf_value(&mut cursor)?;
        log::debug!("GGUF metadata: {key}");
        metadata.insert(key, value);
    }

    if let Some(GgufValue::Str(arch)) = metadata.get("general.architecture") {
        log::info!("Architecture: {arch}");
    }
    if let Some(GgufValue::Str(name)) = metadata.get("general.name") {
        log::info!("Model name: {name}");
    }

    struct TensorInfo {
        name: String,
        dims: Vec<u64>,
        type_id: u32,
        offset: u64,
    }

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
    let alignment = 32;
    let data_start = (header_end + alignment - 1) & !(alignment - 1);

    let mut graph = Graph::new();

    for info in &tensor_infos {
        let shape: Vec<usize> = info.dims.iter().map(|&d| d as usize).collect();
        let dtype = gguf_type_to_dtype(info.type_id);

        let num_elements: usize = shape.iter().product();
        let block_size = gguf_type_block_size(info.type_id);
        let bytes_per_block = gguf_type_bytes_per_block(info.type_id);
        let num_blocks = (num_elements + block_size - 1) / block_size;
        let byte_size = num_blocks * bytes_per_block;

        let abs_offset = data_start + info.offset as usize;
        let abs_end = abs_offset + byte_size;

        if abs_end > mmap.len() {
            log::warn!(
                "Tensor {} data out of bounds: {abs_end} > {}",
                info.name,
                mmap.len()
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
                shape,
                dtype,
            },
        );
    }

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

fn read_gguf_string(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
    let len = read_u64(cursor)? as usize;
    let mut buf = vec![0u8; len];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_string: {e}"))?;
    String::from_utf8(buf).map_err(|e| format!("Invalid UTF-8 in GGUF string: {e}"))
}

fn read_gguf_value(cursor: &mut Cursor<&[u8]>) -> Result<GgufValue, String> {
    let value_type = read_u32(cursor)?;
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
            // Array
            let arr_type = read_u32(cursor)?;
            let arr_len = read_u64(cursor)? as usize;
            let mut values = Vec::with_capacity(arr_len);
            for _ in 0..arr_len {
                let val = read_gguf_value_typed(cursor, arr_type)?;
                values.push(val);
            }
            Ok(GgufValue::Array(values))
        }
        10 => Ok(GgufValue::U64(read_u64(cursor)?)),
        11 => Ok(GgufValue::I64(read_i64(cursor)?)),
        12 => Ok(GgufValue::F64(read_f64_val(cursor)?)),
        _ => Err(format!("Unknown GGUF value type: {value_type}")),
    }
}

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
        10 => Ok(GgufValue::U64(read_u64(cursor)?)),
        11 => Ok(GgufValue::I64(read_i64(cursor)?)),
        12 => Ok(GgufValue::F64(read_f64_val(cursor)?)),
        _ => Err(format!("Unknown GGUF array element type: {value_type}")),
    }
}
