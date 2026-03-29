use std::collections::HashMap;
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

    // LayerNorm on last dim
    let last_dim = input.ndim() - 1;
    let mean = input.clone().mean_dim(last_dim);
    let centered = input.sub(mean);
    let var = centered.clone().powf_scalar(2.0).mean_dim(last_dim);
    let mut normed = centered.div(var.add_scalar(eps).sqrt());

    if let Some(s) = scale { normed = normed.mul(s); }
    if let Some(b) = bias { normed = normed.add(b); }

    values.insert(node.output[0].clone(), normed);
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

    let ndim = input.ndim();
    let axis = axes.first().map(|&a| if a < 0 { (ndim as i64 + a) as usize } else { a as usize }).unwrap_or(ndim - 1);

    let mean = input.mean_dim(axis);

    let result = if keepdims {
        mean
    } else {
        // Remove the reduced dimension from shape
        let shape = mean.shape();
        let new_shape: Vec<usize> = shape.iter().enumerate()
            .filter(|&(i, _)| i != axis)
            .map(|(_, &d)| d)
            .collect();
        if new_shape.is_empty() {
            mean.reshape(vec![1])
        } else {
            mean.reshape(new_shape)
        }
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
    let mut input = values.get(&node.input[0]).ok_or("rms_norm: input not found")?.clone();
    let skip = if is_skip && node.input.len() > 1 && !node.input[1].is_empty() { values.get(&node.input[1]).cloned() } else { None };
    let weight_idx = if is_skip { 2 } else { 1 };
    let weight = if node.input.len() > weight_idx && !node.input[weight_idx].is_empty() { values.get(&node.input[weight_idx]).cloned() } else { None };
    let eps = node.attribute.iter().find(|a| a.name == "epsilon").map(|a| a.f).unwrap_or(1e-5);

    if let Some(s) = skip { input = input.add(s); }
    let input_skip = input.clone();

    let last_dim = input.ndim() - 1;
    let rms = input.clone().powf_scalar(2.0).mean_dim(last_dim).sqrt();
    let mut normed = input.div(rms.add_scalar(eps));

    if let Some(w) = weight { normed = normed.mul(w); }

    values.insert(node.output[0].clone(), normed);
    if is_skip && node.output.len() > 3 && !node.output[3].is_empty() {
        values.insert(node.output[3].clone(), input_skip);
    }
    Ok(())
}

/// GroupQueryAttention — fused multi-head attention with KV groups + RoPE + KV-cache (Llama)
pub fn group_query_attention_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    let query = values.get(&node.input[0]).ok_or("gqa: query not found")?.clone();
    let key = values.get(&node.input[1]).ok_or("gqa: key not found")?.clone();
    let val_input = values.get(&node.input[2]).ok_or("gqa: value not found")?.clone();

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

    // query: [batch, seq_len, q_hidden]
    let q_shape = query.shape();
    let batch = q_shape[0];
    let seq_len = q_shape[1];
    let q_hidden = q_shape[2];
    let head_dim = q_hidden / num_heads;
    let attn_scale = if scale_attr > 0.0 { scale_attr } else { 1.0 / (head_dim as f32).sqrt() };

    // Determine past sequence length for RoPE position offset
    let past_seq_len = match &past_key {
        Some(pk) => pk.shape()[2],
        None => 0,
    };

    // Reshape: [batch, seq, heads, dim] -> [batch, heads, seq, dim]
    let mut q = query.reshape(vec![batch, seq_len, num_heads, head_dim]).swap_dims(1, 2);
    let mut k_new = key.reshape(vec![batch, seq_len, kv_num_heads, head_dim]).swap_dims(1, 2);
    let v_new = val_input.reshape(vec![batch, seq_len, kv_num_heads, head_dim]).swap_dims(1, 2);

    // Apply RoPE with position offset (past_seq_len)
    if do_rotary {
        if let (Some(cos_c), Some(sin_c)) = (&cos_cache, &sin_cache) {
            let half_dim = head_dim / 2;
            // Positions: past_seq_len .. past_seq_len + seq_len
            let cos_slice = cos_c.clone().narrow(0, past_seq_len, seq_len);
            let sin_slice = sin_c.clone().narrow(0, past_seq_len, seq_len);
            let cos_4d = cos_slice.reshape(vec![1, 1, seq_len, half_dim]);
            let sin_4d = sin_slice.reshape(vec![1, 1, seq_len, half_dim]);

            q = apply_rope(q, &cos_4d, &sin_4d, head_dim);
            k_new = apply_rope(k_new, &cos_4d, &sin_4d, head_dim);
        }
    }

    // Concatenate with past KV cache
    let k_full = if let Some(past_k) = &past_key {
        if past_k.shape()[2] > 0 {
            Value::cat(vec![past_k.clone(), k_new], 2) // concat on seq dim
        } else { k_new }
    } else { k_new };

    let v_full = if let Some(past_v) = &past_value {
        if past_v.shape()[2] > 0 {
            Value::cat(vec![past_v.clone(), v_new], 2)
        } else { v_new }
    } else { v_new };

    let total_seq = k_full.shape()[2]; // past_seq + new_seq

    // Repeat KV heads to match Q heads
    let repeats = num_heads / kv_num_heads;
    let k_expanded = if repeats > 1 {
        let mut e = Vec::new();
        for h in 0..kv_num_heads {
            let hd = k_full.clone().narrow(1, h, 1);
            for _ in 0..repeats { e.push(hd.clone()); }
        }
        Value::cat(e, 1)
    } else { k_full.clone() };
    let v_expanded = if repeats > 1 {
        let mut e = Vec::new();
        for h in 0..kv_num_heads {
            let hd = v_full.clone().narrow(1, h, 1);
            for _ in 0..repeats { e.push(hd.clone()); }
        }
        Value::cat(e, 1)
    } else { v_full.clone() };

    // Attention: Q[batch, heads, new_seq, dim] x K^T[batch, heads, dim, total_seq]
    let scores = q.matmul(k_expanded.transpose()).mul_scalar(attn_scale);

    // Causal mask: [new_seq, total_seq] — each new position can attend to past + itself
    let scores = if seq_len > 1 || total_seq > 1 {
        apply_causal_mask_kv(scores, seq_len, total_seq, device)
    } else {
        scores
    };

    let attn = scores.softmax(3); // softmax over last dim (total_seq)
    let output = attn.matmul(v_expanded)
        .swap_dims(1, 2)
        .reshape(vec![batch, seq_len, q_hidden]);

    values.insert(node.output[0].clone(), output);
    // present_key, present_value = full KV (for next step's past_kv)
    if node.output.len() > 1 && !node.output[1].is_empty() {
        values.insert(node.output[1].clone(), k_full);
    }
    if node.output.len() > 2 && !node.output[2].is_empty() {
        values.insert(node.output[2].clone(), v_full);
    }
    Ok(())
}

/// Apply Rotary Position Embeddings (RoPE)
/// x: [batch, heads, seq, dim], cos/sin: [1, 1, seq, dim/2]
fn apply_rope(
    x: Value,
    cos: &Value,
    sin: &Value,
    head_dim: usize,
) -> Value {
    let half = head_dim / 2;

    // Split into first half and second half
    let x1 = x.clone().narrow(3, 0, half);
    let x2 = x.narrow(3, half, half);

    // RoPE: [x1*cos - x2*sin, x1*sin + x2*cos]
    let out1 = x1.clone().mul(cos.clone()).sub(x2.clone().mul(sin.clone()));
    let out2 = x1.mul(sin.clone()).add(x2.mul(cos.clone()));

    Value::cat(vec![out1, out2], 3)
}

/// Apply causal mask for KV-cache: [new_seq, total_seq]
/// Each new position i can attend to positions 0..past_seq+i+1
fn apply_causal_mask_kv(
    scores: Value,
    new_seq: usize,
    total_seq: usize,
    device: &Device,
) -> Value {
    let past_seq = total_seq - new_seq;
    let mut mask_data = vec![0.0f32; new_seq * total_seq];
    for i in 0..new_seq {
        for j in (past_seq + i + 1)..total_seq {
            mask_data[i * total_seq + j] = f32::NEG_INFINITY;
        }
    }
    let mask = Value::from_data(mask_data, vec![1, 1, new_seq, total_seq], device);
    scores.add(mask)
}

/// RotaryEmbedding — standalone RoPE operator (Qwen3, etc.)
/// Inputs: x [batch, seq, hidden], position_ids [1, seq], cos_cache [max_seq, dim/2], sin_cache [max_seq, dim/2]
pub fn rotary_embedding_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let x = values.get(&node.input[0]).ok_or("rope: input not found")?.clone();
    let pos_ids = if node.input.len() > 1 && !node.input[1].is_empty() {
        values.get(&node.input[1]).cloned()
    } else { None };
    let cos_cache = if node.input.len() > 2 && !node.input[2].is_empty() {
        values.get(&node.input[2]).cloned()
    } else { None };
    let sin_cache = if node.input.len() > 3 && !node.input[3].is_empty() {
        values.get(&node.input[3]).cloned()
    } else { None };

    let _interleaved = node.attribute.iter().find(|a| a.name == "interleaved").map(|a| a.i != 0).unwrap_or(false);

    let x_shape = x.shape();
    let batch = x_shape[0];
    let seq_len = x_shape[1];
    let hidden = x_shape[2];

    if let (Some(cos_c), Some(sin_c)) = (cos_cache, sin_cache) {
        let rope_dim = cos_c.shape()[1]; // cos_cache: [max_seq, rope_dim] (= head_dim/2)
        let head_dim = rope_dim * 2;
        let num_heads = hidden / head_dim;

        // Get position indices from pos_ids input
        let positions = if let Some(ref pids) = pos_ids {
            pids.to_vec_f32().iter().map(|&v| v as usize).collect::<Vec<_>>()
        } else {
            (0..seq_len).collect()
        };

        // Gather cos/sin for each position individually (handles non-contiguous positions)
        let start = positions[0];
        let contiguous = positions.iter().enumerate().all(|(i, &p)| p == start + i);

        let (cos_gathered, sin_gathered) = if contiguous {
            (cos_c.clone().narrow(0, start, seq_len),
             sin_c.clone().narrow(0, start, seq_len))
        } else {
            // Non-contiguous: gather each row
            let cos_rows: Vec<_> = positions.iter()
                .map(|&p| cos_c.clone().narrow(0, p, 1))
                .collect();
            let sin_rows: Vec<_> = positions.iter()
                .map(|&p| sin_c.clone().narrow(0, p, 1))
                .collect();
            (Value::cat(cos_rows, 0), Value::cat(sin_rows, 0))
        };

        // x: [batch, seq, hidden] -> [batch, seq, num_heads, head_dim]
        let x_4d = x.reshape(vec![batch, seq_len, num_heads, head_dim]);
        // cos/sin: [seq, rope_dim] -> [1, seq, 1, rope_dim]
        let cos_4d = cos_gathered.reshape(vec![1, seq_len, 1, rope_dim]);
        let sin_4d = sin_gathered.reshape(vec![1, seq_len, 1, rope_dim]);

        // Split head_dim into halves: first rope_dim and second rope_dim
        let x1 = x_4d.clone().narrow(3, 0, rope_dim);
        let x2 = x_4d.narrow(3, rope_dim, rope_dim);

        // RoPE: [x1*cos - x2*sin, x1*sin + x2*cos]
        let out1 = x1.clone().mul(cos_4d.clone()).sub(x2.clone().mul(sin_4d.clone()));
        let out2 = x1.mul(sin_4d).add(x2.mul(cos_4d));

        let result = Value::cat(vec![out1, out2], 3)
            .reshape(vec![batch, seq_len, hidden]);
        values.insert(node.output[0].clone(), result);
    } else {
        values.insert(node.output[0].clone(), x);
    }
    Ok(())
}

/// ReduceSum
pub fn reduce_sum_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
) -> Result<(), String> {
    let input = values.get(&node.input[0]).ok_or("reduce_sum: input not found")?.clone();
    let ndim = input.ndim();
    let keepdims = node.attribute.iter().find(|a| a.name == "keepdims").map(|a| a.i != 0).unwrap_or(true);

    // Get axes — from attribute or second input
    let axes: Vec<i64> = if node.input.len() > 1 && !node.input[1].is_empty() {
        if let Some(ax) = values.get(&node.input[1]) {
            ax.to_vec_f32().iter().map(|&v| v as i64).collect()
        } else {
            vec![]
        }
    } else {
        node.attribute.iter().find(|a| a.name == "axes").map(|a| a.ints.clone()).unwrap_or_default()
    };

    if axes.is_empty() {
        // Sum over all — reduce to scalar
        let result = input.sum();
        values.insert(node.output[0].clone(), result);
    } else {
        let axis = axes[0];
        let axis = if axis < 0 { (ndim as i64 + axis) as usize } else { axis as usize };
        let reduced = input.sum_dim(axis);
        let result = if keepdims {
            reduced
        } else {
            let shape = reduced.shape();
            let new_shape: Vec<usize> = shape.iter().enumerate()
                .filter(|&(i, _)| i != axis)
                .map(|(_, &d)| d)
                .collect();
            if new_shape.is_empty() {
                reduced.reshape(vec![1])
            } else {
                reduced.reshape(new_shape)
            }
        };
        values.insert(node.output[0].clone(), result);
    }
    Ok(())
}
