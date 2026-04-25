//! ONNX loader — parse the protobuf, extract initializer tensors.
//!
//! Importer only needs the weight data — ONNX op-graph semantics (Conv, MatMul,
//! etc.) are irrelevant once we re-pack as a `.model` file. What matters is:
//!   1. Initializer tensors (the actual weights)
//!   2. External-data files — weights are often stored in a sibling
//!      `weights.onnx_data` / `model.onnx_data` with offset/length.
//!   3. Renaming `onnx::MatMul_XXXX` to HF-style names by tracing graph edges
//!      so downstream tooling finds familiar keys.
//!
//! Ported from llm/src/loader/onnx.rs, minus IR Graph / Op translation.

use std::collections::HashMap;
use std::path::Path;

use prost::Message;

use crate::onnx_proto::onnx::ModelProto;
use crate::types::{DType, Weight, Weights};

/// Load an ONNX model and return its weights.
pub fn load_onnx(path: &Path) -> Result<Weights, String> {
    let model = load_model_proto(path)?;
    let onnx_graph = model.graph.ok_or("No graph in ONNX model")?;

    log::info!(
        "ONNX model: {} initializers, {} nodes",
        onnx_graph.initializer.len(),
        onnx_graph.node.len()
    );

    let mut weights = Weights::new();
    let model_dir = path.parent().unwrap_or(Path::new("."));

    // Pull every initializer into our Weights table.
    for init in &onnx_graph.initializer {
        let shape: Vec<usize> = init.dims.iter().map(|&d| d as usize).collect();
        let dtype = onnx_dtype_to_dtype(init.data_type);
        let raw = read_tensor_raw(init, model_dir)?;
        weights.insert(
            init.name.clone(),
            Weight {
                data: raw,
                shape,
                dtype,
                needs_transpose: false,
            },
        );
    }

    // Rename `onnx::MatMul_XXXX` weights to HF-style `model.layers.N.*.weight`
    // names by tracing the MatMul nodes that consume them.
    let rename_map: Vec<(String, String)> = onnx_graph
        .node
        .iter()
        .filter(|n| n.op_type == "MatMul" || n.op_type == "MatMulNBits")
        .filter_map(|n| {
            let weight_input = n.input.iter().find(|i| i.starts_with("onnx::"))?;
            let out = n.output.first()?;
            let path = out
                .trim_start_matches('/')
                .replace("/MatMul_output_0", "")
                .replace("/Gemm_output_0", "")
                .replace('/', ".");
            Some((weight_input.clone(), format!("{path}.weight")))
        })
        .collect();

    let renamed = rename_map.len();
    for (old, new) in rename_map {
        if let Some(w) = weights.weights.remove(&old) {
            weights.weights.insert(new, w);
        }
    }
    if renamed > 0 {
        let sample: Vec<&String> = weights
            .weights
            .keys()
            .filter(|k| k.contains("layers.0"))
            .take(5)
            .collect();
        log::info!(
            "ONNX: renamed {renamed} MatMul weights. Sample layer 0: {:?}",
            sample
        );
    }

    log::info!(
        "ONNX loaded: {} weights from {}",
        weights.len(),
        path.display()
    );
    Ok(weights)
}

/// Load and print ONNX graph info (diagnostic).
pub fn load_onnx_info(path: &str) -> Result<String, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let model = load_model_proto(path)?;
    let graph = model.graph.ok_or("No graph in model")?;

    let mut info = String::new();
    info.push_str(&format!("IR version: {}\n", model.ir_version));
    info.push_str(&format!(
        "Opset: {}\n",
        model.opset_import.first().map(|o| o.version).unwrap_or(0)
    ));
    info.push_str(&format!("Nodes: {}\n", graph.node.len()));
    info.push_str(&format!("Inputs: {}\n", graph.input.len()));
    info.push_str(&format!("Outputs: {}\n", graph.output.len()));
    info.push_str(&format!("Initializers: {}\n", graph.initializer.len()));

    let mut op_counts: HashMap<String, usize> = HashMap::new();
    for node in &graph.node {
        *op_counts.entry(node.op_type.clone()).or_insert(0) += 1;
    }

    info.push_str("\nOperator counts:\n");
    let mut counts: Vec<_> = op_counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    for (op, count) in counts {
        info.push_str(&format!("  {op}: {count}\n"));
    }

    Ok(info)
}

fn load_model_proto(path: &Path) -> Result<ModelProto, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut buf = Vec::new();
    use std::io::Read;
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    ModelProto::decode(&*buf)
        .map_err(|e| format!("Failed to decode ONNX protobuf: {e}"))
}

fn read_tensor_raw(
    tp: &crate::onnx_proto::onnx::TensorProto,
    model_dir: &Path,
) -> Result<Vec<u8>, String> {
    if tp.data_location == 1 {
        // External data — bytes live in a sibling file referenced by
        // "location" / "offset" / "length".
        let mut location = String::new();
        let mut offset: u64 = 0;
        let mut length: u64 = 0;
        for entry in &tp.external_data {
            match entry.key.as_str() {
                "location" => location = entry.value.clone(),
                "offset" => offset = entry.value.parse().unwrap_or(0),
                "length" => length = entry.value.parse().unwrap_or(0),
                _ => {}
            }
        }
        if location.is_empty() || length == 0 {
            return Err(format!("External tensor {} has no location/length", tp.name));
        }

        // Fallback: our converter sometimes renames the external file to
        // `weights.onnx_data` — check that too.
        let data_path = model_dir.join(&location);
        let actual_path = if data_path.exists() {
            data_path.clone()
        } else {
            let alt = model_dir.join("weights.onnx_data");
            if alt.exists() { alt } else { data_path.clone() }
        };
        let file = std::fs::File::open(&actual_path)
            .map_err(|e| format!("Cannot open external data {}: {e}", actual_path.display()))?;
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(|e| format!("Cannot mmap {}: {e}", data_path.display()))?
        };
        let end = (offset + length) as usize;
        if end > mmap.len() {
            return Err(format!(
                "External data {} OOB: offset={offset} length={length} file_size={}",
                tp.name,
                mmap.len()
            ));
        }
        Ok(mmap[offset as usize..end].to_vec())
    } else if !tp.raw_data.is_empty() {
        Ok(tp.raw_data.clone())
    } else if !tp.float_data.is_empty() {
        Ok(bytemuck::cast_slice(&tp.float_data).to_vec())
    } else if !tp.int32_data.is_empty() {
        Ok(bytemuck::cast_slice(&tp.int32_data).to_vec())
    } else {
        // Empty tensor — common for zero_point in symmetric quantization.
        let num_elements: usize = tp.dims.iter().map(|&d| d as usize).product();
        let elem_size = match tp.data_type {
            1 => 4,
            6 => 4,
            7 => 8,
            _ => 1,
        };
        Ok(vec![0u8; num_elements.max(1) * elem_size])
    }
}

fn onnx_dtype_to_dtype(data_type: i32) -> DType {
    match data_type {
        1 => DType::F32,
        10 => DType::F16,
        16 => DType::BF16,
        2 | 3 => DType::Q8_0, // UINT8 / INT8 used for Q4/Q8 packed
        _ => DType::F32,
    }
}
