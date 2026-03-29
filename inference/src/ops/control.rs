use std::collections::HashMap;
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

    // Evaluate condition — check if first element is nonzero
    let condition = match &cond_val {
        Some(v) => {
            let data = v.to_vec_f32();
            data.first().copied().unwrap_or(1.0) != 0.0
        }
        None => true, // default: take then_branch
    };

    // Find then_branch and else_branch in attributes
    let then_attr = node.attribute.iter().find(|a| a.name == "then_branch");
    let else_attr = node.attribute.iter().find(|a| a.name == "else_branch");

    let branch = if condition { then_attr } else { else_attr };

    if let Some(attr) = branch {
        if let Some(ref graph) = attr.g {
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
    let mut ok = 0;
    let mut fail = 0;
    for sub_node in &graph.node {
        // Load constant inputs for this node
        for attr in &sub_node.attribute {
            if attr.name == "value" {
                if let Some(ref t) = attr.t {
                    if let Some(val) = crate::graph::tensor_proto_to_value(t, device) {
                        if !sub_node.output.is_empty() {
                            values.insert(sub_node.output[0].clone(), val);
                        }
                    }
                }
            }
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ops::dispatch_proto(sub_node, values, device)
        }));

        match result {
            Ok(Ok(())) => { ok += 1; }
            Ok(Err(e)) => {
                fail += 1;
                if fail <= 5 {
                    log::debug!("Subgraph {} ({}): {e}", sub_node.name, sub_node.op_type);
                }
            }
            Err(_) => {
                fail += 1;
                if fail <= 5 {
                    log::debug!("Subgraph {} ({}) PANICKED", sub_node.name, sub_node.op_type);
                }
            }
        }
    }
    log::info!("Subgraph: {ok} ok, {fail} failed out of {} nodes", graph.node.len());

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
    let result = Value::from_data(vals, vec![n.max(1)], device);
    values.insert(node.output[0].clone(), result);
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

    let ndim = input.ndim();
    let axis = node.attribute.iter()
        .find(|a| a.name == "axis")
        .map(|a| {
            let v = a.i;
            if v < 0 { (ndim as i64 + v) as usize } else { v as usize }
        })
        .unwrap_or(0);

    let num_outputs = node.output.len();
    let dim_size = input.shape()[axis];

    // Get split sizes from attribute or second input
    let split_sizes: Vec<usize> = if node.input.len() > 1 && !node.input[1].is_empty() {
        if let Some(s) = values.get(&node.input[1]) {
            s.to_vec_f32().iter().map(|&v| v as usize).collect()
        } else { vec![] }
    } else {
        node.attribute.iter()
            .find(|a| a.name == "split")
            .map(|a| a.ints.iter().map(|&v| v as usize).collect())
            .unwrap_or_default()
    };

    let chunk_size = if split_sizes.is_empty() {
        if num_outputs == 0 { return Err("split: zero outputs".into()); }
        dim_size / num_outputs
    } else { 0 };

    let mut offset = 0;
    for (i, out_name) in node.output.iter().enumerate() {
        if out_name.is_empty() { continue; }
        let size = if !split_sizes.is_empty() && i < split_sizes.len() {
            split_sizes[i]
        } else {
            if i == num_outputs - 1 { dim_size - offset } else { chunk_size }
        };
        if offset + size <= dim_size {
            let chunk = input.clone().narrow(axis, offset, size);
            values.insert(out_name.clone(), chunk);
        }
        offset += size;
    }

    Ok(())
}

fn get_scalar_f32(values: &HashMap<String, Value>, name: &str) -> Option<f32> {
    let v = values.get(name)?;
    v.to_vec_f32().first().copied()
}
