//! Source-format loaders for the importer. Every loader returns a
//! [`Weights`] table — the importer then re-packs it into a `.model` file.

pub mod gguf;
pub mod onnx;
pub mod safetensors;

use std::path::Path;

use crate::types::Weights;

/// Detected source-model format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    Onnx,
    Safetensors,
    Gguf,
}

/// Detect format from extension + magic bytes.
pub fn detect_format(path: &Path) -> Result<ModelFormat, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "onnx" => return Ok(ModelFormat::Onnx),
        "safetensors" => return Ok(ModelFormat::Safetensors),
        "gguf" => return Ok(ModelFormat::Gguf),
        _ => {}
    }

    // Magic bytes fallback.
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut magic = [0u8; 8];
    use std::io::Read;
    let mut reader = std::io::BufReader::new(file);
    let bytes_read = reader
        .read(&mut magic)
        .map_err(|e| format!("Cannot read magic bytes: {e}"))?;

    if bytes_read >= 4 && &magic[0..4] == b"GGUF" {
        return Ok(ModelFormat::Gguf);
    }
    // ONNX protobuf starts with 0x08 or 0x0a field tag.
    if bytes_read >= 4 && (magic[0] == 0x08 || magic[0] == 0x0a) {
        return Ok(ModelFormat::Onnx);
    }

    Err(format!("Cannot detect format for {}", path.display()))
}

/// Load the model at `path` into a [`Weights`] table.
pub fn load_model(path: &Path) -> Result<Weights, String> {
    let fmt = detect_format(path)?;
    log::info!("Detected format: {fmt:?} for {}", path.display());
    match fmt {
        ModelFormat::Onnx => onnx::load_onnx(path),
        ModelFormat::Safetensors => safetensors::load_safetensors(path),
        ModelFormat::Gguf => gguf::load_gguf(path),
    }
}
