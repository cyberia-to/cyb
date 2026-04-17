//! aruminium GPU wrapper for honeycrisp.
//!
//! Metal is thread-safe at the command-queue level; we assert Send+Sync
//! unsafely since aruminium holds Objective-C pointers that Rust conservatively
//! marks as !Send+!Sync.

use crate::backend::BackendError;

pub struct HoneycrispDevice {
    pub gpu: aruminium::Gpu,
    pub queue: aruminium::Queue,
    pub dispatch: aruminium::Dispatch,
}

// SAFETY: Metal objects are thread-safe as per Apple's Metal framework
// documentation. The raw Objective-C pointers held inside aruminium types
// can be shared across threads; Rust's conservative defaults don't know.
unsafe impl Send for HoneycrispDevice {}
unsafe impl Sync for HoneycrispDevice {}

impl HoneycrispDevice {
    pub fn new() -> Result<Self, BackendError> {
        let gpu = aruminium::Gpu::open().map_err(|e| BackendError::BackendInit {
            backend: "honeycrisp",
            reason: format!("{e}"),
        })?;
        let queue = gpu.new_command_queue().map_err(|e| BackendError::BackendInit {
            backend: "honeycrisp",
            reason: format!("{e}"),
        })?;
        let dispatch = aruminium::Dispatch::new(&queue);
        Ok(Self {
            gpu,
            queue,
            dispatch,
        })
    }

    /// Compile an MSL source and return the pipeline for `main`.
    /// Pipeline caching is left to the Metal runtime (known fast).
    pub fn pipeline(&self, source: &str) -> Result<aruminium::Pipeline, BackendError> {
        let lib = self.gpu.compile(source).map_err(|e| BackendError::BackendInit {
            backend: "honeycrisp",
            reason: format!("compile: {e}"),
        })?;
        let shader = lib.function("kmain").map_err(|e| BackendError::BackendInit {
            backend: "honeycrisp",
            reason: format!("function lookup: {e}"),
        })?;
        self.gpu.pipeline(&shader).map_err(|e| BackendError::BackendInit {
            backend: "honeycrisp",
            reason: format!("pipeline: {e}"),
        })
    }

    pub fn alloc(&self, size: usize) -> Result<aruminium::Buffer, BackendError> {
        self.gpu
            .buffer(size.max(4))
            .map_err(|_| BackendError::OutOfMemory {
                backend: "honeycrisp",
                requested_bytes: size,
            })
    }
}
