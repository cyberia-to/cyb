use std::collections::HashMap;
use burn::prelude::*;
use crate::onnx_proto::onnx::NodeProto;

use crate::Backend;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

pub fn layer_norm_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("layer_norm: input not found")?.clone();

    let scale = if node.input.len() > 1 && !node.input[1].is_empty() {
        values.get(&node.input[1]).cloned()
    } else {
        None
    };

    let bias = if node.input.len() > 2 && !node.input[2].is_empty() {
        values.get(&node.input[2]).cloned()
    } else {
        None
    };

    let eps = node.attribute.iter()
        .find(|a| a.name == "epsilon")
        .map(|a| a.f)
        .unwrap_or(1e-5);

    let result = match input {
        Value::Float3(t) => {
            let mean = t.clone().mean_dim(2);
            let centered = t.clone() - mean;
            let var = centered.clone().powf_scalar(2.0).mean_dim(2);
            let normed = centered / (var + eps).sqrt();

            let normed = if let Some(Value::Float1(s)) = &scale {
                let s = s.clone().reshape([1, 1, s.dims()[0]]);
                normed * s
            } else {
                normed
            };

            let normed = if let Some(Value::Float1(b)) = &bias {
                let b = b.clone().reshape([1, 1, b.dims()[0]]);
                normed + b
            } else {
                normed
            };

            Value::Float3(normed)
        }
        Value::Float2(t) => {
            let mean = t.clone().mean_dim(1);
            let centered = t.clone() - mean;
            let var = centered.clone().powf_scalar(2.0).mean_dim(1);
            let normed = centered / (var + eps).sqrt();

            let normed = if let Some(Value::Float1(s)) = &scale {
                let s = s.clone().reshape([1, s.dims()[0]]);
                normed * s
            } else {
                normed
            };

            let normed = if let Some(Value::Float1(b)) = &bias {
                let b = b.clone().reshape([1, b.dims()[0]]);
                normed + b
            } else {
                normed
            };

            Value::Float2(normed)
        }
        _ => return Err("layer_norm: unsupported dimensions".into()),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn reduce_mean_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("reduce_mean: input not found")?.clone();

    // Get axes from attribute
    let axes: Vec<i64> = node.attribute.iter()
        .find(|a| a.name == "axes")
        .map(|a| a.ints.clone())
        .unwrap_or_default();

    let keepdims = node.attribute.iter()
        .find(|a| a.name == "keepdims")
        .map(|a| a.i != 0)
        .unwrap_or(true);

    let result = match input {
        Value::Float3(t) => {
            let axis = axes.first().map(|&a| if a < 0 { (3 + a) as usize } else { a as usize }).unwrap_or(2);
            let mean = t.mean_dim(axis);
            if keepdims {
                Value::Float3(mean)
            } else {
                let s = mean.dims();
                let new_shape: Vec<usize> = s.iter().enumerate()
                    .filter(|&(i, _)| i != axis)
                    .map(|(_, &d)| d)
                    .collect();
                match new_shape.len() {
                    1 => Value::Float1(mean.reshape([new_shape[0]])),
                    2 => Value::Float2(mean.reshape([new_shape[0], new_shape[1]])),
                    _ => Value::Float3(mean),
                }
            }
        }
        Value::Float2(t) => {
            let axis = axes.first().map(|&a| if a < 0 { (2 + a) as usize } else { a as usize }).unwrap_or(1);
            let mean = t.mean_dim(axis);
            if keepdims {
                Value::Float2(mean)
            } else {
                let s = mean.dims();
                Value::Float1(mean.reshape([s[0].max(s[1])]))
            }
        }
        _ => return Err("reduce_mean: unsupported dimensions".into()),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}
