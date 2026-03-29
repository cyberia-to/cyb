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

    let trans_b = node.attribute.iter()
        .find(|a| a.name == "transB")
        .map(|a| a.i != 0)
        .unwrap_or(false);

    let result = match (a, b) {
        (Value::Float2(a), Value::Float2(b)) => {
            let b = if trans_b { b.transpose() } else { b };
            let mut out = a.matmul(b);
            if node.input.len() > 2 && !node.input[2].is_empty() {
                if let Some(Value::Float1(bias)) = values.get(&node.input[2]) {
                    out = out + bias.clone().reshape([1, bias.dims()[0]]);
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
                    out = out + bias.clone().reshape([1, 1, bias.dims()[0]]);
                }
            }
            Value::Float3(out)
        }
        (Value::Float1(a), Value::Float2(b)) => {
            let n = a.dims()[0];
            let a_2d: Tensor<Backend, 2> = a.reshape([1, n]);
            let b = if trans_b { b.transpose() } else { b };
            let mut out = a_2d.matmul(b);
            if node.input.len() > 2 && !node.input[2].is_empty() {
                if let Some(Value::Float1(bias)) = values.get(&node.input[2]) {
                    out = out + bias.clone().reshape([1, bias.dims()[0]]);
                }
            }
            Value::Float2(out)
        }
        (Value::Float4(a), Value::Float2(b)) => {
            let dims = a.dims();
            let last = dims[3];
            let rest: usize = dims[..3].iter().product();
            let a_2d: Tensor<Backend, 2> = a.reshape([rest, last]);
            let b = if trans_b { b.transpose() } else { b };
            let mut out = a_2d.matmul(b);
            if node.input.len() > 2 && !node.input[2].is_empty() {
                if let Some(Value::Float1(bias)) = values.get(&node.input[2]) {
                    out = out + bias.clone().reshape([1, bias.dims()[0]]);
                }
            }
            Value::Float2(out)
        }
        _ => return Err("matmul: unsupported tensor dimension combination".into()),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

/// Dequantize 4-bit packed weights to f32 tensor [N, K]
/// Called once at model load time, result is cached as a normal initializer
pub fn dequantize_nbits_weights(
    b_data: &[f32],  // packed uint8 stored as f32
    scale_data: &[f32],
    k: usize,
    n: usize,
    block_size: usize,
    device: &Device,
) -> Tensor<Backend, 2> {
    let num_blocks = k / block_size;
    let mut dequantized = vec![0.0f32; n * k];

    for row in 0..n {
        for block in 0..num_blocks {
            let scale = if row * num_blocks + block < scale_data.len() {
                scale_data[row * num_blocks + block]
            } else {
                1.0
            };

            let zp = 8.0f32; // default zero point for 4-bit unsigned symmetric
            let base = row * num_blocks * (block_size / 2) + block * (block_size / 2);

            for j in 0..(block_size / 2) {
                let idx = base + j;
                if idx >= b_data.len() { break; }

                let packed = b_data[idx] as u8;
                let lo = (packed & 0x0F) as f32 - zp;
                let hi = ((packed >> 4) & 0x0F) as f32 - zp;

                let col = block * block_size + j * 2;
                if col < k { dequantized[row * k + col] = lo * scale; }
                if col + 1 < k { dequantized[row * k + col + 1] = hi * scale; }
            }
        }
    }

    Tensor::from_data(
        burn::tensor::TensorData::new(dequantized, vec![n, k]),
        device,
    )
}

/// MatMulNBits — uses pre-cached dequantized weight (key: "{output_name}__dequant")
/// Falls back to runtime dequantization if cache miss
pub fn matmul_nbits_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    let a = values.get(&node.input[0])
        .ok_or("matmul_nbits: input A not found")?.clone();

    let k = node.attribute.iter().find(|a| a.name == "K").map(|a| a.i as usize).unwrap_or(0);
    let n = node.attribute.iter().find(|a| a.name == "N").map(|a| a.i as usize).unwrap_or(0);
    let block_size = node.attribute.iter().find(|a| a.name == "block_size").map(|a| a.i as usize).unwrap_or(32);

    // Check for pre-cached dequantized+transposed weight [K, N]
    let cache_key = format!("{}__dequant_t", node.output[0]);
    let b_t = if let Some(Value::Float2(cached)) = values.get(&cache_key) {
        cached.clone()
    } else {
        // Runtime dequantization (first call) — cache transposed result
        let b_packed = values.get(&node.input[1]).ok_or("matmul_nbits: weights not found")?.clone();
        let scales = values.get(&node.input[2]).ok_or("matmul_nbits: scales not found")?.clone();

        let b_data = extract_f32_data(&b_packed)?;
        let scale_data = extract_f32_data(&scales)?;

        let dequant = dequantize_nbits_weights(&b_data, &scale_data, k, n, block_size, device);
        let transposed = dequant.transpose(); // [N,K] → [K,N] — cache this
        values.insert(cache_key, Value::Float2(transposed.clone()));
        transposed
    };

    let result = match a {
        Value::Float2(a_t) => Value::Float2(a_t.matmul(b_t)),
        Value::Float3(a_t) => {
            let b_3d = b_t.reshape([1, k, n]);
            Value::Float3(a_t.matmul(b_3d))
        }
        _ => return Err("matmul_nbits: unsupported A dimensions".into()),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

fn extract_f32_data(v: &Value) -> Result<Vec<f32>, String> {
    match v {
        Value::Float1(t) => t.to_data().as_slice::<f32>().map(|s| s.to_vec()).map_err(|e| format!("{e:?}")),
        Value::Float2(t) => t.to_data().as_slice::<f32>().map(|s| s.to_vec()).map_err(|e| format!("{e:?}")),
        Value::Float3(t) => t.to_data().as_slice::<f32>().map(|s| s.to_vec()).map_err(|e| format!("{e:?}")),
        _ => Err("extract_f32: unsupported type".into()),
    }
}

/// DequantizeLinear — pass through (data already stored as float)
pub fn dequantize_linear_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0]).ok_or("dequantize: input not found")?.clone();
    values.insert(node.output[0].clone(), input);
    Ok(())
}
