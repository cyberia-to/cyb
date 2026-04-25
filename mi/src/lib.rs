//! mi — model importer. HuggingFace / GGUF / safetensors → cyb `.model`.
//!
//! Runtime (inference) lives in the `mr/` crate. `mi` owns the ingest side:
//!   load source → [`Weights`] table → re-pack into a `.model` file that
//!   `mr/` can mmap.
//!
//! Spec: `reference/runtime/import.md`.
//!
//! [`Weights`]: types::Weights

pub mod cyb_format;
pub mod hub;
pub mod import;
pub mod loader;
pub mod manifest;
pub mod types;

// Generated ONNX protobuf bindings (shared with `loader::onnx`).
pub mod onnx_proto {
    pub mod onnx {
        include!(concat!(env!("OUT_DIR"), "/onnx.rs"));
    }
}

pub use types::{dequantize_to_f32, DType, Weight, Weights};
