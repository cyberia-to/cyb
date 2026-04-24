//! RoPE (NeoX pairing) — ops.md §3.

use crate::backend::wgpu::device::{compute_pipeline, storage_ro, storage_rw, uniform, Device};

const SHADER: &str = r#"
struct Params {
    total: u32,        // total scalar elements
    head_dim: u32,
    heads_per_pos: u32,
    base: f32,
}

@group(0) @binding(0) var<storage, read> x: array<f32>;
@group(0) @binding(1) var<storage, read> pos: array<f32>;
@group(0) @binding(2) var<storage, read_write> y: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.total) { return; }

    let half = params.head_dim / 2u;
    let head_idx = idx / params.head_dim;
    let d = idx % params.head_dim;
    let p_idx = head_idx / params.heads_per_pos;
    let p = pos[p_idx];

    // j is dim index within half
    let j = select(d - half, d, d < half);
    let theta = p / pow(params.base, 2.0 * f32(j) / f32(params.head_dim));
    let c = cos(theta);
    let s = sin(theta);

    let base_addr = head_idx * params.head_dim;
    let x_lo = x[base_addr + j];
    let x_hi = x[base_addr + j + half];

    if (d < half) {
        y[idx] = x_lo * c - x_hi * s;
    } else {
        y[idx] = x_lo * s + x_hi * c;
    }
}
"#;

pub fn dispatch(
    device: &Device,
    x: &wgpu::Buffer,
    pos: &wgpu::Buffer,
    total: u32,
    head_dim: u32,
    pos_len: u32,
    base: f32,
) -> wgpu::Buffer {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        total: u32,
        head_dim: u32,
        heads_per_pos: u32,
        base: f32,
    }

    let (pipeline, layout) = compute_pipeline(
        &device.device,
        SHADER,
        &[storage_ro(0), storage_ro(1), storage_rw(2), uniform(3)],
    );
    let out = device.alloc_f32(total as usize);
    let n = total / head_dim;
    let heads_per_pos = n / pos_len.max(1);
    let params = Params {
        total,
        head_dim,
        heads_per_pos,
        base,
    };
    let params_buf = device.upload_uniform(bytemuck::bytes_of(&params));

    let bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: x.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: pos.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: out.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buf.as_entire_binding(),
            },
        ],
    });
    let mut enc = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups((total + 63) / 64, 1, 1);
    }
    device.queue.submit(std::iter::once(enc.finish()));
    out
}
