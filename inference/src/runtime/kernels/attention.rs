//! Attention kernel — QK^T, causal mask, softmax, AV

use cubecl::prelude::*;
use cubecl::wgpu::WgpuRuntime;
use crate::runtime::tensor::{Client, GpuTensor};

/// Compute attention scores, apply causal mask, softmax, and multiply by V
/// Q: [batch, heads, new_seq, head_dim]
/// K: [batch, heads, total_seq, head_dim]
/// V: [batch, heads, total_seq, head_dim]
/// output: [batch, heads, new_seq, head_dim]
///
/// For decode (new_seq=1): each head computes one dot product + one weighted sum
#[cube(launch_unchecked)]
fn attention_decode_kernel(
    q: &Tensor<Line<f32>>,
    k: &Tensor<Line<f32>>,
    v: &Tensor<Line<f32>>,
    output: &mut Tensor<Line<f32>>,
    scale_buf: &Tensor<Line<f32>>,
    #[comptime] head_dim: u32,
    #[comptime] total_seq: u32,
) {
    // Each thread handles one (batch, head, output_dim) tuple
    // For decode: new_seq = 1, so output is [batch, heads, 1, head_dim]
    let pos = ABSOLUTE_POS;
    let num_heads = output.len() / head_dim as usize;
    let head_idx = pos / head_dim as usize;
    let d = (pos % head_dim as usize) as u32;

    if head_idx >= num_heads { terminate!(); }

    // Compute attention scores for this head: Q[head, 0, :] · K[head, t, :] for all t
    // Then softmax and weighted sum of V

    let scale = f32::cast_from(scale_buf[0]);
    let q_base = head_idx * head_dim as usize;
    let k_base = head_idx * total_seq as usize * head_dim as usize;
    let v_base = k_base;

    // Step 1: compute scores and find max (for stable softmax)
    let mut max_score: f32 = -1000000000.0f32;
    let mut t: u32 = 0;
    while t < total_seq {
        let mut dot: f32 = 0.0f32;
        let mut dd: u32 = 0;
        while dd < head_dim {
            let q_val = f32::cast_from(q[q_base + dd as usize]);
            let k_val = f32::cast_from(k[k_base + t as usize * head_dim as usize + dd as usize]);
            dot += q_val * k_val;
            dd += 1;
        }
        dot *= scale;
        if dot > max_score { max_score = dot; }
        t += 1;
    }

    // Step 2: compute exp(score - max) and sum
    let mut exp_sum: f32 = 0.0f32;
    t = 0;
    while t < total_seq {
        let mut dot: f32 = 0.0f32;
        let mut dd: u32 = 0;
        while dd < head_dim {
            let q_val = f32::cast_from(q[q_base + dd as usize]);
            let k_val = f32::cast_from(k[k_base + t as usize * head_dim as usize + dd as usize]);
            dot += q_val * k_val;
            dd += 1;
        }
        dot *= scale;
        exp_sum += f32::exp(dot - max_score);
        t += 1;
    }

    // Step 3: compute weighted sum of V for dimension d
    let mut result: f32 = 0.0f32;
    t = 0;
    while t < total_seq {
        let mut dot: f32 = 0.0f32;
        let mut dd: u32 = 0;
        while dd < head_dim {
            let q_val = f32::cast_from(q[q_base + dd as usize]);
            let k_val = f32::cast_from(k[k_base + t as usize * head_dim as usize + dd as usize]);
            dot += q_val * k_val;
            dd += 1;
        }
        dot *= scale;
        let attn_weight = f32::exp(dot - max_score) / exp_sum;
        let v_val = f32::cast_from(v[v_base + t as usize * head_dim as usize + d as usize]);
        result += attn_weight * v_val;
        t += 1;
    }

    output[pos] = Line::cast_from(result);
}

/// Attention for decode step (new_seq = 1)
/// Q: [batch, heads, 1, head_dim]
/// K, V: [batch, heads, total_seq, head_dim]
/// Returns: [batch, heads, 1, head_dim]
pub fn attention_decode(
    client: &Client,
    q: &GpuTensor,
    k: &GpuTensor,
    v: &GpuTensor,
    head_dim: usize,
    total_seq: usize,
    scale: f32,
) -> GpuTensor {
    let output = GpuTensor::empty_f32(client, q.shape.clone());
    let n = q.numel(); // batch * heads * 1 * head_dim

    let cube_dim = CubeDim::new(client, n);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, n, cube_dim);

    let scale_buf = GpuTensor::from_f32(client, &[scale], vec![1]);

    unsafe {
        attention_decode_kernel::launch_unchecked::<WgpuRuntime>(
            client, cube_count, cube_dim,
            q.as_arg(), k.as_arg(), v.as_arg(), output.as_arg(),
            scale_buf.as_arg(),
            head_dim as u32,
            total_seq as u32,
        ).expect("attention_decode failed");
    }

    output
}
