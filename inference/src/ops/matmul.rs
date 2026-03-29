use std::collections::HashMap;
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

    let b = if trans_b { b.transpose() } else { b };
    let mut result = a.matmul(b);

    // Gemm bias (input[2])
    if node.input.len() > 2 && !node.input[2].is_empty() {
        if let Some(bias) = values.get(&node.input[2]) {
            result = result.add(bias.clone());
        }
    }

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
) -> Value {
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

    Value::from_data(dequantized, vec![n, k], device)
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
    let b_t = if let Some(cached) = values.get(&cache_key) {
        cached.clone()
    } else {
        // Runtime dequantization (first call) — cache transposed result
        let b_packed = values.get(&node.input[1]).ok_or("matmul_nbits: weights not found")?.clone();
        let scales = values.get(&node.input[2]).ok_or("matmul_nbits: scales not found")?.clone();

        let b_data = b_packed.to_vec_f32();
        let scale_data = scales.to_vec_f32();

        let dequant = dequantize_nbits_weights(&b_data, &scale_data, k, n, block_size, device);
        let transposed = dequant.transpose(); // [N,K] -> [K,N] — cache this
        values.insert(cache_key, transposed.clone());
        transposed
    };

    let result = a.matmul(b_t);
    values.insert(node.output[0].clone(), result);
    Ok(())
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
