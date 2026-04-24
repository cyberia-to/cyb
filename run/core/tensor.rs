//! Tensor — shape, layout, dtype, host or backend-resident data.
//!
//! Spec: specs/tensor.md

use crate::core::dtype::DType;
use std::sync::Arc;

/// Row-major tensor shape. Most-significant dim first.
pub type Shape = Vec<usize>;

/// Compute row-major strides from shape.
///
/// For rank 0 (scalar) returns empty vec.
pub fn strides_from_shape(shape: &[usize]) -> Vec<usize> {
    if shape.is_empty() {
        return Vec::new();
    }
    let mut strides = vec![0; shape.len()];
    strides[shape.len() - 1] = 1;
    for i in (0..shape.len() - 1).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

/// Total element count for a shape.
pub fn numel(shape: &[usize]) -> usize {
    shape.iter().product()
}

/// A tensor: shape + dtype + data location.
///
/// For CPU tensors, data lives in host memory.
/// For backend tensors, data is opaque (Arc<dyn BackendData>).
#[derive(Clone)]
pub struct Tensor {
    pub shape: Shape,
    pub dtype: DType,
    pub data: TensorData,
}

/// Opaque backend-resident buffer handle.
pub trait BackendData: Send + Sync + std::any::Any {
    fn backend_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn std::any::Any;
}

#[derive(Clone)]
pub enum TensorData {
    /// CPU bytes, tightly packed row-major per dtype layout.
    Host(Arc<Vec<u8>>),
    /// Backend-resident buffer, opaque to caller.
    Backend(Arc<dyn BackendData>),
}

#[derive(thiserror::Error, Debug)]
pub enum TensorError {
    #[error("numel mismatch: shape {shape:?} → {expected} elements, got {got}")]
    NumelMismatch {
        shape: Shape,
        expected: usize,
        got: usize,
    },
    #[error("byte count mismatch: shape {shape:?} dtype {dtype:?} → {expected} bytes, got {got}")]
    ByteCountMismatch {
        shape: Shape,
        dtype: DType,
        expected: usize,
        got: usize,
    },
    #[error("expected dtype {expected:?}, got {got:?}")]
    DTypeMismatch { expected: DType, got: DType },
    #[error("tensor is backend-resident ({backend}), host data requested")]
    NotHostResident { backend: &'static str },
}

impl Tensor {
    /// Create a host tensor from f32 data. Panics on mismatch — caller is
    /// responsible for matching shape to data. For construction from unknown
    /// sources, use `try_from_f32`.
    pub fn from_f32(shape: Shape, data: Vec<f32>) -> Self {
        Self::try_from_f32(shape, data).expect("Tensor::from_f32: invariant violated")
    }

    /// Fallible construction from f32 data.
    pub fn try_from_f32(shape: Shape, data: Vec<f32>) -> Result<Self, TensorError> {
        let expected = numel(&shape);
        if expected != data.len() {
            return Err(TensorError::NumelMismatch {
                shape,
                expected,
                got: data.len(),
            });
        }
        let bytes: Vec<u8> = bytemuck::cast_slice(&data).to_vec();
        Ok(Self {
            shape,
            dtype: DType::F32,
            data: TensorData::Host(Arc::new(bytes)),
        })
    }

    /// Create a host tensor from raw bytes with explicit dtype.
    /// Panics on byte-count mismatch; use `try_from_bytes` for fallible.
    pub fn from_bytes(shape: Shape, dtype: DType, bytes: Vec<u8>) -> Self {
        Self::try_from_bytes(shape, dtype, bytes).expect("Tensor::from_bytes: invariant violated")
    }

    /// Fallible construction from raw bytes.
    pub fn try_from_bytes(shape: Shape, dtype: DType, bytes: Vec<u8>) -> Result<Self, TensorError> {
        let n = numel(&shape);
        let expected = dtype.bytes_for(n);
        if expected != bytes.len() {
            return Err(TensorError::ByteCountMismatch {
                shape,
                dtype,
                expected,
                got: bytes.len(),
            });
        }
        Ok(Self {
            shape,
            dtype,
            data: TensorData::Host(Arc::new(bytes)),
        })
    }

    /// Element count.
    pub fn numel(&self) -> usize {
        numel(&self.shape)
    }

    /// Rank (number of dimensions).
    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    /// Access host bytes or error if backend-resident.
    pub fn as_host_bytes(&self) -> Option<&[u8]> {
        match &self.data {
            TensorData::Host(b) => Some(b.as_slice()),
            TensorData::Backend(_) => None,
        }
    }

    /// Interpret host bytes as f32 slice (panics if dtype != F32 or backend-resident).
    /// Use `try_as_f32` for fallible access.
    pub fn as_f32(&self) -> &[f32] {
        self.try_as_f32().expect("as_f32: invariant violated")
    }

    pub fn try_as_f32(&self) -> Result<&[f32], TensorError> {
        if self.dtype != DType::F32 {
            return Err(TensorError::DTypeMismatch {
                expected: DType::F32,
                got: self.dtype,
            });
        }
        match &self.data {
            TensorData::Host(b) => Ok(bytemuck::cast_slice(b.as_slice())),
            TensorData::Backend(b) => Err(TensorError::NotHostResident {
                backend: b.backend_name(),
            }),
        }
    }

    /// Clone as Vec<f32>. Works for host F32 only.
    pub fn to_f32_vec(&self) -> Vec<f32> {
        self.as_f32().to_vec()
    }

    /// True if this tensor lives on host memory.
    pub fn is_host(&self) -> bool {
        matches!(self.data, TensorData::Host(_))
    }
}

impl std::fmt::Debug for Tensor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let loc = match &self.data {
            TensorData::Host(_) => "host",
            TensorData::Backend(b) => b.backend_name(),
        };
        write!(
            f,
            "Tensor {{ shape: {:?}, dtype: {:?}, loc: {} }}",
            self.shape, self.dtype, loc
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strides_row_major() {
        assert_eq!(strides_from_shape(&[2, 3, 4]), vec![12, 4, 1]);
        assert_eq!(strides_from_shape(&[5]), vec![1]);
        assert_eq!(strides_from_shape(&[]), Vec::<usize>::new());
    }

    #[test]
    fn numel_matches_product() {
        assert_eq!(numel(&[2, 3, 4]), 24);
        assert_eq!(numel(&[]), 1); // scalar
        assert_eq!(numel(&[0, 5]), 0); // empty
    }

    #[test]
    fn f32_roundtrip() {
        let t = Tensor::from_f32(vec![2, 3], (0..6).map(|x| x as f32).collect());
        assert_eq!(t.shape, vec![2, 3]);
        assert_eq!(t.dtype, DType::F32);
        assert_eq!(t.numel(), 6);
        assert_eq!(t.to_f32_vec(), vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
    }
}
