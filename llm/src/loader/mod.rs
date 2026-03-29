//! Model format detection and loading dispatch

pub mod onnx;
pub mod safetensors;
pub mod gguf;

use std::path::Path;

use crate::ir::Graph;

/// Detected model format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    Onnx,
    Safetensors,
    Gguf,
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
        _ => {}
    }

    // Try magic bytes
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut magic = [0u8; 4];
    use std::io::Read;
    let mut reader = std::io::BufReader::new(file);
    reader.read_exact(&mut magic)
        .map_err(|e| format!("Cannot read magic bytes: {e}"))?;

    if &magic == b"GGUF" {
        return Ok(ModelFormat::Gguf);
    }

    // Safetensors starts with a u64 header size (first 8 bytes are numeric)
    // ONNX protobuf starts with field tag bytes
    // Heuristic: if first bytes look like a protobuf, assume ONNX
    if magic[0] == 0x08 || magic[0] == 0x0a {
        return Ok(ModelFormat::Onnx);
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
    }
}
