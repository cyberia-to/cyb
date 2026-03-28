use std::collections::HashMap;
use burn::prelude::*;
use crate::onnx_proto::onnx::NodeProto;

use crate::Backend;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

/// Pass input[0] directly to output[0]
pub fn pass_through(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
) -> Result<(), String> {
    if node.input.is_empty() || node.output.is_empty() {
        return Ok(());
    }
    if let Some(val) = values.get(&node.input[0]).cloned() {
        values.insert(node.output[0].clone(), val);
    }
    Ok(())
}

pub fn reshape_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("reshape: input not found")?.clone();

    // Get target shape from second input
    let target_shape: Vec<i64> = if let Some(Value::Int1(t)) = values.get(&node.input[1]) {
        let data = t.to_data();
        data.as_slice::<i64>().map_err(|e| format!("reshape: {e:?}"))?.to_vec()
    } else if let Some(Value::Float1(t)) = values.get(&node.input[1]) {
        let data = t.to_data();
        data.as_slice::<f32>().map_err(|e| format!("reshape: {e:?}"))?.iter().map(|&v| v as i64).collect()
    } else {
        return Err("reshape: shape input not found".into());
    };

    let input_size: usize = input.shape().iter().product();
    let mut new_shape: Vec<usize> = Vec::new();
    let mut neg_idx = None;
    let mut known_product: usize = 1;

    for (i, &dim) in target_shape.iter().enumerate() {
        if dim == -1 {
            neg_idx = Some(i);
            new_shape.push(0);
        } else if dim == 0 {
            let orig = input.shape().get(i).copied().unwrap_or(1);
            new_shape.push(orig);
            known_product *= orig;
        } else {
            new_shape.push(dim as usize);
            known_product *= dim as usize;
        }
    }

    if let Some(idx) = neg_idx {
        new_shape[idx] = input_size / known_product;
    }

    let result = reshape_value(input, &new_shape)?;
    values.insert(node.output[0].clone(), result);
    Ok(())
}

fn reshape_value(input: Value, shape: &[usize]) -> Result<Value, String> {
    // Flatten input to 1D first, then reshape to target
    let flat: Vec<f32> = match &input {
        Value::Float1(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        Value::Float2(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        Value::Float3(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        Value::Float4(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        _ => return Err("reshape: unsupported input type".into()),
    };

    let data = burn::tensor::TensorData::new(flat, shape.to_vec());
    let device = burn::backend::wgpu::WgpuDevice::default();

    match shape.len() {
        1 => Ok(Value::Float1(Tensor::from_data(data, &device))),
        2 => Ok(Value::Float2(Tensor::from_data(data, &device))),
        3 => Ok(Value::Float3(Tensor::from_data(data, &device))),
        4 => Ok(Value::Float4(Tensor::from_data(data, &device))),
        _ => Err(format!("reshape: unsupported target rank {}", shape.len())),
    }
}

pub fn transpose_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("transpose: input not found")?.clone();

    let result = match input {
        Value::Float2(t) => Value::Float2(t.transpose()),
        Value::Float3(t) => {
            // Check perm attribute
            Value::Float3(t.swap_dims(1, 2))
        }
        _ => return Err("transpose: unsupported dimensions".into()),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn concat_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    // Determine axis from attributes (default 0)
    let axis = node.attribute.iter()
        .find(|a| a.name == "axis")
        .and_then(|a| Some(a.i as usize))
        .unwrap_or(0);

    let mut float1s: Vec<Tensor<Backend, 1>> = Vec::new();
    let mut float2s: Vec<Tensor<Backend, 2>> = Vec::new();
    let mut float3s: Vec<Tensor<Backend, 3>> = Vec::new();
    let mut float4s: Vec<Tensor<Backend, 4>> = Vec::new();

    for name in &node.input {
        if let Some(val) = values.get(name) {
            match val {
                Value::Float1(t) => float1s.push(t.clone()),
                Value::Float2(t) => float2s.push(t.clone()),
                Value::Float3(t) => float3s.push(t.clone()),
                Value::Float4(t) => float4s.push(t.clone()),
                _ => {}
            }
        }
    }

    let result = if !float1s.is_empty() {
        Value::Float1(Tensor::cat(float1s, 0))
    } else if !float2s.is_empty() {
        Value::Float2(Tensor::cat(float2s, axis))
    } else if !float3s.is_empty() {
        Value::Float3(Tensor::cat(float3s, axis))
    } else if !float4s.is_empty() {
        Value::Float4(Tensor::cat(float4s, axis))
    } else {
        return Err("concat: no compatible tensors found".into());
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn squeeze_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("squeeze: input not found")?.clone();

    let shape = input.shape();
    let result = reshape_value(input, &shape.iter().filter(|&&d| d != 1).copied().collect::<Vec<_>>())?;
    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn unsqueeze_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("unsqueeze: input not found")?.clone();

    // Get axes from second input or attribute
    let axes: Vec<i64> = if node.input.len() > 1 && !node.input[1].is_empty() {
        if let Some(Value::Float1(t)) = values.get(&node.input[1]) {
            let d = t.to_data();
            d.as_slice::<f32>().map(|s| s.iter().map(|&v| v as i64).collect()).unwrap_or(vec![0])
        } else {
            vec![0]
        }
    } else {
        node.attribute.iter()
            .find(|a| a.name == "axes")
            .map(|a| a.ints.clone())
            .unwrap_or(vec![0])
    };

    let mut shape = input.shape();
    // Insert dims at specified axes (handle negative axes)
    let mut sorted_axes: Vec<usize> = axes.iter()
        .map(|&a| if a < 0 { (shape.len() as i64 + a + 1) as usize } else { a as usize })
        .collect();
    sorted_axes.sort();
    for (offset, &axis) in sorted_axes.iter().enumerate() {
        shape.insert(axis + offset, 1);
    }

    let result = reshape_value(input, &shape)?;
    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn gather_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    let data = values.get(&node.input[0])
        .ok_or("gather: data not found")?.clone();
    let indices = values.get(&node.input[1])
        .ok_or("gather: indices not found")?.clone();

    let axis = node.attribute.iter()
        .find(|a| a.name == "axis")
        .map(|a| a.i as usize)
        .unwrap_or(0);

    // Get indices as usize vec
    let idx_vec: Vec<usize> = match &indices {
        Value::Float1(t) => {
            let d = t.to_data();
            d.as_slice::<f32>().map_err(|e| format!("{e:?}"))?
                .iter().map(|&v| v as usize).collect()
        }
        Value::Float2(t) => {
            let d = t.to_data();
            d.as_slice::<f32>().map_err(|e| format!("{e:?}"))?
                .iter().map(|&v| v as usize).collect()
        }
        Value::Int1(t) => {
            let d = t.to_data();
            d.as_slice::<i64>().map_err(|e| format!("{e:?}"))?
                .iter().map(|&v| v as usize).collect()
        }
        _ => return Err("gather: unsupported indices type".into()),
    };

    let indices_shape = indices.shape();

    match data {
        Value::Float2(t) if axis == 0 => {
            // Gather rows from [vocab, hidden] using indices → [indices_shape..., hidden]
            let hidden = t.dims()[1];
            let mut rows: Vec<f32> = Vec::with_capacity(idx_vec.len() * hidden);
            let t_data = t.to_data();
            let t_slice = t_data.as_slice::<f32>().map_err(|e| format!("{e:?}"))?;

            for &idx in &idx_vec {
                let start = idx * hidden;
                let end = start + hidden;
                if end <= t_slice.len() {
                    rows.extend_from_slice(&t_slice[start..end]);
                } else {
                    rows.extend(vec![0.0f32; hidden]);
                }
            }

            // Output shape: indices_shape + [hidden]
            let mut out_shape = indices_shape;
            out_shape.push(hidden);

            let data = burn::tensor::TensorData::new(rows, out_shape.clone());
            let result = match out_shape.len() {
                1 => Value::Float1(Tensor::from_data(data, device)),
                2 => Value::Float2(Tensor::from_data(data, device)),
                3 => Value::Float3(Tensor::from_data(data, device)),
                4 => Value::Float4(Tensor::from_data(data, device)),
                _ => return Err("gather: unsupported output rank".into()),
            };
            values.insert(node.output[0].clone(), result);
        }
        Value::Float1(t) if axis == 0 => {
            // Gather elements from 1D tensor
            let t_data = t.to_data();
            let t_slice = t_data.as_slice::<f32>().map_err(|e| format!("{e:?}"))?;

            let gathered: Vec<f32> = idx_vec.iter()
                .map(|&i| if i < t_slice.len() { t_slice[i] } else { 0.0 })
                .collect();

            if gathered.len() == 1 {
                // Scalar output → 1D tensor with 1 element
                let data = burn::tensor::TensorData::new(gathered, vec![1]);
                values.insert(node.output[0].clone(), Value::Float1(Tensor::from_data(data, device)));
            } else {
                let n = gathered.len();
                let data = burn::tensor::TensorData::new(gathered, vec![n]);
                values.insert(node.output[0].clone(), Value::Float1(Tensor::from_data(data, device)));
            }
        }
        _ => {
            log::warn!("gather: unsupported data rank={} axis={}, passing through", data.ndim(), axis);
            values.insert(node.output[0].clone(), data);
        }
    }

    Ok(())
}

pub fn shape_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("shape: input not found")?;

    let shape: Vec<f32> = input.shape().iter().map(|&d| d as f32).collect();
    let n = shape.len();
    let data = burn::tensor::TensorData::new(shape, vec![n]);
    values.insert(node.output[0].clone(), Value::Float1(Tensor::from_data(data, device)));
    Ok(())
}

pub fn slice_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    // Stub: pass data through
    if let Some(val) = values.get(&node.input[0]).cloned() {
        values.insert(node.output[0].clone(), val);
    }
    Ok(())
}

pub fn where_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    // Stub: return X (input[1])
    if node.input.len() >= 2 {
        if let Some(val) = values.get(&node.input[1]).cloned() {
            values.insert(node.output[0].clone(), val);
            return Ok(());
        }
    }
    Err("where: inputs not found".into())
}

pub fn constant_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    if node.output.is_empty() { return Ok(()); }

    // Check for tensor value in attributes
    for attr in &node.attribute {
        if attr.name == "value" {
            if let Some(ref t) = attr.t {
                if let Some(val) = crate::graph::tensor_proto_to_value(t, device) {
                    log::trace!("Constant {} = shape {:?}", node.output[0], val.shape());
                    values.insert(node.output[0].clone(), val);
                    return Ok(());
                } else {
                    log::warn!("Constant {}: tensor_proto_to_value returned None (dtype={} dims={:?} raw={}b float_data={} int64={})",
                        node.output[0], t.data_type, t.dims, t.raw_data.len(), t.float_data.len(), t.int64_data.len());
                }
            }
        }
        // Scalar float
        if attr.name == "value_float" {
            let data = burn::tensor::TensorData::new(vec![attr.f], vec![1]);
            values.insert(node.output[0].clone(), Value::Float1(Tensor::from_data(data, device)));
            return Ok(());
        }
        // Scalar int
        if attr.name == "value_int" {
            let data = burn::tensor::TensorData::new(vec![attr.i as f32], vec![1]);
            values.insert(node.output[0].clone(), Value::Float1(Tensor::from_data(data, device)));
            return Ok(());
        }
        // Float list
        if attr.name == "value_floats" && !attr.floats.is_empty() {
            let n = attr.floats.len();
            let data = burn::tensor::TensorData::new(attr.floats.clone(), vec![n]);
            values.insert(node.output[0].clone(), Value::Float1(Tensor::from_data(data, device)));
            return Ok(());
        }
        // Int list
        if attr.name == "value_ints" && !attr.ints.is_empty() {
            let vals: Vec<f32> = attr.ints.iter().map(|&v| v as f32).collect();
            let n = vals.len();
            let data = burn::tensor::TensorData::new(vals, vec![n]);
            values.insert(node.output[0].clone(), Value::Float1(Tensor::from_data(data, device)));
            return Ok(());
        }
    }

    log::debug!("constant: no value found for {}", node.output[0]);
    Ok(())
}

pub fn constant_of_shape_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    // Get shape from input
    let shape: Vec<usize> = if let Some(Value::Float1(t)) = values.get(&node.input[0]) {
        let d = t.to_data();
        d.as_slice::<f32>().map_err(|e| format!("{e:?}"))?.iter().map(|&v| v as usize).collect()
    } else {
        return Err("constant_of_shape: shape not found".into());
    };

    // Get fill value from attribute (default 0.0)
    let fill_val = node.attribute.iter()
        .find(|a| a.name == "value")
        .and_then(|a| a.t.as_ref())
        .and_then(|t| {
            if !t.float_data.is_empty() { Some(t.float_data[0]) }
            else if !t.raw_data.is_empty() && t.raw_data.len() >= 4 {
                Some(f32::from_le_bytes([t.raw_data[0], t.raw_data[1], t.raw_data[2], t.raw_data[3]]))
            } else { None }
        })
        .unwrap_or(0.0);

    let total: usize = shape.iter().product();
    let vals = vec![fill_val; total.max(1)];
    let data = burn::tensor::TensorData::new(vals, shape.clone());

    let result = match shape.len() {
        0 | 1 => Value::Float1(Tensor::from_data(data, device)),
        2 => Value::Float2(Tensor::from_data(data, device)),
        3 => Value::Float3(Tensor::from_data(data, device)),
        4 => Value::Float4(Tensor::from_data(data, device)),
        _ => return Err(format!("constant_of_shape: unsupported rank {}", shape.len())),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn flatten_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("flatten: input not found")?.clone();

    let total: usize = input.shape().iter().product();
    let result = reshape_value(input, &[total])?;
    values.insert(node.output[0].clone(), result);
    Ok(())
}
