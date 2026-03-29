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
    let input = values.get(&node.input[0]).ok_or("layer_norm: input not found")?.clone();
    let scale = if node.input.len() > 1 && !node.input[1].is_empty() { values.get(&node.input[1]).cloned() } else { None };
    let bias = if node.input.len() > 2 && !node.input[2].is_empty() { values.get(&node.input[2]).cloned() } else { None };
    let eps = node.attribute.iter().find(|a| a.name == "epsilon").map(|a| a.f).unwrap_or(1e-5);

    let result = match input {
        Value::Float3(t) => {
            let mean = t.clone().mean_dim(2);
            let centered = t - mean;
            let var = centered.clone().powf_scalar(2.0).mean_dim(2);
            let mut normed = centered / (var + eps).sqrt();
            if let Some(Value::Float1(s)) = &scale { normed = normed * s.clone().reshape([1, 1, s.dims()[0]]); }
            if let Some(Value::Float1(b)) = &bias { normed = normed + b.clone().reshape([1, 1, b.dims()[0]]); }
            Value::Float3(normed)
        }
        Value::Float2(t) => {
            let mean = t.clone().mean_dim(1);
            let centered = t - mean;
            let var = centered.clone().powf_scalar(2.0).mean_dim(1);
            let mut normed = centered / (var + eps).sqrt();
            if let Some(Value::Float1(s)) = &scale { normed = normed * s.clone().reshape([1, s.dims()[0]]); }
            if let Some(Value::Float1(b)) = &bias { normed = normed + b.clone().reshape([1, b.dims()[0]]); }
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
    let input = values.get(&node.input[0]).ok_or("reduce_mean: input not found")?.clone();
    let axes: Vec<i64> = node.attribute.iter().find(|a| a.name == "axes").map(|a| a.ints.clone()).unwrap_or_default();
    let keepdims = node.attribute.iter().find(|a| a.name == "keepdims").map(|a| a.i != 0).unwrap_or(true);

    let result = match input {
        Value::Float3(t) => {
            let axis = axes.first().map(|&a| if a < 0 { (3 + a) as usize } else { a as usize }).unwrap_or(2);
            let mean = t.mean_dim(axis);
            if keepdims { Value::Float3(mean) }
            else {
                let s = mean.dims();
                let new: Vec<usize> = s.iter().enumerate().filter(|&(i, _)| i != axis).map(|(_, &d)| d).collect();
                match new.len() {
                    1 => Value::Float1(mean.reshape([new[0]])),
                    2 => Value::Float2(mean.reshape([new[0], new[1]])),
                    _ => Value::Float3(mean),
                }
            }
        }
        Value::Float2(t) => {
            let axis = axes.first().map(|&a| if a < 0 { (2 + a) as usize } else { a as usize }).unwrap_or(1);
            let mean = t.mean_dim(axis);
            if keepdims { Value::Float2(mean) } else { let s = mean.dims(); Value::Float1(mean.reshape([s[0].max(s[1])])) }
        }
        _ => return Err("reduce_mean: unsupported dimensions".into()),
    };
    values.insert(node.output[0].clone(), result);
    Ok(())
}

/// RMS Norm (SimplifiedLayerNormalization / SkipSimplifiedLayerNormalization) — Llama
pub fn simplified_layer_norm_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let is_skip = node.op_type == "SkipSimplifiedLayerNormalization";
    let input = values.get(&node.input[0]).ok_or("rms_norm: input not found")?.clone();
    let skip = if is_skip && node.input.len() > 1 && !node.input[1].is_empty() { values.get(&node.input[1]).cloned() } else { None };
    let weight_idx = if is_skip { 2 } else { 1 };
    let weight = if node.input.len() > weight_idx && !node.input[weight_idx].is_empty() { values.get(&node.input[weight_idx]).cloned() } else { None };
    let eps = node.attribute.iter().find(|a| a.name == "epsilon").map(|a| a.f).unwrap_or(1e-5);

    match input {
        Value::Float3(mut t) => {
            if let Some(Value::Float3(s)) = skip { t = t + s; }
            let input_skip = t.clone();
            let rms = t.clone().powf_scalar(2.0).mean_dim(2).sqrt();
            let mut normed = t / (rms + eps);
            if let Some(Value::Float1(w)) = &weight { normed = normed * w.clone().reshape([1, 1, w.dims()[0]]); }
            values.insert(node.output[0].clone(), Value::Float3(normed));
            if is_skip && node.output.len() > 3 && !node.output[3].is_empty() {
                values.insert(node.output[3].clone(), Value::Float3(input_skip));
            }
            Ok(())
        }
        _ => Err("rms_norm: unsupported dimensions".into()),
    }
}

/// GroupQueryAttention — fused multi-head attention with KV groups + RoPE (Llama)
pub fn group_query_attention_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let query = values.get(&node.input[0]).ok_or("gqa: query not found")?.clone();
    let key = values.get(&node.input[1]).ok_or("gqa: key not found")?.clone();
    let value = values.get(&node.input[2]).ok_or("gqa: value not found")?.clone();

    let num_heads = node.attribute.iter().find(|a| a.name == "num_heads").map(|a| a.i as usize).unwrap_or(32);
    let kv_num_heads = node.attribute.iter().find(|a| a.name == "kv_num_heads").map(|a| a.i as usize).unwrap_or(8);
    let do_rotary = node.attribute.iter().find(|a| a.name == "do_rotary").map(|a| a.i != 0).unwrap_or(false);
    let scale = node.attribute.iter().find(|a| a.name == "scale").map(|a| a.f).unwrap_or(0.0);

    // cos_cache, sin_cache — inputs 7, 8
    let cos_cache = if node.input.len() > 7 && !node.input[7].is_empty() { values.get(&node.input[7]).cloned() } else { None };
    let sin_cache = if node.input.len() > 8 && !node.input[8].is_empty() { values.get(&node.input[8]).cloned() } else { None };

    // seqlens_k (input 5) — for total sequence length
    let total_seq_len = if node.input.len() > 6 && !node.input[6].is_empty() {
        if let Some(v) = values.get(&node.input[6]) {
            match v {
                Value::Float1(t) => t.to_data().as_slice::<f32>().ok().map(|s| s[0] as usize),
                _ => None,
            }
        } else { None }
    } else { None };

    match (query, key, value) {
        (Value::Float3(q), Value::Float3(k), Value::Float3(v)) => {
            let [batch, seq_len, q_hidden] = q.dims();
            let head_dim = q_hidden / num_heads;
            let attn_scale = if scale > 0.0 { scale } else { 1.0 / (head_dim as f32).sqrt() };

            // Reshape to [batch, seq, heads, dim] → [batch, heads, seq, dim]
            let mut q = q.reshape([batch, seq_len, num_heads, head_dim]).swap_dims(1, 2);
            let mut k = k.reshape([batch, seq_len, kv_num_heads, head_dim]).swap_dims(1, 2);
            let v = v.reshape([batch, seq_len, kv_num_heads, head_dim]).swap_dims(1, 2);

            // Apply Rotary Position Embeddings
            if do_rotary {
                if let (Some(Value::Float2(cos_c)), Some(Value::Float2(sin_c))) = (&cos_cache, &sin_cache) {
                    // cos_cache, sin_cache: [max_seq, head_dim/2]
                    // We need positions 0..seq_len
                    let half_dim = head_dim / 2;
                    let cos_slice = cos_c.clone().narrow(0, 0, seq_len); // [seq, half_dim]
                    let sin_slice = sin_c.clone().narrow(0, 0, seq_len);

                    // Reshape for broadcasting: [1, 1, seq, half_dim]
                    let cos_4d: Tensor<Backend, 4> = cos_slice.reshape([1, 1, seq_len, half_dim]);
                    let sin_4d: Tensor<Backend, 4> = sin_slice.reshape([1, 1, seq_len, half_dim]);

                    // Apply to Q
                    q = apply_rope(q, &cos_4d, &sin_4d, head_dim);
                    // Apply to K (same for all kv_heads)
                    k = apply_rope(k, &cos_4d, &sin_4d, head_dim);
                }
            }

            // Repeat KV heads to match Q heads
            let repeats = num_heads / kv_num_heads;
            let k = if repeats > 1 {
                let mut e = Vec::new();
                for h in 0..kv_num_heads { let hd = k.clone().narrow(1, h, 1); for _ in 0..repeats { e.push(hd.clone()); } }
                Tensor::cat(e, 1)
            } else { k };
            let v = if repeats > 1 {
                let mut e = Vec::new();
                for h in 0..kv_num_heads { let hd = v.clone().narrow(1, h, 1); for _ in 0..repeats { e.push(hd.clone()); } }
                Tensor::cat(e, 1)
            } else { v };

            // Attention: Q × K^T × scale
            let scores = q.matmul(k.clone().swap_dims(2, 3)) * attn_scale;

            // Causal mask — mask future positions
            let scores = if seq_len > 1 {
                apply_causal_mask(scores, seq_len)
            } else {
                scores
            };

            let attn = burn::tensor::activation::softmax(scores, 3);
            let output = attn.matmul(v.clone()).swap_dims(1, 2).reshape([batch, seq_len, q_hidden]);

            values.insert(node.output[0].clone(), Value::Float3(output));
            if node.output.len() > 1 && !node.output[1].is_empty() { values.insert(node.output[1].clone(), Value::Float4(k)); }
            if node.output.len() > 2 && !node.output[2].is_empty() { values.insert(node.output[2].clone(), Value::Float4(v)); }
            Ok(())
        }
        _ => Err("gqa: unsupported input dimensions".into()),
    }
}

/// Apply Rotary Position Embeddings (RoPE)
/// x: [batch, heads, seq, dim], cos/sin: [1, 1, seq, dim/2]
fn apply_rope(
    x: Tensor<Backend, 4>,
    cos: &Tensor<Backend, 4>,
    sin: &Tensor<Backend, 4>,
    head_dim: usize,
) -> Tensor<Backend, 4> {
    let half = head_dim / 2;
    let [batch, heads, seq, _dim] = x.dims();

    // Split into first half and second half
    let x1 = x.clone().narrow(3, 0, half);
    let x2 = x.clone().narrow(3, half, half);

    // RoPE: [x1*cos - x2*sin, x1*sin + x2*cos]
    let out1 = x1.clone() * cos.clone() - x2.clone() * sin.clone();
    let out2 = x1 * sin.clone() + x2 * cos.clone();

    Tensor::cat(vec![out1, out2], 3)
}

/// Apply causal (triangular) mask to attention scores
fn apply_causal_mask(
    scores: Tensor<Backend, 4>,
    seq_len: usize,
) -> Tensor<Backend, 4> {
    let [batch, heads, _, _] = scores.dims();

    // Create lower triangular mask
    let mut mask_data = vec![0.0f32; seq_len * seq_len];
    for i in 0..seq_len {
        for j in (i + 1)..seq_len {
            mask_data[i * seq_len + j] = f32::NEG_INFINITY;
        }
    }
    let device = scores.device();
    let mask: Tensor<Backend, 4> = Tensor::from_data(
        burn::tensor::TensorData::new(mask_data, vec![1, 1, seq_len, seq_len]),
        &device,
    );

    scores + mask
}

/// ReduceSum
pub fn reduce_sum_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0]).ok_or("reduce_sum: input not found")?.clone();
    let result = match input {
        Value::Float1(t) => Value::Float1(t.sum().reshape([1])),
        Value::Float2(t) => { let s = t.dims(); Value::Float1(t.sum_dim(1).reshape([s[0]])) }
        Value::Float3(t) => { let s = t.dims(); Value::Float2(t.sum_dim(2).reshape([s[0], s[1]])) }
        _ => return Err("reduce_sum: unsupported dimensions".into()),
    };
    values.insert(node.output[0].clone(), result);
    Ok(())
}
