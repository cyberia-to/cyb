use std::collections::HashMap;
use crate::onnx_proto::onnx::NodeProto;

use crate::Backend;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

pub fn activation_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
    op: &str,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or_else(|| format!("{op}: input {} not found", &node.input[0]))?
        .clone();

    let result = match op {
        "relu" => input.relu(),
        "gelu" => input.gelu(),
        "sigmoid" => input.sigmoid(),
        "tanh" => input.tanh(),
        "softmax" => {
            // Get axis from attributes, default to last dim
            let ndim = input.ndim();
            let axis = node.attribute.iter()
                .find(|a| a.name == "axis")
                .map(|a| {
                    let v = a.i;
                    if v < 0 { (ndim as i64 + v) as usize } else { v as usize }
                })
                .unwrap_or(ndim - 1);
            input.softmax(axis)
        }
        _ => return Err(format!("Unknown activation: {op}")),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}
