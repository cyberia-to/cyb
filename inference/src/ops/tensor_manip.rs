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

    // Get target shape from second input (always stored as Value now)
    let shape_val = values.get(&node.input[1])
        .ok_or("reshape: shape input not found")?;

    let target_shape: Vec<i64> = shape_val.to_vec_f32().iter().map(|&v| v as i64).collect();

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

    if new_shape.is_empty() {
        new_shape.push(1);
    }

    let result = input.reshape(new_shape);
    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn transpose_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("transpose: input not found")?.clone();

    let ndim = input.ndim();
    let perm: Vec<usize> = node.attribute.iter()
        .find(|a| a.name == "perm")
        .map(|a| a.ints.iter().map(|&v| {
            if v < 0 { (ndim as i64 + v) as usize } else { v as usize }
        }).collect())
        .unwrap_or_default();

    let result = if ndim <= 1 {
        // 0D/1D: no-op
        input
    } else if ndim == 2 {
        // 2D: always swap(0,1)
        input.transpose()
    } else if perm.is_empty() {
        // Default: reverse all dims — for 2D+ just swap last two
        input.transpose()
    } else {
        // Apply permutation via swap_dims
        // We implement common patterns; for arbitrary permutations we chain swaps
        match ndim {
            3 => {
                if perm == [0, 2, 1] {
                    input.swap_dims(1, 2)
                } else if perm == [1, 0, 2] {
                    input.swap_dims(0, 1)
                } else if perm == [2, 0, 1] {
                    input.swap_dims(0, 2).swap_dims(1, 2)
                } else if perm == [1, 2, 0] {
                    input.swap_dims(0, 1).swap_dims(1, 2)
                } else if perm == [2, 1, 0] {
                    input.swap_dims(0, 2)
                } else {
                    input.swap_dims(1, 2) // fallback
                }
            }
            4 => {
                if perm == [0, 2, 1, 3] {
                    input.swap_dims(1, 2)
                } else if perm == [0, 2, 3, 1] {
                    input.swap_dims(1, 2).swap_dims(2, 3)
                } else if perm == [0, 1, 3, 2] {
                    input.swap_dims(2, 3)
                } else if perm == [0, 3, 1, 2] {
                    input.swap_dims(1, 3).swap_dims(2, 3)
                } else {
                    input.swap_dims(1, 2) // common attention pattern fallback
                }
            }
            _ => input.transpose(), // fallback
        }
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn concat_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let ndim_first = node.input.iter()
        .find_map(|name| values.get(name))
        .map(|v| v.ndim())
        .unwrap_or(1);

    let raw_axis = node.attribute.iter()
        .find(|a| a.name == "axis")
        .map(|a| a.i)
        .unwrap_or(0);
    let axis = if raw_axis < 0 { (ndim_first as i64 + raw_axis) as usize } else { raw_axis as usize };

    let tensors: Vec<Value> = node.input.iter()
        .filter_map(|name| values.get(name).cloned())
        .collect();

    if tensors.is_empty() {
        return Err("concat: no tensors found".into());
    }

    let result = Value::cat(tensors, axis);
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

    let new_shape: Vec<usize> = input.shape().iter()
        .filter(|&&d| d != 1)
        .copied()
        .collect();

    let result = if new_shape.is_empty() {
        input.reshape(vec![1])
    } else {
        input.reshape(new_shape)
    };

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
        if let Some(ax) = values.get(&node.input[1]) {
            ax.to_vec_f32().iter().map(|&v| v as i64).collect()
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
    let mut sorted_axes: Vec<usize> = axes.iter()
        .map(|&a| if a < 0 { (shape.len() as i64 + a + 1) as usize } else { a as usize })
        .collect();
    sorted_axes.sort();
    for (offset, &axis) in sorted_axes.iter().enumerate() {
        shape.insert(axis + offset, 1);
    }

    let result = input.reshape(shape);
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
        .map(|a| {
            let v = a.i;
            if v < 0 { (data.ndim() as i64 + v) as usize } else { v as usize }
        })
        .unwrap_or(0);

    // Get indices as i32 vec for GPU tensor
    let idx_vec: Vec<i32> = indices.to_vec_f32().iter().map(|&v| v as i32).collect();
    let indices_shape = indices.shape();
    let num_indices = idx_vec.len();

    // Create indices tensor on GPU
    let idx_tensor: Tensor<Backend, 1, burn::tensor::Int> = Tensor::from_data(
        burn::tensor::TensorData::new(idx_vec, vec![num_indices]),
        device,
    );

    // Use Value::select for the gather
    let selected = data.clone().select(axis, idx_tensor);

    // For axis==0 on 2D+ data, the output shape is indices_shape + trailing dims
    // The select call already handles the shape correctly for the gathered dimension
    // But we may need to reshape if indices had a specific shape
    let result = if indices_shape.len() > 1 && axis == 0 {
        let data_shape = data.shape();
        let mut out_shape = indices_shape;
        out_shape.extend_from_slice(&data_shape[1..]);
        selected.reshape(out_shape)
    } else {
        selected
    };

    values.insert(node.output[0].clone(), result);
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
    let result = Value::from_data(shape, vec![n], device);
    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn slice_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let data = values.get(&node.input[0])
        .ok_or("slice: data not found")?.clone();

    // ONNX Slice: inputs are data, starts, ends, [axes], [steps]
    let get_i64_vec = |idx: usize| -> Vec<i64> {
        if idx >= node.input.len() || node.input[idx].is_empty() { return vec![]; }
        if let Some(v) = values.get(&node.input[idx]) {
            v.to_vec_f32().iter().map(|&f| {
                if f > 1e15 { i64::MAX } else if f < -1e15 { i64::MIN } else { f as i64 }
            }).collect()
        } else { vec![] }
    };

    let starts = get_i64_vec(1);
    let ends = get_i64_vec(2);
    let axes_input = get_i64_vec(3);

    if starts.is_empty() || ends.is_empty() {
        values.insert(node.output[0].clone(), data);
        return Ok(());
    }

    let ndim = data.ndim();
    let shape = data.shape();

    // Apply slicing for each axis
    let mut result = data;
    for i in 0..starts.len() {
        let axis = if i < axes_input.len() {
            let a = axes_input[i];
            if a < 0 { (ndim as i64 + a) as usize } else { a as usize }
        } else {
            i
        };

        let dim_len = shape[axis] as i64;
        let s = if starts[i] < 0 { (dim_len + starts[i]).max(0) } else { starts[i].min(dim_len) } as usize;
        let e = if ends[i] < 0 { (dim_len + ends[i]).max(0) } else { ends[i].min(dim_len) } as usize;

        if e > s {
            result = result.narrow(axis, s, e - s);
        }
    }

    values.insert(node.output[0].clone(), result);
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
            let result = Value::from_data(vec![attr.f], vec![1], device);
            values.insert(node.output[0].clone(), result);
            return Ok(());
        }
        // Scalar int
        if attr.name == "value_int" {
            let result = Value::from_data(vec![attr.i as f32], vec![1], device);
            values.insert(node.output[0].clone(), result);
            return Ok(());
        }
        // Float list
        if attr.name == "value_floats" && !attr.floats.is_empty() {
            let n = attr.floats.len();
            let result = Value::from_data(attr.floats.clone(), vec![n], device);
            values.insert(node.output[0].clone(), result);
            return Ok(());
        }
        // Int list
        if attr.name == "value_ints" && !attr.ints.is_empty() {
            let vals: Vec<f32> = attr.ints.iter().map(|&v| v as f32).collect();
            let n = vals.len();
            let result = Value::from_data(vals, vec![n], device);
            values.insert(node.output[0].clone(), result);
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
    let shape_val = values.get(&node.input[0])
        .ok_or("constant_of_shape: shape not found")?;
    let shape: Vec<usize> = shape_val.to_vec_f32().iter().map(|&v| v as usize).collect();

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

    let total: usize = shape.iter().product::<usize>().max(1);
    let vals = vec![fill_val; total];
    let shape = if shape.is_empty() { vec![1] } else { shape };
    let result = Value::from_data(vals, shape, device);

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

    let result = input.flatten();
    values.insert(node.output[0].clone(), result);
    Ok(())
}
