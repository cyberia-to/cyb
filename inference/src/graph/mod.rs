pub mod value;

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use burn::prelude::*;
use burn::tensor::TensorData;
use crate::onnx_proto::onnx::ModelProto;
use prost::Message;

use crate::Backend;
use value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

/// Load and parse an ONNX model from a file
fn load_model_proto(path: &Path) -> Result<ModelProto, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    ModelProto::decode(&*buf)
        .map_err(|e| format!("Failed to decode ONNX protobuf: {e}"))
}

/// Load an ONNX file and print graph info
pub fn load_onnx_info(path: &str) -> Result<String, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let model = load_model_proto(path)?;
    let graph = model.graph.ok_or("No graph in model")?;

    let mut info = String::new();
    info.push_str(&format!("IR version: {}\n", model.ir_version));
    info.push_str(&format!("Opset: {}\n", model.opset_import.first().map(|o| o.version).unwrap_or(0)));
    info.push_str(&format!("Nodes: {}\n", graph.node.len()));
    info.push_str(&format!("Inputs: {}\n", graph.input.len()));
    info.push_str(&format!("Outputs: {}\n", graph.output.len()));
    info.push_str(&format!("Initializers: {}\n", graph.initializer.len()));

    // Count op types
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

/// Convert an ONNX TensorProto to a burn Value, with external data support
pub fn tensor_proto_to_value_ext(
    tp: &crate::onnx_proto::onnx::TensorProto,
    device: &Device,
    model_dir: &Path,
) -> Option<Value> {
    // Check if data is external
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
            log::warn!("External tensor {} has no location/length", tp.name);
            return None;
        }

        let data_path = model_dir.join(&location);
        if !data_path.exists() {
            log::warn!("External data file not found: {}", data_path.display());
            return None;
        }

        // Memory-map the external file and read the slice
        let file = std::fs::File::open(&data_path).ok()?;
        let mmap = unsafe { memmap2::Mmap::map(&file).ok()? };
        let end = (offset + length) as usize;
        if end > mmap.len() {
            log::warn!("External data {} out of bounds: offset={offset} length={length} file_size={}", tp.name, mmap.len());
            return None;
        }

        let raw_data = &mmap[offset as usize..end];

        // Create a modified TensorProto with the raw data inlined
        let mut tp_copy = tp.clone();
        tp_copy.raw_data = raw_data.to_vec();
        tp_copy.data_location = 0;

        return tensor_proto_to_value(&tp_copy, device);
    }

    tensor_proto_to_value(tp, device)
}

/// Convert an ONNX TensorProto to a burn Value
pub fn tensor_proto_to_value(
    tp: &crate::onnx_proto::onnx::TensorProto,
    device: &Device,
) -> Option<Value> {
    let shape: Vec<usize> = tp.dims.iter().map(|&d| d as usize).collect();
    let rank = shape.len();

    // Extract float data
    let floats: Vec<f32> = if !tp.float_data.is_empty() {
        tp.float_data.clone()
    } else if !tp.raw_data.is_empty() {
        // data_type 1 = FLOAT
        match tp.data_type {
            1 => { // FLOAT
                tp.raw_data.chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect()
            }
            7 => { // INT64 -> convert to f32
                tp.raw_data.chunks_exact(8)
                    .map(|c| i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32)
                    .collect()
            }
            6 => { // INT32 -> convert to f32
                tp.raw_data.chunks_exact(4)
                    .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32)
                    .collect()
            }
            10 => { // FLOAT16
                tp.raw_data.chunks_exact(2)
                    .map(|c| half::f16::from_le_bytes([c[0], c[1]]).to_f32())
                    .collect()
            }
            2 => { // UINT8
                tp.raw_data.iter().map(|&b| b as f32).collect()
            }
            3 => { // INT8
                tp.raw_data.iter().map(|&b| b as i8 as f32).collect()
            }
            9 => { // BOOL
                tp.raw_data.iter().map(|&b| if b != 0 { 1.0 } else { 0.0 }).collect()
            }
            5 => { // INT16
                tp.raw_data.chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32)
                    .collect()
            }
            11 => { // DOUBLE
                tp.raw_data.chunks_exact(8)
                    .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32)
                    .collect()
            }
            _ => {
                log::warn!("Unsupported tensor data type: {}", tp.data_type);
                return None;
            }
        }
    } else if !tp.int64_data.is_empty() {
        tp.int64_data.iter().map(|&v| v as f32).collect()
    } else {
        return None;
    };

    let total: usize = if shape.is_empty() { 1 } else { shape.iter().product() };
    if floats.len() != total {
        log::warn!("Tensor {} shape {:?} mismatch: expected {total}, got {}", tp.name, shape, floats.len());
        return None;
    }

    // Scalars (rank 0) → treat as rank 1 with size 1
    let (shape, rank) = if shape.is_empty() {
        (vec![1_usize], 1)
    } else {
        (shape, rank)
    };

    let data = TensorData::new(floats, shape.clone());
    match rank {
        1 => Some(Value::Float1(Tensor::from_data(data, device))),
        2 => Some(Value::Float2(Tensor::from_data(data, device))),
        3 => Some(Value::Float3(Tensor::from_data(data, device))),
        4 => Some(Value::Float4(Tensor::from_data(data, device))),
        _ => {
            log::warn!("Unsupported rank {rank} for tensor {}", tp.name);
            None
        }
    }
}

/// Runtime ONNX graph executor over burn tensors
pub struct OnnxExecutor {
    nodes: Vec<crate::onnx_proto::onnx::NodeProto>,
    graph_outputs: Vec<String>,
    values: HashMap<String, Value>,
    /// Names of initializer tensors (weights) — preserved between runs
    initializer_names: std::collections::HashSet<String>,
}

impl OnnxExecutor {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let model = load_model_proto(path)?;
        let graph = model.graph.ok_or("No graph in model")?;

        let graph_outputs: Vec<String> = graph.output.iter()
            .map(|o| o.name.clone())
            .collect();

        Ok(Self {
            nodes: graph.node,
            graph_outputs,
            values: HashMap::new(),
            initializer_names: std::collections::HashSet::new(),
        })
    }

    /// Load all model weights from the ONNX file
    pub fn load_from_file(&mut self, path: &Path, device: &Device) -> Result<(), String> {
        let model = load_model_proto(path)?;
        let graph = model.graph.ok_or("No graph in model")?;

        let model_dir = path.parent().unwrap_or(Path::new("."));

        let mut count = 0;
        for init in &graph.initializer {
            if let Some(val) = tensor_proto_to_value_ext(init, device, model_dir) {
                log::debug!("Loaded initializer: {} shape={:?}", init.name, val.shape());
                self.initializer_names.insert(init.name.clone());
                self.values.insert(init.name.clone(), val);
                count += 1;
            }
        }
        log::info!("Loaded {count} initializer tensors");

        self.nodes = graph.node;
        self.graph_outputs = graph.output.iter().map(|o| o.name.clone()).collect();
        Ok(())
    }

    /// Clear intermediate values, keeping only initializers and cached weights
    pub fn clear_intermediates(&mut self) {
        self.values.retain(|name, _| {
            self.initializer_names.contains(name) || name.ends_with("__dequant_t")
        });
    }

    /// Execute the graph with given inputs
    pub fn run(
        &mut self,
        inputs: HashMap<String, Value>,
        device: &Device,
    ) -> Result<HashMap<String, Value>, String> {
        for (name, val) in inputs {
            self.values.insert(name, val);
        }

        let mut ok_count = 0;
        let mut fail_count = 0;
        for (i, node) in self.nodes.iter().enumerate() {
            // Catch panics from burn tensor operations
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::ops::dispatch_proto(node, &mut self.values, device)
            }));

            match result {
                Ok(Ok(())) => {
                    ok_count += 1;
                    // Only log on trace level for performance
                    if log::log_enabled!(log::Level::Trace) && i < 300 {
                        for out in &node.output {
                            if !out.is_empty() {
                                if let Some(v) = self.values.get(out) {
                                    log::trace!("[{i}] {} {} → {} {:?}", node.op_type, node.name, out, v.shape());
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    fail_count += 1;
                    if fail_count <= 10 {
                        log::warn!("[{i}] {} ({}): {e}", node.name, node.op_type);
                    }
                }
                Err(_) => {
                    fail_count += 1;
                    if fail_count <= 10 {
                        log::warn!("[{i}] {} ({}) PANICKED", node.name, node.op_type);
                    }
                }
            }
        }
        log::info!("Graph execution: {ok_count} ok, {fail_count} failed out of {} nodes", self.nodes.len());

        let mut outputs = HashMap::new();
        for name in &self.graph_outputs {
            if let Some(val) = self.values.get(name) {
                outputs.insert(name.clone(), val.clone());
            }
        }

        Ok(outputs)
    }
}
