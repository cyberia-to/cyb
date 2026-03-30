//! Accelerate.framework FFI — cblas, vDSP, vvrsqrtf
//!
//! Zero dependencies. These are system libraries on every macOS.

#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::too_many_arguments
)]

// ── CBLAS ──

pub const CblasRowMajor: i32 = 101;
pub const CblasNoTrans: i32 = 111;
pub const CblasTrans: i32 = 112;

type CBLAS_ORDER = i32;
type CBLAS_TRANSPOSE = i32;

#[link(name = "Accelerate", kind = "framework")]
unsafe extern "C" {
    // General matrix multiply: C = alpha * A @ B + beta * C
    pub fn cblas_sgemm(
        order: CBLAS_ORDER,
        transA: CBLAS_TRANSPOSE,
        transB: CBLAS_TRANSPOSE,
        M: i32,
        N: i32,
        K: i32,
        alpha: f32,
        A: *const f32,
        lda: i32,
        B: *const f32,
        ldb: i32,
        beta: f32,
        C: *mut f32,
        ldc: i32,
    );

    // Matrix-vector multiply: y = alpha * A @ x + beta * y
    pub fn cblas_sgemv(
        order: CBLAS_ORDER,
        trans: CBLAS_TRANSPOSE,
        M: i32,
        N: i32,
        alpha: f32,
        A: *const f32,
        lda: i32,
        x: *const f32,
        incx: i32,
        beta: f32,
        y: *mut f32,
        incy: i32,
    );

    // Copy: y = x (with strides)
    pub fn cblas_scopy(N: i32, x: *const f32, incx: i32, y: *mut f32, incy: i32);
}

// ── vDSP ──

pub type vDSP_Length = u64;
pub type vDSP_Stride = i64;

unsafe extern "C" {
    // Element-wise multiply: C = A * B
    pub fn vDSP_vmul(
        A: *const f32,
        IA: vDSP_Stride,
        B: *const f32,
        IB: vDSP_Stride,
        C: *mut f32,
        IC: vDSP_Stride,
        N: vDSP_Length,
    );

    // Element-wise add: C = A + B
    pub fn vDSP_vadd(
        A: *const f32,
        IA: vDSP_Stride,
        B: *const f32,
        IB: vDSP_Stride,
        C: *mut f32,
        IC: vDSP_Stride,
        N: vDSP_Length,
    );

    // Scalar multiply-add: C = A * scalar + B
    pub fn vDSP_vsma(
        A: *const f32,
        IA: vDSP_Stride,
        B: *const f32, // scalar pointer
        C: *const f32,
        IC: vDSP_Stride,
        D: *mut f32,
        ID: vDSP_Stride,
        N: vDSP_Length,
    );

    // Scalar multiply: C = A * scalar
    pub fn vDSP_vsmul(
        A: *const f32,
        IA: vDSP_Stride,
        B: *const f32, // scalar pointer
        C: *mut f32,
        IC: vDSP_Stride,
        N: vDSP_Length,
    );

    // Scalar multiply + scalar add: C = A * scalar1 + scalar2
    pub fn vDSP_vsmsa(
        A: *const f32,
        IA: vDSP_Stride,
        B: *const f32, // multiply scalar
        C: *const f32, // add scalar
        D: *mut f32,
        ID: vDSP_Stride,
        N: vDSP_Length,
    );

    // Element-wise subtract: C = B - A  (note: reversed!)
    pub fn vDSP_vsub(
        A: *const f32,
        IA: vDSP_Stride,
        B: *const f32,
        IB: vDSP_Stride,
        C: *mut f32,
        IC: vDSP_Stride,
        N: vDSP_Length,
    );

    // Sum of elements
    pub fn vDSP_sve(A: *const f32, IA: vDSP_Stride, C: *mut f32, N: vDSP_Length);

    // Max value
    pub fn vDSP_maxv(A: *const f32, IA: vDSP_Stride, C: *mut f32, N: vDSP_Length);

    // Scalar add: C = A + scalar
    pub fn vDSP_vsadd(
        A: *const f32,
        IA: vDSP_Stride,
        B: *const f32, // scalar
        C: *mut f32,
        IC: vDSP_Stride,
        N: vDSP_Length,
    );
}

// ── vecLib ──

unsafe extern "C" {
    // Vectorized reciprocal sqrt: y[i] = 1/sqrt(x[i])
    pub fn vvrsqrtf(y: *mut f32, x: *const f32, n: *const i32);

    // Vectorized exp: y[i] = exp(x[i])
    pub fn vvexpf(y: *mut f32, x: *const f32, n: *const i32);
}

// ── High-level helpers ──

/// BLAS sgemm wrapper: C[M,N] = alpha * A[M,K] @ B[K,N] + beta * C[M,N]
/// Row-major layout.
#[inline]
pub fn sgemm(
    trans_a: bool,
    trans_b: bool,
    m: usize,
    n: usize,
    k: usize,
    alpha: f32,
    a: &[f32],
    lda: usize,
    b: &[f32],
    ldb: usize,
    beta: f32,
    c: &mut [f32],
    ldc: usize,
) {
    unsafe {
        cblas_sgemm(
            CblasRowMajor,
            if trans_a { CblasTrans } else { CblasNoTrans },
            if trans_b { CblasTrans } else { CblasNoTrans },
            m as i32,
            n as i32,
            k as i32,
            alpha,
            a.as_ptr(),
            lda as i32,
            b.as_ptr(),
            ldb as i32,
            beta,
            c.as_mut_ptr(),
            ldc as i32,
        );
    }
}

/// BLAS sgemv wrapper: y[M] = alpha * A[M,N] @ x[N] + beta * y[M]
#[inline]
pub fn sgemv(
    trans: bool,
    m: usize,
    n: usize,
    alpha: f32,
    a: &[f32],
    lda: usize,
    x: &[f32],
    beta: f32,
    y: &mut [f32],
) {
    unsafe {
        cblas_sgemv(
            CblasRowMajor,
            if trans { CblasTrans } else { CblasNoTrans },
            m as i32,
            n as i32,
            alpha,
            a.as_ptr(),
            lda as i32,
            x.as_ptr(),
            1,
            beta,
            y.as_mut_ptr(),
            1,
        );
    }
}
