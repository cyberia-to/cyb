//! Graph IR — typed DAG for model representation

pub mod graph;
pub mod ops;
pub mod dtype;
pub mod fusion;
pub mod templates;
pub mod atoms;
pub mod jets;
pub mod executor;

pub use graph::{Graph, Node, TensorId, TensorMeta, WeightData, Dim, Shape, Residency, BackendHint, Attrs, AttrValue, MemoryPlan, BufferLifetime};
pub use ops::{Op, PoolMode, InterpolateMode, SampleMethod};
pub use dtype::DType;
pub use templates::{TransformerConfig, Activation, DiTConfig, CnnDetectorConfig, MoeDecoderConfig};
pub use atoms::{Atom, CmpOp, ReduceOp, SlidePattern, AtomInterpreter, decompose};
pub use jets::{JetRegistry, Jet, FormulaHash, formula_hash};
pub use executor::{GraphExecutor, GpuWeight, GpuQuantFormat, ExecKVCache, ExecConfig};
