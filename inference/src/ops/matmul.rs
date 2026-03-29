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

/// MatMulNBits — 4-bit quantized matrix multiplication
/// Input A (float) × dequantized B (from 4-bit packed) = output C (float)
/// Attributes: K (rows of B), N (cols of B), bits (4), block_size (32/128)
pub fn matmul_nbits_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    // Input 0: A — the activation tensor (float, [batch, K])
    let a = values.get(&node.input[0])
        .ok_or("matmul_nbits: input A not found")?.clone();

    // Input 1: B — packed 4-bit weights (uint8, [N, K/block_size, block_size/2])
    let b_packed = values.get(&node.input[1])
        .ok_or("matmul_nbits: packed weights B not found")?.clone();

    // Input 2: scales — per-block scales (float16, [N, K/block_size])
    let scales = values.get(&node.input[2])
        .ok_or("matmul_nbits: scales not found")?.clone();

    // Input 3 (optional): zero_points
    let zero_points = if node.input.len() > 3 && !node.input[3].is_empty() {
        values.get(&node.input[3]).cloned()
    } else {
        None
    };

    // Get attributes
    let k = node.attribute.iter().find(|a| a.name == "K").map(|a| a.i as usize).unwrap_or(0);
    let n = node.attribute.iter().find(|a| a.name == "N").map(|a| a.i as usize).unwrap_or(0);
    let bits = node.attribute.iter().find(|a| a.name == "bits").map(|a| a.i as usize).unwrap_or(4);
    let block_size = node.attribute.iter().find(|a| a.name == "block_size").map(|a| a.i as usize).unwrap_or(32);

    if bits != 4 {
        return Err(format!("matmul_nbits: only 4-bit supported, got {bits}"));
    }

    // Dequantize B: unpack 4-bit values and apply scales
    let b_data = match &b_packed {
        Value::Float1(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        Value::Float2(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        Value::Float3(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        _ => return Err("matmul_nbits: unsupported B type".into()),
    };

    let scale_data = match &scales {
        Value::Float1(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        Value::Float2(t) => t.to_data().as_slice::<f32>().map_err(|e| format!("{e:?}"))?.to_vec(),
        _ => return Err("matmul_nbits: unsupported scales type".into()),
    };

    // Dequantize: each byte has 2 x 4-bit values
    let num_blocks = k / block_size;
    let mut dequantized = vec![0.0f32; n * k];

    for row in 0..n {
        for block in 0..num_blocks {
            let scale = if row * num_blocks + block < scale_data.len() {
                scale_data[row * num_blocks + block]
            } else {
                1.0
            };

            let zp = 8.0f32; // default zero point for 4-bit symmetric

            let base_packed_idx = row * num_blocks * (block_size / 2) + block * (block_size / 2);

            for j in 0..(block_size / 2) {
                let packed_idx = base_packed_idx + j;
                if packed_idx >= b_data.len() { break; }

                let packed = b_data[packed_idx] as u8;
                let lo = (packed & 0x0F) as f32 - zp;
                let hi = ((packed >> 4) & 0x0F) as f32 - zp;

                let col = block * block_size + j * 2;
                if col < k {
                    dequantized[row * k + col] = lo * scale;
                }
                if col + 1 < k {
                    dequantized[row * k + col + 1] = hi * scale;
                }
            }
        }
    }

    // Create dequantized weight tensor [N, K]
    let b_tensor: Tensor<Backend, 2> = Tensor::from_data(
        burn::tensor::TensorData::new(dequantized, vec![n, k]),
        device,
    );

    // MatMul: A × B^T
    let result = match a {
        Value::Float2(a_t) => {
            // [batch, K] × [K, N] (transpose B)
            Value::Float2(a_t.matmul(b_tensor.transpose()))
        }
        Value::Float3(a_t) => {
            let [b, s, _k] = a_t.dims();
            let b_3d = b_tensor.transpose().reshape([1, k, n]);
            Value::Float3(a_t.matmul(b_3d))
        }
        _ => return Err("matmul_nbits: unsupported A dimensions".into()),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

/// DequantizeLinear — convert quantized tensor to float
pub fn dequantize_linear_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or("dequantize: input not found")?.clone();

    // For now, just pass through (data already stored as float)
    values.insert(node.output[0].clone(), input);
    Ok(())
}
