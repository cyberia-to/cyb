//! Metal canonical Q8 fused dequant+matmul.
//!
//! Block layout (34 bytes, 32 values):
//!   i16 scale (8.8 fixed-point)   — bytes 0..2
//!   i8  qs[32]                    — bytes 2..34
//! Dequant: x[i] = qs[i] * (scale_i16 / 256) / 127
//!
//! Output: y[b, row] = sum_k x[b, k] * dequant(W[row, k])
//!
//! One SIMD group (32 threads) computes one (batch, row).
//! Each thread walks stride-32 over K blocks, simd_sum reduces.
//! SIMDS_PER_GROUP=8 → 8 rows per threadgroup, 256 threads total.
//! Inner loop uses float4 vectorised FMA for higher ALU throughput.

use crate::backend::BackendError;
use crate::backend::honeycrisp::device::HoneycrispDevice;

pub const BLOCK_SIZE: usize = 32;
pub const BLOCK_BYTES: usize = 34;
pub const SIMDS_PER_GROUP: u32 = 16;
pub const N_DST: u32 = 1;
/// Threadgroup-cached pipeline max blocks. For n_blocks > this, use the
/// no-cache pipeline (MSL_LARGE) to avoid threadgroup memory overflow.
pub const TG_MAX_BLOCKS: usize = 128;

const MSL_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = __MAX_BLOCKS__;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const uchar   *w      [[buffer(1)]],
    device       float   *y      [[buffer(2)]],
    constant     Dims    &dims   [[buffer(3)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *w_row = w + row * dims.n_blocks * 34u;

    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 34u;
        device const short *scale_p = (device const short *)(block);
        float scale = float(int(*scale_p)) * (1.0f / 256.0f / 127.0f);

        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);
        device const char2 *qs2 = (device const char2 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a  = qs2[c * 2u];
            char2 b2 = qs2[c * 2u + 1u];
            float4 q  = float4(float(a.x), float(a.y), float(b2.x), float(b2.y));
            acc4 = fma(q, xb4[c], acc4);
        }
        my_sum = fma(scale, acc4.x + acc4.y + acc4.z + acc4.w, my_sum);
    }
    float reduced = simd_sum(my_sum);
    if (lane == 0) {
        y[b * dims.n_rows + row] = reduced;
    }
}
"#;

pub fn msl_mb(n: usize) -> String { MSL_TEMPLATE.replace("__MAX_BLOCKS__", &n.to_string()) }
pub fn msl() -> String { msl_mb(128) }

pub fn msl_nrm_mb(n: usize) -> String { MSL_NRM_TEMPLATE.replace("__MAX_BLOCKS__", &n.to_string()) }
pub fn msl_nrm() -> String { msl_nrm_mb(128) }
pub fn msl_dual_nrm_mb(n: usize) -> String { MSL_DUAL_NRM_TEMPLATE.replace("__MAX_BLOCKS__", &n.to_string()) }
pub fn msl_dual_nrm() -> String { msl_dual_nrm_mb(128) }

const MSL_NRM_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = __MAX_BLOCKS__;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *w      [[buffer(2)]],
    device       float   *y      [[buffer(3)]],
    constant     Dims    &dims   [[buffer(4)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];
    threadgroup float tg_sq[SIMDS_PER_GROUP];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) { x_shared[i] = x_b[i]; }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    float sq = 0.0f;
    for (uint i = tid; i < k_floats; i += TGSIZE) { sq += x_shared[i] * x_shared[i]; }
    float simd_sq = simd_sum(sq);
    if (lane == 0u) tg_sq[sgitg] = simd_sq;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float total_sq = (lane < SIMDS_PER_GROUP) ? tg_sq[lane] : 0.0f;
    float inv_rms = rsqrt(simd_sum(total_sq) / k_floats + dims.eps);
    for (uint i = tid; i < k_floats; i += TGSIZE) { x_shared[i] *= inv_rms * norm_w[i]; }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *w_row = w + row * dims.n_blocks * 34u;
    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 34u;
        float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
        device const char2 *q2 = (device const char2 *)(block + 2u);
        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);
        float4 acc = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a0 = q2[c*2u]; char2 a1 = q2[c*2u+1u];
            acc = fma(float4(float(a0.x),float(a0.y),float(a1.x),float(a1.y)), xb4[c], acc);
        }
        my_sum = fma(scale, acc.x + acc.y + acc.z + acc.w, my_sum);
    }
    float total = simd_sum(my_sum);
    if (lane == 0) { y[b * dims.n_rows + row] = total; }
}
"#;

const MSL_DUAL_NRM_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = __MAX_BLOCKS__;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *wa     [[buffer(2)]],
    device const uchar   *wb     [[buffer(3)]],
    device       float   *ya     [[buffer(4)]],
    device       float   *yb     [[buffer(5)]],
    constant     Dims    &dims   [[buffer(6)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];
    threadgroup float tg_sq[SIMDS_PER_GROUP];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) { x_shared[i] = x_b[i]; }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    float sq = 0.0f;
    for (uint i = tid; i < k_floats; i += TGSIZE) { sq += x_shared[i] * x_shared[i]; }
    float simd_sq = simd_sum(sq);
    if (lane == 0u) tg_sq[sgitg] = simd_sq;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float total_sq = (lane < SIMDS_PER_GROUP) ? tg_sq[lane] : 0.0f;
    float inv_rms = rsqrt(simd_sum(total_sq) / k_floats + dims.eps);
    for (uint i = tid; i < k_floats; i += TGSIZE) { x_shared[i] *= inv_rms * norm_w[i]; }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *wa_row = wa + row * dims.n_blocks * 34u;
    device const uchar *wb_row = wb + row * dims.n_blocks * 34u;
    float sum_a = 0.0f, sum_b = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);
        device const uchar *ba = wa_row + blk * 34u;
        float scale_a = float(int(*((device const short *)ba))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qa2 = (device const char2 *)(ba + 2u);
        float4 acc_a = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a0 = qa2[c*2u]; char2 a1 = qa2[c*2u+1u];
            acc_a = fma(float4(float(a0.x),float(a0.y),float(a1.x),float(a1.y)), xb4[c], acc_a);
        }
        sum_a = fma(scale_a, acc_a.x + acc_a.y + acc_a.z + acc_a.w, sum_a);
        device const uchar *bb = wb_row + blk * 34u;
        float scale_b = float(int(*((device const short *)bb))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qb2 = (device const char2 *)(bb + 2u);
        float4 acc_b = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 b0 = qb2[c*2u]; char2 b1 = qb2[c*2u+1u];
            acc_b = fma(float4(float(b0.x),float(b0.y),float(b1.x),float(b1.y)), xb4[c], acc_b);
        }
        sum_b = fma(scale_b, acc_b.x + acc_b.y + acc_b.z + acc_b.w, sum_b);
    }
    float ta = simd_sum(sum_a), tb = simd_sum(sum_b);
    if (lane == 0) { uint oi = b * dims.n_rows + row; ya[oi] = ta; yb[oi] = tb; }
}
"#;

/// Fused Q/K/V Q8 triple dispatch. Buffers: 0=x, 1=wq, 2=wk, 3=wv, 4=yq, 5=yk, 6=yv, 7=dims.
pub const MSL_QKV: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct DimsQKV { uint q_rows; uint kv_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = 128;

kernel void kmain(
    device const float   *x    [[buffer(0)]],
    device const uchar   *wq   [[buffer(1)]],
    device const uchar   *wk   [[buffer(2)]],
    device const uchar   *wv   [[buffer(3)]],
    device       float   *yq   [[buffer(4)]],
    device       float   *yk   [[buffer(5)]],
    device       float   *yv   [[buffer(6)]],
    constant     DimsQKV &dims [[buffer(7)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint total_rows = dims.q_rows + 2u * dims.kv_rows;
    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint row_local  = row_global % total_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    for (uint i = tid; i < k_floats; i += TGSIZE) {
        x_shared[i] = x[i];
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (row_global >= total_rows) return;

    device const uchar *w_row;
    device       float *y_out;
    uint out_row;
    if (row_local < dims.q_rows) {
        w_row   = wq + row_local * dims.n_blocks * 34u;
        y_out   = yq;
        out_row = row_local;
    } else if (row_local < dims.q_rows + dims.kv_rows) {
        uint kr = row_local - dims.q_rows;
        w_row   = wk + kr * dims.n_blocks * 34u;
        y_out   = yk;
        out_row = kr;
    } else {
        uint vr = row_local - dims.q_rows - dims.kv_rows;
        w_row   = wv + vr * dims.n_blocks * 34u;
        y_out   = yv;
        out_row = vr;
    }

    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 34u;
        device const short *scale_p = (device const short *)(block);
        float scale = float(int(*scale_p)) * (1.0f / 256.0f / 127.0f);

        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);
        device const char2 *qs2 = (device const char2 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a  = qs2[c * 2u];
            char2 b2 = qs2[c * 2u + 1u];
            float4 q  = float4(float(a.x), float(a.y), float(b2.x), float(b2.y));
            acc4 = fma(q, xb4[c], acc4);
        }
        my_sum = fma(scale, acc4.x + acc4.y + acc4.z + acc4.w, my_sum);
    }
    float total = simd_sum(my_sum);
    if (lane == 0) {
        y_out[out_row] = total;
    }
}
"#;

pub fn msl_dual_mb(n: usize) -> String { MSL_DUAL_TEMPLATE.replace("__MAX_BLOCKS__", &n.to_string()) }
pub fn msl_dual() -> String { msl_dual_mb(128) }

/// Fused gate+up for Q8.
const MSL_DUAL_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = __MAX_BLOCKS__;

kernel void kmain(
    device const float   *x   [[buffer(0)]],
    device const uchar   *wa  [[buffer(1)]],
    device const uchar   *wb  [[buffer(2)]],
    device       float   *ya  [[buffer(3)]],
    device       float   *yb  [[buffer(4)]],
    constant     Dims    &dims [[buffer(5)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *wa_row = wa + row * dims.n_blocks * 34u;
    device const uchar *wb_row = wb + row * dims.n_blocks * 34u;
    float sum_a = 0.0f, sum_b = 0.0f;

    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);

        device const uchar *ba = wa_row + blk * 34u;
        float scale_a = float(int(*((device const short *)ba))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qa2 = (device const char2 *)(ba + 2u);
        float4 acc_a = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a0 = qa2[c * 2u]; char2 a1 = qa2[c * 2u + 1u];
            acc_a = fma(float4(float(a0.x), float(a0.y), float(a1.x), float(a1.y)), xb4[c], acc_a);
        }
        sum_a = fma(scale_a, acc_a.x + acc_a.y + acc_a.z + acc_a.w, sum_a);

        device const uchar *bb = wb_row + blk * 34u;
        float scale_b = float(int(*((device const short *)bb))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qb2 = (device const char2 *)(bb + 2u);
        float4 acc_b = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 b0 = qb2[c * 2u]; char2 b1 = qb2[c * 2u + 1u];
            acc_b = fma(float4(float(b0.x), float(b0.y), float(b1.x), float(b1.y)), xb4[c], acc_b);
        }
        sum_b = fma(scale_b, acc_b.x + acc_b.y + acc_b.z + acc_b.w, sum_b);
    }

    float ta = simd_sum(sum_a), tb = simd_sum(sum_b);
    if (lane == 0) {
        uint out_idx = b * dims.n_rows + row;
        ya[out_idx] = ta;
        yb[out_idx] = tb;
    }
}
"#;

pub fn msl_gus_mb(n: usize) -> String { MSL_GUS_TEMPLATE.replace("__MAX_BLOCKS__", &n.to_string()) }
pub fn msl_gus() -> String { msl_gus_mb(128) }

/// gate+up+silu fused: computes silu(gate) * up → y in one dispatch.
/// Buffers: 0=x, 1=wa(gate), 2=wb(up), 3=y(mid=silu(gate)*up), 4=dims.
const MSL_GUS_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = __MAX_BLOCKS__;

kernel void kmain(
    device const float   *x   [[buffer(0)]],
    device const uchar   *wa  [[buffer(1)]],  // gate weights
    device const uchar   *wb  [[buffer(2)]],  // up weights
    device       float   *y   [[buffer(3)]],  // mid = silu(gate)*up
    constant     Dims    &dims [[buffer(4)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *wa_row = wa + row * dims.n_blocks * 34u;
    device const uchar *wb_row = wb + row * dims.n_blocks * 34u;
    float sum_a = 0.0f, sum_b = 0.0f;

    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);

        device const uchar *ba = wa_row + blk * 34u;
        float scale_a = float(int(*((device const short *)ba))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qa2 = (device const char2 *)(ba + 2u);
        float4 acc_a = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a0 = qa2[c * 2u]; char2 a1 = qa2[c * 2u + 1u];
            acc_a = fma(float4(float(a0.x), float(a0.y), float(a1.x), float(a1.y)), xb4[c], acc_a);
        }
        sum_a = fma(scale_a, acc_a.x + acc_a.y + acc_a.z + acc_a.w, sum_a);

        device const uchar *bb = wb_row + blk * 34u;
        float scale_b = float(int(*((device const short *)bb))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qb2 = (device const char2 *)(bb + 2u);
        float4 acc_b = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 b0 = qb2[c * 2u]; char2 b1 = qb2[c * 2u + 1u];
            acc_b = fma(float4(float(b0.x), float(b0.y), float(b1.x), float(b1.y)), xb4[c], acc_b);
        }
        sum_b = fma(scale_b, acc_b.x + acc_b.y + acc_b.z + acc_b.w, sum_b);
    }

    float ta = simd_sum(sum_a), tb = simd_sum(sum_b);
    if (lane == 0) {
        float gate = ta;
        float silu = gate * (1.0f / (1.0f + exp(-gate)));
        y[b * dims.n_rows + row] = silu * tb;
    }
}
"#;

pub fn msl_gus_nrm_mb(n: usize) -> String { MSL_GUS_NRM_TEMPLATE.replace("__MAX_BLOCKS__", &n.to_string()) }
pub fn msl_gus_nrm() -> String { msl_gus_nrm_mb(128) }

/// 4-rows-per-SIMD gate+up+silu+norm with TG x-cache (MAX_BLOCKS=64).
/// Same 8 KB threadgroup memory as MB64 variant; ROWS=4 reduces TG count
/// from n_rows/16 to n_rows/64 (560→140 for 8960 rows, fitting one wave).
/// Buffers: 0=x, 1=norm_w, 2=wa(gate), 3=wb(up), 4=y, 5=Dims.
pub const MSL_GUS_NRM_MB64_R4: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = 64;
constant constexpr uint ROWS = 4;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *wa     [[buffer(2)]],
    device const uchar   *wb     [[buffer(3)]],
    device       float   *y      [[buffer(4)]],
    constant     Dims    &dims   [[buffer(5)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];
    threadgroup float tg_sq[SIMDS_PER_GROUP];

    uint row_global = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = row_global / dims.n_rows;
    uint row0 = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) { x_shared[i] = x_b[i]; }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    float sq = 0.0f;
    for (uint i = tid; i < k_floats; i += TGSIZE) { sq += x_shared[i] * x_shared[i]; }
    float simd_sq = simd_sum(sq);
    if (lane == 0u) tg_sq[sgitg] = simd_sq;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float total_sq = (lane < SIMDS_PER_GROUP) ? tg_sq[lane] : 0.0f;
    float inv_rms = rsqrt(simd_sum(total_sq) / k_floats + dims.eps);
    for (uint i = tid; i < k_floats; i += TGSIZE) { x_shared[i] *= inv_rms * norm_w[i]; }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row0 >= dims.n_rows) return;

    float gate_acc[ROWS] = {0.0f, 0.0f, 0.0f, 0.0f};
    float up_acc[ROWS]   = {0.0f, 0.0f, 0.0f, 0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *ba = wa + row * dims.n_blocks * 34u + blk * 34u;
            device const uchar *bb = wb + row * dims.n_blocks * 34u + blk * 34u;
            float sa = float(int(*((device const short *)ba))) * (1.0f / 256.0f / 127.0f);
            float sb = float(int(*((device const short *)bb))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qa2 = (device const char2 *)(ba + 2u);
            device const char2 *qb2 = (device const char2 *)(bb + 2u);
            float4 ag4 = float4(0.0f), bg4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                float4 xn = xb4[c];
                char2 a0=qa2[c*2u],a1=qa2[c*2u+1u]; char2 b0=qb2[c*2u],b1=qb2[c*2u+1u];
                ag4 = fma(float4(float(a0.x),float(a0.y),float(a1.x),float(a1.y)), xn, ag4);
                bg4 = fma(float4(float(b0.x),float(b0.y),float(b1.x),float(b1.y)), xn, bg4);
            }
            gate_acc[r] = fma(sa, ag4.x+ag4.y+ag4.z+ag4.w, gate_acc[r]);
            up_acc[r]   = fma(sb, bg4.x+bg4.y+bg4.z+bg4.w, up_acc[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float g = simd_sum(gate_acc[r]);
        float u = simd_sum(up_acc[r]);
        if (lane == 0) {
            float sig = 1.0f / (1.0f + exp(-g));
            y[b * dims.n_rows + row] = g * sig * u;
        }
    }
}
"#;

const MSL_GUS_NRM_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = __MAX_BLOCKS__;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *wa     [[buffer(2)]],
    device const uchar   *wb     [[buffer(3)]],
    device       float   *y      [[buffer(4)]],
    constant     Dims    &dims   [[buffer(5)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];
    threadgroup float tg_sq[SIMDS_PER_GROUP];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    float sq = 0.0f;
    for (uint i = tid; i < k_floats; i += TGSIZE) { sq += x_shared[i] * x_shared[i]; }
    float simd_sq = simd_sum(sq);
    if (lane == 0u) tg_sq[sgitg] = simd_sq;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    float total_sq = (lane < SIMDS_PER_GROUP) ? tg_sq[lane] : 0.0f;
    float inv_rms = rsqrt(simd_sum(total_sq) / k_floats + dims.eps);
    for (uint i = tid; i < k_floats; i += TGSIZE) { x_shared[i] *= inv_rms * norm_w[i]; }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *wa_row = wa + row * dims.n_blocks * 34u;
    device const uchar *wb_row = wb + row * dims.n_blocks * 34u;
    float sum_a = 0.0f, sum_b = 0.0f;

    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);

        device const uchar *ba = wa_row + blk * 34u;
        float scale_a = float(int(*((device const short *)ba))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qa2 = (device const char2 *)(ba + 2u);
        float4 acc_a = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a0 = qa2[c * 2u]; char2 a1 = qa2[c * 2u + 1u];
            acc_a = fma(float4(float(a0.x), float(a0.y), float(a1.x), float(a1.y)), xb4[c], acc_a);
        }
        sum_a = fma(scale_a, acc_a.x + acc_a.y + acc_a.z + acc_a.w, sum_a);

        device const uchar *bb = wb_row + blk * 34u;
        float scale_b = float(int(*((device const short *)bb))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qb2 = (device const char2 *)(bb + 2u);
        float4 acc_b = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 b0 = qb2[c * 2u]; char2 b1 = qb2[c * 2u + 1u];
            acc_b = fma(float4(float(b0.x), float(b0.y), float(b1.x), float(b1.y)), xb4[c], acc_b);
        }
        sum_b = fma(scale_b, acc_b.x + acc_b.y + acc_b.z + acc_b.w, sum_b);
    }

    float ta = simd_sum(sum_a), tb = simd_sum(sum_b);
    if (lane == 0) {
        float gate = ta;
        float silu = gate * (1.0f / (1.0f + exp(-gate)));
        y[b * dims.n_rows + row] = silu * tb;
    }
}
"#;

/// Matmul + residual add fused: y[row] = matmul(x, w)[row] + residual[row].
/// Buffers: 0=x, 1=w, 2=residual, 3=y, 4=dims.
const MSL_RES_TEMPLATE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint TGSIZE = SIMDS_PER_GROUP * LANES;  // 512
constant constexpr uint MAX_BLOCKS = __MAX_BLOCKS__;

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    threadgroup float x_shared[MAX_BLOCKS * 32];

    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    uint k_floats = dims.n_blocks * 32u;
    uint tid = sgitg * LANES + lane;
    if (b < dims.batch) {
        device const float *x_b = x + b * k_floats;
        for (uint i = tid; i < k_floats; i += TGSIZE) {
            x_shared[i] = x_b[i];
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (b >= dims.batch || row >= dims.n_rows) return;

    device const uchar *w_row = w + row * dims.n_blocks * 34u;

    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 34u;
        device const short *scale_p = (device const short *)(block);
        float scale = float(int(*scale_p)) * (1.0f / 256.0f / 127.0f);

        threadgroup const float4 *xb4 = (threadgroup const float4 *)(&x_shared[blk * 32u]);
        device const char2 *qs2 = (device const char2 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a  = qs2[c * 2u];
            char2 b2 = qs2[c * 2u + 1u];
            float4 q  = float4(float(a.x), float(a.y), float(b2.x), float(b2.y));
            acc4 = fma(q, xb4[c], acc4);
        }
        my_sum = fma(scale, acc4.x + acc4.y + acc4.z + acc4.w, my_sum);
    }
    float reduced = simd_sum(my_sum);
    if (lane == 0) {
        uint idx = b * dims.n_rows + row;
        y[idx] = reduced + residual[idx];
    }
}
"#;

pub fn msl_res_mb(n: usize) -> String { MSL_RES_TEMPLATE.replace("__MAX_BLOCKS__", &n.to_string()) }
pub fn msl_res() -> String { msl_res_mb(128) }

/// No-threadgroup-cache Q8 matmul: reads x directly from device memory.
/// Correct for any n_blocks (no threadgroup memory overflow risk).
/// Used when n_blocks > TG_MAX_BLOCKS (e.g., intermediate_size=8960 → n_blocks=280).
pub const MSL_LARGE: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const uchar   *w      [[buffer(1)]],
    device       float   *y      [[buffer(2)]],
    constant     Dims    &dims   [[buffer(3)]],
    uint         tgpig_x [[threadgroup_position_in_grid]],
    uint         lane    [[thread_index_in_simdgroup]],
    uint         sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;

    if (b >= dims.batch || row >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;
    device const uchar *w_row = w + row * dims.n_blocks * 34u;

    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 34u;
        float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const char2 *qs2 = (device const char2 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a  = qs2[c * 2u];
            char2 b2 = qs2[c * 2u + 1u];
            float4 q  = float4(float(a.x), float(a.y), float(b2.x), float(b2.y));
            acc4 = fma(q, xb4[c], acc4);
        }
        my_sum = fma(scale, acc4.x + acc4.y + acc4.z + acc4.w, my_sum);
    }
    float reduced = simd_sum(my_sum);
    if (lane == 0) {
        y[b * dims.n_rows + row] = reduced;
    }
}
"#;

/// No-TG-cache + inline RMSnorm + Q8 matmul (2-pass within each SIMD group).
/// Works for any n_blocks — simd_sum covers all blocks assigned to the SIMD.
/// Buffers: 0=x(f32), 1=norm_w(f32), 2=w(Q8), 3=y(f32), 4=Dims.
pub const MSL_LARGE_NRM: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *w      [[buffer(2)]],
    device       float   *y      [[buffer(3)]],
    constant     Dims    &dims   [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;
    if (b >= dims.batch || row >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    // Pass 1: compute sum of squares for RMSnorm
    float sq_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        for (uint c = 0; c < 8u; c++) { float4 v = xb4[c]; sq_sum += dot(v, v); }
    }
    float inv_rms = rsqrt(simd_sum(sq_sum) / float(k_floats) + dims.eps);

    // Pass 2: matmul with norm applied inline
    device const uchar *w_row = w + row * dims.n_blocks * 34u;
    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 34u;
        float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const float4 *nb4 = (device const float4 *)(norm_w + blk * 32u);
        device const char2  *qs2 = (device const char2 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            float4 xn = xb4[c] * inv_rms * nb4[c];
            char2 a  = qs2[c * 2u];
            char2 b2 = qs2[c * 2u + 1u];
            float4 q = float4(float(a.x), float(a.y), float(b2.x), float(b2.y));
            acc4 += q * xn;
        }
        my_sum = fma(scale, acc4.x + acc4.y + acc4.z + acc4.w, my_sum);
    }
    float reduced = simd_sum(my_sum);
    if (lane == 0) y[b * dims.n_rows + row] = reduced;
}
"#;

/// No-TG-cache + inline RMSnorm + SwiGLU (gate × silu(gate) × up).
/// Works for any n_blocks. Buffers: 0=x, 1=norm_w, 2=wa(gate), 3=wb(up), 4=y, 5=Dims.
pub const MSL_LARGE_GUS_NRM: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *wa     [[buffer(2)]],
    device const uchar   *wb     [[buffer(3)]],
    device       float   *y      [[buffer(4)]],
    constant     Dims    &dims   [[buffer(5)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;
    if (b >= dims.batch || row >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    // Pass 1: RMSnorm
    float sq_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        for (uint c = 0; c < 8u; c++) { float4 v = xb4[c]; sq_sum += dot(v, v); }
    }
    float inv_rms = rsqrt(simd_sum(sq_sum) / float(k_floats) + dims.eps);

    // Pass 2: gate + up matmuls with norm inline, then SwiGLU
    device const uchar *wa_row = wa + row * dims.n_blocks * 34u;
    device const uchar *wb_row = wb + row * dims.n_blocks * 34u;
    float sum_a = 0.0f, sum_b = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const float4 *nb4 = (device const float4 *)(norm_w + blk * 32u);

        device const uchar *ba = wa_row + blk * 34u;
        float scale_a = float(int(*((device const short *)ba))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qa2 = (device const char2 *)(ba + 2u);

        device const uchar *bb = wb_row + blk * 34u;
        float scale_b = float(int(*((device const short *)bb))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qb2 = (device const char2 *)(bb + 2u);

        float4 acc_a = float4(0.0f), acc_b = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            float4 xn = xb4[c] * inv_rms * nb4[c];
            char2 a0 = qa2[c*2u], a1 = qa2[c*2u+1u];
            char2 b0 = qb2[c*2u], b1 = qb2[c*2u+1u];
            acc_a += float4(float(a0.x), float(a0.y), float(a1.x), float(a1.y)) * xn;
            acc_b += float4(float(b0.x), float(b0.y), float(b1.x), float(b1.y)) * xn;
        }
        sum_a = fma(scale_a, acc_a.x + acc_a.y + acc_a.z + acc_a.w, sum_a);
        sum_b = fma(scale_b, acc_b.x + acc_b.y + acc_b.z + acc_b.w, sum_b);
    }
    float gate = simd_sum(sum_a), up = simd_sum(sum_b);
    if (lane == 0) {
        y[b * dims.n_rows + row] = gate * (1.0f / (1.0f + exp(-gate))) * up;
    }
}
"#;

/// No-TG-cache Q8 matmul + residual add: y = matmul(x, w) + residual.
/// Works for any n_blocks. Buffers: 0=x, 1=w, 2=residual, 3=y, 4=Dims.
pub const MSL_LARGE_RES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };

constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint row_global = tgpig_x * SIMDS_PER_GROUP + sgitg;
    uint b   = row_global / dims.n_rows;
    uint row = row_global - b * dims.n_rows;
    if (b >= dims.batch || row >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;
    device const uchar *w_row = w + row * dims.n_blocks * 34u;

    float my_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const uchar *block = w_row + blk * 34u;
        float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const char2  *qs2 = (device const char2 *)(block + 2u);
        float4 acc4 = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 a  = qs2[c * 2u];
            char2 b2 = qs2[c * 2u + 1u];
            float4 q = float4(float(a.x), float(a.y), float(b2.x), float(b2.y));
            acc4 = fma(q, xb4[c], acc4);
        }
        my_sum = fma(scale, acc4.x + acc4.y + acc4.z + acc4.w, my_sum);
    }
    float reduced = simd_sum(my_sum);
    if (lane == 0) {
        uint idx = b * dims.n_rows + row;
        y[idx] = reduced + residual[idx];
    }
}
"#;

/// 4-rows-per-SIMD variant of LARGE_NRM. Each SIMD group computes 4 output
/// rows simultaneously, sharing the RMSnorm inverse across all 4. TG count
/// is 4× lower than MSL_LARGE_NRM (560→140 for gate+up at 8960 rows), cutting
/// GPU wave serialization from ~7 to ~1.75 on M4 Max with 40 GPU cores.
pub const MSL_LARGE4_NRM: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 4;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *w      [[buffer(2)]],
    device       float   *y      [[buffer(3)]],
    constant     Dims    &dims   [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    float sq_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        for (uint c = 0; c < 8u; c++) { float4 v = xb4[c]; sq_sum += dot(v, v); }
    }
    float inv_rms = rsqrt(simd_sum(sq_sum) / float(k_floats) + dims.eps);

    float sums[ROWS] = {0.0f, 0.0f, 0.0f, 0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const float4 *nb4 = (device const float4 *)(norm_w + blk * 32u);
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *block = w + row * dims.n_blocks * 34u + blk * 34u;
            float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qs2 = (device const char2 *)(block + 2u);
            float4 acc4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                float4 xn = xb4[c] * inv_rms * nb4[c];
                char2 a = qs2[c*2u]; char2 b2 = qs2[c*2u+1u];
                acc4 = fma(float4(float(a.x),float(a.y),float(b2.x),float(b2.y)), xn, acc4);
            }
            sums[r] = fma(scale, acc4.x+acc4.y+acc4.z+acc4.w, sums[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float reduced = simd_sum(sums[r]);
        if (lane == 0) y[b * dims.n_rows + row] = reduced;
    }
}
"#;

/// 4-rows-per-SIMD gate+up+silu+norm variant. Each SIMD computes 4 gate rows
/// and 4 up rows (8 total dot products), sharing one RMSnorm inverse.
/// Buffers: 0=x, 1=norm_w, 2=gate_w, 3=up_w, 4=y (silu(gate)*up), 5=Dims.
pub const MSL_LARGE4_GUS_NRM: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 4;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *wg     [[buffer(2)]],
    device const uchar   *wu     [[buffer(3)]],
    device       float   *y      [[buffer(4)]],
    constant     Dims    &dims   [[buffer(5)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    float sq_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        for (uint c = 0; c < 8u; c++) { float4 v = xb4[c]; sq_sum += dot(v, v); }
    }
    float inv_rms = rsqrt(simd_sum(sq_sum) / float(k_floats) + dims.eps);

    float gate[ROWS] = {0.0f,0.0f,0.0f,0.0f};
    float up[ROWS]   = {0.0f,0.0f,0.0f,0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const float4 *nb4 = (device const float4 *)(norm_w + blk * 32u);
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *bg = wg + row * dims.n_blocks * 34u + blk * 34u;
            device const uchar *bu = wu + row * dims.n_blocks * 34u + blk * 34u;
            float sg = float(int(*((device const short *)bg))) * (1.0f / 256.0f / 127.0f);
            float su = float(int(*((device const short *)bu))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qg = (device const char2 *)(bg + 2u);
            device const char2 *qu = (device const char2 *)(bu + 2u);
            float4 ag4 = float4(0.0f), au4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                float4 xn = xb4[c] * inv_rms * nb4[c];
                char2 g0=qg[c*2u],g1=qg[c*2u+1u]; char2 u0=qu[c*2u],u1=qu[c*2u+1u];
                ag4 = fma(float4(float(g0.x),float(g0.y),float(g1.x),float(g1.y)), xn, ag4);
                au4 = fma(float4(float(u0.x),float(u0.y),float(u1.x),float(u1.y)), xn, au4);
            }
            gate[r] = fma(sg, ag4.x+ag4.y+ag4.z+ag4.w, gate[r]);
            up[r]   = fma(su, au4.x+au4.y+au4.z+au4.w, up[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float g = simd_sum(gate[r]);
        float u = simd_sum(up[r]);
        if (lane == 0) {
            float sig = 1.0f / (1.0f + exp(-g));
            y[b * dims.n_rows + row] = g * sig * u;
        }
    }
}
"#;

/// 4-rows-per-SIMD gate+up+gelu_tanh+norm variant (Gemma-4 GeluTanh activation).
/// Same structure as MSL_LARGE4_GUS_NRM but uses gelu_pytorch_tanh instead of SiLU.
/// Buffers: 0=x, 1=norm_w, 2=gate_w, 3=up_w, 4=y (gelu(gate)*up), 5=Dims.
pub const MSL_LARGE4_GUS_NRM_GELU: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; float eps; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 4;

kernel void kmain(
    device const float   *x      [[buffer(0)]],
    device const float   *norm_w [[buffer(1)]],
    device const uchar   *wg     [[buffer(2)]],
    device const uchar   *wu     [[buffer(3)]],
    device       float   *y      [[buffer(4)]],
    constant     Dims    &dims   [[buffer(5)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    float sq_sum = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        for (uint c = 0; c < 8u; c++) { float4 v = xb4[c]; sq_sum += dot(v, v); }
    }
    float inv_rms = rsqrt(simd_sum(sq_sum) / float(k_floats) + dims.eps);

    float gate[ROWS] = {0.0f,0.0f,0.0f,0.0f};
    float up[ROWS]   = {0.0f,0.0f,0.0f,0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const float4 *nb4 = (device const float4 *)(norm_w + blk * 32u);
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *bg = wg + row * dims.n_blocks * 34u + blk * 34u;
            device const uchar *bu = wu + row * dims.n_blocks * 34u + blk * 34u;
            float sg = float(int(*((device const short *)bg))) * (1.0f / 256.0f / 127.0f);
            float su = float(int(*((device const short *)bu))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qg = (device const char2 *)(bg + 2u);
            device const char2 *qu = (device const char2 *)(bu + 2u);
            float4 ag4 = float4(0.0f), au4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                float4 xn = xb4[c] * inv_rms * nb4[c];
                char2 g0=qg[c*2u],g1=qg[c*2u+1u]; char2 u0=qu[c*2u],u1=qu[c*2u+1u];
                ag4 = fma(float4(float(g0.x),float(g0.y),float(g1.x),float(g1.y)), xn, ag4);
                au4 = fma(float4(float(u0.x),float(u0.y),float(u1.x),float(u1.y)), xn, au4);
            }
            gate[r] = fma(sg, ag4.x+ag4.y+ag4.z+ag4.w, gate[r]);
            up[r]   = fma(su, au4.x+au4.y+au4.z+au4.w, up[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float g = simd_sum(gate[r]);
        float u = simd_sum(up[r]);
        if (lane == 0) {
            // gelu_pytorch_tanh(g) = 0.5 * g * (1 + tanh(sqrt(2/pi) * (g + 0.044715*g^3)))
            // Clamp tanh arg: Metal fast-tanh returns NaN for |arg| > ~88; mathematically
            // tanh saturates to ±1 well before that (tanh(20) = 1.0f in float32).
            float tanh_arg = clamp(0.7978845608f * (g + 0.044715f * g * g * g), -20.0f, 20.0f);
            float gelu = 0.5f * g * (1.0f + tanh(tanh_arg));
            y[b * dims.n_rows + row] = gelu * u;
        }
    }
}
"#;

/// 2-rows-per-SIMD matmul + residual add. No norm (uses pre-normed input).
/// Buffers: 0=x, 1=w, 2=residual, 3=y, 4=Dims (eps field unused).
pub const MSL_LARGE2_RES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 2;

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    float sums[ROWS] = {0.0f, 0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *block = w + row * dims.n_blocks * 34u + blk * 34u;
            float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qs2 = (device const char2 *)(block + 2u);
            float4 acc4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                char2 a = qs2[c*2u]; char2 b2 = qs2[c*2u+1u];
                acc4 = fma(float4(float(a.x),float(a.y),float(b2.x),float(b2.y)), xb4[c], acc4);
            }
            sums[r] = fma(scale, acc4.x+acc4.y+acc4.z+acc4.w, sums[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float reduced = simd_sum(sums[r]);
        if (lane == 0) {
            uint idx = b * dims.n_rows + row;
            y[idx] = reduced + residual[idx];
        }
    }
}
"#;

/// 4-rows-per-SIMD matmul + residual add. No norm (uses pre-normed input).
/// Buffers: 0=x, 1=w, 2=residual, 3=y, 4=Dims (eps field unused).
pub const MSL_LARGE4_RES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 4;

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    float sums[ROWS] = {0.0f, 0.0f, 0.0f, 0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *block = w + row * dims.n_blocks * 34u + blk * 34u;
            float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qs2 = (device const char2 *)(block + 2u);
            float4 acc4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                char2 a = qs2[c*2u]; char2 b2 = qs2[c*2u+1u];
                acc4 = fma(float4(float(a.x),float(a.y),float(b2.x),float(b2.y)), xb4[c], acc4);
            }
            sums[r] = fma(scale, acc4.x+acc4.y+acc4.z+acc4.w, sums[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float reduced = simd_sum(sums[r]);
        if (lane == 0) {
            uint idx = b * dims.n_rows + row;
            y[idx] = reduced + residual[idx];
        }
    }
}
"#;

/// 8-rows-per-SIMD matmul + residual add. No norm (uses pre-normed input).
/// Buffers: 0=x, 1=w, 2=residual, 3=y, 4=Dims (eps field unused).
pub const MSL_LARGE8_RES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 8;

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    float sums[ROWS] = {0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *block = w + row * dims.n_blocks * 34u + blk * 34u;
            float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qs2 = (device const char2 *)(block + 2u);
            float4 acc4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                char2 a = qs2[c*2u]; char2 b2 = qs2[c*2u+1u];
                acc4 = fma(float4(float(a.x),float(a.y),float(b2.x),float(b2.y)), xb4[c], acc4);
            }
            sums[r] = fma(scale, acc4.x+acc4.y+acc4.z+acc4.w, sums[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float reduced = simd_sum(sums[r]);
        if (lane == 0) {
            uint idx = b * dims.n_rows + row;
            y[idx] = reduced + residual[idx];
        }
    }
}
"#;

/// 4-rows-per-SIMD matmul + residual add, TRANSPOSED weight layout.
/// Buffers: 0=x, 1=w_transposed, 2=residual, 3=y, 4=Dims.
pub const MSL_LARGE4T_RES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 4;

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    float sums[ROWS] = {0.0f, 0.0f, 0.0f, 0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const uchar *blk_base = w + blk * dims.n_rows * 34u + row0 * 34u;
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *block = blk_base + r * 34u;
            float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qs2 = (device const char2 *)(block + 2u);
            float4 acc4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                char2 a = qs2[c*2u]; char2 b2 = qs2[c*2u+1u];
                acc4 = fma(float4(float(a.x),float(a.y),float(b2.x),float(b2.y)), xb4[c], acc4);
            }
            sums[r] = fma(scale, acc4.x+acc4.y+acc4.z+acc4.w, sums[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float reduced = simd_sum(sums[r]);
        if (lane == 0) {
            uint idx = b * dims.n_rows + row;
            y[idx] = reduced + residual[idx];
        }
    }
}
"#;

/// 16-rows-per-SIMD matmul + residual add, TRANSPOSED weight layout.
/// Buffers: 0=x, 1=w_transposed, 2=residual, 3=y, 4=Dims.
pub const MSL_LARGE16T_RES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 16;

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    float sums[ROWS] = {0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f,0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        device const uchar *blk_base = w + blk * dims.n_rows * 34u + row0 * 34u;
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *block = blk_base + r * 34u;
            float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qs2 = (device const char2 *)(block + 2u);
            float4 acc4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                char2 a = qs2[c*2u]; char2 b2 = qs2[c*2u+1u];
                acc4 = fma(float4(float(a.x),float(a.y),float(b2.x),float(b2.y)), xb4[c], acc4);
            }
            sums[r] = fma(scale, acc4.x+acc4.y+acc4.z+acc4.w, sums[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float reduced = simd_sum(sums[r]);
        if (lane == 0) {
            uint idx = b * dims.n_rows + row;
            y[idx] = reduced + residual[idx];
        }
    }
}
"#;

/// 8-rows-per-SIMD matmul + residual add, TRANSPOSED weight layout.
/// Weights stored as [n_blocks, n_rows, 34] (block-major) instead of [n_rows, n_blocks, 34].
/// Consecutive rows for the same block are contiguous → ~3× better cache line utilization.
/// Buffers: 0=x, 1=w_transposed, 2=residual, 3=y, 4=Dims.
pub const MSL_LARGE8T_RES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };
constant constexpr uint SIMDS_PER_GROUP = 16;
constant constexpr uint LANES = 32;
constant constexpr uint ROWS  = 8;

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    uint first_row = (tgpig_x * SIMDS_PER_GROUP + sgitg) * ROWS;
    uint b   = first_row / dims.n_rows;
    uint row0 = first_row - b * dims.n_rows;
    if (b >= dims.batch || row0 >= dims.n_rows) return;

    uint k_floats = dims.n_blocks * 32u;
    device const float *x_b = x + b * k_floats;

    // Transposed weight layout: block blk, row r → w + blk * n_rows * 34 + r * 34
    // Consecutive rows at same blk are 34 bytes apart (contiguous cache lines).
    float sums[ROWS] = {0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f};
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        device const float4 *xb4 = (device const float4 *)(x_b + blk * 32u);
        // Base pointer for this block in transposed layout: rows row0..row0+ROWS-1 are contiguous
        device const uchar *blk_base = w + blk * dims.n_rows * 34u + row0 * 34u;
        for (uint r = 0; r < ROWS; r++) {
            uint row = row0 + r;
            if (row >= dims.n_rows) continue;
            device const uchar *block = blk_base + r * 34u;
            float scale = float(int(*((device const short *)block))) * (1.0f / 256.0f / 127.0f);
            device const char2 *qs2 = (device const char2 *)(block + 2u);
            float4 acc4 = float4(0.0f);
            for (uint c = 0; c < 8u; c++) {
                char2 a = qs2[c*2u]; char2 b2 = qs2[c*2u+1u];
                acc4 = fma(float4(float(a.x),float(a.y),float(b2.x),float(b2.y)), xb4[c], acc4);
            }
            sums[r] = fma(scale, acc4.x+acc4.y+acc4.z+acc4.w, sums[r]);
        }
    }
    for (uint r = 0; r < ROWS; r++) {
        uint row = row0 + r;
        if (row >= dims.n_rows) break;
        float reduced = simd_sum(sums[r]);
        if (lane == 0) {
            uint idx = b * dims.n_rows + row;
            y[idx] = reduced + residual[idx];
        }
    }
}
"#;

/// 16-SIMD cooperative FP16 x-cache + residual, for large n_blocks (up to 320).
/// All 16 SIMDs per TG cooperate to load x once into 20 KB of threadgroup FP16 memory,
/// then each SIMD independently computes one output row from TG memory.
/// 1 barrier vs 15+ for tiled-cache variants → low overhead for large n_blocks.
/// Buffers: 0=x, 1=w, 2=residual, 3=y, 4=Dims.
pub const MSL_MBX16_RES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Dims { uint batch; uint n_rows; uint n_blocks; uint pad; };
constant constexpr uint SIMDS  = 16;
constant constexpr uint LANES  = 32;
constant constexpr uint TG_SZ  = SIMDS * LANES;  // 512 threads per TG
constant constexpr uint MAX_BLOCKS = 320;         // supports n_blocks up to 320

kernel void kmain(
    device const float   *x        [[buffer(0)]],
    device const uchar   *w        [[buffer(1)]],
    device const float   *residual [[buffer(2)]],
    device       float   *y        [[buffer(3)]],
    constant     Dims    &dims     [[buffer(4)]],
    uint tgpig_x [[threadgroup_position_in_grid]],
    uint lane    [[thread_index_in_simdgroup]],
    uint sgitg   [[simdgroup_index_in_threadgroup]]
) {
    // Each TG handles SIMDS consecutive rows. One SIMD per row.
    uint row = tgpig_x * SIMDS + sgitg;
    uint b   = row / dims.n_rows;
    uint r   = row - b * dims.n_rows;
    bool valid = (b < dims.batch) && (r < dims.n_rows);

    // Cooperative FP16 x-cache: 320 * 32 * 2 = 20480 bytes = 20 KB
    threadgroup half x_tg[MAX_BLOCKS * LANES];

    uint k_floats = dims.n_blocks * LANES;
    device const float *x_b = x + b * k_floats;

    // Phase 1: all 512 threads cooperatively load x into x_tg as FP16
    uint tid = sgitg * LANES + lane;
    for (uint blk = tid; blk < dims.n_blocks; blk += TG_SZ) {
        device const float4 *src = (device const float4 *)(x_b + blk * LANES);
        threadgroup half4 *dst   = (threadgroup half4 *)(x_tg + blk * LANES);
        for (uint c = 0; c < 8u; c++) dst[c] = half4(src[c]);
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);

    if (!valid) return;

    // Phase 2: each SIMD computes its row's dot product from TG x-cache
    float partial = 0.0f;
    for (uint blk = lane; blk < dims.n_blocks; blk += LANES) {
        threadgroup const half4 *xh = (threadgroup const half4 *)(x_tg + blk * LANES);
        device const uchar *blk_ptr = w + r * dims.n_blocks * 34u + blk * 34u;
        float scale = float(int(*((device const short *)blk_ptr))) * (1.0f / 256.0f / 127.0f);
        device const char2 *qs2 = (device const char2 *)(blk_ptr + 2u);
        float4 acc4 = float4(0.0f);
        for (uint c = 0; c < 8u; c++) {
            char2 q0 = qs2[c*2u]; char2 q1 = qs2[c*2u+1u];
            half4 xv = xh[c];
            acc4 = fma(float4(float(q0.x),float(q0.y),float(q1.x),float(q1.y)),
                       float4(xv.x, xv.y, xv.z, xv.w), acc4);
        }
        partial += scale * (acc4.x + acc4.y + acc4.z + acc4.w);
    }

    float total = simd_sum(partial);
    if (lane == 0) {
        uint idx = b * dims.n_rows + r;
        y[idx] = total + residual[idx];
    }
}
"#;

pub fn dispatch(
    dev: &HoneycrispDevice,
    pipeline: &aruminium::Pipeline,
    x: &aruminium::Buffer,
    w: &aruminium::Buffer,
    batch: u32,
    n_rows: u32,
    n_blocks: u32,
) -> Result<aruminium::Buffer, BackendError> {
    let total_rows = batch * n_rows;
    let out = dev.alloc((total_rows * 4) as usize)?;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
    let dims = Dims { batch, n_rows, n_blocks, pad: 0 };

    let rows_per_tg = N_DST * SIMDS_PER_GROUP;
    let groups_x = (total_rows + rows_per_tg - 1) / rows_per_tg;
    let threads_per_group = SIMDS_PER_GROUP * 32;

    unsafe {
        aruminium::autorelease_pool(|| {
            dev.dispatch.batch_raw(|enc| {
                enc.bind(pipeline);
                enc.bind_buffer(x, 0, 0);
                enc.bind_buffer(w, 0, 1);
                enc.bind_buffer(&out, 0, 2);
                let bytes = std::slice::from_raw_parts(
                    &dims as *const Dims as *const u8,
                    std::mem::size_of::<Dims>(),
                );
                enc.push(bytes, 3);
                enc.launch_groups(
                    (groups_x as usize, 1, 1),
                    (threads_per_group as usize, 1, 1),
                );
            });
        });
    }
    Ok(out)
}
