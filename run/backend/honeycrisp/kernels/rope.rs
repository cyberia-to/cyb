//! Metal Rotary Position Embedding (NeoX-style pairing).
//!
//! Input shape: x is f32 [n_rows, head_dim] flattened. pos is f32 length 1.
//! Each row gets independently rotated. NeoX pairs (j, j+head_dim/2).
//! Identical math to backend/cpu/rope.rs.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

// MSL template — caller substitutes `__HEAD_DIM__` and `__HALF__` with literal
// constants at pipeline-build time. Apple Metal scheduler regresses when more
// than one non-constant integer is used as an address-offset; baking these in
// keeps the kernel single-non-constant.
const MSL_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint HEAD_DIM = __HEAD_DIM__u;
constant constexpr uint HALF     = __HALF__u;

struct Dims { uint n_rows; uint rope_half; uint pad0; uint pad1; };

kernel void kmain(
    device const float  *x      [[buffer(0)]],
    device const float  *cos_t  [[buffer(1)]],
    device const float  *sin_t  [[buffer(2)]],
    device       float  *y      [[buffer(3)]],
    constant     Dims   &dims   [[buffer(4)]],
    uint2                gid    [[thread_position_in_grid]]
) {
    uint row = gid.y;
    uint j   = gid.x;
    if (row >= dims.n_rows || j >= HALF) return;
    uint base = row * HEAD_DIM;

    float x1 = x[base + j];
    float x2 = x[base + j + HALF];

    if (j < dims.rope_half) {
        float c = cos_t[j];
        float s = sin_t[j];
        y[base + j]         = x1 * c - x2 * s;
        y[base + j + HALF]  = x1 * s + x2 * c;
    } else {
        y[base + j]         = x1;
        y[base + j + HALF]  = x2;
    }
}
"#;

/// Fused Q+K RoPE: single dispatch over (num_q_heads + kv_heads) rows.
/// Rows 0..num_q_heads → Q buffers; rows num_q_heads..total → K buffers.
/// Buffers: 0=xq, 1=xk, 2=cos, 3=sin, 4=yq, 5=yk, 6=dims.
/// Launch: groups=(1, num_q_heads+kv_heads, 1), threads=(HALF, 1, 1).
pub const MSL_QK: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint HEAD_DIM = 128u;
constant constexpr uint HALF     = 64u;

struct Dims { uint q_rows; uint kv_rows; uint rope_half; uint pad; };

kernel void kmain(
    device const float  *xq     [[buffer(0)]],
    device const float  *xk     [[buffer(1)]],
    device const float  *cos_t  [[buffer(2)]],
    device const float  *sin_t  [[buffer(3)]],
    device       float  *yq     [[buffer(4)]],
    device       float  *yk     [[buffer(5)]],
    constant     Dims   &dims   [[buffer(6)]],
    uint2                gid    [[thread_position_in_grid]]
) {
    uint j   = gid.x;
    uint row = gid.y;
    if (j >= HALF) return;

    bool is_q   = row < dims.q_rows;
    uint lrow   = is_q ? row : row - dims.q_rows;
    uint base   = lrow * HEAD_DIM;
    device const float *xin = is_q ? xq + base : xk + base;
    device       float *yout= is_q ? yq + base : yk + base;

    float x1 = xin[j];
    float x2 = xin[j + HALF];

    if (j < dims.rope_half) {
        float c = cos_t[j], s = sin_t[j];
        yout[j]        = x1 * c - x2 * s;
        yout[j + HALF] = x1 * s + x2 * c;
    } else {
        yout[j]        = x1;
        yout[j + HALF] = x2;
    }
}
"#;

/// Fused Q+K qk_norm + RoPE in a single dispatch: eliminates qn/kn intermediate buffers.
/// Each group (one head) computes RMSNorm then applies RoPE.
/// Groups: (q_heads + kv_heads, 1, 1), Threads: (head_dim, 1, 1).
/// HEAD_DIM is a template variable — compile via msl_qk_norm_rope_for(head_dim).
/// Buffers: 0=xq, 1=xk, 2=gq, 3=gk, 4=cos, 5=sin, 6=yq, 7=yk, 8=params.
const MSL_QK_NORM_ROPE_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint HEAD_DIM = __HEAD_DIM__u;
constant constexpr uint HALF     = __HALF__u;
constant constexpr uint N_SIMDS  = __N_SIMDS__u;  // HEAD_DIM / 32

struct ParamsNR { uint q_heads; uint kv_heads; uint rope_half; float eps; };

kernel void kmain(
    device const float   *xq    [[buffer(0)]],
    device const float   *xk    [[buffer(1)]],
    device const float   *gq    [[buffer(2)]],
    device const float   *gk    [[buffer(3)]],
    device const float   *cos_t [[buffer(4)]],
    device const float   *sin_t [[buffer(5)]],
    device       float   *yq    [[buffer(6)]],
    device       float   *yk    [[buffer(7)]],
    constant ParamsNR    &p     [[buffer(8)]],
    uint tid   [[thread_position_in_threadgroup]],
    uint lane  [[thread_index_in_simdgroup]],
    uint sgitg [[simdgroup_index_in_threadgroup]],
    uint bid   [[threadgroup_position_in_grid]]
) {
    threadgroup float shared_sq[N_SIMDS];

    bool is_q = bid < p.q_heads;
    uint lbid = is_q ? bid : bid - p.q_heads;
    device const float *x = is_q ? xq + lbid * HEAD_DIM : xk + lbid * HEAD_DIM;
    device const float *g = is_q ? gq : gk;
    device       float *y = is_q ? yq + lbid * HEAD_DIM : yk + lbid * HEAD_DIM;

    // Phase 1: RMS via simd_sum (2 barriers vs 7 for manual tree)
    float sq = (tid < HEAD_DIM) ? (x[tid] * x[tid]) : 0.0f;
    float simd_sq = simd_sum(sq);
    if (lane == 0) shared_sq[sgitg] = simd_sq;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float total_sq = (lane < N_SIMDS) ? shared_sq[lane] : 0.0f;
    float inv_rms = rsqrt(simd_sum(total_sq) / HEAD_DIM + p.eps);

    // Phase 2: norm + rope (first HALF threads handle paired elements)
    if (tid < HALF) {
        uint j = tid;
        float nj  = x[j]        * inv_rms * g[j];
        float njh = x[j + HALF] * inv_rms * g[j + HALF];
        if (j < p.rope_half) {
            float c = cos_t[j], s = sin_t[j];
            y[j]        = nj  * c - njh * s;
            y[j + HALF] = nj  * s + njh * c;
        } else {
            y[j]        = nj;
            y[j + HALF] = njh;
        }
    }
}
"#;

/// Build the QK-norm+RoPE MSL string for a given head_dim.
pub fn msl_qk_norm_rope_for(head_dim: usize) -> String {
    MSL_QK_NORM_ROPE_TEMPLATE
        .replace("__HEAD_DIM__", &head_dim.to_string())
        .replace("__HALF__", &(head_dim / 2).to_string())
        .replace("__N_SIMDS__", &(head_dim / 32).to_string())
}

/// Legacy alias for head_dim=128 (kept for the static pipeline in HoneycrispBackend::new).
pub const MSL_QK_NORM_ROPE: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint HEAD_DIM = 128u;
constant constexpr uint HALF     = 64u;
constant constexpr uint N_SIMDS  = 4u;

struct ParamsNR { uint q_heads; uint kv_heads; uint rope_half; float eps; };

kernel void kmain(
    device const float   *xq    [[buffer(0)]],
    device const float   *xk    [[buffer(1)]],
    device const float   *gq    [[buffer(2)]],
    device const float   *gk    [[buffer(3)]],
    device const float   *cos_t [[buffer(4)]],
    device const float   *sin_t [[buffer(5)]],
    device       float   *yq    [[buffer(6)]],
    device       float   *yk    [[buffer(7)]],
    constant ParamsNR    &p     [[buffer(8)]],
    uint tid   [[thread_position_in_threadgroup]],
    uint lane  [[thread_index_in_simdgroup]],
    uint sgitg [[simdgroup_index_in_threadgroup]],
    uint bid   [[threadgroup_position_in_grid]]
) {
    threadgroup float shared_sq[N_SIMDS];

    bool is_q = bid < p.q_heads;
    uint lbid = is_q ? bid : bid - p.q_heads;
    device const float *x = is_q ? xq + lbid * HEAD_DIM : xk + lbid * HEAD_DIM;
    device const float *g = is_q ? gq : gk;
    device       float *y = is_q ? yq + lbid * HEAD_DIM : yk + lbid * HEAD_DIM;

    float sq = (tid < HEAD_DIM) ? (x[tid] * x[tid]) : 0.0f;
    float simd_sq = simd_sum(sq);
    if (lane == 0) shared_sq[sgitg] = simd_sq;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float total_sq = (lane < N_SIMDS) ? shared_sq[lane] : 0.0f;
    float inv_rms = rsqrt(simd_sum(total_sq) / HEAD_DIM + p.eps);

    if (tid < HALF) {
        uint j = tid;
        float nj  = x[j]        * inv_rms * g[j];
        float njh = x[j + HALF] * inv_rms * g[j + HALF];
        if (j < p.rope_half) {
            float c = cos_t[j], s = sin_t[j];
            y[j]        = nj  * c - njh * s;
            y[j + HALF] = nj  * s + njh * c;
        } else {
            y[j]        = nj;
            y[j + HALF] = njh;
        }
    }
}
"#;

pub fn msl_for(head_dim: usize) -> String {
    MSL_TEMPLATE
        .replace("__HEAD_DIM__", &head_dim.to_string())
        .replace("__HALF__", &(head_dim / 2).to_string())
}

// Default MSL for the qwen3-0.6b head_dim=128 path. The runtime can rebuild
// pipelines for other models via `msl_for(head_dim)`.
pub const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant constexpr uint HEAD_DIM = 128u;
constant constexpr uint HALF     = 64u;

struct Dims { uint n_rows; uint rope_half; uint pad0; uint pad1; };

kernel void kmain(
    device const float  *x      [[buffer(0)]],
    device const float  *cos_t  [[buffer(1)]],
    device const float  *sin_t  [[buffer(2)]],
    device       float  *y      [[buffer(3)]],
    constant     Dims   &dims   [[buffer(4)]],
    uint2                gid    [[thread_position_in_grid]]
) {
    uint row = gid.y;
    uint j   = gid.x;
    if (row >= dims.n_rows || j >= HALF) return;
    uint base = row * HEAD_DIM;

    float x1 = x[base + j];
    float x2 = x[base + j + HALF];

    if (j < dims.rope_half) {
        float c = cos_t[j];
        float s = sin_t[j];
        y[base + j]         = x1 * c - x2 * s;
        y[base + j + HALF]  = x1 * s + x2 * c;
    } else {
        y[base + j]         = x1;
        y[base + j + HALF]  = x2;
    }
}
"#;

pub fn dispatch(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    x: &aruminium::Buffer,
    cos_t: &aruminium::Buffer,
    sin_t: &aruminium::Buffer,
    n_rows: u32,
    head_dim: u32,
    rope_dim: u32,
) -> Result<aruminium::Buffer, BackendError> {
    let out = dev.alloc((n_rows as usize * head_dim as usize * 4).max(4))?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Dims { n_rows: u32, rope_half: u32, pad0: u32, pad1: u32 }
    let dims = Dims { n_rows, rope_half: rope_dim / 2, pad0: 0, pad1: 0 };
    let half = (head_dim / 2) as usize;

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|enc| {
                enc.bind(pipeline);
                enc.bind_buffer(x, 0, 0);
                enc.bind_buffer(cos_t, 0, 1);
                enc.bind_buffer(sin_t, 0, 2);
                enc.bind_buffer(&out, 0, 3);
                let bytes = std::slice::from_raw_parts(
                    &dims as *const Dims as *const u8,
                    std::mem::size_of::<Dims>(),
                );
                enc.push(bytes, 4);
                // 2D dispatch: (half threads, n_rows groups)
                enc.launch_groups((1, n_rows as usize, 1), (half, 1, 1));
            });
        });
    }
    Ok(out)
}
