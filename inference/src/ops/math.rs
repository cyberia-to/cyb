use std::collections::HashMap;
use burn::prelude::*;
use crate::onnx_proto::onnx::NodeProto;

use crate::Backend;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

pub fn binary_op_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
    op: &str,
) -> Result<(), String> {
    let a = values.get(&node.input[0])
        .ok_or_else(|| format!("{op}: input {} not found", &node.input[0]))?
        .clone();
    let b = values.get(&node.input[1])
        .ok_or_else(|| format!("{op}: input {} not found", &node.input[1]))?
        .clone();

    let result = match (a, b) {
        (Value::Float2(a), Value::Float2(b)) => Value::Float2(apply_binary_2d(a, b, op)?),
        (Value::Float3(a), Value::Float3(b)) => Value::Float3(apply_binary_3d(a, b, op)?),
        (Value::Float4(a), Value::Float4(b)) => Value::Float4(apply_binary_4d(a, b, op)?),
        // Broadcasting: 1D into 2D/3D
        (Value::Float2(a), Value::Float1(b)) => {
            let n = b.dims()[0];
            let b = b.reshape([1, n]);
            Value::Float2(apply_binary_2d(a, b, op)?)
        }
        (Value::Float3(a), Value::Float1(b)) => {
            let n = b.dims()[0];
            let b = b.reshape([1, 1, n]);
            Value::Float3(apply_binary_3d(a, b, op)?)
        }
        (Value::Float1(a), Value::Float1(b)) => Value::Float1(apply_binary_1d(a, b, op)?),
        _ => return Err(format!("{op}: unsupported tensor dimension combination")),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

fn apply_binary_1d(a: Tensor<Backend, 1>, b: Tensor<Backend, 1>, op: &str) -> Result<Tensor<Backend, 1>, String> {
    match op {
        "add" => Ok(a + b),
        "sub" => Ok(a - b),
        "mul" => Ok(a * b),
        "div" => Ok(a / b),
        "pow" => Ok(a.powf(b)),
        _ => Err(format!("Unknown binary op: {op}")),
    }
}

fn apply_binary_2d(a: Tensor<Backend, 2>, b: Tensor<Backend, 2>, op: &str) -> Result<Tensor<Backend, 2>, String> {
    match op {
        "add" => Ok(a + b),
        "sub" => Ok(a - b),
        "mul" => Ok(a * b),
        "div" => Ok(a / b),
        "pow" => Ok(a.powf(b)),
        _ => Err(format!("Unknown binary op: {op}")),
    }
}

fn apply_binary_3d(a: Tensor<Backend, 3>, b: Tensor<Backend, 3>, op: &str) -> Result<Tensor<Backend, 3>, String> {
    match op {
        "add" => Ok(a + b),
        "sub" => Ok(a - b),
        "mul" => Ok(a * b),
        "div" => Ok(a / b),
        "pow" => Ok(a.powf(b)),
        _ => Err(format!("Unknown binary op: {op}")),
    }
}

fn apply_binary_4d(a: Tensor<Backend, 4>, b: Tensor<Backend, 4>, op: &str) -> Result<Tensor<Backend, 4>, String> {
    match op {
        "add" => Ok(a + b),
        "sub" => Ok(a - b),
        "mul" => Ok(a * b),
        "div" => Ok(a / b),
        "pow" => Ok(a.powf(b)),
        _ => Err(format!("Unknown binary op: {op}")),
    }
}

pub fn unary_op_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
    op: &str,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or_else(|| format!("{op}: input {} not found", &node.input[0]))?
        .clone();

    let result = match input {
        Value::Float1(t) => Value::Float1(apply_unary_1d(t, op)?),
        Value::Float2(t) => Value::Float2(apply_unary_2d(t, op)?),
        Value::Float3(t) => Value::Float3(apply_unary_3d(t, op)?),
        Value::Float4(t) => Value::Float4(apply_unary_4d(t, op)?),
        _ => return Err(format!("{op}: unsupported tensor type")),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

fn apply_unary_1d(t: Tensor<Backend, 1>, op: &str) -> Result<Tensor<Backend, 1>, String> {
    match op {
        "sqrt" => Ok(t.sqrt()),
        "neg" => Ok(t.neg()),
        "exp" => Ok(t.exp()),
        "log" => Ok(t.log()),
        "recip" => Ok(t.recip()),
        "abs" => Ok(t.abs()),
        "erf" => Ok(t.erf()),
        _ => Err(format!("Unknown unary op: {op}")),
    }
}

fn apply_unary_2d(t: Tensor<Backend, 2>, op: &str) -> Result<Tensor<Backend, 2>, String> {
    match op {
        "sqrt" => Ok(t.sqrt()),
        "neg" => Ok(t.neg()),
        "exp" => Ok(t.exp()),
        "log" => Ok(t.log()),
        "recip" => Ok(t.recip()),
        "abs" => Ok(t.abs()),
        "erf" => Ok(t.erf()),
        _ => Err(format!("Unknown unary op: {op}")),
    }
}

fn apply_unary_3d(t: Tensor<Backend, 3>, op: &str) -> Result<Tensor<Backend, 3>, String> {
    match op {
        "sqrt" => Ok(t.sqrt()),
        "neg" => Ok(t.neg()),
        "exp" => Ok(t.exp()),
        "log" => Ok(t.log()),
        "recip" => Ok(t.recip()),
        "abs" => Ok(t.abs()),
        "erf" => Ok(t.erf()),
        _ => Err(format!("Unknown unary op: {op}")),
    }
}

fn apply_unary_4d(t: Tensor<Backend, 4>, op: &str) -> Result<Tensor<Backend, 4>, String> {
    match op {
        "sqrt" => Ok(t.sqrt()),
        "neg" => Ok(t.neg()),
        "exp" => Ok(t.exp()),
        "log" => Ok(t.log()),
        "recip" => Ok(t.recip()),
        "abs" => Ok(t.abs()),
        "erf" => Ok(t.erf()),
        _ => Err(format!("Unknown unary op: {op}")),
    }
}
