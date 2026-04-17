//! Silu: y = x * sigmoid(x)

use crate::wgpu_rs::device::{compute_pipeline, storage_ro, storage_rw, uniform, Device};

const SHADER: &str = r#"
struct Params { n: u32, _p0: u32, _p1: u32, _p2: u32 }

@group(0) @binding(0) var<storage, read> x: array<f32>;
@group(0) @binding(1) var<storage, read_write> y: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.n) { return; }
    let v = x[idx];
    y[idx] = v / (1.0 + exp(-v));
}
"#;

pub fn dispatch(device: &Device, x: &wgpu::Buffer, n: u32) -> wgpu::Buffer {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        _p0: u32,
        _p1: u32,
        _p2: u32,
    }
    let (pipeline, layout) = compute_pipeline(
        &device.device,
        SHADER,
        &[storage_ro(0), storage_rw(1), uniform(2)],
    );
    let out = device.alloc_f32(n as usize);
    let params = Params {
        n,
        _p0: 0,
        _p1: 0,
        _p2: 0,
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
                resource: out.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
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
        pass.dispatch_workgroups((n + 255) / 256, 1, 1);
    }
    device.queue.submit(std::iter::once(enc.finish()));
    out
}
