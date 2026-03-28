use burn::prelude::*;
use crate::Backend;

/// Dynamic tensor value — wraps burn tensors of varying dimensionality.
/// ONNX graphs have dynamic shapes; burn requires compile-time dimension count.
/// We handle this by dispatching on dimensionality at each operation boundary.
#[derive(Clone, Debug)]
pub enum Value {
    Float1(Tensor<Backend, 1>),
    Float2(Tensor<Backend, 2>),
    Float3(Tensor<Backend, 3>),
    Float4(Tensor<Backend, 4>),
    Int1(Tensor<Backend, 1, Int>),
    Int2(Tensor<Backend, 2, Int>),
    Int3(Tensor<Backend, 3, Int>),
    Bool1(Tensor<Backend, 1, Bool>),
    Bool2(Tensor<Backend, 2, Bool>),
    None,
}

impl Value {
    pub fn shape(&self) -> Vec<usize> {
        match self {
            Value::Float1(t) => t.dims().to_vec(),
            Value::Float2(t) => t.dims().to_vec(),
            Value::Float3(t) => t.dims().to_vec(),
            Value::Float4(t) => t.dims().to_vec(),
            Value::Int1(t) => t.dims().to_vec(),
            Value::Int2(t) => t.dims().to_vec(),
            Value::Int3(t) => t.dims().to_vec(),
            Value::Bool1(t) => t.dims().to_vec(),
            Value::Bool2(t) => t.dims().to_vec(),
            Value::None => vec![],
        }
    }

    pub fn ndim(&self) -> usize {
        self.shape().len()
    }
}
