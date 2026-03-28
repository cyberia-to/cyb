use std::collections::HashMap;
use burn::prelude::*;
use prost::Message;
use crate::onnx_proto::onnx::{NodeProto, GraphProto};

use crate::Backend;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

/// Execute an If node — picks then_branch or else_branch based on condition
pub fn if_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    // Get condition from input[0]
    let cond_val = if !node.input.is_empty() && !node.input[0].is_empty() {
        values.get(&node.input[0]).cloned()
    } else {
        None
    };

    // Evaluate condition — default to true (then_branch)
    let condition = match &cond_val {
        Some(Value::Bool1(t)) => {
            let data = t.to_data();
            data.as_slice::<bool>().map(|s| s.first().copied().unwrap_or(true)).unwrap_or(true)
        }
        Some(Value::Float1(t)) => {
            let data = t.to_data();
            data.as_slice::<f32>().map(|s| s.first().copied().unwrap_or(1.0) != 0.0).unwrap_or(true)
        }
        Some(Value::Int1(t)) => {
            let data = t.to_data();
            data.as_slice::<i64>().map(|s| s.first().copied().unwrap_or(1) != 0).unwrap_or(true)
        }
        _ => true, // default: take then_branch
    };

    // Find then_branch and else_branch in attributes
    let then_attr = node.attribute.iter().find(|a| a.name == "then_branch");
    let else_attr = node.attribute.iter().find(|a| a.name == "else_branch");

    let branch = if condition { then_attr } else { else_attr };

    if let Some(attr) = branch {
        if let Some(ref graph) = attr.g {
            // Execute subgraph
            execute_subgraph(graph, values, device, &node.output)?;
        } else {
            log::warn!("If: branch attribute has no graph");
        }
    } else {
        log::warn!("If: missing {} branch", if condition { "then" } else { "else" });
    }

    Ok(())
}

/// Execute a subgraph within the current value context
fn execute_subgraph(
    graph: &GraphProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
    parent_outputs: &[String],
) -> Result<(), String> {
    // Load subgraph initializers
    for init in &graph.initializer {
        if let Some(val) = crate::graph::tensor_proto_to_value(init, device) {
            values.insert(init.name.clone(), val);
        }
    }

    // Execute subgraph nodes
    for sub_node in &graph.node {
        // Load constant inputs for this node
        for attr in &sub_node.attribute {
            if let Some(ref t) = attr.t {
                if let Some(val) = crate::graph::tensor_proto_to_value(t, device) {
                    // Constants embedded in attributes
                    if !sub_node.output.is_empty() {
                        values.insert(sub_node.output[0].clone(), val);
                    }
                }
            }
        }

        if let Err(e) = crate::ops::dispatch_proto(sub_node, values, device) {
            log::debug!("Subgraph node {} ({}): {e}", sub_node.name, sub_node.op_type);
        }
    }

    // Map subgraph outputs to parent outputs
    for (i, sub_out) in graph.output.iter().enumerate() {
        if i < parent_outputs.len() {
            if let Some(val) = values.get(&sub_out.name).cloned() {
                values.insert(parent_outputs[i].clone(), val);
            }
        }
    }

    Ok(())
}

/// Range operator — generates a sequence [start, start+delta, start+2*delta, ..., limit)
pub fn range_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    let start = get_scalar_f32(values, &node.input[0]).unwrap_or(0.0);
    let limit = get_scalar_f32(values, &node.input[1]).unwrap_or(1.0);
    let delta = get_scalar_f32(values, &node.input[2]).unwrap_or(1.0);

    let mut vals = Vec::new();
    let mut v = start;
    while (delta > 0.0 && v < limit) || (delta < 0.0 && v > limit) {
        vals.push(v);
        v += delta;
    }

    let n = vals.len();
    if n == 0 {
        vals.push(start);
    }
    let data = burn::tensor::TensorData::new(vals, vec![n.max(1)]);
    values.insert(node.output[0].clone(), Value::Float1(Tensor::from_data(data, device)));
    Ok(())
}

/// Split operator — split a tensor along an axis
pub fn split_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("split: input not found")?.clone();

    let axis = node.attribute.iter()
        .find(|a| a.name == "axis")
        .map(|a| a.i as usize)
        .unwrap_or(0);

    // Number of outputs determines split count
    let num_outputs = node.output.len();

    match input {
        Value::Float3(t) => {
            let dim_size = t.dims()[axis];
            let chunk_size = dim_size / num_outputs;

            for (i, out_name) in node.output.iter().enumerate() {
                if out_name.is_empty() { continue; }
                let start = i * chunk_size;
                let end = if i == num_outputs - 1 { dim_size } else { (i + 1) * chunk_size };

                // Use narrow/slice
                let chunk = t.clone().narrow(axis, start, end - start);
                values.insert(out_name.clone(), Value::Float3(chunk));
            }
        }
        Value::Float2(t) => {
            let dim_size = t.dims()[axis];
            let chunk_size = dim_size / num_outputs;

            for (i, out_name) in node.output.iter().enumerate() {
                if out_name.is_empty() { continue; }
                let start = i * chunk_size;
                let end = if i == num_outputs - 1 { dim_size } else { (i + 1) * chunk_size };

                let chunk = t.clone().narrow(axis, start, end - start);
                values.insert(out_name.clone(), Value::Float2(chunk));
            }
        }
        _ => return Err("split: unsupported dimensions".into()),
    }

    Ok(())
}

fn get_scalar_f32(values: &HashMap<String, Value>, name: &str) -> Option<f32> {
    match values.get(name)? {
        Value::Float1(t) => {
            let data = t.to_data();
            data.as_slice::<f32>().ok().and_then(|s| s.first().copied())
        }
        _ => None,
    }
}
