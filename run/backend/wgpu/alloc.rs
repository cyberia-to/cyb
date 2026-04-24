//! Frame-based GPU buffer pool — zero-allocation decode after the first step.
//!
//! Ported from llm/src/backend/wgpu/alloc.rs. The idea is that a
//! single decode step needs ~hundreds of wgpu::Buffer objects for
//! intermediates; creating them fresh every step costs 0.1-0.5 ms per
//! create on Metal. A pooled allocator amortizes this to zero after
//! the first step.
//!
//! Usage pattern:
//!   let alloc = FrameAllocator::new(device);
//!   for step in 0..num_steps {
//!       alloc.reset();                       // all last-frame buffers → pool
//!       let scratch = alloc.alloc_storage(n);
//!       // ... use scratch in this step ...
//!   }
//!
//! `reset()` is cheap (no GPU work, just moves handles between Vecs).
//!
//! Not yet wired into mr's executor path — part of the Phase B scaffolding
//! that Phase C will consume when op dispatch lands.

use std::collections::HashMap;
use std::sync::Arc;

/// Pool of reusable wgpu buffers keyed by byte size. Uniform and storage
/// buffers are pooled separately (different `BufferUsages`).
pub struct FrameAllocator {
    device: Arc<wgpu::Device>,
    storage_pools: HashMap<u64, Vec<wgpu::Buffer>>,
    uniform_pools: HashMap<u64, Vec<wgpu::Buffer>>,
    in_use: Vec<(u64, wgpu::Buffer, Kind)>,
    pub reused: usize,
    pub allocated: usize,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Kind {
    Storage,
    Uniform,
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

    /// Allocate a storage buffer of exactly `size` bytes. Reuses a pooled
    /// buffer of the same size if available, else creates one.
    pub fn alloc_storage(&mut self, size: u64) -> wgpu::Buffer {
        if let Some(buffers) = self.storage_pools.get_mut(&size) {
            if let Some(buf) = buffers.pop() {
                self.reused += 1;
                self.in_use.push((size, buf.clone(), Kind::Storage));
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
        self.in_use.push((size, buf.clone(), Kind::Storage));
        buf
    }

    /// Allocate a uniform buffer (aligned up to 16 bytes per wgpu spec).
    pub fn alloc_uniform(&mut self, size: u64) -> wgpu::Buffer {
        let aligned = (size + 15) & !15;
        if let Some(buffers) = self.uniform_pools.get_mut(&aligned) {
            if let Some(buf) = buffers.pop() {
                self.reused += 1;
                self.in_use.push((aligned, buf.clone(), Kind::Uniform));
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
        self.in_use.push((aligned, buf.clone(), Kind::Uniform));
        buf
    }

    /// Return every buffer handed out this frame to its pool. Call at the
    /// start of each forward step.
    pub fn reset(&mut self) {
        for (size, buf, kind) in self.in_use.drain(..) {
            match kind {
                Kind::Storage => self.storage_pools.entry(size).or_default().push(buf),
                Kind::Uniform => self.uniform_pools.entry(size).or_default().push(buf),
            }
        }
    }

    /// Dump pool stats to log; reset counters.
    pub fn log_stats(&mut self) {
        if self.allocated > 0 || self.reused > 0 {
            log::debug!(
                "frame alloc: {} reused, {} new",
                self.reused,
                self.allocated
            );
        }
        self.reused = 0;
        self.allocated = 0;
    }
}

/// Compute a (x, y) dispatch grid for a linear problem of `num_wg`
/// workgroups, respecting wgpu's 65535-per-axis limit.
///
/// Without this, `dispatch_workgroups(num_wg, 1, 1)` silently caps at
/// 65535 when `num_wg > 65535`, producing wrong output for large N
/// (e.g. lm_head over a 150k vocab).
///
/// WGSL-side: shader sees `workgroup_id.x + workgroup_id.y * x_extent`
/// as the flat index.
pub fn dispatch_2d(num_wg: u32) -> (u32, u32) {
    let x = num_wg.min(65535);
    let y = (num_wg + x - 1) / x;
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_2d_small_stays_1d() {
        assert_eq!(dispatch_2d(1), (1, 1));
        assert_eq!(dispatch_2d(65535), (65535, 1));
    }

    #[test]
    fn dispatch_2d_exceeds_limit_splits_y() {
        let (x, y) = dispatch_2d(131072);
        assert_eq!(x, 65535);
        assert_eq!(y, 3); // ceil(131072 / 65535)
        assert!(x as u64 * y as u64 >= 131072);
    }

    #[test]
    fn dispatch_2d_covers_problem_exactly_or_more() {
        for n in [1, 100, 65535, 65536, 100_000, 151_936, 1_000_000] {
            let (x, y) = dispatch_2d(n);
            assert!(x as u64 * y as u64 >= n as u64, "n={n}: {x}*{y} < n");
            assert!(x <= 65535);
            assert!(y <= 65535);
        }
    }
}
