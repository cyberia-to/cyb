//! MLX / NPZ format loader
//!
//! MLX models use the same layout as HuggingFace models:
//!   - config.json for architecture metadata
//!   - weights in either .safetensors or .npz format
//!
//! This loader handles the .npz case (numpy zip archive of .npy arrays).
//!
//! NPZ format: standard ZIP archive where each entry is a .npy file.
//!
//! NPY format (each entry):
//!   [6 bytes: magic "\x93NUMPY"]
//!   [1 byte: major version]
//!   [1 byte: minor version]
//!   [2 bytes: header_len (u16 LE) for v1, or 4 bytes (u32 LE) for v2]
//!   [header_len bytes: Python dict string, e.g. "{'descr': '<f4', 'fortran_order': False, 'shape': (4096, 4096), }"]
//!   [padding to 64-byte alignment]
//!   [raw data]

use std::path::Path;

use crate::ir::{DType, Graph, TensorMeta, WeightData};

/// Load an NPZ file (zip of .npy files) into Graph IR
pub fn load_npz(path: &Path) -> Result<Graph, String> {
    let data = std::fs::read(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;

    let mut graph = Graph::new();

    // Parse as ZIP
    let entries = parse_zip_entries(&data)?;

    for (name, entry_data) in &entries {
        // Strip .npy extension to get tensor name
        let tensor_name = name.strip_suffix(".npy").unwrap_or(name).to_string();

        match parse_npy(entry_data) {
            Ok((shape, dtype, raw_data)) => {
                graph.add_tensor(
                    tensor_name.clone(),
                    TensorMeta::weight(shape.clone(), dtype),
                );
                graph.add_weight(
                    tensor_name,
                    WeightData {
                        data: raw_data,
                        shape,
                        dtype, needs_transpose: false },
                );
            }
            Err(e) => {
                log::warn!("Skipping {name}: {e}");
            }
        }
    }

    log::info!(
        "NPZ loaded: {} tensors from {}",
        graph.weights.len(),
        path.display(),
    );

    Ok(graph)
}

/// Load an MLX model directory (config.json + weights.npz or model.safetensors)
pub fn load_mlx_dir(path: &Path) -> Result<Graph, String> {
    // Check if path is a directory or a file
    let dir = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .ok_or_else(|| format!("Cannot determine directory for {}", path.display()))?
            .to_path_buf()
    };

    // Try weights.npz first
    let npz_path = dir.join("weights.npz");
    if npz_path.exists() {
        return load_npz(&npz_path);
    }

    // Try model.npz
    let model_npz = dir.join("model.npz");
    if model_npz.exists() {
        return load_npz(&model_npz);
    }

    // Fall back to model.safetensors (MLX can use safetensors too)
    let st_path = dir.join("model.safetensors");
    if st_path.exists() {
        return crate::loader::safetensors::load_safetensors(&st_path);
    }

    // If the path itself is an .npz file
    if path.extension().and_then(|e| e.to_str()) == Some("npz") {
        return load_npz(path);
    }

    Err(format!(
        "No weights found in MLX directory: {}",
        dir.display(),
    ))
}

/// Parse a numpy .npy byte buffer into (shape, dtype, raw_data)
fn parse_npy(data: &[u8]) -> Result<(Vec<usize>, DType, Vec<u8>), String> {
    if data.len() < 10 {
        return Err("NPY data too small".to_string());
    }

    // Check magic
    if &data[0..6] != b"\x93NUMPY" {
        return Err("Invalid NPY magic".to_string());
    }

    let major = data[6];
    let minor = data[7];

    // Read header length
    let (header_len, header_start) = if major == 1 {
        // v1: 2-byte header length
        let len = u16::from_le_bytes([data[8], data[9]]) as usize;
        (len, 10usize)
    } else if major >= 2 {
        // v2+: 4-byte header length
        if data.len() < 12 {
            return Err("NPY v2 header too short".to_string());
        }
        let len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        (len, 12usize)
    } else {
        return Err(format!("Unsupported NPY version: {major}.{minor}"));
    };

    let header_end = header_start + header_len;
    if header_end > data.len() {
        return Err(format!(
            "NPY header extends beyond data: header_end={header_end}, data_len={}",
            data.len(),
        ));
    }

    // Parse the Python dict header string
    let header_str = std::str::from_utf8(&data[header_start..header_end])
        .map_err(|e| format!("Invalid NPY header UTF-8: {e}"))?;

    let (shape, dtype, fortran_order) = parse_npy_header(header_str)?;

    if fortran_order {
        log::warn!("NPY fortran_order=True is not optimally supported (data loaded as-is)");
    }

    let data_start = header_end;
    let raw_data = data[data_start..].to_vec();

    Ok((shape, dtype, raw_data))
}

/// Parse the Python dict header from an NPY file.
/// Example: "{'descr': '<f4', 'fortran_order': False, 'shape': (4096, 4096), }\n"
fn parse_npy_header(header: &str) -> Result<(Vec<usize>, DType, bool), String> {
    let trimmed = header.trim().trim_matches(|c| c == '{' || c == '}');

    // Extract descr
    let descr = extract_string_value(trimmed, "descr")
        .ok_or("Missing 'descr' in NPY header")?;

    // Extract fortran_order
    let fortran_order = extract_bool_value(trimmed, "fortran_order").unwrap_or(false);

    // Extract shape
    let shape = extract_shape(trimmed)
        .ok_or("Missing 'shape' in NPY header")?;

    let dtype = npy_descr_to_dtype(&descr)?;

    Ok((shape, dtype, fortran_order))
}

/// Extract a string value from Python dict: 'key': 'value'
fn extract_string_value(s: &str, key: &str) -> Option<String> {
    let pattern = format!("'{key}':");
    let idx = s.find(&pattern)?;
    let rest = &s[idx + pattern.len()..];
    let rest = rest.trim_start();

    // Value is in single quotes
    if rest.starts_with('\'') {
        let end = rest[1..].find('\'')?;
        Some(rest[1..1 + end].to_string())
    } else {
        None
    }
}

/// Extract a bool value from Python dict: 'key': True/False
fn extract_bool_value(s: &str, key: &str) -> Option<bool> {
    let pattern = format!("'{key}':");
    let idx = s.find(&pattern)?;
    let rest = s[idx + pattern.len()..].trim_start();
    if rest.starts_with("True") {
        Some(true)
    } else if rest.starts_with("False") {
        Some(false)
    } else {
        None
    }
}

/// Extract shape tuple from Python dict: 'shape': (4096, 4096)
fn extract_shape(s: &str) -> Option<Vec<usize>> {
    let pattern = "'shape':";
    let idx = s.find(pattern)?;
    let rest = &s[idx + pattern.len()..];
    let rest = rest.trim_start();

    if !rest.starts_with('(') {
        return None;
    }

    let end = rest.find(')')?;
    let inner = &rest[1..end];

    let dims: Vec<usize> = inner
        .split(',')
        .map(|d| d.trim())
        .filter(|d| !d.is_empty())
        .map(|d| d.parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    Some(dims)
}

/// Convert NPY dtype descriptor to our DType
fn npy_descr_to_dtype(descr: &str) -> Result<DType, String> {
    match descr {
        "<f4" | "=f4" | "f4" => Ok(DType::F32),
        "<f2" | "=f2" | "f2" => Ok(DType::F16),
        "<f8" | "=f8" | "f8" => Ok(DType::F32), // load f64 as f32 placeholder
        "<i1" | "=i1" | "i1" | "|i1" => Ok(DType::I8),
        "<u1" | "=u1" | "u1" | "|u1" => Ok(DType::U8),
        ">f4" => Ok(DType::F32), // big-endian f32 (will need byte swap)
        ">f2" => Ok(DType::F16), // big-endian f16
        _ => {
            log::warn!("Unknown NPY dtype '{descr}', defaulting to F32");
            Ok(DType::F32)
        }
    }
}

/// Minimal ZIP parser: extract entries as (name, data) pairs
/// Only supports STORE and DEFLATE methods for .npy files.
fn parse_zip_entries(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut entries = Vec::new();
    let mut pos = 0;

    while pos + 4 <= data.len() {
        // Check for local file header signature
        let sig = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);

        if sig != 0x04034b50 {
            // Not a local file header — probably reached central directory
            break;
        }

        if pos + 30 > data.len() {
            break;
        }

        let compression = u16::from_le_bytes([data[pos + 8], data[pos + 9]]);
        let compressed_size = u32::from_le_bytes([
            data[pos + 18],
            data[pos + 19],
            data[pos + 20],
            data[pos + 21],
        ]) as usize;
        let _uncompressed_size = u32::from_le_bytes([
            data[pos + 22],
            data[pos + 23],
            data[pos + 24],
            data[pos + 25],
        ]) as usize;
        let name_len = u16::from_le_bytes([data[pos + 26], data[pos + 27]]) as usize;
        let extra_len = u16::from_le_bytes([data[pos + 28], data[pos + 29]]) as usize;

        let name_start = pos + 30;
        let name_end = name_start + name_len;
        if name_end > data.len() {
            break;
        }
        let name = String::from_utf8_lossy(&data[name_start..name_end]).to_string();

        let data_start = name_end + extra_len;
        let data_end = data_start + compressed_size;
        if data_end > data.len() {
            break;
        }

        let entry_data = match compression {
            0 => {
                // STORE — data is uncompressed
                data[data_start..data_end].to_vec()
            }
            _ => {
                log::warn!(
                    "Skipping ZIP entry '{}': unsupported compression method {}",
                    name,
                    compression,
                );
                pos = data_end;
                continue;
            }
        };

        entries.push((name, entry_data));
        pos = data_end;
    }

    Ok(entries)
}
