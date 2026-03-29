//! High-level GPU operations built on WGSL compute shaders
//! Each op dispatches one or more compute passes — zero CPU roundtrip

use super::pipelines::Pipelines;
use wgpu;

/// Q4 matrix-vector multiply: [1, K] × Q4[N, K] → [1, N]
/// packed_weights: u32-packed 4-bit weights [N * num_blocks * block_size/2 / 4]
/// scales: f32 per-block scales [N * num_blocks]
pub fn q4_matmul(
    p: &Pipelines,
    activation: &wgpu::Buffer,
    packed_weights: &wgpu::Buffer,
    scales: &wgpu::Buffer,
    n: u32, k: u32, block_size: u32,
) -> wgpu::Buffer {
    let num_blocks = k / block_size;
    let output = p.alloc((n as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { n: u32, k: u32, num_blocks: u32, u32s_per_row: u32 }

    let params = Params {
        n, k, num_blocks,
        u32s_per_row: num_blocks * (block_size / 2) / 4,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    p.dispatch(&p.q4_matmul, &[
        activation.as_entire_binding(),
        packed_weights.as_entire_binding(),
        scales.as_entire_binding(),
        output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], (n, 1, 1));

    output
}

/// RMS Normalization: output = input / rms(input) * weight
pub fn rms_norm(
    p: &Pipelines,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    positions: u32,  // batch * seq
    hidden: u32,
    eps: f32,
) -> wgpu::Buffer {
    let output = p.alloc((positions as u64) * (hidden as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { hidden: u32, eps: f32 }

    let params = Params { hidden, eps };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    p.dispatch(&p.rms_norm, &[
        input.as_entire_binding(),
        weight.as_entire_binding(),
        output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], (positions, 1, 1));

    output
}

/// Element-wise add: output = a + b
pub fn add(p: &Pipelines, a: &wgpu::Buffer, b: &wgpu::Buffer, n: u32) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.dispatch(&p.add, &[
        a.as_entire_binding(),
        b.as_entire_binding(),
        output.as_entire_binding(),
    ], ((n + 255) / 256, 1, 1));
    output
}

/// Fused SiLU gate: output = silu(gate) * up
pub fn silu_mul(p: &Pipelines, gate: &wgpu::Buffer, up: &wgpu::Buffer, n: u32) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.dispatch(&p.silu_mul, &[
        gate.as_entire_binding(),
        up.as_entire_binding(),
        output.as_entire_binding(),
    ], ((n + 255) / 256, 1, 1));
    output
}

/// RoPE: apply rotary position embeddings
pub fn rope(
    p: &Pipelines,
    input: &wgpu::Buffer,
    cos_cache: &wgpu::Buffer,
    sin_cache: &wgpu::Buffer,
    total_elements: u32,
    head_dim: u32,
    seq_len: u32,
) -> wgpu::Buffer {
    let output = p.alloc((total_elements as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { half_dim: u32, head_dim: u32, seq_len: u32, total_elements: u32 }

    let params = Params {
        half_dim: head_dim / 2,
        head_dim,
        seq_len,
        total_elements,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    p.dispatch(&p.rope, &[
        input.as_entire_binding(),
        cos_cache.as_entire_binding(),
        sin_cache.as_entire_binding(),
        output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], ((total_elements + 255) / 256, 1, 1));

    output
}

/// Attention decode: softmax(Q·K^T/√d)·V
pub fn attention_decode(
    p: &Pipelines,
    q: &wgpu::Buffer,      // [num_heads * head_dim]
    k: &wgpu::Buffer,      // [num_heads * total_seq * head_dim]
    v: &wgpu::Buffer,
    num_heads: u32,
    head_dim: u32,
    total_seq: u32,
    scale: f32,
) -> wgpu::Buffer {
    let output_size = num_heads * head_dim;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { head_dim: u32, total_seq: u32, num_heads: u32, scale: f32 }

    let params = Params { head_dim, total_seq, num_heads, scale };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    p.dispatch(&p.attention, &[
        q.as_entire_binding(),
        k.as_entire_binding(),
        v.as_entire_binding(),
        output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], (output_size, 1, 1));

    output
}

/// Embedding lookup
pub fn embed(
    p: &Pipelines,
    table: &wgpu::Buffer,
    token_ids: &wgpu::Buffer,
    hidden: u32,
    seq_len: u32,
) -> wgpu::Buffer {
    let output_size = seq_len * hidden;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { hidden: u32, seq_len: u32 }

    let params = Params { hidden, seq_len };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    p.dispatch(&p.embed, &[
        table.as_entire_binding(),
        token_ids.as_entire_binding(),
        output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], ((output_size + 255) / 256, 1, 1));

    output
}
