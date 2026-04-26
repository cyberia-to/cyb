//! HuggingFace model download.
//!
//! Contract: see [`import/specs/hf.md`]. Selection priority is
//! safetensors > GGUF > ONNX, with the sibling metadata files
//! (config.json, tokenizer.json, etc.) downloaded alongside.

use hf_hub::api::sync::Api;
use std::path::PathBuf;

/// Files downloaded for an HF model: the artifact + its metadata. The
/// `artifact` is the principal weights file; `siblings` are everything else
/// fetched (sharded safetensors files, ONNX external data, tokenizer/config
/// JSONs). All paths share the same `hf-hub` snapshot directory; calling
/// `snapshot_dir()` returns it.
#[derive(Debug)]
pub struct DownloadedModel {
    pub artifact: PathBuf,
    pub kind: ArtifactKind,
    pub siblings: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Safetensors,
    Gguf,
    Onnx,
}

impl DownloadedModel {
    /// Snapshot directory containing the artifact and siblings.
    pub fn snapshot_dir(&self) -> Option<&std::path::Path> {
        self.artifact.parent()
    }
}

/// Files we always try to fetch alongside a model artifact. Missing entries
/// are not errors — different model families ship different metadata sets.
const SIBLING_CANDIDATES: &[&str] = &[
    "config.json",
    "tokenizer.json",
    "tokenizer.model",
    "tokenizer_config.json",
    "special_tokens_map.json",
    "generation_config.json",
];

/// Download a model from HuggingFace Hub. Selects the canonical artifact
/// per the priority list (safetensors > GGUF > ONNX), then fetches metadata
/// siblings when present.
///
/// Returns the artifact path, kind, and the set of metadata files actually
/// downloaded. The caller can then point `mi import` at
/// `result.snapshot_dir()`.
pub fn download_model(model_id: &str) -> Result<DownloadedModel, String> {
    let api = Api::new().map_err(|e| format!("HF API init failed: {e}"))?;
    let repo = api.model(model_id.to_string());

    let info = repo
        .info()
        .map_err(|e| format!("repo info for {model_id} failed: {e}"))?;
    let filenames: Vec<String> = info.siblings.iter().map(|s| s.rfilename.clone()).collect();

    let (artifact_name, kind) = pick_artifact(&filenames).ok_or_else(|| {
        format!("no recognized artifact in {model_id}; tried safetensors / gguf / onnx")
    })?;

    log::info!("Downloading artifact: {artifact_name}");
    let artifact = repo
        .get(&artifact_name)
        .map_err(|e| format!("download {artifact_name} from {model_id} failed: {e}"))?;

    let mut siblings: Vec<PathBuf> = Vec::new();

    // Sharded safetensors: fetch every shard listed in the index.
    if artifact_name.ends_with(".safetensors.index.json") {
        let shards: Vec<&String> = filenames
            .iter()
            .filter(|f| f.ends_with(".safetensors") && f.contains("of"))
            .collect();
        for shard in &shards {
            log::info!("Downloading shard: {shard}");
            let p = repo
                .get(shard.as_str())
                .map_err(|e| format!("download {shard} failed: {e}"))?;
            siblings.push(p);
        }
    } else if artifact_name.ends_with(".onnx") {
        // ONNX models often have a `<model>_data` external data file.
        let data_name = format!("{artifact_name}_data");
        if filenames.iter().any(|f| f == &data_name) {
            if let Ok(p) = repo.get(&data_name) {
                log::info!("Downloaded ONNX external data: {}", p.display());
                siblings.push(p);
            }
        }
    }

    // Sibling metadata files.
    for name in SIBLING_CANDIDATES {
        if filenames.iter().any(|f| f == name) {
            match repo.get(name) {
                Ok(p) => {
                    log::info!("Downloaded sibling: {name}");
                    siblings.push(p);
                }
                Err(e) => log::warn!("sibling {name} listed but download failed: {e}"),
            }
        }
    }

    Ok(DownloadedModel {
        artifact,
        kind,
        siblings,
    })
}

/// Walk the canonical priority list against the listing, return the first
/// match. Returned String is the path-within-repo, owned (the caller used
/// it directly with `repo.get()`).
fn pick_artifact(filenames: &[String]) -> Option<(String, ArtifactKind)> {
    // Priority 1: single-file safetensors at repo root.
    if filenames.iter().any(|f| f == "model.safetensors") {
        return Some(("model.safetensors".to_string(), ArtifactKind::Safetensors));
    }
    // Priority 1b: sharded safetensors (download via index).
    if filenames
        .iter()
        .any(|f| f == "model.safetensors.index.json")
    {
        return Some((
            "model.safetensors.index.json".to_string(),
            ArtifactKind::Safetensors,
        ));
    }
    // Priority 2: GGUF — pick the first .gguf file. Quantization tag varies
    // by repo, so don't filter on that.
    if let Some(name) = filenames.iter().find(|f| f.ends_with(".gguf")) {
        return Some((name.clone(), ArtifactKind::Gguf));
    }
    // Priority 3: ONNX — first .onnx file in the repo.
    if let Some(name) = filenames.iter().find(|f| f.ends_with(".onnx")) {
        return Some((name.clone(), ArtifactKind::Onnx));
    }
    None
}
