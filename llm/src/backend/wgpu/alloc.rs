//! GPU Frame Allocator — zero-allocation decode after first step
//! Reuses wgpu::Buffers between forward passes. No Metal API calls after warmup.

use std::collections::HashMap;
use std::sync::Arc;

/// Frame-based GPU buffer allocator
/// At the start of each forward pass, all previous buffers become available for reuse.
/// After the first step, zero new GPU allocations occur.
pub struct FrameAllocator {
    device: Arc<wgpu::Device>,
    /// Available STORAGE buffers grouped by size
    storage_pools: HashMap<u64, Vec<wgpu::Buffer>>,
    /// Available UNIFORM buffers grouped by size
    uniform_pools: HashMap<u64, Vec<wgpu::Buffer>>,
    /// Buffers allocated this frame (will be recycled on reset)
    /// (size, buffer, is_uniform)
    in_use: Vec<(u64, wgpu::Buffer, bool)>,
    /// Stats
    pub reused: usize,
    pub allocated: usize,
}

impl FrameAllocator {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            device,
            storage_pools: HashMap::new(),
            uniform_pools: HashMap::new(),
            in_use: Vec::with_capacity(1024),
            reused: 0,
            allocated: 0,
        }
    }

    /// Allocate a storage buffer. Reuses existing buffer if available.
    pub fn alloc_storage(&mut self, size: u64) -> wgpu::Buffer {
        if let Some(buffers) = self.storage_pools.get_mut(&size) {
            if let Some(buf) = buffers.pop() {
                self.reused += 1;
                self.in_use.push((size, buf.clone(), false));
                return buf;
            }
        }

        self.allocated += 1;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.in_use.push((size, buf.clone(), false));
        buf
    }

    /// Allocate a uniform buffer (small, for params).
    pub fn alloc_uniform(&mut self, size: u64) -> wgpu::Buffer {
        let aligned = (size + 15) & !15;
        if let Some(buffers) = self.uniform_pools.get_mut(&aligned) {
            if let Some(buf) = buffers.pop() {
                self.reused += 1;
                self.in_use.push((aligned, buf.clone(), true));
                return buf;
            }
        }

        self.allocated += 1;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: aligned,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.in_use.push((aligned, buf.clone(), true));
        buf
    }

    /// Reset: all buffers from previous frame become available for reuse
    pub fn reset(&mut self) {
        for (size, buf, is_uniform) in self.in_use.drain(..) {
            if is_uniform {
                self.uniform_pools.entry(size).or_default().push(buf);
            } else {
                self.storage_pools.entry(size).or_default().push(buf);
            }
        }
    }

    /// Log stats and reset counters
    pub fn log_stats(&mut self) {
        if self.allocated > 0 {
            log::debug!("Frame alloc: {} reused, {} new", self.reused, self.allocated);
        }
        self.reused = 0;
        self.allocated = 0;
    }
}
