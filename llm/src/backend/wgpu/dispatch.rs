//! GPU dispatch functions — all take &mut CommandEncoder for batched dispatch
//! Zero separate GPU submissions — all ops accumulate into one command buffer
//!
//! `_prepare` variants return (output_buffer, bind_group, workgroups) without
//! dispatching — caller batches many dispatches into one compute pass.

use super::pipelines::Pipelines;
use wgpu;

/// Q4 matmul: [1, K] x Q4[N, K] -> [1, N]
pub fn q4_matmul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    activation: &wgpu::Buffer,
    packed_weights: &wgpu::Buffer,
    scales: &wgpu::Buffer,
    n: u32,
    k: u32,
    block_size: u32,
) -> wgpu::Buffer {
    let num_blocks = k / block_size;
    let output = p.alloc((n as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        k: u32,
        num_blocks: u32,
        u32s_per_row: u32,
    }

    let params = Params {
        n,
        k,
        num_blocks,
        u32s_per_row: num_blocks * (block_size / 2) / 4,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    p.encode(
        enc,
        &p.q4_matmul,
        &[
            activation.as_entire_binding(),
            packed_weights.as_entire_binding(),
            scales.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (x, y, 1),
    );

    output
}

/// Q8 matmul: [1, K] x Q8[N, K] -> [1, N]
pub fn q8_matmul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    activation: &wgpu::Buffer,
    packed_weights: &wgpu::Buffer,
    scales: &wgpu::Buffer,
    n: u32,
    k: u32,
    block_size: u32,
) -> wgpu::Buffer {
    let num_blocks = k / block_size;
    let output = p.alloc((n as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        k: u32,
        num_blocks: u32,
        u32s_per_row: u32,
    }

    let params = Params {
        n,
        k,
        num_blocks,
        u32s_per_row: k / 4, // 4 int8 values per u32
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    p.encode(
        enc,
        &p.q8_matmul,
        &[
            activation.as_entire_binding(),
            packed_weights.as_entire_binding(),
            scales.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (x, y, 1),
    );

    output
}

/// Q4_K matmul: [1, K] x Q4_K[N, K] -> [1, N]
/// Weights stored as raw Q4_K superblocks (144 bytes = 256 values each)
pub fn q4k_matmul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    activation: &wgpu::Buffer,
    weights: &wgpu::Buffer,
    n: u32,
    k: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    let blocks_per_row = k / 256;

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        k: u32,
        blocks_per_row: u32,
        _pad: u32,
    }

    let params = Params { n, k, blocks_per_row, _pad: 0 };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    p.encode(
        enc,
        &p.q4k_matmul,
        &[
            activation.as_entire_binding(),
            weights.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (x, y, 1),
    );

    output
}

/// Q4_K matmul prepare with pre-computed params (decode path)
pub fn q4k_matmul_prepare_precomputed(
    p: &Pipelines,
    activation: &wgpu::Buffer,
    weights: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.q4k_matmul,
        &[
            activation.as_entire_binding(),
            weights.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// Q6_K matmul: [1, K] x Q6_K[N, K] -> [1, N]
/// Weights stored as raw Q6_K superblocks (210 bytes = 256 values each)
pub fn q6k_matmul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    activation: &wgpu::Buffer,
    weights: &wgpu::Buffer,
    n: u32,
    k: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    let blocks_per_row = k / 256;

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        k: u32,
        blocks_per_row: u32,
        _pad: u32,
    }

    let params = Params { n, k, blocks_per_row, _pad: 0 };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let num_wg = (n + 3) / 4;
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    p.encode(
        enc,
        &p.q6k_matmul,
        &[
            activation.as_entire_binding(),
            weights.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (x, y, 1),
    );

    output
}

/// Q6_K matmul prepare with pre-computed params (decode path)
pub fn q6k_matmul_prepare_precomputed(
    p: &Pipelines,
    activation: &wgpu::Buffer,
    weights: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.q6k_matmul,
        &[
            activation.as_entire_binding(),
            weights.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// LayerNorm: (input - mean) / sqrt(var + eps) * weight + bias
pub fn layernorm(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    positions: u32,
    hidden: u32,
    eps: f32,
) -> wgpu::Buffer {
    let output = p.alloc((positions as u64) * (hidden as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        hidden: u32,
        eps: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { hidden, eps }));

    p.encode(
        enc,
        &p.layernorm,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (positions, 1, 1),
    );

    output
}

/// GELU activation
pub fn gelu(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.gelu,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
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
    struct Params {
        hidden: u32,
        eps: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { hidden, eps }));

    p.encode(
        enc,
        &p.rms_norm,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (positions, 1, 1),
    );

    output
}

/// Element-wise add
pub fn add(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    a: &wgpu::Buffer,
    b: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.add,
        &[
            a.as_entire_binding(),
            b.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Fused SiLU gate (SwiGLU)
pub fn silu_mul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    gate: &wgpu::Buffer,
    up: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.silu_mul,
        &[
            gate.as_entire_binding(),
            up.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// RoPE
pub fn rope(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
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
    struct Params {
        half_dim: u32,
        head_dim: u32,
        seq_len: u32,
        total_elements: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        half_dim: head_dim / 2,
        head_dim,
        seq_len,
        total_elements,
    }));

    p.encode(
        enc,
        &p.rope,
        &[
            input.as_entire_binding(),
            cos_cache.as_entire_binding(),
            sin_cache.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_elements + 255) / 256, 1, 1),
    );

    output
}

/// Attention decode: fused softmax(Q*K^T/sqrt(d))*V
pub fn attention_decode(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    q: &wgpu::Buffer,
    k: &wgpu::Buffer,
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
    struct Params {
        head_dim: u32,
        total_seq: u32,
        num_heads: u32,
        scale: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        head_dim,
        total_seq,
        num_heads,
        scale,
    }));

    p.encode(
        enc,
        &p.attention,
        &[
            q.as_entire_binding(),
            k.as_entire_binding(),
            v.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (num_heads, 1, 1),
    );

    output
}

/// KV cache append
pub fn kv_append(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    past_kv: &wgpu::Buffer,
    new_kv: &wgpu::Buffer,
    num_heads: u32,
    head_dim: u32,
    past_seq: u32,
    new_seq: u32,
) -> wgpu::Buffer {
    let total_seq = past_seq + new_seq;
    let total_elements = num_heads * total_seq * head_dim;
    let output = p.alloc((total_elements as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        num_heads: u32,
        head_dim: u32,
        past_seq: u32,
        new_seq: u32,
        total_seq: u32,
        kv_heads: u32,
        _p0: u32,
        _p1: u32,
    }

    let params = Params {
        num_heads,
        head_dim,
        past_seq,
        new_seq,
        total_seq,
        kv_heads: num_heads,
        _p0: 0,
        _p1: 0,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    p.encode(
        enc,
        &p.kv_append,
        &[
            past_kv.as_entire_binding(),
            new_kv.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_elements + 255) / 256, 1, 1),
    );

    output
}

/// f32 vector x matrix
pub fn f32_matmul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    activation: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    n: u32,
    k: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        k: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { n, k }));

    let x = n.min(65535);
    let y = (n + x - 1) / x;

    p.encode(
        enc,
        &p.f32_matmul,
        &[
            activation.as_entire_binding(),
            weight.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (x, y, 1),
    );

    output
}

/// Argmax on GPU
pub fn argmax_gpu(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc(4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { n }));

    p.encode(
        enc,
        &p.argmax,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (1, 1, 1),
    );

    output
}

/// Embedding lookup
pub fn embed(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    table: &wgpu::Buffer,
    token_ids: &wgpu::Buffer,
    hidden: u32,
    seq_len: u32,
) -> wgpu::Buffer {
    let output_size = seq_len * hidden;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        hidden: u32,
        seq_len: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { hidden, seq_len }));

    p.encode(
        enc,
        &p.embed,
        &[
            table.as_entire_binding(),
            token_ids.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((output_size + 255) / 256, 1, 1),
    );

    output
}

// ========================================================================
// Batch 1: Trivial element-wise dispatch functions
// ========================================================================

/// Element-wise subtract
pub fn sub(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    a: &wgpu::Buffer,
    b: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.sub,
        &[
            a.as_entire_binding(),
            b.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Element-wise divide
pub fn div(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    a: &wgpu::Buffer,
    b: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.div,
        &[
            a.as_entire_binding(),
            b.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// ReLU activation
pub fn relu(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.relu,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Leaky ReLU activation
pub fn leaky_relu(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
    slope: f32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        slope: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { slope }));

    p.encode(
        enc,
        &p.leaky_relu,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Tanh activation
pub fn tanh_act(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.tanh_act,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Clamp values to [min, max]
pub fn clamp_op(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
    min_val: f32,
    max_val: f32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        min_val: f32,
        max_val: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { min_val, max_val }));

    p.encode(
        enc,
        &p.clamp,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Absolute value
pub fn abs_op(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.abs_op,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Negate
pub fn neg(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.neg,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Square root
pub fn sqrt_op(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.sqrt_op,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Exponential
pub fn exp_op(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.exp_op,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// Sigmoid activation
pub fn sigmoid(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.sigmoid,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

// ========================================================================
// Batch 2: Compound activations
// ========================================================================

/// GeGLU: gelu(gate) * up
pub fn geglu(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    gate: &wgpu::Buffer,
    up: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.geglu,
        &[
            gate.as_entire_binding(),
            up.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// SwiGLU: silu(gate) * up
pub fn swiglu(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    gate: &wgpu::Buffer,
    up: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.swiglu,
        &[
            gate.as_entire_binding(),
            up.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// GLU: sigmoid(gate) * value
pub fn glu(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    gate: &wgpu::Buffer,
    value: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.glu,
        &[
            gate.as_entire_binding(),
            value.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

/// PReLU: x > 0 ? x : x * slope[i]
pub fn prelu(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    slopes: &wgpu::Buffer,
    n: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    p.encode(
        enc,
        &p.prelu,
        &[
            input.as_entire_binding(),
            slopes.as_entire_binding(),
            output.as_entire_binding(),
        ],
        ((n + 255) / 256, 1, 1),
    );
    output
}

// ========================================================================
// Batch 3: Normalization
// ========================================================================

/// Batch normalization (inference mode with running stats)
pub fn batchnorm(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    running_mean: &wgpu::Buffer,
    running_var: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    channels: u32,
    spatial_size: u32,
    batch_size: u32,
    eps: f32,
) -> wgpu::Buffer {
    let total = batch_size as u64 * channels as u64 * spatial_size as u64;
    let output = p.alloc(total * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels: u32,
        spatial_size: u32,
        batch_size: u32,
        eps: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels,
        spatial_size,
        batch_size,
        eps,
    }));

    p.encode(
        enc,
        &p.batchnorm,
        &[
            input.as_entire_binding(),
            running_mean.as_entire_binding(),
            running_var.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (channels, 1, 1),
    );

    output
}

/// Group normalization
pub fn groupnorm(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    channels: u32,
    spatial_size: u32,
    num_groups: u32,
    batch_size: u32,
    eps: f32,
) -> wgpu::Buffer {
    let total = batch_size as u64 * channels as u64 * spatial_size as u64;
    let output = p.alloc(total * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels: u32,
        spatial_size: u32,
        num_groups: u32,
        channels_per_group: u32,
        eps: f32,
        _pad: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels,
        spatial_size,
        num_groups,
        channels_per_group: channels / num_groups,
        eps,
        _pad: 0,
    }));

    p.encode(
        enc,
        &p.groupnorm,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (batch_size, num_groups, 1),
    );

    output
}

/// Instance normalization
pub fn instance_norm(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    channels: u32,
    spatial_size: u32,
    batch_size: u32,
    eps: f32,
) -> wgpu::Buffer {
    let total = batch_size as u64 * channels as u64 * spatial_size as u64;
    let output = p.alloc(total * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels: u32,
        spatial_size: u32,
        eps: f32,
        _pad: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels,
        spatial_size,
        eps,
        _pad: 0,
    }));

    p.encode(
        enc,
        &p.instance_norm,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (batch_size, channels, 1),
    );

    output
}

/// Adaptive layer norm: (1 + scale) * layernorm(x) + shift
pub fn adaln(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    scale: &wgpu::Buffer,
    shift: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    positions: u32,
    hidden: u32,
    eps: f32,
) -> wgpu::Buffer {
    let output = p.alloc((positions as u64) * (hidden as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        hidden: u32,
        eps: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { hidden, eps }));

    p.encode(
        enc,
        &p.adaln,
        &[
            input.as_entire_binding(),
            scale.as_entire_binding(),
            shift.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (positions, 1, 1),
    );

    output
}

// ========================================================================
// Batch 4: Convolution
// ========================================================================

/// 2D Convolution
pub fn conv2d(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    batch_size: u32,
    in_channels: u32,
    out_channels: u32,
    in_h: u32,
    in_w: u32,
    kernel_h: u32,
    kernel_w: u32,
    stride_h: u32,
    stride_w: u32,
    pad_h: u32,
    pad_w: u32,
    groups: u32,
) -> wgpu::Buffer {
    let out_h = (in_h + 2 * pad_h - kernel_h) / stride_h + 1;
    let out_w = (in_w + 2 * pad_w - kernel_w) / stride_w + 1;
    let total_output = batch_size * out_channels * out_h * out_w;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        in_channels: u32,
        out_channels: u32,
        in_h: u32,
        in_w: u32,
        out_h: u32,
        out_w: u32,
        kernel_h: u32,
        kernel_w: u32,
        stride_h: u32,
        stride_w: u32,
        pad_h: u32,
        pad_w: u32,
        groups: u32,
        batch_size: u32,
        total_output: u32,
        _pad: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        in_channels,
        out_channels,
        in_h,
        in_w,
        out_h,
        out_w,
        kernel_h,
        kernel_w,
        stride_h,
        stride_w,
        pad_h,
        pad_w,
        groups,
        batch_size,
        total_output,
        _pad: 0,
    }));

    p.encode(
        enc,
        &p.conv2d,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

/// 1D Convolution
pub fn conv1d(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    batch_size: u32,
    in_channels: u32,
    out_channels: u32,
    in_length: u32,
    kernel_size: u32,
    stride: u32,
    padding: u32,
    groups: u32,
) -> wgpu::Buffer {
    let out_length = (in_length + 2 * padding - kernel_size) / stride + 1;
    let total_output = batch_size * out_channels * out_length;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        in_channels: u32,
        out_channels: u32,
        in_length: u32,
        out_length: u32,
        kernel_size: u32,
        stride: u32,
        padding: u32,
        groups: u32,
        batch_size: u32,
        total_output: u32,
        _pad0: u32,
        _pad1: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        in_channels,
        out_channels,
        in_length,
        out_length,
        kernel_size,
        stride,
        padding,
        groups,
        batch_size,
        total_output,
        _pad0: 0,
        _pad1: 0,
    }));

    p.encode(
        enc,
        &p.conv1d,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

/// Depthwise convolution (groups = channels)
pub fn depthwise_conv(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    batch_size: u32,
    channels: u32,
    in_length: u32,
    kernel_size: u32,
    stride: u32,
    padding: u32,
) -> wgpu::Buffer {
    let out_length = (in_length + 2 * padding - kernel_size) / stride + 1;
    let total_output = batch_size * channels * out_length;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels: u32,
        in_length: u32,
        out_length: u32,
        kernel_size: u32,
        stride: u32,
        padding: u32,
        batch_size: u32,
        total_output: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels,
        in_length,
        out_length,
        kernel_size,
        stride,
        padding,
        batch_size,
        total_output,
    }));

    p.encode(
        enc,
        &p.depthwise_conv,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

/// 2D Pooling (max or avg)
pub fn pool(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    batch_size: u32,
    channels: u32,
    in_h: u32,
    in_w: u32,
    kernel_h: u32,
    kernel_w: u32,
    stride_h: u32,
    stride_w: u32,
    mode: u32, // 0 = max, 1 = avg
) -> wgpu::Buffer {
    let out_h = (in_h - kernel_h) / stride_h + 1;
    let out_w = (in_w - kernel_w) / stride_w + 1;
    let total_output = batch_size * channels * out_h * out_w;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels: u32,
        in_h: u32,
        in_w: u32,
        out_h: u32,
        out_w: u32,
        kernel_h: u32,
        kernel_w: u32,
        stride_h: u32,
        stride_w: u32,
        batch_size: u32,
        total_output: u32,
        mode: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels,
        in_h,
        in_w,
        out_h,
        out_w,
        kernel_h,
        kernel_w,
        stride_h,
        stride_w,
        batch_size,
        total_output,
        mode,
    }));

    p.encode(
        enc,
        &p.pool,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

// ========================================================================
// Batch 5: Spatial
// ========================================================================

/// Nearest-neighbor interpolation (upsampling)
pub fn interpolate_nearest(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    batch_size: u32,
    channels: u32,
    in_h: u32,
    in_w: u32,
    out_h: u32,
    out_w: u32,
) -> wgpu::Buffer {
    let total_output = batch_size * channels * out_h * out_w;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels: u32,
        in_h: u32,
        in_w: u32,
        out_h: u32,
        out_w: u32,
        batch_size: u32,
        total_output: u32,
        _pad: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels,
        in_h,
        in_w,
        out_h,
        out_w,
        batch_size,
        total_output,
        _pad: 0,
    }));

    p.encode(
        enc,
        &p.interpolate_nearest,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

/// Bilinear interpolation (upsampling)
pub fn interpolate_bilinear(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    batch_size: u32,
    channels: u32,
    in_h: u32,
    in_w: u32,
    out_h: u32,
    out_w: u32,
) -> wgpu::Buffer {
    let total_output = batch_size * channels * out_h * out_w;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels: u32,
        in_h: u32,
        in_w: u32,
        out_h: u32,
        out_w: u32,
        batch_size: u32,
        total_output: u32,
        _pad: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels,
        in_h,
        in_w,
        out_h,
        out_w,
        batch_size,
        total_output,
        _pad: 0,
    }));

    p.encode(
        enc,
        &p.interpolate_bilinear,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

/// Pixel shuffle: (batch, C*r^2, H, W) -> (batch, C, H*r, W*r)
pub fn pixel_shuffle(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    batch_size: u32,
    channels_out: u32,
    in_h: u32,
    in_w: u32,
    upscale_factor: u32,
) -> wgpu::Buffer {
    let out_h = in_h * upscale_factor;
    let out_w = in_w * upscale_factor;
    let total_output = batch_size * channels_out * out_h * out_w;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels_out: u32,
        in_h: u32,
        in_w: u32,
        out_h: u32,
        out_w: u32,
        upscale_factor: u32,
        batch_size: u32,
        total_output: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels_out,
        in_h,
        in_w,
        out_h,
        out_w,
        upscale_factor,
        batch_size,
        total_output,
    }));

    p.encode(
        enc,
        &p.pixel_shuffle,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

/// Pixel unshuffle: (batch, C, H, W) -> (batch, C*r^2, H/r, W/r)
pub fn pixel_unshuffle(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    batch_size: u32,
    channels_in: u32,
    in_h: u32,
    in_w: u32,
    downscale_factor: u32,
) -> wgpu::Buffer {
    let out_h = in_h / downscale_factor;
    let out_w = in_w / downscale_factor;
    let channels_out = channels_in * downscale_factor * downscale_factor;
    let total_output = batch_size * channels_out * out_h * out_w;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channels_in: u32,
        in_h: u32,
        in_w: u32,
        out_h: u32,
        out_w: u32,
        downscale_factor: u32,
        batch_size: u32,
        total_output: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        channels_in,
        in_h,
        in_w,
        out_h,
        out_w,
        downscale_factor,
        batch_size,
        total_output,
    }));

    p.encode(
        enc,
        &p.pixel_unshuffle,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

/// Patch embedding: conv2d with kernel=stride=patch_size
pub fn patch_embed(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    batch_size: u32,
    in_channels: u32,
    embed_dim: u32,
    in_h: u32,
    in_w: u32,
    patch_size: u32,
) -> wgpu::Buffer {
    let num_patches_h = in_h / patch_size;
    let num_patches_w = in_w / patch_size;
    let num_patches = num_patches_h * num_patches_w;
    let total_output = batch_size * num_patches * embed_dim;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        in_channels: u32,
        embed_dim: u32,
        in_h: u32,
        in_w: u32,
        patch_size: u32,
        num_patches_h: u32,
        num_patches_w: u32,
        total_output: u32,
        batch_size: u32,
        _pad0: u32,
        _pad1: u32,
        _pad2: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        in_channels,
        embed_dim,
        in_h,
        in_w,
        patch_size,
        num_patches_h,
        num_patches_w,
        total_output,
        batch_size,
        _pad0: 0,
        _pad1: 0,
        _pad2: 0,
    }));

    p.encode(
        enc,
        &p.patch_embed,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((total_output + 255) / 256, 1, 1),
    );

    output
}

// ========================================================================
// Batch 6: Special ops
// ========================================================================

/// Sinusoidal timestep embedding for diffusion models
pub fn sinusoidal_embed(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    dim: u32,
    timestep: f32,
) -> wgpu::Buffer {
    let output = p.alloc((dim as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        dim: u32,
        half_dim: u32,
        timestep: f32,
        _pad: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        dim,
        half_dim: dim / 2,
        timestep,
        _pad: 0,
    }));

    p.encode(
        enc,
        &p.sinusoidal_embed,
        &[
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((dim + 255) / 256, 1, 1),
    );

    output
}

/// Noise schedule: compute sigma from timestep
pub fn noise_schedule(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    timesteps: &wgpu::Buffer,
    num_steps: u32,
    max_sigma: f32,
    schedule_type: u32, // 0=linear, 1=cosine, 2=flow_matching
) -> wgpu::Buffer {
    let output = p.alloc((num_steps as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        num_steps: u32,
        max_sigma: f32,
        schedule_type: u32,
        _pad: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        num_steps,
        max_sigma,
        schedule_type,
        _pad: 0,
    }));

    p.encode(
        enc,
        &p.noise_schedule,
        &[
            timesteps.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((num_steps + 255) / 256, 1, 1),
    );

    output
}

// ========================================================================
// Batch 7: Cross-attention + Flash attention
// ========================================================================

/// Cross-attention: Q from decoder, K/V from encoder
pub fn cross_attention(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    q: &wgpu::Buffer,
    k: &wgpu::Buffer,
    v: &wgpu::Buffer,
    num_heads: u32,
    head_dim: u32,
    src_seq: u32,
    scale: f32,
) -> wgpu::Buffer {
    let output_size = num_heads * head_dim;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        head_dim: u32,
        src_seq: u32,
        num_heads: u32,
        scale: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        head_dim,
        src_seq,
        num_heads,
        scale,
    }));

    p.encode(
        enc,
        &p.cross_attention,
        &[
            q.as_entire_binding(),
            k.as_entire_binding(),
            v.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (num_heads, 1, 1),
    );

    output
}

/// Flash attention: tiled attention with online softmax
pub fn flash_attention(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    q: &wgpu::Buffer,
    k: &wgpu::Buffer,
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
    struct Params {
        head_dim: u32,
        total_seq: u32,
        num_heads: u32,
        scale: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        head_dim,
        total_seq,
        num_heads,
        scale,
    }));

    p.encode(
        enc,
        &p.flash_attention,
        &[
            q.as_entire_binding(),
            k.as_entire_binding(),
            v.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (num_heads, 1, 1),
    );

    output
}

// ========================================================================
// Prepare variants — return (output_buffer, bind_group, workgroups)
// without dispatching. Caller batches into one compute pass.
// ========================================================================

/// Prepare embedding lookup
pub fn embed_prepare(
    p: &Pipelines,
    table: &wgpu::Buffer,
    token_ids: &wgpu::Buffer,
    hidden: u32,
    seq_len: u32,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output_size = seq_len * hidden;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        hidden: u32,
        seq_len: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params { hidden, seq_len }));

    let bg = p.create_bind_group(
        &p.embed,
        &[
            table.as_entire_binding(),
            token_ids.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );

    (output, bg, ((output_size + 255) / 256, 1, 1))
}

// ---- Pre-computed param variants ----

/// Q8 matmul with pre-computed params buffer
pub fn q8_matmul_prepare_precomputed(
    p: &Pipelines,
    activation: &wgpu::Buffer,
    packed_weights: &wgpu::Buffer,
    scales: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.q8_matmul,
        &[
            activation.as_entire_binding(),
            packed_weights.as_entire_binding(),
            scales.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// Q4 matmul with pre-computed params buffer
pub fn q4_matmul_prepare_precomputed(
    p: &Pipelines,
    activation: &wgpu::Buffer,
    packed_weights: &wgpu::Buffer,
    scales: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.q4_matmul,
        &[
            activation.as_entire_binding(),
            packed_weights.as_entire_binding(),
            scales.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// RMS norm with pre-computed params buffer
pub fn rms_norm_prepare_precomputed(
    p: &Pipelines,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    positions: u32,
    hidden: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((positions as u64) * (hidden as u64) * 4);
    let bg = p.create_bind_group(
        &p.rms_norm,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// Fused RMSNorm + Q4 matmul — single dispatch replaces norm + q4_matmul.
pub fn fused_norm_q4_prepare_precomputed(
    p: &Pipelines,
    input: &wgpu::Buffer,
    norm_weight: &wgpu::Buffer,
    packed_weights: &wgpu::Buffer,
    scales: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.fused_norm_q4,
        &[
            input.as_entire_binding(),
            norm_weight.as_entire_binding(),
            packed_weights.as_entire_binding(),
            scales.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// Fused skip connection + RMSNorm — single dispatch replaces add + norm.
pub fn fused_skip_norm_prepare_precomputed(
    p: &Pipelines,
    input: &wgpu::Buffer,
    skip: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    positions: u32,
    hidden: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let size = (positions as u64) * (hidden as u64) * 4;
    let normed_output = p.alloc(size);
    let skip_output = p.alloc(size);
    let bg = p.create_bind_group(
        &p.fused_skip_norm,
        &[
            input.as_entire_binding(),
            skip.as_entire_binding(),
            weight.as_entire_binding(),
            normed_output.as_entire_binding(),
            skip_output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (normed_output, skip_output, bg, wg)
}

/// RoPE with pre-computed params buffer
pub fn rope_prepare_precomputed(
    p: &Pipelines,
    input: &wgpu::Buffer,
    cos_cache: &wgpu::Buffer,
    sin_cache: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    total_elements: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((total_elements as u64) * 4);
    let bg = p.create_bind_group(
        &p.rope,
        &[
            input.as_entire_binding(),
            cos_cache.as_entire_binding(),
            sin_cache.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// f32 matmul with pre-computed params buffer
pub fn f32_matmul_prepare_precomputed(
    p: &Pipelines,
    activation: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.f32_matmul,
        &[
            activation.as_entire_binding(),
            weight.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// Argmax with pre-computed params buffer
pub fn argmax_gpu_prepare_precomputed(
    p: &Pipelines,
    input: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc(4);
    let bg = p.create_bind_group(
        &p.argmax,
        &[
            input.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

/// Prepare element-wise add
pub fn add_prepare(
    p: &Pipelines,
    a: &wgpu::Buffer,
    b: &wgpu::Buffer,
    n: u32,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.add,
        &[
            a.as_entire_binding(),
            b.as_entire_binding(),
            output.as_entire_binding(),
        ],
    );
    (output, bg, ((n + 255) / 256, 1, 1))
}

/// Prepare SiLU gate (SwiGLU)
pub fn silu_mul_prepare(
    p: &Pipelines,
    gate: &wgpu::Buffer,
    up: &wgpu::Buffer,
    n: u32,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.silu_mul,
        &[
            gate.as_entire_binding(),
            up.as_entire_binding(),
            output.as_entire_binding(),
        ],
    );
    (output, bg, ((n + 255) / 256, 1, 1))
}

/// Prepare attention decode
pub fn attention_decode_prepare(
    p: &Pipelines,
    q: &wgpu::Buffer,
    k: &wgpu::Buffer,
    v: &wgpu::Buffer,
    num_heads: u32,
    head_dim: u32,
    total_seq: u32,
    scale: f32,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output_size = num_heads * head_dim;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        head_dim: u32,
        total_seq: u32,
        num_heads: u32,
        scale: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        head_dim,
        total_seq,
        num_heads,
        scale,
    }));

    let bg = p.create_bind_group(
        &p.attention,
        &[
            q.as_entire_binding(),
            k.as_entire_binding(),
            v.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );

    (output, bg, (num_heads, 1, 1))
}

/// Prepare encoder attention (multi-position self-attention, no causal mask)
/// Q, K, V: [num_heads * seq_len * head_dim]
/// Output:  [num_heads * seq_len * head_dim]
/// Dispatch: (num_heads, seq_len, 1)
pub fn attention_encode_prepare(
    p: &Pipelines,
    q: &wgpu::Buffer,
    k: &wgpu::Buffer,
    v: &wgpu::Buffer,
    num_heads: u32,
    head_dim: u32,
    seq_len: u32,
    scale: f32,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output_size = num_heads * seq_len * head_dim;
    let output = p.alloc((output_size as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        head_dim: u32,
        total_seq: u32,  // = seq_len for encoder
        num_heads: u32,
        scale: f32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        head_dim,
        total_seq: seq_len,
        num_heads,
        scale,
    }));

    let bg = p.create_bind_group(
        &p.attention_encode,
        &[
            q.as_entire_binding(),
            k.as_entire_binding(),
            v.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );

    (output, bg, (num_heads, seq_len, 1))
}

/// Prepare KV head expansion
pub fn kv_expand_prepare(
    p: &Pipelines,
    kv_src: &wgpu::Buffer,
    kv_heads: u32,
    num_heads: u32,
    head_dim: u32,
    total_seq: u32,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let total_elements = num_heads * total_seq * head_dim;
    let output = p.alloc((total_elements as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        num_heads: u32,
        head_dim: u32,
        past_seq: u32,
        new_seq: u32,
        total_seq: u32,
        kv_heads: u32,
        _p0: u32,
        _p1: u32,
    }

    let params = Params {
        num_heads,
        head_dim,
        past_seq: 0,
        new_seq: 0,
        total_seq,
        kv_heads,
        _p0: 0,
        _p1: 0,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let bg = p.create_bind_group(
        &p.kv_expand,
        &[
            kv_src.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );

    (output, bg, ((total_elements + 255) / 256, 1, 1))
}

/// Prepare KV append into permanent buffer
pub fn kv_append_prepare_permanent(
    p: &Pipelines,
    past_kv: &wgpu::Buffer,
    new_kv: &wgpu::Buffer,
    num_heads: u32,
    head_dim: u32,
    past_seq: u32,
    new_seq: u32,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let total_seq = past_seq + new_seq;
    let total_elements = num_heads * total_seq * head_dim;
    let output = p.alloc_permanent((total_elements as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        num_heads: u32,
        head_dim: u32,
        past_seq: u32,
        new_seq: u32,
        total_seq: u32,
        kv_heads: u32,
        _p0: u32,
        _p1: u32,
    }

    let params = Params {
        num_heads,
        head_dim,
        past_seq,
        new_seq,
        total_seq,
        kv_heads: num_heads,
        _p0: 0,
        _p1: 0,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let bg = p.create_bind_group(
        &p.kv_append,
        &[
            past_kv.as_entire_binding(),
            new_kv.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );

    (output, bg, ((total_elements + 255) / 256, 1, 1))
}

// ========================================================================
// F16 matmul dispatch
// ========================================================================

/// F16 matmul: [1, K] f32 x [N, K] f16_packed -> [1, N] f32
/// Weights stored as packed f16 pairs in u32 array.
pub fn f16_matmul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    activation: &wgpu::Buffer,
    weight_packed: &wgpu::Buffer,
    n: u32,
    k: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        k: u32,
        k_half: u32,
        _pad: u32,
    }

    let params = Params {
        n,
        k,
        k_half: k / 2,
        _pad: 0,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let x = n.min(65535);
    let y = (n + x - 1) / x;
    p.encode(
        enc,
        &p.f16_matmul,
        &[
            activation.as_entire_binding(),
            weight_packed.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (x, y, 1),
    );

    output
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct F16MatmulParams {
    n: u32,
    k: u32,
    k_half: u32,
    _pad: u32,
}

#[allow(dead_code)]
fn precompute_f16_matmul(p: &Pipelines, n: u32, k: u32) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = F16MatmulParams {
        n,
        k,
        k_half: k / 2,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let x = n.min(65535);
    let y = (n + x - 1) / x;
    (buf, (x, y, 1))
}

/// F16 matmul with pre-computed params buffer
pub fn f16_matmul_prepare_precomputed(
    p: &Pipelines,
    activation: &wgpu::Buffer,
    weight_packed: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.f16_matmul,
        &[
            activation.as_entire_binding(),
            weight_packed.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

// ========================================================================
// Ternary (BitNet) matmul dispatch
// ========================================================================

/// Ternary matmul: [1, K] f32 x [N, K] ternary_packed + [N] scale -> [1, N] f32
/// Weights packed as 2-bit ternary values in u32 (16 values per u32).
pub fn ternary_matmul(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    activation: &wgpu::Buffer,
    weight_packed: &wgpu::Buffer,
    scale: &wgpu::Buffer,
    n: u32,
    k: u32,
) -> wgpu::Buffer {
    let output = p.alloc((n as u64) * 4);
    let u32s_per_row = (k + 15) / 16;

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        k: u32,
        u32s_per_row: u32,
        _pad: u32,
    }

    let params = Params {
        n,
        k,
        u32s_per_row,
        _pad: 0,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    let x = n.min(65535);
    let y = (n + x - 1) / x;
    p.encode(
        enc,
        &p.ternary_matmul,
        &[
            activation.as_entire_binding(),
            weight_packed.as_entire_binding(),
            scale.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        (x, y, 1),
    );

    output
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TernaryMatmulParams {
    n: u32,
    k: u32,
    u32s_per_row: u32,
    _pad: u32,
}

#[allow(dead_code)]
fn precompute_ternary_matmul(p: &Pipelines, n: u32, k: u32) -> (wgpu::Buffer, (u32, u32, u32)) {
    let params = TernaryMatmulParams {
        n,
        k,
        u32s_per_row: (k + 15) / 16,
        _pad: 0,
    };
    let buf = p.upload_uniform_permanent(bytemuck::bytes_of(&params));
    let x = n.min(65535);
    let y = (n + x - 1) / x;
    (buf, (x, y, 1))
}

/// Ternary matmul with pre-computed params buffer
pub fn ternary_matmul_prepare_precomputed(
    p: &Pipelines,
    activation: &wgpu::Buffer,
    weight_packed: &wgpu::Buffer,
    scale: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let output = p.alloc((n as u64) * 4);
    let bg = p.create_bind_group(
        &p.ternary_matmul,
        &[
            activation.as_entire_binding(),
            weight_packed.as_entire_binding(),
            scale.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );
    (output, bg, wg)
}

// ========================================================================
// Runtime quantize/dequantize dispatch
// ========================================================================

/// Quantize f32 tensor to Q4 packed format on GPU.
/// Returns (packed_u32_buffer, scales_buffer).
pub fn quantize_f32_to_q4(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
    block_size: u32,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let num_blocks = (n + block_size - 1) / block_size;
    let u32s_per_block = block_size / 8; // 8 nibbles per u32
    let packed_size = (num_blocks * u32s_per_block) as u64 * 4;
    let scales_size = (num_blocks as u64) * 4;

    let packed = p.alloc(packed_size);
    let scales = p.alloc(scales_size);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct QParams {
        n: u32,
        block_size: u32,
        num_blocks: u32,
        bits: u32,
    }

    let params = QParams {
        n,
        block_size,
        num_blocks,
        bits: 4,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    p.encode(
        enc,
        &p.quantize,
        &[
            input.as_entire_binding(),
            packed.as_entire_binding(),
            scales.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((num_blocks + 255) / 256, 1, 1),
    );

    (packed, scales)
}

/// Quantize f32 tensor to Q8 packed format on GPU.
/// Returns (packed_u32_buffer, scales_buffer).
pub fn quantize_f32_to_q8(
    p: &Pipelines,
    enc: &mut wgpu::CommandEncoder,
    input: &wgpu::Buffer,
    n: u32,
    block_size: u32,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let num_blocks = (n + block_size - 1) / block_size;
    let u32s_per_block = block_size / 4; // 4 int8 per u32
    let packed_size = (num_blocks * u32s_per_block) as u64 * 4;
    let scales_size = (num_blocks as u64) * 4;

    let packed = p.alloc(packed_size);
    let scales = p.alloc(scales_size);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct QParams {
        n: u32,
        block_size: u32,
        num_blocks: u32,
        bits: u32,
    }

    let params = QParams {
        n,
        block_size,
        num_blocks,
        bits: 8,
    };
    let params_buf = p.upload_uniform(bytemuck::bytes_of(&params));

    p.encode(
        enc,
        &p.quantize,
        &[
            input.as_entire_binding(),
            packed.as_entire_binding(),
            scales.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
        ((num_blocks + 255) / 256, 1, 1),
    );

    (packed, scales)
}

/// Dequantize Q4 packed + scales to f32 on CPU.
/// GPU dequant not needed (matmul shaders consume Q4 directly).
/// Provided for testing and format conversion.
pub fn dequantize_q4_to_f32(
    packed: &[u32],
    scales: &[f32],
    block_size: usize,
) -> Vec<f32> {
    let num_blocks = scales.len();
    let mut output = Vec::with_capacity(num_blocks * block_size);

    for b in 0..num_blocks {
        let scale = scales[b];
        let u32s_per_block = block_size / 8;
        let base = b * u32s_per_block;

        for j in 0..u32s_per_block {
            if base + j >= packed.len() {
                break;
            }
            let word = packed[base + j];
            for k in 0..8usize {
                let nibble = ((word >> (k as u32 * 4)) & 0xF) as i32;
                // Signed: subtract 8 for zero-point
                let val = (nibble as f32 - 8.0) * scale;
                output.push(val);
            }
        }
    }

    output
}

/// Dequantize Q8 packed + scales to f32 on CPU.
/// GPU dequant not needed (matmul shaders consume Q8 directly).
/// Provided for testing and format conversion.
pub fn dequantize_q8_to_f32(
    packed: &[u32],
    scales: &[f32],
    block_size: usize,
) -> Vec<f32> {
    let num_blocks = scales.len();
    let mut output = Vec::with_capacity(num_blocks * block_size);

    for b in 0..num_blocks {
        let scale = scales[b];
        let u32s_per_block = block_size / 4;
        let base = b * u32s_per_block;

        for j in 0..u32s_per_block {
            if base + j >= packed.len() {
                break;
            }
            let word = packed[base + j];
            for k in 0..4usize {
                let byte = ((word >> (k as u32 * 8)) & 0xFF) as u8;
                let val = (byte as i8 as f32) * scale;
                output.push(val);
            }
        }
    }

    output
}

// ========================================================================
// Unified matmul dispatch helper for per-tensor quantization
// ========================================================================

/// QuantFormat for per-tensor dispatch selection
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuantFormat {
    F32,
    F16,
    Q8,
    Q4,
    Q4K,
    Q6K,
    Ternary,
}

/// Prepare matmul dispatch for a given quant format (decode path).
/// Returns (output_buffer, bind_group, workgroups, shader_ref).
///
/// For Q4/Q8: uses packed weights + scales buffers.
/// For F32/F16: uses weight buffer (scales ignored).
/// For Ternary: uses packed weights + per-row scale buffer.
pub fn prepare_matmul_for_quant<'a>(
    p: &'a Pipelines,
    activation: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    scales: &wgpu::Buffer,
    params_buf: &wgpu::Buffer,
    n: u32,
    wg: (u32, u32, u32),
    quant: QuantFormat,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32), &'a super::pipelines::ComputeShader) {
    match quant {
        QuantFormat::Q4 => {
            let (buf, bg, wg) = q4_matmul_prepare_precomputed(p, activation, weight, scales, params_buf, n, wg);
            (buf, bg, wg, &p.q4_matmul)
        }
        QuantFormat::Q8 => {
            let (buf, bg, wg) = q8_matmul_prepare_precomputed(p, activation, weight, scales, params_buf, n, wg);
            (buf, bg, wg, &p.q8_matmul)
        }
        QuantFormat::F32 => {
            let (buf, bg, wg) = f32_matmul_prepare_precomputed(p, activation, weight, params_buf, n, wg);
            (buf, bg, wg, &p.f32_matmul)
        }
        QuantFormat::F16 => {
            let (buf, bg, wg) = f16_matmul_prepare_precomputed(p, activation, weight, params_buf, n, wg);
            (buf, bg, wg, &p.f16_matmul)
        }
        QuantFormat::Q4K => {
            // Q4_K: no separate scales buffer — everything is in the weight superblocks
            let (buf, bg, wg) = q4k_matmul_prepare_precomputed(p, activation, weight, params_buf, n, wg);
            (buf, bg, wg, &p.q4k_matmul)
        }
        QuantFormat::Q6K => {
            // Q6_K: no separate scales buffer — everything is in the weight superblocks
            let (buf, bg, wg) = q6k_matmul_prepare_precomputed(p, activation, weight, params_buf, n, wg);
            (buf, bg, wg, &p.q6k_matmul)
        }
        QuantFormat::Ternary => {
            let (buf, bg, wg) = ternary_matmul_prepare_precomputed(p, activation, weight, scales, params_buf, n, wg);
            (buf, bg, wg, &p.ternary_matmul)
        }
    }
}

// Note: precompute_matmul_for_quant is defined in model.rs where the
// per-format precompute functions (precompute_q4_matmul etc.) are in scope.

/// Prepare Conv1d dispatch — returns (output_buffer, bind_group, workgroups)
pub fn conv1d_prepare(
    p: &Pipelines,
    input: &wgpu::Buffer,
    weight: &wgpu::Buffer,
    bias: &wgpu::Buffer,
    batch_size: u32,
    in_channels: u32,
    out_channels: u32,
    in_length: u32,
    kernel_size: u32,
    stride: u32,
    padding: u32,
    groups: u32,
) -> (wgpu::Buffer, wgpu::BindGroup, (u32, u32, u32)) {
    let out_length = (in_length + 2 * padding - kernel_size) / stride + 1;
    let total_output = batch_size * out_channels * out_length;
    let output = p.alloc((total_output as u64) * 4);

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        in_channels: u32,
        out_channels: u32,
        in_length: u32,
        out_length: u32,
        kernel_size: u32,
        stride: u32,
        padding: u32,
        groups: u32,
        batch_size: u32,
        total_output: u32,
        _pad0: u32,
        _pad1: u32,
    }

    let params_buf = p.upload_uniform(bytemuck::bytes_of(&Params {
        in_channels,
        out_channels,
        in_length,
        out_length,
        kernel_size,
        stride,
        padding,
        groups,
        batch_size,
        total_output,
        _pad0: 0,
        _pad1: 0,
    }));

    let bg = p.create_bind_group(
        &p.conv1d,
        &[
            input.as_entire_binding(),
            weight.as_entire_binding(),
            bias.as_entire_binding(),
            output.as_entire_binding(),
            params_buf.as_entire_binding(),
        ],
    );

    let wg = ((total_output + 255) / 256, 1, 1);
    (output, bg, wg)
}
