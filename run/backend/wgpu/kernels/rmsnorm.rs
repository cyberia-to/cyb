//! RmsNorm over last dim.

use crate::backend::wgpu::device::{compute_pipeline, storage_ro, storage_rw, uniform, Device};

const SHADER: &str = r#"
struct Params {
    batch: u32,
    d: u32,
    eps: f32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> x: array<f32>;
@group(0) @binding(1) var<storage, read> g: array<f32>;
@group(0) @binding(2) var<storage, read_write> y: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> shared_sum: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let b = wg.x;
    let tid = lid.x;
    if (b >= params.batch) { return; }

    // Partial sum of squares
    var acc: f32 = 0.0;
    var j: u32 = tid;
    while (j < params.d) {
        let v = x[b * params.d + j];
        acc = acc + v * v;
        j = j + 256u;
    }
    shared_sum[tid] = acc;
    workgroupBarrier();

    // Tree reduction
    var stride: u32 = 128u;
    while (stride > 0u) {
        if (tid < stride) {
            shared_sum[tid] = shared_sum[tid] + shared_sum[tid + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    let mean_sq = shared_sum[0] / f32(params.d);
    let inv_rms = 1.0 / sqrt(mean_sq + params.eps);

    // Scale + gain
    j = tid;
    while (j < params.d) {
        y[b * params.d + j] = x[b * params.d + j] * inv_rms * g[j];
        j = j + 256u;
    }
}
"#;

pub fn dispatch(
    device: &Device,
    x: &wgpu::Buffer,
    g: &wgpu::Buffer,
    batch: u32,
    d: u32,
    eps: f32,
) -> wgpu::Buffer {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        batch: u32,
        d: u32,
        eps: f32,
        _pad: u32,
    }

    let (pipeline, layout) = compute_pipeline(
        &device.device,
        SHADER,
        &[storage_ro(0), storage_ro(1), storage_rw(2), uniform(3)],
    );

    let out = device.alloc_f32((batch * d) as usize);
    let params = Params {
        batch,
        d,
        eps,
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
                resource: g.as_entire_binding(),
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
        pass.dispatch_workgroups(batch, 1, 1);
    }
    device.queue.submit(std::iter::once(enc.finish()));
    out
}
