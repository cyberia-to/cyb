use hf_hub::api::sync::Api;
use std::path::PathBuf;

/// Download an ONNX model from HuggingFace
pub fn download_model(model_id: &str) -> Result<PathBuf, String> {
    let api = Api::new().map_err(|e| format!("HF API init failed: {e}"))?;
    let repo = api.model(model_id.to_string());

    // Try common ONNX model file names
    let candidates = [
        "model.onnx",
        "model_q4f16.onnx",
        "model_quantized.onnx",
        "onnx/model.onnx",
        "onnx/decoder_model.onnx",
        "decoder_model.onnx",
        "onnx/model_quantized.onnx",
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
    let model_path = model_path
        .ok_or_else(|| format!("No ONNX model found in {model_id}. Tried: {candidates:?}"))?;

    log::info!("Model downloaded to: {}", model_path.display());
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
