//! Compute pipeline cache — creates and caches wgpu pipelines for each shader

use std::cell::RefCell;
use std::sync::Arc;

use wgpu;
use wgpu::util::DeviceExt;

use super::alloc::FrameAllocator;

/// All compute pipelines for inference
pub struct Pipelines {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,

    // Shader pipelines
    pub q4_matmul: ComputeShader,
    pub q8_matmul: ComputeShader,
    pub rms_norm: ComputeShader,
    pub layernorm: ComputeShader,
    pub rope: ComputeShader,
    pub attention: ComputeShader,
    pub attention_encode: ComputeShader,
    pub add: ComputeShader,
    pub add_broadcast: ComputeShader,
    pub mul: ComputeShader,
    pub silu_mul: ComputeShader,
    pub gelu: ComputeShader,
    pub embed: ComputeShader,
    pub f32_matmul: ComputeShader,
    pub argmax: ComputeShader,
    pub fused_norm_q4: ComputeShader,
    pub fused_skip_norm: ComputeShader,
    pub kv_append: ComputeShader,
    pub kv_expand: ComputeShader,

    // --- Batch 1: Trivial element-wise ---
    pub sub: ComputeShader,
    pub div: ComputeShader,
    pub relu: ComputeShader,
    pub leaky_relu: ComputeShader,
    pub tanh_act: ComputeShader,
    pub clamp: ComputeShader,
    pub abs_op: ComputeShader,
    pub neg: ComputeShader,
    pub sqrt_op: ComputeShader,
    pub exp_op: ComputeShader,
    pub sigmoid: ComputeShader,

    // --- Batch 2: Compound activations ---
    pub geglu: ComputeShader,
    pub swiglu: ComputeShader,
    pub glu: ComputeShader,
    pub prelu: ComputeShader,

    // --- Batch 3: Normalization ---
    pub batchnorm: ComputeShader,
    pub groupnorm: ComputeShader,
    pub instance_norm: ComputeShader,
    pub adaln: ComputeShader,

    // --- Batch 4: Convolution ---
    pub conv2d: ComputeShader,
    pub conv1d: ComputeShader,
    pub depthwise_conv: ComputeShader,
    pub pool: ComputeShader,

    // --- Batch 5: Spatial ---
    pub interpolate_nearest: ComputeShader,
    pub interpolate_bilinear: ComputeShader,
    pub pixel_shuffle: ComputeShader,
    pub pixel_unshuffle: ComputeShader,
    pub patch_embed: ComputeShader,

    // --- Batch 6: Special ---
    pub sinusoidal_embed: ComputeShader,
    pub noise_schedule: ComputeShader,

    // --- Batch 7: Attention variants ---
    pub cross_attention: ComputeShader,
    pub flash_attention: ComputeShader,

    // --- Batch 8: Additional matmul variants ---
    pub f16_matmul: ComputeShader,
    pub ternary_matmul: ComputeShader,

    // --- Batch 9: Runtime quantization ---
    pub quantize: ComputeShader,

    /// Frame allocator for zero-allocation decode after warmup
    pub frame_alloc: RefCell<FrameAllocator>,
}

pub struct ComputeShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl Pipelines {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let q4_matmul = create_pipeline(
            &device,
            include_str!("shaders/q4_matmul.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let q8_matmul = create_pipeline(
            &device,
            include_str!("shaders/q8_matmul.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let rms_norm = create_pipeline(
            &device,
            include_str!("shaders/rms_norm.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let layernorm = create_pipeline(
            &device,
            include_str!("shaders/layernorm.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let rope = create_pipeline(
            &device,
            include_str!("shaders/rope.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let attention = create_pipeline(
            &device,
            include_str!("shaders/attention.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let attention_encode = create_pipeline(
            &device,
            include_str!("shaders/attention.wgsl"),
            "attention_encode",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let add = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "add_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );
        let add_broadcast = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "add_broadcast_kernel",
            &[storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let mul = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "mul_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );
        let silu_mul = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "silu_mul_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );
        let gelu = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "gelu_kernel",
            &[storage_ro(), storage_rw()],
        );
        let embed = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "embed_kernel",
            &[storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let f32_matmul = create_pipeline(
            &device,
            include_str!("shaders/f32_matmul.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let argmax = create_pipeline(
            &device,
            include_str!("shaders/argmax.wgsl"),
            "main",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let kv_append = create_pipeline(
            &device,
            include_str!("shaders/kv_cache.wgsl"),
            "kv_append",
            &[storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let kv_expand = create_pipeline(
            &device,
            include_str!("shaders/kv_cache.wgsl"),
            "kv_expand",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let fused_norm_q4 = create_pipeline(
            &device,
            include_str!("shaders/fused_norm_q4.wgsl"),
            "main",
            &[
                storage_ro(),
                storage_ro(),
                storage_ro(),
                storage_ro(),
                storage_rw(),
                uniform(),
            ],
        );
        let fused_skip_norm = create_pipeline(
            &device,
            include_str!("shaders/fused_skip_norm.wgsl"),
            "main",
            &[
                storage_ro(),
                storage_ro(),
                storage_ro(),
                storage_rw(),
                storage_rw(),
                uniform(),
            ],
        );

        // --- Batch 1: Trivial element-wise ---
        let sub = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "sub_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );
        let div = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "div_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );
        let relu = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "relu_kernel",
            &[storage_ro(), storage_rw()],
        );
        let leaky_relu = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "leaky_relu_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let tanh_act = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "tanh_kernel",
            &[storage_ro(), storage_rw()],
        );
        let clamp = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "clamp_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let abs_op = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "abs_kernel",
            &[storage_ro(), storage_rw()],
        );
        let neg = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "neg_kernel",
            &[storage_ro(), storage_rw()],
        );
        let sqrt_op = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "sqrt_kernel",
            &[storage_ro(), storage_rw()],
        );
        let exp_op = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "exp_kernel",
            &[storage_ro(), storage_rw()],
        );
        let sigmoid = create_pipeline(
            &device,
            include_str!("shaders/elementwise.wgsl"),
            "sigmoid_kernel",
            &[storage_ro(), storage_rw()],
        );

        // --- Batch 2: Compound activations ---
        let geglu = create_pipeline(
            &device,
            include_str!("shaders/activations.wgsl"),
            "geglu_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );
        let swiglu = create_pipeline(
            &device,
            include_str!("shaders/activations.wgsl"),
            "swiglu_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );
        let glu = create_pipeline(
            &device,
            include_str!("shaders/activations.wgsl"),
            "glu_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );
        let prelu = create_pipeline(
            &device,
            include_str!("shaders/activations.wgsl"),
            "prelu_kernel",
            &[storage_ro(), storage_ro(), storage_rw()],
        );

        // --- Batch 3: Normalization ---
        let batchnorm = create_pipeline(
            &device,
            include_str!("shaders/norm.wgsl"),
            "batchnorm_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let groupnorm = create_pipeline(
            &device,
            include_str!("shaders/norm.wgsl"),
            "groupnorm_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let instance_norm = create_pipeline(
            &device,
            include_str!("shaders/norm.wgsl"),
            "instance_norm_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let adaln = create_pipeline(
            &device,
            include_str!("shaders/norm.wgsl"),
            "adaln_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );

        // --- Batch 4: Convolution ---
        let conv2d = create_pipeline(
            &device,
            include_str!("shaders/conv.wgsl"),
            "conv2d_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let conv1d = create_pipeline(
            &device,
            include_str!("shaders/conv.wgsl"),
            "conv1d_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let depthwise_conv = create_pipeline(
            &device,
            include_str!("shaders/conv.wgsl"),
            "depthwise_conv_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let pool = create_pipeline(
            &device,
            include_str!("shaders/conv.wgsl"),
            "pool_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );

        // --- Batch 5: Spatial ---
        let interpolate_nearest = create_pipeline(
            &device,
            include_str!("shaders/spatial.wgsl"),
            "interpolate_nearest_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let interpolate_bilinear = create_pipeline(
            &device,
            include_str!("shaders/spatial.wgsl"),
            "interpolate_bilinear_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let pixel_shuffle = create_pipeline(
            &device,
            include_str!("shaders/spatial.wgsl"),
            "pixel_shuffle_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let pixel_unshuffle = create_pipeline(
            &device,
            include_str!("shaders/spatial.wgsl"),
            "pixel_unshuffle_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );
        let patch_embed = create_pipeline(
            &device,
            include_str!("shaders/spatial.wgsl"),
            "patch_embed_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );

        // --- Batch 6: Special ---
        let sinusoidal_embed = create_pipeline(
            &device,
            include_str!("shaders/special.wgsl"),
            "sinusoidal_embed_kernel",
            &[storage_rw(), uniform()],
        );
        let noise_schedule = create_pipeline(
            &device,
            include_str!("shaders/special.wgsl"),
            "noise_schedule_kernel",
            &[storage_ro(), storage_rw(), uniform()],
        );

        // --- Batch 7: Attention variants ---
        let cross_attention = create_pipeline(
            &device,
            include_str!("shaders/special.wgsl"),
            "cross_attention_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let flash_attention = create_pipeline(
            &device,
            include_str!("shaders/special.wgsl"),
            "flash_attention_kernel",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );

        // --- Batch 8: Additional matmul variants ---
        let f16_matmul = create_pipeline(
            &device,
            include_str!("shaders/f16_matmul.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_rw(), uniform()],
        );
        let ternary_matmul = create_pipeline(
            &device,
            include_str!("shaders/ternary_matmul.wgsl"),
            "main",
            &[storage_ro(), storage_ro(), storage_ro(), storage_rw(), uniform()],
        );

        // --- Batch 9: Runtime quantization ---
        let quantize = create_pipeline(
            &device,
            include_str!("shaders/special.wgsl"),
            "quantize_kernel",
            &[storage_ro(), storage_rw(), storage_rw(), uniform()],
        );

        let frame_alloc = RefCell::new(FrameAllocator::new(device.clone()));

        Self {
            device,
            queue,
            q4_matmul,
            q8_matmul,
            rms_norm,
            layernorm,
            rope,
            attention,
            attention_encode,
            add,
            add_broadcast,
            mul,
            silu_mul,
            gelu,
            embed,
            f32_matmul,
            argmax,
            fused_norm_q4,
            fused_skip_norm,
            kv_append,
            kv_expand,
            // Batch 1
            sub,
            div,
            relu,
            leaky_relu,
            tanh_act,
            clamp,
            abs_op,
            neg,
            sqrt_op,
            exp_op,
            sigmoid,
            // Batch 2
            geglu,
            swiglu,
            glu,
            prelu,
            // Batch 3
            batchnorm,
            groupnorm,
            instance_norm,
            adaln,
            // Batch 4
            conv2d,
            conv1d,
            depthwise_conv,
            pool,
            // Batch 5
            interpolate_nearest,
            interpolate_bilinear,
            pixel_shuffle,
            pixel_unshuffle,
            patch_embed,
            // Batch 6
            sinusoidal_embed,
            noise_schedule,
            // Batch 7
            cross_attention,
            flash_attention,
            // Batch 8
            f16_matmul,
            ternary_matmul,
            // Batch 9
            quantize,
            frame_alloc,
        }
    }

    /// Create a GPU buffer from f32 data
    pub fn upload_f32(&self, data: &[f32]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
    }

    /// Upload raw bytes as a storage buffer
    pub fn upload_bytes(&self, data: &[u8]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: data,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
    }

    /// Create a GPU buffer from u32 data
    pub fn upload_u32(&self, data: &[u32]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
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
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
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

    /// Create a bind group for a shader
    pub fn create_bind_group(
        &self,
        shader: &ComputeShader,
        bindings: &[wgpu::BindingResource],
    ) -> wgpu::BindGroup {
        let entries: Vec<wgpu::BindGroupEntry> = bindings
            .iter()
            .enumerate()
            .map(|(i, r)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: r.clone(),
            })
            .collect();

        self.device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &shader.bind_group_layout,
                entries: &entries,
            })
    }

    /// Add a compute pass to an existing encoder
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
}

// Helper functions for bind group layout entries
fn storage_ro() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
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

    let entries: Vec<wgpu::BindGroupLayoutEntry> = layout_entries
        .iter()
        .enumerate()
        .map(|(i, e)| wgpu::BindGroupLayoutEntry {
            binding: i as u32,
            ..e.clone()
        })
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

    ComputeShader {
        pipeline,
        bind_group_layout,
    }
}
