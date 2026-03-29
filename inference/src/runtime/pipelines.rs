//! Compute pipeline cache — creates and caches wgpu pipelines for each shader

use std::collections::HashMap;
use std::cell::RefCell;
use wgpu;
use wgpu::util::DeviceExt;
use std::sync::Arc;

use super::tensor::GpuTensor;
use super::alloc::FrameAllocator;

/// All compute pipelines for inference
pub struct Pipelines {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,

    // Shader pipelines
    pub q4_matmul: ComputeShader,
    pub rms_norm: ComputeShader,
    pub rope: ComputeShader,
    pub attention: ComputeShader,
    pub add: ComputeShader,
    pub mul: ComputeShader,
    pub silu_mul: ComputeShader,
    pub embed: ComputeShader,
    pub f32_matmul: ComputeShader,
    pub argmax: ComputeShader,
    pub fused_norm_q4: ComputeShader,
    pub fused_skip_norm: ComputeShader,
    pub kv_append: ComputeShader,
    pub kv_expand: ComputeShader,

    /// Frame allocator for zero-allocation decode after warmup
    pub frame_alloc: RefCell<FrameAllocator>,
}

pub struct ComputeShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl Pipelines {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let q4_matmul = create_pipeline(&device, include_str!("shaders/q4_matmul.wgsl"), "main", &[
            storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform(),
        ]);
        let rms_norm = create_pipeline(&device, include_str!("shaders/rms_norm.wgsl"), "main", &[
            storage_ro(), storage_ro(), storage_rw(), uniform(),
        ]);
        let rope = create_pipeline(&device, include_str!("shaders/rope.wgsl"), "main", &[
            storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform(),
        ]);
        let attention = create_pipeline(&device, include_str!("shaders/attention.wgsl"), "main", &[
            storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform(),
        ]);
        let add = create_pipeline(&device, include_str!("shaders/elementwise.wgsl"), "add_kernel", &[
            storage_ro(), storage_ro(), storage_rw(),
        ]);
        let mul = create_pipeline(&device, include_str!("shaders/elementwise.wgsl"), "mul_kernel", &[
            storage_ro(), storage_ro(), storage_rw(),
        ]);
        let silu_mul = create_pipeline(&device, include_str!("shaders/elementwise.wgsl"), "silu_mul_kernel", &[
            storage_ro(), storage_ro(), storage_rw(),
        ]);
        let embed = create_pipeline(&device, include_str!("shaders/elementwise.wgsl"), "embed_kernel", &[
            storage_ro(), storage_ro(), storage_rw(), uniform(),
        ]);

        let f32_matmul = create_pipeline(&device, include_str!("shaders/f32_matmul.wgsl"), "main", &[
            storage_ro(), storage_ro(), storage_rw(), uniform(),
        ]);

        let argmax = create_pipeline(&device, include_str!("shaders/argmax.wgsl"), "main", &[
            storage_ro(), storage_rw(), uniform(),
        ]);

        let kv_append = create_pipeline(&device, include_str!("shaders/kv_cache.wgsl"), "kv_append", &[
            storage_ro(), storage_ro(), storage_rw(), uniform(),
        ]);
        let kv_expand = create_pipeline(&device, include_str!("shaders/kv_cache.wgsl"), "kv_expand", &[
            storage_ro(), storage_rw(), uniform(),
        ]);

        let fused_norm_q4 = create_pipeline(&device, include_str!("shaders/fused_norm_q4.wgsl"), "main", &[
            storage_ro(), storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform(),
        ]);
        let fused_skip_norm = create_pipeline(&device, include_str!("shaders/fused_skip_norm.wgsl"), "main", &[
            storage_ro(), storage_ro(), storage_ro(), storage_rw(), storage_rw(), uniform(),
        ]);

        let frame_alloc = RefCell::new(FrameAllocator::new(device.clone()));

        Self { device, queue, q4_matmul, rms_norm, rope, attention, add, mul, silu_mul, embed, f32_matmul, argmax, fused_norm_q4, fused_skip_norm, kv_append, kv_expand, frame_alloc }
    }

    /// Create a GPU buffer from f32 data
    pub fn upload_f32(&self, data: &[f32]) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        })
    }

    /// Create a GPU buffer from u32 data
    pub fn upload_u32(&self, data: &[u32]) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        })
    }

    /// Create a uniform buffer (reused from pool after first step)
    pub fn upload_uniform(&self, data: &[u8]) -> wgpu::Buffer {
        let buf = self.frame_alloc.borrow_mut().alloc_uniform(data.len() as u64);
        self.queue.write_buffer(&buf, 0, data);
        buf
    }

    /// Allocate storage buffer (reused from pool after first step)
    pub fn alloc(&self, size_bytes: u64) -> wgpu::Buffer {
        self.frame_alloc.borrow_mut().alloc_storage(size_bytes)
    }

    /// Create a permanent uniform buffer (NOT pooled — for pre-computed params)
    pub fn upload_uniform_permanent(&self, data: &[u8]) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: data,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    /// Allocate permanent storage buffer (NOT pooled — for model weights)
    pub fn alloc_permanent(&self, size_bytes: u64) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Reset frame allocator — call at start of each forward pass
    pub fn begin_frame(&self) {
        self.frame_alloc.borrow_mut().reset();
    }

    /// Read buffer to CPU (blocking — forces GPU sync)
    pub fn read_f32(&self, buffer: &wgpu::Buffer, count: usize) -> Vec<f32> {
        let size = (count * 4) as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { tx.send(r).unwrap(); });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();
        result
    }

    /// Add a dispatch to an existing compute pass (ZERO pass overhead!)
    pub fn dispatch_in_pass<'a>(
        &self,
        pass: &mut wgpu::ComputePass<'a>,
        shader: &ComputeShader,
        bind_group: &'a wgpu::BindGroup,
        workgroups: (u32, u32, u32),
    ) {
        pass.set_pipeline(&shader.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
    }

    /// Create a bind group for a shader (call before starting compute pass)
    pub fn create_bind_group(
        &self,
        shader: &ComputeShader,
        bindings: &[wgpu::BindingResource],
    ) -> wgpu::BindGroup {
        let entries: Vec<wgpu::BindGroupEntry> = bindings.iter().enumerate()
            .map(|(i, r)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: r.clone(),
            })
            .collect();

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &shader.bind_group_layout,
            entries: &entries,
        })
    }

    /// Add a compute pass to an existing encoder (legacy — one pass per dispatch)
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        shader: &ComputeShader,
        bindings: &[wgpu::BindingResource],
        workgroups: (u32, u32, u32),
    ) {
        let bind_group = self.create_bind_group(shader, bindings);
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&shader.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
    }

    /// Execute a single compute pass with bind group (creates own encoder + submit)
    pub fn dispatch(
        &self,
        shader: &ComputeShader,
        bindings: &[wgpu::BindingResource],
        workgroups: (u32, u32, u32),
    ) {
        let entries: Vec<wgpu::BindGroupEntry> = bindings.iter().enumerate()
            .map(|(i, r)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: r.clone(),
            })
            .collect();

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &shader.bind_group_layout,
            entries: &entries,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&shader.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Batch multiple dispatches into one command buffer submission
    pub fn dispatch_batch(&self, ops: &[(&ComputeShader, Vec<wgpu::BindingResource>, (u32, u32, u32))]) {
        let mut encoder = self.device.create_command_encoder(&Default::default());

        for (shader, bindings, workgroups) in ops {
            let entries: Vec<wgpu::BindGroupEntry> = bindings.iter().enumerate()
                .map(|(i, r)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: r.clone(),
                })
                .collect();

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &shader.bind_group_layout,
                entries: &entries,
            });

            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&shader.pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

// Helper functions for bind group layout entries
fn storage_ro() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0, // will be overridden
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn storage_rw() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    source: &str,
    entry_point: &str,
    layout_entries: &[wgpu::BindGroupLayoutEntry],
) -> ComputeShader {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(entry_point),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });

    let entries: Vec<wgpu::BindGroupLayoutEntry> = layout_entries.iter().enumerate()
        .map(|(i, e)| wgpu::BindGroupLayoutEntry { binding: i as u32, ..e.clone() })
        .collect();

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &entries,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry_point),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        cache: None,
    });

    ComputeShader { pipeline, bind_group_layout }
}
