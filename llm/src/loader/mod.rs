//! Model format detection and loading dispatch

pub mod onnx;
pub mod safetensors;
pub mod gguf;
pub mod ggml;
pub mod bin;
pub mod mlx;

use std::path::Path;

use crate::ir::Graph;

/// Detected model format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    Onnx,
    Safetensors,
    Gguf,
    Ggml,
    Bin,
    Npz,
}

/// Detect model format from file path and magic bytes
pub fn detect_format(path: &Path) -> Result<ModelFormat, String> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "onnx" => return Ok(ModelFormat::Onnx),
        "safetensors" => return Ok(ModelFormat::Safetensors),
        "gguf" => return Ok(ModelFormat::Gguf),
        "npz" => return Ok(ModelFormat::Npz),
        // .bin is ambiguous — could be GGML, fasttext, or raw. Check magic first.
        _ => {}
    }

    // Try magic bytes
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut magic = [0u8; 8];
    use std::io::Read;
    let mut reader = std::io::BufReader::new(file);
    let bytes_read = reader.read(&mut magic)
        .map_err(|e| format!("Cannot read magic bytes: {e}"))?;

    if bytes_read >= 4 {
        // GGUF magic
        if &magic[0..4] == b"GGUF" {
            return Ok(ModelFormat::Gguf);
        }

        // GGML magic (0x67676d6c = "ggml" little-endian)
        if magic[0..4] == [0x6c, 0x6d, 0x67, 0x67] {
            return Ok(ModelFormat::Ggml);
        }

        // ZIP magic (PK\x03\x04) — NPZ files are ZIP archives
        if &magic[0..4] == b"PK\x03\x04" {
            return Ok(ModelFormat::Npz);
        }

        // NPY magic (\x93NUMPY) — single array file, treat as bin
        if bytes_read >= 6 && &magic[0..6] == b"\x93NUMPY" {
            return Ok(ModelFormat::Npz);
        }
    }

    // Safetensors starts with a u64 header size (first 8 bytes are numeric)
    // ONNX protobuf starts with field tag bytes
    // Heuristic: if first bytes look like a protobuf, assume ONNX
    if bytes_read >= 4 && (magic[0] == 0x08 || magic[0] == 0x0a) {
        return Ok(ModelFormat::Onnx);
    }

    // Check if it could be a fasttext .bin (first 4 bytes as i32 could be vocab_size)
    if bytes_read >= 8 {
        let first = i32::from_le_bytes([magic[0], magic[1], magic[2], magic[3]]);
        let second = i32::from_le_bytes([magic[4], magic[5], magic[6], magic[7]]);
        if first > 0 && first < 10_000_000 && second > 0 && second < 100_000 {
            return Ok(ModelFormat::Bin);
        }
    }

    Err(format!("Cannot detect format for {}", path.display()))
}

/// Load a model from any supported format
pub fn load_model(path: &Path) -> Result<Graph, String> {
    let format = detect_format(path)?;
    log::info!("Detected format: {format:?} for {}", path.display());

    match format {
        ModelFormat::Onnx => onnx::load_onnx(path),
        ModelFormat::Safetensors => safetensors::load_safetensors(path),
        ModelFormat::Gguf => gguf::load_gguf(path),
        ModelFormat::Ggml => ggml::load_ggml(path),
        ModelFormat::Bin => bin::load_bin(path),
        ModelFormat::Npz => mlx::load_npz(path),
    }
}
