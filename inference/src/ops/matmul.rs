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

    if node.input.len() > 2 && !node.input[2].is_empty() {
        if let Some(bias) = values.get(&node.input[2]) {
            result = result.add(bias.clone());
        }
    }

    values.insert(node.output[0].clone(), result);
    Ok(())
}

/// MatMulNBits — Q4 quantized matmul
/// For decode (M<=4): custom WGSL Q4 vecmat shader (single kernel, no dequant)
/// For prefill (M>4): GPU matmul with dequantized f32 weights
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

    let a_shape = a.shape();
    let m: usize = a_shape[..a_shape.len()-1].iter().product();

    // Try cubecl Q4 kernel for decode step (M <= 4)
    if m <= 4 {
        let result = crate::quant::q4_launch::q4_vecmat_cubecl(
            &node.output[0],
            &a,
            values.get(&node.input[1]).ok_or("matmul_nbits: weights not found")?,
            values.get(&node.input[2]).ok_or("matmul_nbits: scales not found")?,
            n, k, block_size,
            device,
        );
        values.insert(node.output[0].clone(), result);
        return Ok(());
    }

    // GPU matmul with dequantized weights for prefill (M > 4)
    let cache_key = format!("{}__dequant_t", node.output[0]);
    let b_t = if let Some(cached) = values.get(&cache_key) {
        cached.clone()
    } else {
        let b_packed = values.get(&node.input[1]).ok_or("matmul_nbits: weights not found")?.clone();
        let scales = values.get(&node.input[2]).ok_or("matmul_nbits: scales not found")?.clone();

        let b_data = b_packed.to_vec_f32();
        let scale_data = scales.to_vec_f32();

        let dequant = dequantize_nbits_weights(&b_data, &scale_data, k, n, block_size, device);
        let transposed = dequant.transpose();
        values.insert(cache_key, transposed.clone());
        transposed
    };

    let result = a.matmul(b_t);
    values.insert(node.output[0].clone(), result);
    Ok(())
}

/// Try Q4 vecmat via custom WGSL shader
fn try_q4_vecmat(
    node: &NodeProto,
    a: &Value,
    k: usize,
    n: usize,
    block_size: usize,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Option<Value> {
    use crate::quant;

    let q4 = quant::get_q4_compute();

    // Cache Q4 weight buffers on GPU
    let cache_key = node.output[0].clone();
    let mut weight_cache = quant::Q4_WEIGHTS.lock().ok()?;

    if !weight_cache.contains_key(&cache_key) {
        let b_packed = values.get(&node.input[1])?;
        let scales_val = values.get(&node.input[2])?;

        let b_raw = b_packed.to_vec_f32();
        let scales = scales_val.to_vec_f32();

        let packed_bytes: Vec<u8> = b_raw.iter().map(|&v| v as u8).collect();

        let weight_buf = q4.upload_weights(&packed_bytes, &scales, n, k, block_size);
        weight_cache.insert(cache_key.clone(), weight_buf);
    }

    let weight = weight_cache.get(&cache_key)?;

    // Get activation vector (GPU → CPU for custom shader input)
    let a_data = a.to_vec_f32();
    // Use only last K elements (activation is [1,1,...,K] in 4D)
    let act = if a_data.len() >= k {
        &a_data[a_data.len() - k..]
    } else {
        &a_data
    };

    // Execute custom Q4 vecmat shader
    let output = q4.vecmat(act, weight);

    // Build output shape
    let mut out_shape = a.shape();
    *out_shape.last_mut()? = n;

    Some(Value::from_data(output, out_shape, device))
}

/// Dequantize 4-bit packed weights to f32 (for GPU prefill path)
fn dequantize_nbits_weights(
    b_data: &[f32],
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
            } else { 1.0 };

            let base = row * num_blocks * (block_size / 2) + block * (block_size / 2);

            for j in 0..(block_size / 2) {
                let idx = base + j;
                if idx >= b_data.len() { break; }

                let packed = b_data[idx] as u8;
                let lo = (packed & 0x0F) as f32 - 8.0;
                let hi = (packed >> 4) as f32 - 8.0;

                let col = block * block_size + j * 2;
                if col < k { dequantized[row * k + col] = lo * scale; }
                if col + 1 < k { dequantized[row * k + col + 1] = hi * scale; }
            }
        }
    }

    Value::from_data(dequantized, vec![n, k], device)
}

/// DequantizeLinear — pass through
pub fn dequantize_linear_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0]).ok_or("dequantize: input not found")?.clone();
    values.insert(node.output[0].clone(), input);
    Ok(())
}
