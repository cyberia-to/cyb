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
        })
    }

    pub fn from_model_proto(model: ModelProto) -> Result<Self, String> {
        let graph = model.graph.ok_or("No graph in model")?;
        let graph_outputs: Vec<String> = graph.output.iter()
            .map(|o| o.name.clone())
            .collect();

        let mut executor = Self {
            nodes: graph.node,
            graph_outputs,
            values: HashMap::new(),
        };

        // Pre-load initializers
        log::info!("Loading {} initializers", graph.initializer.len());
        // Note: graph was moved, we need to handle this differently
        // For now, use from_file which loads everything
        let _ = &executor;
        Ok(executor)
    }

    /// Load all model weights from the ONNX file
    pub fn load_from_file(&mut self, path: &Path, device: &Device) -> Result<(), String> {
        let model = load_model_proto(path)?;
        let graph = model.graph.ok_or("No graph in model")?;

        let mut count = 0;
        for init in &graph.initializer {
            if let Some(val) = tensor_proto_to_value(init, device) {
                log::debug!("Loaded initializer: {} shape={:?}", init.name, val.shape());
                self.values.insert(init.name.clone(), val);
                count += 1;
            }
        }
        log::info!("Loaded {count} initializer tensors");

        self.nodes = graph.node;
        self.graph_outputs = graph.output.iter().map(|o| o.name.clone()).collect();
        Ok(())
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
            match crate::ops::dispatch_proto(node, &mut self.values, device) {
                Ok(()) => {
                    ok_count += 1;
                    if i < 240 {
                        // Debug first nodes
                        let produced: Vec<_> = node.output.iter()
                            .filter(|o| !o.is_empty() && self.values.contains_key(*o))
                            .map(|o| format!("{}={:?}", o, self.values[o].shape()))
                            .collect();
                        if !produced.is_empty() {
                            log::debug!("[{i}] {} {} → {}", node.op_type, node.name, produced.join(", "));
                        }
                    }
                }
                Err(e) => {
                    fail_count += 1;
                    if fail_count <= 5 {
                        log::warn!("[{i}] Node {} ({}): {e}", node.name, node.op_type);
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
