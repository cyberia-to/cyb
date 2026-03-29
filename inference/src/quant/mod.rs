pub mod q4_kernel;
pub mod q4_launch;

use std::collections::HashMap;
use std::sync::Arc;
use wgpu;
use wgpu::util::DeviceExt;

/// Q4 VecMat compute pipeline — custom WGSL shader for
/// quantized 4-bit vector × matrix multiply on GPU
///
/// Eliminates:
/// 1. CPU-side dequantization
/// 2. f32 weight storage (4x memory reduction)
/// 3. Separate dequant + matmul kernel launches
pub struct Q4Compute {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Q4Params {
    n: u32,
    k: u32,
    block_size: u32,
    num_blocks: u32,
}

/// Cached weight buffers on GPU (packed Q4 + scales)
pub struct Q4WeightBuffer {
    pub packed_buf: wgpu::Buffer,  // u32-packed weights
    pub scales_buf: wgpu::Buffer,  // f32 scales
    pub n: usize,
    pub k: usize,
    pub block_size: usize,
}

impl Q4Compute {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let shader_source = include_str!("q4_vecmat.wgsl");
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Q4 VecMat Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Q4 Bind Group Layout"),
            entries: &[
                // activation: storage<read>
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // weights_packed: storage<read>
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // scales: storage<read>
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // output: storage<read_write>
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params: uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Q4 Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Q4 VecMat Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self { device, queue, pipeline, bind_group_layout }
    }

    /// Upload packed Q4 weights to GPU buffer
    /// packed_bytes: raw uint8 weights [N * num_blocks * block_size/2]
    /// scales: f32 per-block scales [N * num_blocks]
    pub fn upload_weights(
        &self,
        packed_bytes: &[u8],
        scales: &[f32],
        n: usize,
        k: usize,
        block_size: usize,
    ) -> Q4WeightBuffer {
        // Pack bytes into u32 for GPU (4 bytes per u32)
        let mut packed_u32s: Vec<u32> = Vec::with_capacity((packed_bytes.len() + 3) / 4);
        for chunk in packed_bytes.chunks(4) {
            let mut val: u32 = 0;
            for (i, &byte) in chunk.iter().enumerate() {
                val |= (byte as u32) << (i * 8);
            }
            packed_u32s.push(val);
        }

        let packed_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Q4 Packed Weights"),
            contents: bytemuck::cast_slice(&packed_u32s),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let scales_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Q4 Scales"),
            contents: bytemuck::cast_slice(scales),
            usage: wgpu::BufferUsages::STORAGE,
        });

        Q4WeightBuffer { packed_buf, scales_buf, n, k, block_size }
    }

    /// Execute Q4 vector × matrix: [1, K] × [N, K]^T → [1, N]
    /// activation: f32 data [K]
    /// Returns f32 output [N]
    ///
    /// TODO: zero-copy version that takes wgpu::Buffer directly
    /// to avoid GPU→CPU→GPU roundtrip for activation
    pub fn vecmat(&self, activation: &[f32], weight: &Q4WeightBuffer) -> Vec<f32> {
        let n = weight.n;
        let k = weight.k;
        let block_size = weight.block_size;
        let num_blocks = k / block_size;

        // Upload activation
        let act_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Q4 Activation"),
            contents: bytemuck::cast_slice(activation),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Output buffer
        let output_size = (n * std::mem::size_of::<f32>()) as u64;
        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Q4 Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Staging buffer for readback
        let staging_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Q4 Staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Params
        let params = Q4Params {
            n: n as u32,
            k: k as u32,
            block_size: block_size as u32,
            num_blocks: num_blocks as u32,
        };
        let params_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Q4 Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Q4 Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: act_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: weight.packed_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: weight.scales_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: output_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: params_buf.as_entire_binding() },
            ],
        });

        // Dispatch
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Q4 Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Q4 Compute Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(n as u32, 1, 1); // one workgroup per output row
        }

        // Copy output to staging
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);

        self.queue.submit(std::iter::once(encoder.finish()));

        // Read back
        let slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        result
    }
}

/// Global Q4 compute instance
static Q4_INSTANCE: std::sync::OnceLock<Q4Compute> = std::sync::OnceLock::new();
pub static Q4_WEIGHTS: std::sync::LazyLock<std::sync::Mutex<HashMap<String, Q4WeightBuffer>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Initialize the Q4 compute pipeline (call once at startup)
pub fn init_q4_compute() {
    if Q4_INSTANCE.get().is_some() { return; }

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    })).expect("No GPU adapter found");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Q4 Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
        },
        None,
    )).expect("Failed to create GPU device");

    let q4 = Q4Compute::new(Arc::new(device), Arc::new(queue));
    let _ = Q4_INSTANCE.set(q4);
    log::info!("Q4 compute pipeline initialized on GPU: {}", adapter.get_info().name);
}

/// Get the global Q4 compute instance
pub fn get_q4_compute() -> &'static Q4Compute {
    Q4_INSTANCE.get().expect("Q4 compute not initialized — call init_q4_compute() first")
}
