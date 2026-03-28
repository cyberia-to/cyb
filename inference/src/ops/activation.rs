use std::collections::HashMap;
use burn::tensor::activation;
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

    let result = match input {
        Value::Float1(t) => Value::Float1(apply_activation_1d(t, op)?),
        Value::Float2(t) => Value::Float2(apply_activation_2d(t, op)?),
        Value::Float3(t) => Value::Float3(apply_activation_3d(t, op)?),
        Value::Float4(t) => Value::Float4(apply_activation_4d(t, op)?),
        _ => return Err(format!("{op}: unsupported tensor type")),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

fn apply_activation_1d(t: burn::prelude::Tensor<Backend, 1>, op: &str) -> Result<burn::prelude::Tensor<Backend, 1>, String> {
    match op {
        "relu" => Ok(activation::relu(t)),
        "gelu" => Ok(activation::gelu(t)),
        "sigmoid" => Ok(activation::sigmoid(t)),
        "softmax" => Ok(activation::softmax(t, 0)),
        "tanh" => Ok(activation::tanh(t)),
        _ => Err(format!("Unknown activation: {op}")),
    }
}

fn apply_activation_2d(t: burn::prelude::Tensor<Backend, 2>, op: &str) -> Result<burn::prelude::Tensor<Backend, 2>, String> {
    match op {
        "relu" => Ok(activation::relu(t)),
        "gelu" => Ok(activation::gelu(t)),
        "sigmoid" => Ok(activation::sigmoid(t)),
        "softmax" => Ok(activation::softmax(t, 1)),
        "tanh" => Ok(activation::tanh(t)),
        _ => Err(format!("Unknown activation: {op}")),
    }
}

fn apply_activation_3d(t: burn::prelude::Tensor<Backend, 3>, op: &str) -> Result<burn::prelude::Tensor<Backend, 3>, String> {
    match op {
        "relu" => Ok(activation::relu(t)),
        "gelu" => Ok(activation::gelu(t)),
        "sigmoid" => Ok(activation::sigmoid(t)),
        "softmax" => Ok(activation::softmax(t, 2)),
        "tanh" => Ok(activation::tanh(t)),
        _ => Err(format!("Unknown activation: {op}")),
    }
}

fn apply_activation_4d(t: burn::prelude::Tensor<Backend, 4>, op: &str) -> Result<burn::prelude::Tensor<Backend, 4>, String> {
    match op {
        "relu" => Ok(activation::relu(t)),
        "gelu" => Ok(activation::gelu(t)),
        "sigmoid" => Ok(activation::sigmoid(t)),
        "softmax" => Ok(activation::softmax(t, 3)),
        "tanh" => Ok(activation::tanh(t)),
        _ => Err(format!("Unknown activation: {op}")),
    }
}
