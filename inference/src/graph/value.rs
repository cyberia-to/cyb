use burn::prelude::*;
use crate::Backend;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

/// Dynamic-rank tensor — always stored as 4D internally.
/// Reshape is free (metadata only, no GPU ops).
/// All arithmetic broadcasts through 4D, then trims output shape.
#[derive(Clone, Debug)]
pub struct Value {
    /// Always 4D: leading dims padded with 1
    inner: Tensor<Backend, 4>,
    /// Logical shape (1D..4D)
    pub logical_shape: Vec<usize>,
}

impl Value {
    /// Create from a 4D burn tensor with logical shape
    pub fn new(inner: Tensor<Backend, 4>, logical_shape: Vec<usize>) -> Self {
        Self { inner, logical_shape }
    }

    /// Create from raw f32 data + shape
    /// Data is provided as f32 and auto-converted to backend's float type (f16 or f32)
    pub fn from_data(data: Vec<f32>, shape: Vec<usize>, device: &Device) -> Self {
        let shape4d = to_4d_shape(&shape);
        // Convert f32 → backend float element type via TensorData
        let td = burn::tensor::TensorData::new(data, shape4d.clone()).convert::<<Backend as burn::tensor::backend::Backend>::FloatElem>();
        let inner = Tensor::from_data(td, device);
        Self { inner, logical_shape: shape }
    }

    /// Create zeros
    pub fn zeros(shape: Vec<usize>, device: &Device) -> Self {
        let shape4d = to_4d_shape(&shape);
        let inner = Tensor::zeros(shape4d, device);
        Self { inner, logical_shape: shape }
    }

    pub fn shape(&self) -> Vec<usize> {
        self.logical_shape.clone()
    }

    pub fn ndim(&self) -> usize {
        self.logical_shape.len()
    }

    /// Total number of elements
    pub fn numel(&self) -> usize {
        self.logical_shape.iter().product()
    }

    /// Get the underlying 4D tensor
    pub fn as_4d(&self) -> &Tensor<Backend, 4> {
        &self.inner
    }

    /// Get the underlying 4D tensor (consuming)
    pub fn into_4d(self) -> Tensor<Backend, 4> {
        self.inner
    }

    /// Reshape — avoids GPU copy when possible
    pub fn reshape(self, new_shape: Vec<usize>) -> Self {
        let new_4d = to_4d_shape(&new_shape);
        let old_4d = self.inner.dims();
        if old_4d == new_4d {
            return Self { inner: self.inner, logical_shape: new_shape };
        }
        let inner = self.inner.reshape(new_4d);
        Self { inner, logical_shape: new_shape }
    }

    /// Flatten to 1D
    pub fn flatten(self) -> Self {
        let n = self.numel();
        self.reshape(vec![n])
    }

    /// Get device
    pub fn device(&self) -> Device {
        self.inner.device()
    }

    /// Read data to CPU (only for small tensors — shapes, indices, logits)
    pub fn to_data(&self) -> burn::tensor::TensorData {
        self.inner.to_data()
    }

    /// Read as f32 vec (forces GPU sync! — converts from backend float type)
    pub fn to_vec_f32(&self) -> Vec<f32> {
        let data = self.inner.to_data().convert::<f32>();
        data.as_slice::<f32>().unwrap_or(&[]).to_vec()
    }

    // ============ Tensor Ops (stay on GPU) ============

    /// Element-wise add (with broadcast)
    pub fn add(self, rhs: Self) -> Self {
        let out_shape = broadcast_shape(&self.logical_shape, &rhs.logical_shape);
        let inner = self.inner + rhs.inner;
        Self { inner, logical_shape: out_shape }
    }

    pub fn sub(self, rhs: Self) -> Self {
        let out_shape = broadcast_shape(&self.logical_shape, &rhs.logical_shape);
        let inner = self.inner - rhs.inner;
        Self { inner, logical_shape: out_shape }
    }

    pub fn mul(self, rhs: Self) -> Self {
        let out_shape = broadcast_shape(&self.logical_shape, &rhs.logical_shape);
        let inner = self.inner * rhs.inner;
        Self { inner, logical_shape: out_shape }
    }

    pub fn div(self, rhs: Self) -> Self {
        let out_shape = broadcast_shape(&self.logical_shape, &rhs.logical_shape);
        let inner = self.inner / rhs.inner;
        Self { inner, logical_shape: out_shape }
    }

    /// Scalar operations
    pub fn add_scalar(self, s: f32) -> Self {
        Self { inner: self.inner + s, logical_shape: self.logical_shape }
    }

    pub fn mul_scalar(self, s: f32) -> Self {
        Self { inner: self.inner * s, logical_shape: self.logical_shape }
    }

    pub fn div_scalar(self, s: f32) -> Self {
        Self { inner: self.inner / s, logical_shape: self.logical_shape }
    }

    // Unary ops
    pub fn sqrt(self) -> Self { Self { inner: self.inner.sqrt(), logical_shape: self.logical_shape } }
    pub fn neg(self) -> Self { Self { inner: self.inner.neg(), logical_shape: self.logical_shape } }
    pub fn exp(self) -> Self { Self { inner: self.inner.exp(), logical_shape: self.logical_shape } }
    pub fn log(self) -> Self { Self { inner: self.inner.log(), logical_shape: self.logical_shape } }
    pub fn abs(self) -> Self { Self { inner: self.inner.abs(), logical_shape: self.logical_shape } }
    pub fn recip(self) -> Self { Self { inner: self.inner.recip(), logical_shape: self.logical_shape } }
    pub fn powf_scalar(self, p: f32) -> Self { Self { inner: self.inner.powf_scalar(p), logical_shape: self.logical_shape } }

    // Activations
    pub fn relu(self) -> Self { Self { inner: burn::tensor::activation::relu(self.inner), logical_shape: self.logical_shape } }
    pub fn gelu(self) -> Self { Self { inner: burn::tensor::activation::gelu(self.inner), logical_shape: self.logical_shape } }
    pub fn sigmoid(self) -> Self { Self { inner: burn::tensor::activation::sigmoid(self.inner), logical_shape: self.logical_shape } }
    pub fn tanh(self) -> Self { Self { inner: self.inner.tanh(), logical_shape: self.logical_shape } }

    pub fn softmax(self, dim: usize) -> Self {
        let dim4 = to_4d_dim(dim, self.logical_shape.len());
        Self { inner: burn::tensor::activation::softmax(self.inner, dim4), logical_shape: self.logical_shape }
    }

    // Reductions
    pub fn sum_dim(self, dim: usize) -> Self {
        let dim4 = to_4d_dim(dim, self.logical_shape.len());
        let inner = self.inner.sum_dim(dim4);
        let mut shape = self.logical_shape.clone();
        shape[dim] = 1;
        Self { inner, logical_shape: shape }
    }

    pub fn mean_dim(self, dim: usize) -> Self {
        let dim4 = to_4d_dim(dim, self.logical_shape.len());
        let inner = self.inner.mean_dim(dim4);
        let mut shape = self.logical_shape.clone();
        shape[dim] = 1;
        Self { inner, logical_shape: shape }
    }

    pub fn sum(self) -> Self {
        let inner = self.inner.sum();
        // sum() returns scalar, reshape to [1,1,1,1]
        let inner = inner.reshape([1, 1, 1, 1]);
        Self { inner, logical_shape: vec![1] }
    }

    /// Matrix multiply — last 2 dims are matmul'd, leading dims batch
    pub fn matmul(self, rhs: Self) -> Self {
        let inner = self.inner.matmul(rhs.inner);
        let mut shape = self.logical_shape.clone();
        let rhs_last = rhs.logical_shape.last().copied().unwrap_or(1);
        if let Some(last) = shape.last_mut() {
            *last = rhs_last;
        }
        // Fix batch dims from broadcast
        let out_shape = inner.dims();
        let ndim = shape.len();
        let offset = 4 - ndim;
        for i in 0..ndim {
            shape[i] = out_shape[i + offset];
        }
        Self { inner, logical_shape: shape }
    }

    /// Transpose last two dimensions
    pub fn transpose(self) -> Self {
        let inner = self.inner.swap_dims(2, 3);
        let mut shape = self.logical_shape.clone();
        let n = shape.len();
        if n >= 2 {
            shape.swap(n - 2, n - 1);
        }
        Self { inner, logical_shape: shape }
    }

    /// Swap two logical dimensions
    pub fn swap_dims(self, d1: usize, d2: usize) -> Self {
        let ndim = self.logical_shape.len();
        let d1_4d = to_4d_dim(d1, ndim);
        let d2_4d = to_4d_dim(d2, ndim);
        if d1_4d == d2_4d { return self; }
        let inner = self.inner.swap_dims(d1_4d, d2_4d);
        let mut shape = self.logical_shape.clone();
        shape.swap(d1, d2);
        Self { inner, logical_shape: shape }
    }

    /// Narrow (slice) along a dimension
    pub fn narrow(self, dim: usize, start: usize, len: usize) -> Self {
        let dim4 = to_4d_dim(dim, self.logical_shape.len());
        let inner = self.inner.narrow(dim4, start, len);
        let mut shape = self.logical_shape.clone();
        shape[dim] = len;
        Self { inner, logical_shape: shape }
    }

    /// Concatenate along a dimension
    pub fn cat(tensors: Vec<Self>, dim: usize) -> Self {
        if tensors.is_empty() { panic!("cat: empty list"); }
        let ndim = tensors[0].logical_shape.len();
        let dim4 = to_4d_dim(dim, ndim);
        let inners: Vec<_> = tensors.iter().map(|t| t.inner.clone()).collect();
        let inner = Tensor::cat(inners, dim4);
        let mut shape = tensors[0].logical_shape.clone();
        shape[dim] = tensors.iter().map(|t| t.logical_shape[dim]).sum();
        Self { inner, logical_shape: shape }
    }

    /// Select indices along a dimension (GPU-native, no CPU roundtrip)
    pub fn select(self, dim: usize, indices: Tensor<Backend, 1, Int>) -> Self {
        let dim4 = to_4d_dim(dim, self.logical_shape.len());
        let n = indices.dims()[0];
        let inner = self.inner.select(dim4, indices);
        let mut shape = self.logical_shape.clone();
        shape[dim] = n;
        Self { inner, logical_shape: shape }
    }

    /// Erf (error function) — approximate
    pub fn erf(self) -> Self {
        // tanh approximation: erf(x) ≈ tanh(sqrt(2/π) * (x + 0.044715 * x³))
        let x = self.inner;
        let x3 = x.clone().powf_scalar(3.0);
        let inner_val = (x.clone() + x3 * 0.044715) * 0.7978845608; // sqrt(2/π)
        let result = inner_val.tanh();
        Self { inner: result, logical_shape: self.logical_shape }
    }
}

/// Pad shape to 4D: [a, b] → [1, 1, a, b]
fn to_4d_shape(shape: &[usize]) -> [usize; 4] {
    match shape.len() {
        0 => [1, 1, 1, 1],
        1 => [1, 1, 1, shape[0]],
        2 => [1, 1, shape[0], shape[1]],
        3 => [1, shape[0], shape[1], shape[2]],
        4 => [shape[0], shape[1], shape[2], shape[3]],
        _ => panic!("to_4d_shape: rank {} > 4 not supported", shape.len()),
    }
}

/// Map logical dim to 4D dim: dim 0 of 2D → dim 2 of 4D
fn to_4d_dim(dim: usize, ndim: usize) -> usize {
    let offset = 4 - ndim;
    dim + offset
}

/// Compute broadcast output shape
fn broadcast_shape(a: &[usize], b: &[usize]) -> Vec<usize> {
    let max_len = a.len().max(b.len());
    let mut result = vec![1usize; max_len];
    for i in 0..max_len {
        let da = if i < max_len - a.len() { 1 } else { a[i - (max_len - a.len())] };
        let db = if i < max_len - b.len() { 1 } else { b[i - (max_len - b.len())] };
        result[i] = da.max(db);
    }
    result
}
