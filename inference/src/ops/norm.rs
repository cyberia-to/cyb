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

/// GroupQueryAttention — fused multi-head attention with KV groups + RoPE + KV-cache (Llama)
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
    let scale_attr = node.attribute.iter().find(|a| a.name == "scale").map(|a| a.f).unwrap_or(0.0);

    // past_key, past_value — inputs 3, 4 (from KV-cache)
    let past_key = if node.input.len() > 3 && !node.input[3].is_empty() {
        values.get(&node.input[3]).cloned()
    } else { None };
    let past_value = if node.input.len() > 4 && !node.input[4].is_empty() {
        values.get(&node.input[4]).cloned()
    } else { None };

    // cos_cache, sin_cache — inputs 7, 8
    let cos_cache = if node.input.len() > 7 && !node.input[7].is_empty() { values.get(&node.input[7]).cloned() } else { None };
    let sin_cache = if node.input.len() > 8 && !node.input[8].is_empty() { values.get(&node.input[8]).cloned() } else { None };

    match (query, key, value) {
        (Value::Float3(q), Value::Float3(k), Value::Float3(v)) => {
            let [batch, seq_len, q_hidden] = q.dims();
            let head_dim = q_hidden / num_heads;
            let attn_scale = if scale_attr > 0.0 { scale_attr } else { 1.0 / (head_dim as f32).sqrt() };

            // Determine past sequence length for RoPE position offset
            let past_seq_len = match &past_key {
                Some(Value::Float4(pk)) => pk.dims()[2],
                _ => 0,
            };

            // Reshape: [batch, seq, heads, dim] → [batch, heads, seq, dim]
            let mut q = q.reshape([batch, seq_len, num_heads, head_dim]).swap_dims(1, 2);
            let mut k_new = k.reshape([batch, seq_len, kv_num_heads, head_dim]).swap_dims(1, 2);
            let v_new = v.reshape([batch, seq_len, kv_num_heads, head_dim]).swap_dims(1, 2);

            // Apply RoPE with position offset (past_seq_len)
            if do_rotary {
                if let (Some(Value::Float2(cos_c)), Some(Value::Float2(sin_c))) = (&cos_cache, &sin_cache) {
                    let half_dim = head_dim / 2;
                    // Positions: past_seq_len .. past_seq_len + seq_len
                    let cos_slice = cos_c.clone().narrow(0, past_seq_len, seq_len);
                    let sin_slice = sin_c.clone().narrow(0, past_seq_len, seq_len);
                    let cos_4d: Tensor<Backend, 4> = cos_slice.reshape([1, 1, seq_len, half_dim]);
                    let sin_4d: Tensor<Backend, 4> = sin_slice.reshape([1, 1, seq_len, half_dim]);

                    q = apply_rope(q, &cos_4d, &sin_4d, head_dim);
                    k_new = apply_rope(k_new, &cos_4d, &sin_4d, head_dim);
                }
            }

            // Concatenate with past KV cache
            let k_full = if let Some(Value::Float4(past_k)) = &past_key {
                if past_k.dims()[2] > 0 {
                    Tensor::cat(vec![past_k.clone(), k_new], 2) // concat on seq dim
                } else { k_new }
            } else { k_new };

            let v_full = if let Some(Value::Float4(past_v)) = &past_value {
                if past_v.dims()[2] > 0 {
                    Tensor::cat(vec![past_v.clone(), v_new], 2)
                } else { v_new }
            } else { v_new };

            let total_seq = k_full.dims()[2]; // past_seq + new_seq

            // Repeat KV heads to match Q heads
            let repeats = num_heads / kv_num_heads;
            let k_expanded = if repeats > 1 {
                let mut e = Vec::new();
                for h in 0..kv_num_heads { let hd = k_full.clone().narrow(1, h, 1); for _ in 0..repeats { e.push(hd.clone()); } }
                Tensor::cat(e, 1)
            } else { k_full.clone() };
            let v_expanded = if repeats > 1 {
                let mut e = Vec::new();
                for h in 0..kv_num_heads { let hd = v_full.clone().narrow(1, h, 1); for _ in 0..repeats { e.push(hd.clone()); } }
                Tensor::cat(e, 1)
            } else { v_full.clone() };

            // Attention: Q[batch, heads, new_seq, dim] × K^T[batch, heads, dim, total_seq]
            let scores = q.matmul(k_expanded.swap_dims(2, 3)) * attn_scale;

            // Causal mask: [new_seq, total_seq] — each new position can attend to past + itself
            let scores = if seq_len > 1 || total_seq > 1 {
                apply_causal_mask_kv(scores, seq_len, total_seq)
            } else {
                scores
            };

            let attn = burn::tensor::activation::softmax(scores, 3);
            let output = attn.matmul(v_expanded).swap_dims(1, 2).reshape([batch, seq_len, q_hidden]);

            values.insert(node.output[0].clone(), Value::Float3(output));
            // present_key, present_value = full KV (for next step's past_kv)
            if node.output.len() > 1 && !node.output[1].is_empty() {
                values.insert(node.output[1].clone(), Value::Float4(k_full));
            }
            if node.output.len() > 2 && !node.output[2].is_empty() {
                values.insert(node.output[2].clone(), Value::Float4(v_full));
            }
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

/// Apply causal mask for KV-cache: [new_seq, total_seq]
/// Each new position i can attend to positions 0..past_seq+i+1
fn apply_causal_mask_kv(
    scores: Tensor<Backend, 4>,
    new_seq: usize,
    total_seq: usize,
) -> Tensor<Backend, 4> {
    let past_seq = total_seq - new_seq;
    let mut mask_data = vec![0.0f32; new_seq * total_seq];
    for i in 0..new_seq {
        // Position i in new sequence = past_seq + i in total
        // Can attend to 0..past_seq+i+1, mask past_seq+i+1..total_seq
        for j in (past_seq + i + 1)..total_seq {
            mask_data[i * total_seq + j] = f32::NEG_INFINITY;
        }
    }
    let device = scores.device();
    let mask: Tensor<Backend, 4> = Tensor::from_data(
        burn::tensor::TensorData::new(mask_data, vec![1, 1, new_seq, total_seq]),
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
