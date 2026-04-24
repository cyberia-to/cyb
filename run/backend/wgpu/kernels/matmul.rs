//! Matmul: y = x @ W^T  (x: [B, K], W: [N, K], y: [B, N])

use crate::backend::wgpu::device::{compute_pipeline, storage_ro, storage_rw, uniform, Device};

const SHADER: &str = r#"
struct Params {
    batch: u32,
    n: u32,
    k: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> x: array<f32>;
@group(0) @binding(1) var<storage, read> w: array<f32>;
@group(0) @binding(2) var<storage, read_write> y: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total = params.batch * params.n;
    let idx = gid.x;
    if (idx >= total) { return; }
    let b = idx / params.n;
    let i = idx % params.n;
    var acc: f32 = 0.0;
    for (var j: u32 = 0u; j < params.k; j = j + 1u) {
        acc = acc + x[b * params.k + j] * w[i * params.k + j];
    }
    y[idx] = acc;
}
"#;

pub fn dispatch(
    device: &Device,
    x: &wgpu::Buffer,
    w: &wgpu::Buffer,
    batch: u32,
    n: u32,
    k: u32,
) -> wgpu::Buffer {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        batch: u32,
        n: u32,
        k: u32,
        _pad: u32,
    }

    let (pipeline, layout) = compute_pipeline(
        &device.device,
        SHADER,
        &[storage_ro(0), storage_ro(1), storage_rw(2), uniform(3)],
    );

    let out = device.alloc_f32((batch * n) as usize);
    let params = Params {
        batch,
        n,
        k,
        _pad: 0,
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
                resource: w.as_entire_binding(),
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
        let total = batch * n;
        pass.dispatch_workgroups((total + 63) / 64, 1, 1);
    }
    device.queue.submit(std::iter::once(enc.finish()));
    out
}
