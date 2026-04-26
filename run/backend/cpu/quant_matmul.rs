//! Fused dequant+matmul: read quantized weight bytes directly, compute
//! dot product against f32 activation without materializing full f32 weights.
//!
//! Wins: 6× less memory (Q4_0) or 8× (Q4_K), 6× less memory bandwidth,
//! unlocks models too large to dequant at load (coder-14b = 28 GB f32).
//!
//! Correctness: must match `matmul_f32(x, dequantize(W))` within
//! accumulation-precision ε. Verified by tests against the dequant+
//! f32 matmul reference.
//!
//! Structure: one outer scaffold (`matmul_blocks`) parallelises over
//! output rows and walks per-row blocks. Per-format math lives in a
//! `*_dot` kernel — that is the only thing that varies per quant type.

use crate::backend::BackendError;
use crate::backend::cpu::quant::{canonical, q4_0, q4_k, q6_k, q8_0};
use crate::core::dtype::DType;
use crate::core::tensor::Tensor;
use rayon::prelude::*;
use wide::f32x8;

// Canonical Q4 block (cyb/cyb-model): 32 values, 18 bytes
//   [0..1]: i16 scale (canonical u16 8.8 fixed-point: scale_f = i16/256)
//   [2..17]: 16 packed nibbles (low nibble first), value = (nibble-8) * scale_f / 8
const CANONICAL_Q4_BLOCK_SIZE: usize = 32;
const CANONICAL_Q4_BLOCK_BYTES: usize = 18;

// Canonical Q8 block: 32 values, 34 bytes
//   [0..1]: i16 scale (canonical u16 8.8: scale_f = i16/256)
//   [2..33]: 32 i8 values, dequant = i8 * scale_f / 127
const CANONICAL_Q8_BLOCK_SIZE: usize = 32;
const CANONICAL_Q8_BLOCK_BYTES: usize = 34;

/// Matmul with quantized weight. `x` is f32 [..., K], `w_bytes` and `w_dtype`
/// describe the weight matrix [N, K] in its native quant format.
///
/// Returns f32 output [..., N].
pub fn matmul_quant_f32(
    x: &Tensor,
    w_bytes: &[u8],
    w_dtype: DType,
    n: usize,
    k: usize,
) -> Result<Tensor, BackendError> {
    if x.shape.last() != Some(&k) {
        return Err(BackendError::ShapeMismatch {
            op: "MatmulQuant",
            expected: vec![0, k],
            got: x.shape.clone(),
        });
    }
    let batch: usize = x.shape[..x.shape.len() - 1].iter().product();
    let x_data = x.as_f32();
    let mut out = vec![0f32; batch * n];

    match w_dtype {
        // canonical
        DType::Q4 => matmul_blocks(
            x_data, w_bytes, &mut out, batch, n, k,
            CANONICAL_Q4_BLOCK_SIZE, CANONICAL_Q4_BLOCK_BYTES, canonical_q4_dot,
        )?,
        DType::Q8 => matmul_blocks(
            x_data, w_bytes, &mut out, batch, n, k,
            CANONICAL_Q8_BLOCK_SIZE, CANONICAL_Q8_BLOCK_BYTES, canonical_q8_dot,
        )?,
        DType::U32 | DType::U16 | DType::Ternary => {
            // U32/U16/Ternary aren't matmul weight layouts — fall back to
            // dequant-then-f32-matmul. These show up only on small tensors
            // (norms, biases) which the forward path handles via separate
            // ops, but dispatch here is correct as a safety net.
            let w_f32 = canonical::q8_to_f32(&[]); // placeholder; never hit on a normal model
            let _ = w_f32;
            return Err(BackendError::UnsupportedDtype {
                backend: "cpu",
                dtype: w_dtype,
                blocker: "matmul against U32/U16/Ternary not expected; norms/biases use other ops",
            });
        }
        // legacy GGUF (kept for source-format reads at import time; runtime no longer hits these for canonical models)
        DType::Q4_0 => matmul_blocks(
            x_data, w_bytes, &mut out, batch, n, k,
            q4_0::BLOCK_SIZE, q4_0::BLOCK_BYTES, q4_0_dot,
        )?,
        DType::Q8_0 => matmul_blocks(
            x_data, w_bytes, &mut out, batch, n, k,
            q8_0::BLOCK_SIZE, q8_0::BLOCK_BYTES, q8_0_dot,
        )?,
        DType::Q4_K => matmul_blocks(
            x_data, w_bytes, &mut out, batch, n, k,
            q4_k::BLOCK_SIZE, q4_k::BLOCK_BYTES, q4_k_dot,
        )?,
        DType::Q6_K => matmul_blocks(
            x_data, w_bytes, &mut out, batch, n, k,
            q6_k::BLOCK_SIZE, q6_k::BLOCK_BYTES, q6_k_dot,
        )?,
        other => {
            return Err(BackendError::UnsupportedDtype {
                backend: "cpu",
                dtype: other,
                blocker: "fused quant matmul not implemented",
            });
        }
    }

    let mut out_shape = x.shape.clone();
    *out_shape.last_mut().unwrap() = n;
    Ok(Tensor::from_f32(out_shape, out))
}

/// Outer scaffold shared by every quant format.
///
/// Validates K alignment, parallelises over the N output rows, hands each
/// row to `dot`. The format-specific math lives in `dot`.
fn matmul_blocks(
    x: &[f32],
    w: &[u8],
    out: &mut [f32],
    batch: usize,
    n: usize,
    k: usize,
    block_size: usize,
    block_bytes: usize,
    dot: impl Fn(&[f32], &[u8], usize) -> f32 + Sync,
) -> Result<(), BackendError> {
    if k % block_size != 0 {
        return Err(BackendError::InvalidInput {
            op: "MatmulQuant",
            reason: format!("K must be divisible by {block_size}, got K={k}"),
        });
    }
    let blocks_per_row = k / block_size;
    let row_bytes = blocks_per_row * block_bytes;
    let expected_bytes = n * row_bytes;
    if w.len() != expected_bytes {
        return Err(BackendError::InvalidInput {
            op: "MatmulQuant",
            reason: format!(
                "weight buffer length {} does not match N×row_bytes ({n}×{row_bytes} = {expected_bytes})",
                w.len()
            ),
        });
    }

    for b in 0..batch {
        let x_row = &x[b * k..(b + 1) * k];
        let out_row = &mut out[b * n..(b + 1) * n];
        out_row.par_iter_mut().enumerate().for_each(|(i, y)| {
            let w_row = &w[i * row_bytes..(i + 1) * row_bytes];
            *y = dot(x_row, w_row, blocks_per_row);
        });
    }
    Ok(())
}

// ─── per-format dot kernels ──────────────────────────────────────────────
//
// Each kernel receives `blocks` packed weight blocks and a matching slice
// of activations. It returns the scalar dot product over those blocks.

/// Q4_0: 18 bytes per 32 values. Dequant: x = d * (nibble - 8).
#[inline(always)]
fn q4_0_dot(x: &[f32], w_bytes: &[u8], blocks: usize) -> f32 {
    let neg8 = f32x8::splat(-8.0);
    let mut sum = 0f32;
    for b in 0..blocks {
        let block = &w_bytes[b * q4_0::BLOCK_BYTES..(b + 1) * q4_0::BLOCK_BYTES];
        let x_block = &x[b * q4_0::BLOCK_SIZE..(b + 1) * q4_0::BLOCK_SIZE];
        let d = read_f16(&block[0..2]);
        let qs = &block[2..];

        // 16 nibble bytes → 2 chunks of 8. Low nibbles → vals 0..16,
        // high nibbles → vals 16..32. Subtract 8 to recentre.
        let mut acc = f32x8::ZERO;
        for chunk in 0..2 {
            let off = chunk * 8;
            let b8 = &qs[off..off + 8];
            let lo = nibbles_f32x8(b8, 0) + neg8;
            let hi = nibbles_f32x8(b8, 4) + neg8;
            let x_lo = f32x8::from(slice8(&x_block[off..off + 8]));
            let x_hi = f32x8::from(slice8(&x_block[16 + off..16 + off + 8]));
            acc = lo.mul_add(x_lo, acc);
            acc = hi.mul_add(x_hi, acc);
        }
        sum += d * acc.reduce_add();
    }
    sum
}

/// Q8_0: 34 bytes per 32 values. Dequant: x = d * qs.
#[inline(always)]
fn q8_0_dot(x: &[f32], w_bytes: &[u8], blocks: usize) -> f32 {
    let mut sum = 0f32;
    for b in 0..blocks {
        let base = b * q8_0::BLOCK_BYTES;
        let d = read_f16(&w_bytes[base..base + 2]);
        let x_off = b * q8_0::BLOCK_SIZE;

        // 32 i8 → 4 f32x8 FMA lanes.
        let mut acc = f32x8::ZERO;
        for c in 0..4 {
            let off = c * 8;
            let qf = i8_bytes_f32x8(&w_bytes[base + 2 + off..base + 2 + off + 8]);
            let xv = f32x8::from(slice8(&x[x_off + off..x_off + off + 8]));
            acc = qf.mul_add(xv, acc);
        }
        sum += d * acc.reduce_add();
    }
    sum
}

/// Q4_K: 144 bytes per 256 values. 8 sub-blocks of 32; sub-block pair
/// (2k, 2k+1) shares 32 nibble bytes — 2k uses low nibbles, 2k+1 high.
/// Dequant: x = d·s[j]·nibble - dmin·m[j].
#[inline(always)]
fn q4_k_dot(x: &[f32], w_bytes: &[u8], blocks: usize) -> f32 {
    let mut sum = 0f32;
    for b in 0..blocks {
        let base = b * q4_k::BLOCK_BYTES;
        let d = read_f16(&w_bytes[base..base + 2]);
        let dmin = read_f16(&w_bytes[base + 2..base + 4]);
        let scales = &w_bytes[base + 4..base + 16];
        let qs = &w_bytes[base + 16..base + q4_k::BLOCK_BYTES];

        for j in 0..8 {
            let (s, m) = q4_k::unpack_scale_min_k4_public(j, scales);
            let d_scaled = d * s as f32;
            let m_scaled = dmin * m as f32;

            let qs_off = (j / 2) * 32;
            let shift = (j % 2) as u32 * 4;
            let x_off = b * q4_k::BLOCK_SIZE + j * 32;

            // 32 vals per sub-block = 4 f32x8 lanes.
            let mut acc = f32x8::ZERO;
            let mut x_acc = f32x8::ZERO;
            for c in 0..4 {
                let off = c * 8;
                let nibs = nibbles_f32x8(&qs[qs_off + off..qs_off + off + 8], shift);
                let xv = f32x8::from(slice8(&x[x_off + off..x_off + off + 8]));
                acc = nibs.mul_add(xv, acc);
                x_acc += xv;
            }
            sum += d_scaled * acc.reduce_add() - m_scaled * x_acc.reduce_add();
        }
    }
    sum
}

/// Q6_K: 210 bytes per 256 values. Layout per llama.cpp dequantize_row_q6_K:
/// 2 halves of 128 values. Per half, l iterates 0..32 producing FOUR outputs
/// (q1..q4) at flat offsets l+0, l+32, l+64, l+96 within the half. They differ
/// in which qh-nibble is read (shifts 0/2/4/6) and which ql byte/nibble pairs
/// with it. Dequant: x = d · scale[sc_idx] · (q6 - 32).
#[inline(always)]
fn q6_k_dot(x: &[f32], w_bytes: &[u8], blocks: usize) -> f32 {
    let mut sum = 0f32;
    for b in 0..blocks {
        let base = b * q6_k::BLOCK_BYTES;
        let ql = &w_bytes[base..base + 128];
        let qh = &w_bytes[base + 128..base + 192];
        let scales = &w_bytes[base + 192..base + 208];
        let d = read_f16(&w_bytes[base + 208..base + 210]);

        for half in 0..2 {
            let ql_off = half * 64;
            let qh_off = half * 32;
            let sc_off = half * 8;
            let x_off = b * q6_k::BLOCK_SIZE + half * 128;

            // Build 128 dequantised coefficients for this half, then SIMD-dot
            // with the matching 128 x values.
            let mut coeffs = [0f32; 128];
            for l in 0..32 {
                let is = l / 16;
                let qh_byte = qh[qh_off + l];
                let q1 = ((ql[ql_off + l] & 0x0F) as i32
                    | (((qh_byte & 0x03) as i32) << 4)) - 32;
                let q2 = ((ql[ql_off + l + 32] & 0x0F) as i32
                    | (((qh_byte & 0x0C) as i32) << 2)) - 32;
                let q3 = ((ql[ql_off + l] >> 4) as i32
                    | (((qh_byte & 0x30) as i32) << 0)) - 32;
                let q4 = ((ql[ql_off + l + 32] >> 4) as i32
                    | (((qh_byte & 0xC0) as i32) >> 2)) - 32;
                let s1 = scales[sc_off + is + 0] as i8 as f32;
                let s2 = scales[sc_off + is + 2] as i8 as f32;
                let s3 = scales[sc_off + is + 4] as i8 as f32;
                let s4 = scales[sc_off + is + 6] as i8 as f32;
                coeffs[l + 0] = s1 * q1 as f32;
                coeffs[l + 32] = s2 * q2 as f32;
                coeffs[l + 64] = s3 * q3 as f32;
                coeffs[l + 96] = s4 * q4 as f32;
            }

            // SIMD dot of 128 vals: 16 lanes of f32x8.
            let mut acc = f32x8::ZERO;
            for c in 0..16 {
                let off = c * 8;
                let cv = f32x8::from(slice8(&coeffs[off..off + 8]));
                let xv = f32x8::from(slice8(&x[x_off + off..x_off + off + 8]));
                acc = cv.mul_add(xv, acc);
            }
            sum += d * acc.reduce_add();
        }
    }
    sum
}

/// Canonical Q4 (cyb/cyb-model): 18 bytes per 32 values.
///   scale_f = i16 / 256;  value[i] = (nibble[i] - 8) * scale_f / 8.
/// Split nibble layout: byte i holds value[i] (lo) and value[i+16] (hi).
#[inline(always)]
fn canonical_q4_dot(x: &[f32], w_bytes: &[u8], blocks: usize) -> f32 {
    let neg8 = f32x8::splat(-8.0);
    let mut sum = 0f32;
    for b in 0..blocks {
        let block = &w_bytes[b * CANONICAL_Q4_BLOCK_BYTES..(b + 1) * CANONICAL_Q4_BLOCK_BYTES];
        let x_block = &x[b * CANONICAL_Q4_BLOCK_SIZE..(b + 1) * CANONICAL_Q4_BLOCK_SIZE];
        let scale_f = i16::from_le_bytes([block[0], block[1]]) as f32 / 256.0;
        let qs = &block[2..];

        let mut acc = f32x8::ZERO;
        for chunk in 0..2 {
            let off = chunk * 8;
            let b8 = &qs[off..off + 8];
            // Low nibbles → first half, high nibbles → second half.
            let lo = nibbles_f32x8(b8, 0) + neg8;
            let hi = nibbles_f32x8(b8, 4) + neg8;
            let x_lo = f32x8::from(slice8(&x_block[off..off + 8]));
            let x_hi = f32x8::from(slice8(&x_block[16 + off..16 + off + 8]));
            acc = lo.mul_add(x_lo, acc);
            acc = hi.mul_add(x_hi, acc);
        }
        sum += scale_f / 8.0 * acc.reduce_add();
    }
    sum
}

/// Canonical Q8 (cyb/cyb-model): 34 bytes per 32 values.
///   scale_f = i16 / 256;  value[i] = i8[i] * scale_f / 127.
#[inline(always)]
fn canonical_q8_dot(x: &[f32], w_bytes: &[u8], blocks: usize) -> f32 {
    let mut sum = 0f32;
    for b in 0..blocks {
        let base = b * CANONICAL_Q8_BLOCK_BYTES;
        let block = &w_bytes[base..base + CANONICAL_Q8_BLOCK_BYTES];
        let x_block = &x[b * CANONICAL_Q8_BLOCK_SIZE..(b + 1) * CANONICAL_Q8_BLOCK_SIZE];
        let scale_f = i16::from_le_bytes([block[0], block[1]]) as f32 / 256.0;
        let qs = &block[2..];

        // 32 i8 → 4 f32x8 FMA lanes.
        let mut acc = f32x8::ZERO;
        for chunk in 0..4 {
            let off = chunk * 8;
            let cv = i8_bytes_f32x8(&qs[off..off + 8]);
            let xv = f32x8::from(slice8(&x_block[off..off + 8]));
            acc = cv.mul_add(xv, acc);
        }
        sum += scale_f / 127.0 * acc.reduce_add();
    }
    sum
}

// ─── helpers ─────────────────────────────────────────────────────────────

#[inline(always)]
fn read_f16(bytes: &[u8]) -> f32 {
    debug_assert_eq!(bytes.len(), 2);
    half::f16::from_bits(u16::from_le_bytes([bytes[0], bytes[1]])).to_f32()
}

#[inline(always)]
fn slice8(s: &[f32]) -> [f32; 8] {
    [s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]
}

/// Extract 8 nibbles from 8 bytes as f32x8 in [0.0, 15.0].
/// `shift = 0` selects low nibbles, `shift = 4` selects high nibbles.
#[inline(always)]
fn nibbles_f32x8(bytes: &[u8], shift: u32) -> f32x8 {
    debug_assert_eq!(bytes.len(), 8);
    f32x8::from([
        ((bytes[0] >> shift) & 0x0F) as f32,
        ((bytes[1] >> shift) & 0x0F) as f32,
        ((bytes[2] >> shift) & 0x0F) as f32,
        ((bytes[3] >> shift) & 0x0F) as f32,
        ((bytes[4] >> shift) & 0x0F) as f32,
        ((bytes[5] >> shift) & 0x0F) as f32,
        ((bytes[6] >> shift) & 0x0F) as f32,
        ((bytes[7] >> shift) & 0x0F) as f32,
    ])
}

/// Convert 8 signed bytes to f32x8.
#[inline(always)]
fn i8_bytes_f32x8(bytes: &[u8]) -> f32x8 {
    debug_assert_eq!(bytes.len(), 8);
    f32x8::from([
        bytes[0] as i8 as f32,
        bytes[1] as i8 as f32,
        bytes[2] as i8 as f32,
        bytes[3] as i8 as f32,
        bytes[4] as i8 as f32,
        bytes[5] as i8 as f32,
        bytes[6] as i8 as f32,
        bytes[7] as i8 as f32,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::cpu::matmul::matmul_f32;
    use crate::backend::cpu::quant::dequantize_to_f32;

    /// Deterministic xorshift byte stream for synthetic weight bytes.
    fn rand_bytes(n: usize, seed: u64) -> Vec<u8> {
        let mut s = seed | 1;
        (0..n)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                s as u8
            })
            .collect()
    }

    fn rand_x(k: usize, seed: u64) -> Vec<f32> {
        let mut s = seed | 1;
        (0..k)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                ((s as f32) / (u64::MAX as f32) - 0.5) * 0.2
            })
            .collect()
    }

    /// Per-format f16 scale field offsets within one block. Random bytes
    /// would put NaN/Inf into these and break the test, so we overwrite
    /// each one with a sane f16 value.
    fn scale_offsets(dtype: DType) -> &'static [usize] {
        match dtype {
            DType::Q4_0 | DType::Q8_0 => &[0],
            DType::Q4_K => &[0, 2],
            DType::Q6_K => &[208],
            // canonical: i16 at byte 0; sanitize via different writer.
            DType::Q4 | DType::Q8 => &[],
            _ => &[],
        }
    }

    /// Replace the f16 scale fields of every block with deterministic sane
    /// positive values (~0.05..1.0). Keeps the rest of the bytes random so
    /// the test exercises real bit patterns in the quant payload.
    fn sanitize_scales(bytes: &mut [u8], dtype: DType, block_bytes: usize) {
        let n_blocks = bytes.len() / block_bytes;
        for blk in 0..n_blocks {
            // Canonical Q4/Q8 use i16 scale at byte 0 (8.8 fixed-point).
            if matches!(dtype, DType::Q4 | DType::Q8) {
                let v = 0.05 + ((blk as f32 * 0.13).fract() * 0.9);
                let i = (v * 256.0).round() as i16;
                let off = blk * block_bytes;
                bytes[off..off + 2].copy_from_slice(&i.to_le_bytes());
                continue;
            }
            for &off in scale_offsets(dtype) {
                let v = 0.05 + ((blk as f32 * 0.13).fract() * 0.9);
                let bits = half::f16::from_f32(v).to_bits();
                bytes[blk * block_bytes + off] = (bits & 0xFF) as u8;
                bytes[blk * block_bytes + off + 1] = (bits >> 8) as u8;
            }
        }
    }

    /// Cross-check: fused matmul must equal dequantize + f32 matmul within
    /// relative ε (output magnitude varies, so absolute ε is meaningless).
    /// Single source of truth; one helper covers every quant format.
    fn assert_fused_matches_dequant(dtype: DType, n: usize, k: usize, rel_eps: f32) {
        let (block_bytes, block_size) = match dtype {
            DType::Q4_0 => (q4_0::BLOCK_BYTES, q4_0::BLOCK_SIZE),
            DType::Q8_0 => (q8_0::BLOCK_BYTES, q8_0::BLOCK_SIZE),
            DType::Q4_K => (q4_k::BLOCK_BYTES, q4_k::BLOCK_SIZE),
            DType::Q6_K => (q6_k::BLOCK_BYTES, q6_k::BLOCK_SIZE),
            DType::Q4 => (CANONICAL_Q4_BLOCK_BYTES, CANONICAL_Q4_BLOCK_SIZE),
            DType::Q8 => (CANONICAL_Q8_BLOCK_BYTES, CANONICAL_Q8_BLOCK_SIZE),
            other => panic!("unsupported dtype in test: {other:?}"),
        };
        assert_eq!(k % block_size, 0, "test misuse: k must be a multiple of block_size");

        let row_bytes = (k / block_size) * block_bytes;
        let mut w_bytes = rand_bytes(n * row_bytes, 0xC0FFEE_u64.wrapping_mul(dtype as u64 + 1));
        // Walk row-by-row so per-row block indexing matches sanitize_scales.
        for row in 0..n {
            sanitize_scales(
                &mut w_bytes[row * row_bytes..(row + 1) * row_bytes],
                dtype,
                block_bytes,
            );
        }
        let x_data = rand_x(k, 0xBADC0DE_u64.wrapping_mul(dtype as u64 + 1));
        let x = Tensor::from_f32(vec![1, k], x_data);

        let w_f32 = dequantize_to_f32(&w_bytes, dtype);
        let w_tensor = Tensor::from_f32(vec![n, k], w_f32);
        let y_ref = matmul_f32(&x, &w_tensor).unwrap().to_f32_vec();

        let y_fused = matmul_quant_f32(&x, &w_bytes, dtype, n, k)
            .unwrap()
            .to_f32_vec();

        assert_eq!(y_ref.len(), y_fused.len());
        let mut worst = 0f32;
        for (i, (a, b)) in y_ref.iter().zip(y_fused.iter()).enumerate() {
            assert!(a.is_finite() && b.is_finite(), "{dtype:?} out[{i}]: a={a} b={b}");
            let scale = a.abs().max(b.abs()).max(1.0);
            let rel = (a - b).abs() / scale;
            if rel > worst {
                worst = rel;
            }
            assert!(
                rel < rel_eps,
                "{dtype:?} out[{i}]: {a} vs {b}, rel={rel:.3e} (eps={rel_eps:.0e})"
            );
        }
        eprintln!("{dtype:?} fused vs dequant worst rel-diff = {worst:.3e} (eps={rel_eps:.0e})");
    }

    /// Relative ε of 1e-5 covers the worst rounding divergence between the
    /// two accumulation orderings (per-block reduce + scale vs full SIMD dot)
    /// across all four formats at f32 precision (~1.2e-7 per op, ~k accumulations).
    const REL_EPS: f32 = 1e-5;

    #[test]
    fn q4_0_fused_matches_dequant_matmul() {
        assert_fused_matches_dequant(DType::Q4_0, 64, 128, REL_EPS);
    }

    #[test]
    fn q8_0_fused_matches_dequant_matmul() {
        assert_fused_matches_dequant(DType::Q8_0, 64, 128, REL_EPS);
    }

    #[test]
    fn q4_k_fused_matches_dequant_matmul() {
        assert_fused_matches_dequant(DType::Q4_K, 64, 512, REL_EPS);
    }

    #[test]
    fn q6_k_fused_matches_dequant_matmul() {
        assert_fused_matches_dequant(DType::Q6_K, 64, 256, REL_EPS);
    }

    #[test]
    fn canonical_q4_fused_matches_dequant_matmul() {
        assert_fused_matches_dequant(DType::Q4, 64, 128, REL_EPS);
    }

    #[test]
    fn canonical_q8_fused_matches_dequant_matmul() {
        assert_fused_matches_dequant(DType::Q8, 64, 128, REL_EPS);
    }

    #[test]
    fn weight_buffer_length_mismatch_errors() {
        // n×k=64×128 of Q4_0 needs 64 × (128/32) × 18 bytes; pass less.
        let bad = vec![0u8; 16];
        let x = Tensor::from_f32(vec![1, 128], vec![0.0; 128]);
        let err = matmul_quant_f32(&x, &bad, DType::Q4_0, 64, 128).unwrap_err();
        match err {
            BackendError::InvalidInput { op, .. } => assert_eq!(op, "MatmulQuant"),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }
}
