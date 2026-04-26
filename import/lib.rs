//! import — model importer. HuggingFace / GGUF / safetensors → cyb `.model`.
//!
//! `import/` is the ingest side: load source → [`Weights`] table → write a
//! canonical `.model` file. The runtime in `run/` consumes the result; this
//! crate never runs inference. The CLI binary is named `mi`.
//!
//! Spec: `import/specs/import.md`.
//!
//! [`Weights`]: types::Weights

pub mod cyb_format;
pub mod hf;
pub mod loader;
pub mod manifest;
pub mod naming;
pub mod quant;
pub mod types;

// Generated ONNX protobuf bindings (shared with `loader::onnx`).
pub mod onnx_proto {
    pub mod onnx {
        include!(concat!(env!("OUT_DIR"), "/onnx.rs"));
    }
}

pub use types::{dequantize_to_f32, DType, Weight, Weights};
