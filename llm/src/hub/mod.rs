//! HuggingFace model download

use hf_hub::api::sync::Api;
use std::path::PathBuf;

/// Download an ONNX model from HuggingFace
pub fn download_model(model_id: &str) -> Result<PathBuf, String> {
    let api = Api::new().map_err(|e| format!("HF API init failed: {e}"))?;
    let repo = api.model(model_id.to_string());

    // Try common ONNX model file names
    let candidates = [
        "onnx/model_q4.onnx",
        "onnx/model_q4f16.onnx",
        "onnx/model_bnb4.onnx",
        "onnx/model_quantized.onnx",
        "onnx/model_int8.onnx",
        "model_q4.onnx",
        "model_q4f16.onnx",
        "model_quantized.onnx",
        "onnx/model.onnx",
        "model.onnx",
        "onnx/decoder_model.onnx",
        "decoder_model.onnx",
    ];
    let mut model_path = None;
    for name in &candidates {
        match repo.get(name) {
            Ok(p) => {
                model_path = Some(p);
                log::info!("Found model: {name}");
                break;
            }
            Err(_) => continue,
        }
    }
    let model_path =
        model_path.ok_or_else(|| format!("No ONNX model found in {model_id}. Tried: {candidates:?}"))?;

    log::info!("Model downloaded to: {}", model_path.display());

    // Try to download external data file
    let model_filename = model_path.file_name().unwrap().to_str().unwrap();
    let data_filename = format!("{}_data", model_filename);
    let found_candidate = candidates
        .iter()
        .find(|c| c.ends_with(model_filename))
        .unwrap();
    let prefix = found_candidate
        .strip_suffix(model_filename)
        .unwrap_or("");
    let data_repo_path = format!("{}{}", prefix, data_filename);

    match repo.get(&data_repo_path) {
        Ok(p) => log::info!("External data downloaded: {}", p.display()),
        Err(_) => log::debug!("No external data file: {data_repo_path}"),
    }

    Ok(model_path)
}

/// Download tokenizer from HuggingFace
pub fn download_tokenizer(model_id: &str) -> Result<PathBuf, String> {
    let api = Api::new().map_err(|e| format!("HF API init failed: {e}"))?;
    let repo = api.model(model_id.to_string());

    let tokenizer_path = repo
        .get("tokenizer.json")
        .map_err(|e| format!("Could not find tokenizer in {model_id}: {e}"))?;

    Ok(tokenizer_path)
}

/// Download a specific file from a HuggingFace repo
pub fn download_file(model_id: &str, filename: &str) -> Result<PathBuf, String> {
    let api = Api::new().map_err(|e| format!("HF API init failed: {e}"))?;
    let repo = api.model(model_id.to_string());

    repo.get(filename)
        .map_err(|e| format!("Could not download {filename} from {model_id}: {e}"))
}
