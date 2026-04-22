//! Model format detection and loading dispatch

pub mod onnx;
pub mod safetensors;
pub mod gguf;
// ggml/pytorch/bin/mlx loaders deleted — the three manifest models ship as
// safetensors or GGUF, and the dead loaders were either never triggered
// (ggml, bin) or were skeletons that returned empty results (pytorch, mlx).

use std::path::Path;

use crate::ir::Graph;

/// Detected model format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    Cyb,
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
        "cyb" => return Ok(ModelFormat::Cyb),
        "onnx" => return Ok(ModelFormat::Onnx),
        "safetensors" => return Ok(ModelFormat::Safetensors),
        "gguf" => return Ok(ModelFormat::Gguf),
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
        if &magic[0..4] == b"CYB\x01" {
            return Ok(ModelFormat::Cyb);
        }
        if &magic[0..4] == b"GGUF" {
            return Ok(ModelFormat::Gguf);
        }
    }

    // ONNX protobuf starts with field tag bytes 0x08 or 0x0a.
    if bytes_read >= 4 && (magic[0] == 0x08 || magic[0] == 0x0a) {
        return Ok(ModelFormat::Onnx);
    }

    Err(format!("Cannot detect format for {}", path.display()))
}

/// Load a model from any supported format
pub fn load_model(path: &Path) -> Result<Graph, String> {
    let format = detect_format(path)?;
    log::info!("Detected format: {format:?} for {}", path.display());

    match format {
        ModelFormat::Cyb => {
            let (graph, _config) = crate::cyb_format::read_cyb(path)
                .map_err(|e| format!("CYB load failed: {e}"))?;
            Ok(graph)
        }
        ModelFormat::Onnx => onnx::load_onnx(path),
        ModelFormat::Safetensors => safetensors::load_safetensors(path),
        ModelFormat::Gguf => gguf::load_gguf(path),
    }
}
