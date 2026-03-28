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

    // Handle Gemm transB attribute
    let trans_b = node.attribute.iter()
        .find(|a| a.name == "transB")
        .map(|a| a.i != 0)
        .unwrap_or(false);

    let result = match (a, b) {
        (Value::Float2(a), Value::Float2(b)) => {
            let b = if trans_b { b.transpose() } else { b };
            let mut out = a.matmul(b);
            // Gemm: add bias if present (input[2])
            if node.input.len() > 2 && !node.input[2].is_empty() {
                if let Some(Value::Float1(bias)) = values.get(&node.input[2]) {
                    let n = bias.dims()[0];
                    out = out + bias.clone().reshape([1, n]);
                }
            }
            Value::Float2(out)
        }
        (Value::Float3(a), Value::Float3(b)) => Value::Float3(a.matmul(b)),
        (Value::Float4(a), Value::Float4(b)) => Value::Float4(a.matmul(b)),
        (Value::Float3(a), Value::Float2(b)) => {
            let b = if trans_b { b.transpose() } else { b };
            let [k, n] = b.dims();
            let b = b.reshape([1, k, n]);
            let mut out = a.matmul(b);
            if node.input.len() > 2 && !node.input[2].is_empty() {
                if let Some(Value::Float1(bias)) = values.get(&node.input[2]) {
                    let bn = bias.dims()[0];
                    out = out + bias.clone().reshape([1, 1, bn]);
                }
            }
            Value::Float3(out)
        }
        // Float1 (1D) × Float2 — treat as [1, N] × [N, M]
        (Value::Float1(a), Value::Float2(b)) => {
            let n = a.dims()[0];
            let a_2d: Tensor<Backend, 2> = a.reshape([1, n]);
            let b = if trans_b { b.transpose() } else { b };
            let mut out = a_2d.matmul(b);
            if node.input.len() > 2 && !node.input[2].is_empty() {
                if let Some(Value::Float1(bias)) = values.get(&node.input[2]) {
                    let bn = bias.dims()[0];
                    out = out + bias.clone().reshape([1, bn]);
                }
            }
            Value::Float2(out)
        }
        // Float4 (4D from wrong reshape) — flatten to 2D first
        (Value::Float4(a), Value::Float2(b)) => {
            let dims = a.dims();
            let last = dims[3];
            let rest: usize = dims[..3].iter().product();
            let a_2d: Tensor<Backend, 2> = a.reshape([rest, last]);
            let b = if trans_b { b.transpose() } else { b };
            let mut out = a_2d.matmul(b);
            if node.input.len() > 2 && !node.input[2].is_empty() {
                if let Some(Value::Float1(bias)) = values.get(&node.input[2]) {
                    let bn = bias.dims()[0];
                    out = out + bias.clone().reshape([1, bn]);
                }
            }
            Value::Float2(out)
        }
        _ => return Err("matmul: unsupported tensor dimension combination".into()),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}
