use std::collections::HashMap;
use burn::prelude::*;
use crate::onnx_proto::onnx::NodeProto;

use crate::Backend;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

pub fn matmul_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let a = values.get(&node.input[0])
        .ok_or_else(|| format!("matmul: input {} not found", &node.input[0]))?
        .clone();
    let b = values.get(&node.input[1])
        .ok_or_else(|| format!("matmul: input {} not found", &node.input[1]))?
        .clone();

    let result = match (a, b) {
        (Value::Float2(a), Value::Float2(b)) => Value::Float2(a.matmul(b)),
        (Value::Float3(a), Value::Float3(b)) => Value::Float3(a.matmul(b)),
        (Value::Float4(a), Value::Float4(b)) => Value::Float4(a.matmul(b)),
        (Value::Float3(a), Value::Float2(b)) => {
            // [batch, m, k] × [k, n] → broadcast b to [1, k, n]
            let [k, n] = b.dims();
            let b = b.reshape([1, k, n]);
            Value::Float3(a.matmul(b))
        }
        _ => return Err("matmul: unsupported tensor dimension combination".into()),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}
