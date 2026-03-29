//! High-level GPU ops — all take &mut CommandEncoder for batched dispatch
//! Zero separate GPU submissions — all ops accumulate into one command buffer

use super::pipelines::Pipelines;
use wgpu;

/// Q4 matrix-vector multiply: [1, K] × Q4[N, K] → [1, N]
pub fn q4_matmul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
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

    let params = Params { n, k, num_blocks, u32s_per_row: num_blocks * (block_size / 2) / 4 };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let x = n.min(65535);
    let y = (n + x - 1) / x;
    p.encode(enc, &p.q4_matmul, &[
        activation.as_entire_binding(),
        packed_weights.as_entire_binding(),
        scales.as_entire_binding(),
        output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], (x, y, 1));

    output
}

/// RMS Normalization
pub fn rms_norm(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    positions: u32,
    hidden: u32,
    eps: f32,
) -> wgpu::Buffer {
    let output = p.alloc((positions as u64) * (hidden as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { hidden: u32, eps: f32 }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { hidden, eps }));

    p.encode(enc, &p.rms_norm, &[
        input.as_entire_binding(),
        weight.as_entire_binding(),
        output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], (positions, 1, 1));

    output
}

/// Element-wise add
pub fn add(p: &Pipelines, enc: &mut wgpu::CommandEncoder, a: &wgpu::Buffer, b: &wgpu::Buffer, n: u32) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(enc, &p.add, &[
        a.as_entire_binding(), b.as_entire_binding(), output.as_entire_binding(),
    ], ((n + 255) / 256, 1, 1));
    output
}

/// Fused SiLU gate (SwiGLU)
pub fn silu_mul(p: &Pipelines, enc: &mut wgpu::CommandEncoder, gate: &wgpu::Buffer, up: &wgpu::Buffer, n: u32) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(enc, &p.silu_mul, &[
        gate.as_entire_binding(), up.as_entire_binding(), output.as_entire_binding(),
    ], ((n + 255) / 256, 1, 1));
    output
}

/// RoPE
pub fn rope(
    p: &Pipelines, enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer, cos_cache: &wgpu::Buffer, sin_cache: &wgpu::Buffer,
    total_elements: u32, head_dim: u32, seq_len: u32,
) -> wgpu::Buffer {
    let output = p.alloc((total_elements as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { half_dim: u32, head_dim: u32, seq_len: u32, total_elements: u32 }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        half_dim: head_dim / 2, head_dim, seq_len, total_elements,
    }));

    p.encode(enc, &p.rope, &[
        input.as_entire_binding(), cos_cache.as_entire_binding(),
        sin_cache.as_entire_binding(), output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], ((total_elements + 255) / 256, 1, 1));

    output
}

/// Attention decode: fused softmax(Q·K^T/√d)·V
pub fn attention_decode(
    p: &Pipelines, enc: &mut wgpu::CommandEncoder,
    q: &wgpu::Buffer, k: &wgpu::Buffer, v: &wgpu::Buffer,
    num_heads: u32, head_dim: u32, total_seq: u32, scale: f32,
) -> wgpu::Buffer {
    let output_size = num_heads * head_dim;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { head_dim: u32, total_seq: u32, num_heads: u32, scale: f32 }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { head_dim, total_seq, num_heads, scale }));

    // One workgroup per head (threads cooperate within head)
    p.encode(enc, &p.attention, &[
        q.as_entire_binding(), k.as_entire_binding(), v.as_entire_binding(),
        output.as_entire_binding(), params_buf.as_entire_binding(),
    ], (num_heads, 1, 1));

    output
}

/// f32 vector × matrix: [K] × [N, K] → [N]
pub fn f32_matmul(
    p: &Pipelines, enc: &mut wgpu::CommandEncoder,
    activation: &wgpu::Buffer, weight: &wgpu::Buffer,
    n: u32, k: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { n: u32, k: u32 }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { n, k }));

    // Split into 2D dispatch to stay under 65535 limit per dimension
    let x = n.min(65535);
    let y = (n + x - 1) / x;

    p.encode(enc, &p.f32_matmul, &[
        activation.as_entire_binding(),
        weight.as_entire_binding(),
        output.as_entire_binding(),
        params_buf.as_entire_binding(),
    ], (x, y, 1));

    output
}

/// Embedding lookup
pub fn embed(
    p: &Pipelines, enc: &mut wgpu::CommandEncoder,
    table: &wgpu::Buffer, token_ids: &wgpu::Buffer,
    hidden: u32, seq_len: u32,
) -> wgpu::Buffer {
    let output_size = seq_len * hidden;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params { hidden: u32, seq_len: u32 }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { hidden, seq_len }));

    p.encode(enc, &p.embed, &[
        table.as_entire_binding(), token_ids.as_entire_binding(),
        output.as_entire_binding(), params_buf.as_entire_binding(),
    ], ((output_size + 255) / 256, 1, 1));

    output
}
