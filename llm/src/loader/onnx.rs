//! ONNX model loader — parse protobuf, extract weights as typed tensors, build Graph IR

use std::collections::HashMap;
use std::path::Path;

use prost::Message;

use crate::ir::{DType, Graph, Op, TensorMeta, WeightData};
use crate::onnx_proto::onnx::ModelProto;

/// Load an ONNX model and build Graph IR
pub fn load_onnx(path: &Path) -> Result<Graph, String> {
    let model = load_model_proto(path)?;
    let onnx_graph = model.graph.ok_or("No graph in ONNX model")?;

    log::info!(
        "ONNX model: {} initializers, {} nodes",
        onnx_graph.initializer.len(),
        onnx_graph.node.len()
    );

    let mut graph = Graph::new();
    let model_dir = path.parent().unwrap_or(Path::new("."));

    // Load initializers (weights)
    for init in &onnx_graph.initializer {
        let shape: Vec<usize> = init.dims.iter().map(|&d| d as usize).collect();
        let dtype = onnx_dtype_to_ir(init.data_type);

        let raw_data = read_tensor_raw(init, model_dir)?;

        graph.add_weight(
            init.name.clone(),
            WeightData {
                data: raw_data,
                shape: shape.clone(),
                dtype, needs_transpose: false },
        );
        graph.add_tensor(
            init.name.clone(),
            TensorMeta::weight(shape, dtype),
        );
    }

    // Build nodes from ONNX ops
    for (i, node) in onnx_graph.node.iter().enumerate() {
        let op = onnx_op_to_ir(node);
        let inputs: Vec<String> = node.input.iter().cloned().collect();
        let outputs: Vec<String> = node.output.iter().cloned().collect();

        graph.add_node(op, inputs, outputs);

        // Register output tensor metadata (shape unknown at this point)
        for out in &node.output {
            if !graph.tensors.contains_key(out) {
                graph.add_tensor(
                    out.clone(),
                    TensorMeta::fixed(vec![], DType::F32),
                );
            }
        }

        if (i + 1) % 500 == 0 {
            log::debug!("Loaded {}/{} ONNX nodes", i + 1, onnx_graph.node.len());
        }
    }

    // Rename onnx::MatMul_* weights by tracing graph edges
    // ONNX nodes have output names like "/model/layers.0/self_attn/q_proj/MatMul_output_0"
    // Their weight input is "onnx::MatMul_XXXX" — rename to match output path
    let rename_map: Vec<(String, String)> = onnx_graph.node.iter()
        .filter(|n| n.op_type == "MatMul" || n.op_type == "MatMulNBits")
        .filter_map(|n| {
            // Find the weight input (starts with "onnx::")
            let weight_input = n.input.iter().find(|i| i.starts_with("onnx::"))?;
            // Extract layer path from output name
            let out = n.output.first()?;
            // Convert "/model/layers.0/self_attn/q_proj/MatMul_output_0" → "model.layers.0.self_attn.q_proj.weight"
            let path = out.trim_start_matches('/')
                .replace("/MatMul_output_0", "")
                .replace("/Gemm_output_0", "")
                .replace('/', ".");
            let new_name = format!("{path}.weight");
            Some((weight_input.clone(), new_name))
        })
        .collect();

    let renamed = rename_map.len();
    for (old, new) in rename_map {
        if let Some(w) = graph.weights.remove(&old) {
            graph.weights.insert(new, w);
        }
    }
    if renamed > 0 {
        let sample: Vec<&String> = graph.weights.keys().filter(|k| k.contains("layers.0")).take(5).collect();
        log::info!("ONNX: renamed {renamed} MatMul weights. Sample layer 0: {:?}", sample);
    }

    log::info!(
        "ONNX graph loaded: {} nodes, {} weights",
        graph.len(),
        graph.weights.len()
    );

    Ok(graph)
}

/// Load and print ONNX graph info
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

/// Load and parse an ONNX model protobuf from a file
pub fn load_model_proto(path: &Path) -> Result<ModelProto, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut buf = Vec::new();
    use std::io::Read;
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    ModelProto::decode(&*buf)
        .map_err(|e| format!("Failed to decode ONNX protobuf: {e}"))
}

/// Read raw bytes from a TensorProto, supporting external data
pub fn read_tensor_raw(
    tp: &crate::onnx_proto::onnx::TensorProto,
    model_dir: &Path,
) -> Result<Vec<u8>, String> {
    if tp.data_location == 1 {
        // External data
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

        let data_path = model_dir.join(&location);
        // Try original path, then fallback to weights.onnx_data (converter renames)
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
                "External data {} out of bounds: offset={offset} length={length} file_size={}",
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
        // Empty tensor — common for zero_point in symmetric quantization
        // Return zeros based on dims
        let num_elements: usize = tp.dims.iter().map(|&d| d as usize).product();
        let elem_size = match tp.data_type {
            1 => 4, // FLOAT
            6 => 4, // INT32
            7 => 8, // INT64
            _ => 1, // UINT8, INT8, etc
        };
        if num_elements > 0 {
            log::debug!("Tensor {} has no data, returning {} zero bytes", tp.name, num_elements * elem_size);
            Ok(vec![0u8; num_elements * elem_size])
        } else {
            // Scalar zero
            Ok(vec![0u8; elem_size])
        }
    }
}

/// Convert ONNX data type integer to our DType
fn onnx_dtype_to_ir(data_type: i32) -> DType {
    match data_type {
        1 => DType::F32,   // FLOAT
        10 => DType::F16,  // FLOAT16
        16 => DType::BF16, // BFLOAT16
        2 => DType::Q8,    // UINT8 (used for Q4 packed)
        3 => DType::Q8,    // INT8
        _ => DType::F32,   // default
    }
}

/// Convert ONNX op type to Graph IR Op
fn onnx_op_to_ir(node: &crate::onnx_proto::onnx::NodeProto) -> Op {
    match node.op_type.as_str() {
        "MatMul" => Op::Matmul,
        "MatMulNBits" => Op::Matmul,
        "Add" => Op::Add,
        "Mul" => Op::Mul,
        "Sub" => Op::Sub,
        "Div" => Op::Div,
        "Relu" => Op::Relu,
        "Sigmoid" => Op::Sigmoid,
        "Softmax" => Op::Softmax { dim: -1 },
        "Tanh" => Op::Tanh,
        "Gelu" | "FastGelu" => Op::Gelu { approximate: false },
        "Silu" => Op::Silu,
        "LayerNormalization" => {
            let eps = get_attr_f(node, "epsilon").unwrap_or(1e-5);
            Op::LayerNorm { eps }
        }
        "SimplifiedLayerNormalization" | "RmsNormalization" => {
            let eps = get_attr_f(node, "epsilon").unwrap_or(1e-6);
            Op::RmsNorm { eps }
        }
        "RotaryEmbedding" => {
            Op::Rope { head_dim: 0, base: 10000.0 } // detected from weights later
        }
        "GroupQueryAttention" | "MultiHeadAttention" => {
            let num_heads = get_attr_i(node, "num_heads").unwrap_or(1) as u32;
            let kv_heads = get_attr_i(node, "kv_num_heads").unwrap_or(num_heads as i64) as u32;
            Op::Sdpa {
                num_heads,
                kv_heads,
                head_dim: 0,
                causal: true,
            }
        }
        "Reshape" => Op::Reshape { shape: vec![] },
        "Transpose" => {
            let perm = get_attr_ints(node, "perm")
                .unwrap_or_default()
                .iter()
                .map(|&v| v as usize)
                .collect();
            Op::Transpose { perm }
        }
        "Concat" => {
            let axis = get_attr_i(node, "axis").unwrap_or(0) as usize;
            Op::Concat { axis }
        }
        "Gather" => Op::TokenEmbed, // embedding lookup
        "DequantizeLinear" => Op::Dequantize,
        "ArgMax" => Op::Argmax,
        _ => {
            // Unknown op — treat as generic matmul placeholder
            log::debug!("Unknown ONNX op: {}", node.op_type);
            Op::Add // fallback
        }
    }
}

/// Get float attribute from ONNX node
fn get_attr_f(node: &crate::onnx_proto::onnx::NodeProto, name: &str) -> Option<f32> {
    node.attribute
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.f)
}

/// Get integer attribute from ONNX node
fn get_attr_i(node: &crate::onnx_proto::onnx::NodeProto, name: &str) -> Option<i64> {
    node.attribute
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.i)
}

/// Get integer list attribute from ONNX node
fn get_attr_ints(node: &crate::onnx_proto::onnx::NodeProto, name: &str) -> Option<Vec<i64>> {
    node.attribute
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.ints.clone())
}

/// Convert raw bytes to f32 based on ONNX data_type
pub fn raw_to_f32(raw: &[u8], data_type: i32) -> Vec<f32> {
    match data_type {
        1 => {
            // FLOAT
            raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        }
        10 => {
            // FLOAT16
            raw.chunks_exact(2)
                .map(|c| half::f16::from_le_bytes([c[0], c[1]]).to_f32())
                .collect()
        }
        2 => {
            // UINT8 -> f32
            raw.iter().map(|&b| b as f32).collect()
        }
        7 => {
            // INT64 -> f32
            raw.chunks_exact(8)
                .map(|c| {
                    i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32
                })
                .collect()
        }
        6 => {
            // INT32 -> f32
            raw.chunks_exact(4)
                .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32)
                .collect()
        }
        _ => {
            log::warn!("Unsupported data type {data_type}, treating as f32 bytes");
            raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        }
    }
}

/// Pack raw UINT8 bytes into u32 (4 bytes -> 1 u32, little-endian)
pub fn pack_bytes_to_u32(raw: &[u8]) -> Vec<u32> {
    raw.chunks(4)
        .map(|chunk| {
            let mut val = 0u32;
            for (i, &b) in chunk.iter().enumerate() {
                val |= (b as u32) << (i * 8);
            }
            val
        })
        .collect()
}
