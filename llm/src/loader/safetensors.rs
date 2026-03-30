//! Safetensors loader — parse JSON header + mmap tensor data
//!
//! Format:
//! [8 bytes: header_size as u64 LE]
//! [header_size bytes: JSON metadata]
//! [rest: raw tensor data]
//!
//! JSON metadata: { "tensor_name": { "dtype": "F16", "shape": [4096, 4096], "data_offsets": [start, end] }, ... }

use std::collections::HashMap;
use std::path::Path;

use crate::ir::{DType, Graph, TensorMeta, WeightData};

/// Safetensors tensor descriptor (from JSON header)
#[derive(Debug, serde::Deserialize)]
struct TensorDescriptor {
    dtype: String,
    shape: Vec<usize>,
    data_offsets: [u64; 2],
}

/// Load a safetensors file (or multi-shard) into Graph IR
pub fn load_safetensors(path: &Path) -> Result<Graph, String> {
    // Check for multi-shard: if model.safetensors.index.json exists, load all shards
    if let Some(dir) = path.parent() {
        let index_path = dir.join("model.safetensors.index.json");
        if index_path.exists() {
            return load_safetensors_sharded(dir, &index_path);
        }
    }

    load_safetensors_single(path)
}

/// Load all shards from a safetensors index
fn load_safetensors_sharded(dir: &Path, index_path: &Path) -> Result<Graph, String> {
    let index_str = std::fs::read_to_string(index_path)
        .map_err(|e| format!("Cannot read index: {e}"))?;
    let index: serde_json::Value = serde_json::from_str(&index_str)
        .map_err(|e| format!("Invalid index JSON: {e}"))?;

    let weight_map = index.get("weight_map")
        .and_then(|v| v.as_object())
        .ok_or("No weight_map in index")?;

    // Collect unique shard files
    let mut shard_files: Vec<String> = weight_map.values()
        .filter_map(|v| v.as_str().map(String::from))
        .collect::<std::collections::HashSet<_>>()
        .into_iter().collect();
    shard_files.sort();

    let mut graph = Graph::new();
    for shard_file in &shard_files {
        let shard_path = dir.join(shard_file);
        if !shard_path.exists() {
            log::warn!("Shard {} not found, skipping", shard_file);
            continue;
        }
        let shard_graph = load_safetensors_single(&shard_path)?;
        // Merge weights
        for (name, weight) in shard_graph.weights {
            graph.add_weight(name.clone(), weight);
            if let Some(meta) = shard_graph.tensors.get(&name) {
                graph.add_tensor(name, meta.clone());
            }
        }
    }

    log::info!("Safetensors sharded: {} weights from {} shards in {}",
        graph.weights.len(), shard_files.len(), dir.display());
    Ok(graph)
}

/// Load a single safetensors file into Graph IR
fn load_safetensors_single(path: &Path) -> Result<Graph, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mmap = unsafe {
        memmap2::Mmap::map(&file)
            .map_err(|e| format!("Cannot mmap {}: {e}", path.display()))?
    };

    if mmap.len() < 8 {
        return Err("File too small for safetensors".to_string());
    }

    // Read header size (first 8 bytes, u64 LE)
    let header_size = u64::from_le_bytes([
        mmap[0], mmap[1], mmap[2], mmap[3],
        mmap[4], mmap[5], mmap[6], mmap[7],
    ]) as usize;

    if 8 + header_size > mmap.len() {
        return Err(format!(
            "Header size {header_size} exceeds file size {}",
            mmap.len()
        ));
    }

    // Parse JSON header
    let header_bytes = &mmap[8..8 + header_size];
    let header_str = std::str::from_utf8(header_bytes)
        .map_err(|e| format!("Invalid UTF-8 in safetensors header: {e}"))?;

    let descriptors: HashMap<String, serde_json::Value> = serde_json::from_str(header_str)
        .map_err(|e| format!("Invalid JSON in safetensors header: {e}"))?;

    let data_start = 8 + header_size;
    let mut graph = Graph::new();
    let mut loaded = 0;

    for (name, value) in &descriptors {
        // Skip __metadata__ key
        if name == "__metadata__" {
            continue;
        }

        let desc: TensorDescriptor = serde_json::from_value(value.clone())
            .map_err(|e| format!("Invalid tensor descriptor for {name}: {e}"))?;

        let dtype = safetensors_dtype(&desc.dtype);
        let [offset_start, offset_end] = desc.data_offsets;
        let byte_start = data_start + offset_start as usize;
        let byte_end = data_start + offset_end as usize;

        if byte_end > mmap.len() {
            log::warn!(
                "Tensor {name} data out of bounds: {byte_end} > {}",
                mmap.len()
            );
            continue;
        }

        let raw_data = mmap[byte_start..byte_end].to_vec();

        graph.add_tensor(
            name.clone(),
            TensorMeta::weight(desc.shape.clone(), dtype),
        );

        graph.add_weight(
            name.clone(),
            WeightData {
                data: raw_data,
                shape: desc.shape,
                dtype,
            },
        );

        loaded += 1;
    }

    log::info!(
        "Safetensors loaded: {} tensors from {}",
        loaded,
        path.display()
    );

    // Try to load config.json from same directory to detect architecture
    if let Some(dir) = path.parent() {
        let config_path = dir.join("config.json");
        if config_path.exists() {
            if let Ok(config_str) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) {
                    if let Some(archs) = config.get("architectures").and_then(|v| v.as_array()) {
                        let arch_names: Vec<String> = archs
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                        log::info!("Detected architecture: {:?}", arch_names);
                    }
                    if let Some(hidden) = config.get("hidden_size").and_then(|v| v.as_u64()) {
                        log::info!("Hidden size: {hidden}");
                    }
                    if let Some(layers) = config
                        .get("num_hidden_layers")
                        .and_then(|v| v.as_u64())
                    {
                        log::info!("Num layers: {layers}");
                    }
                }
            }
        }
    }

    Ok(graph)
}

/// Convert safetensors dtype string to DType
fn safetensors_dtype(s: &str) -> DType {
    match s {
        "F32" => DType::F32,
        "F16" => DType::F16,
        "BF16" => DType::BF16,
        "I8" => DType::I8,
        "U8" => DType::U8,
        _ => {
            log::warn!("Unknown safetensors dtype: {s}, defaulting to F32");
            DType::F32
        }
    }
}
