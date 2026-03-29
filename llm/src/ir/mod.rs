//! Graph IR — typed DAG for model representation

pub mod graph;
pub mod ops;
pub mod dtype;
pub mod fusion;

pub use graph::{Graph, Node, TensorId, TensorMeta, WeightData};
pub use ops::Op;
pub use dtype::DType;
