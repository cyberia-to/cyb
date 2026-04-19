//! Fused dequant+matmul: read quantized weight bytes directly, compute
//! dot product against f32 activation without materializing full f32 weights.
//!
//! Wins: 6× less memory (Q4_0) or 8× (Q4_K), 6× less memory bandwidth,
//! unlocks models too large to dequant at load (coder-14b = 28GB f32).
//!
//! Correctness: must match `matmul_f32(x, dequantize(W))` within
//! accumulation-precision ε. Verified by tests against the dequant+
//! f32 matmul reference.

use crate::backend::BackendError;
use crate::cpu::quant::{q4_0, q4_k, q6_k, q8_0};
use crate::dtype::DType;
use crate::tensor::Tensor;
use rayon::prelude::*;
use wide::f32x8;

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
        DType::Q4_0 => matmul_q4_0(x_data, w_bytes, &mut out, batch, n, k)?,
        DType::Q8_0 => matmul_q8_0(x_data, w_bytes, &mut out, batch, n, k)?,
        DType::Q4_K => matmul_q4_k(x_data, w_bytes, &mut out, batch, n, k)?,
        DType::Q6_K => matmul_q6_k(x_data, w_bytes, &mut out, batch, n, k)?,
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

/// Q4_0: 18 bytes per 32 values. scale f16 + 16 nibble bytes.
fn matmul_q4_0(
    x: &[f32],
    w: &[u8],
    out: &mut [f32],
    batch: usize,
    n: usize,
    k: usize,
) -> Result<(), BackendError> {
    if k % q4_0::BLOCK_SIZE != 0 {
        return Err(BackendError::InvalidInput {
            op: "MatmulQuant",
            reason: format!("Q4_0 requires K divisible by 32, got K={k}"),
        });
    }
    let blocks_per_row = k / q4_0::BLOCK_SIZE;
    let row_bytes = blocks_per_row * q4_0::BLOCK_BYTES;

    for b in 0..batch {
        let x_row = &x[b * k..(b + 1) * k];
        let out_row = &mut out[b * n..(b + 1) * n];
        out_row
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, y)| {
                let w_row = &w[i * row_bytes..(i + 1) * row_bytes];
                *y = q4_0_dot(x_row, w_row, blocks_per_row);
            });
    }
    Ok(())
}

#[inline]
fn q4_0_dot(x: &[f32], w_bytes: &[u8], blocks: usize) -> f32 {
    // SIMD path: process each 32-value block as 4 f32x8 lanes.
    // Layout: low nibbles of qs[0..16] → values 0..16, high nibbles → values 16..32.
    // Values are (nibble - 8) pre-scaled by d.
    let neg8 = f32x8::splat(-8.0);
    let mut sum = 0f32;
    for b in 0..blocks {
        let block = &w_bytes[b * 18..(b + 1) * 18];
        let x_block = &x[b * 32..(b + 1) * 32];
        let d_bits = u16::from_le_bytes([block[0], block[1]]);
        let d = half::f16::from_bits(d_bits).to_f32();
        let qs = &block[2..18];

        // 16 nibble bytes → 2 chunks of 8 bytes. Each chunk contributes 8 lo + 8 hi nibbles.
        let mut acc = f32x8::ZERO;
        for chunk in 0..2 {
            let off = chunk * 8;
            let b8 = &qs[off..off + 8];
            let lo = nibbles_low_f32x8(b8) + neg8;
            let hi = nibbles_high_f32x8(b8) + neg8;
            let x_lo = f32x8::from(slice8(&x_block[off..off + 8]));
            let x_hi = f32x8::from(slice8(&x_block[16 + off..16 + off + 8]));
            acc = lo.mul_add(x_lo, acc);
            acc = hi.mul_add(x_hi, acc);
        }
        sum += d * acc.reduce_add();
    }
    sum
}

#[inline(always)]
fn slice8(s: &[f32]) -> [f32; 8] {
    [s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]
}

/// Extract low nibbles (byte & 0x0F) of 8 bytes as f32x8 in [0.0, 15.0].
#[inline(always)]
fn nibbles_low_f32x8(bytes: &[u8]) -> f32x8 {
    debug_assert_eq!(bytes.len(), 8);
    f32x8::from([
        (bytes[0] & 0x0F) as f32,
        (bytes[1] & 0x0F) as f32,
        (bytes[2] & 0x0F) as f32,
        (bytes[3] & 0x0F) as f32,
        (bytes[4] & 0x0F) as f32,
        (bytes[5] & 0x0F) as f32,
        (bytes[6] & 0x0F) as f32,
        (bytes[7] & 0x0F) as f32,
    ])
}

/// Extract high nibbles (byte >> 4) of 8 bytes as f32x8 in [0.0, 15.0].
#[inline(always)]
fn nibbles_high_f32x8(bytes: &[u8]) -> f32x8 {
    debug_assert_eq!(bytes.len(), 8);
    f32x8::from([
        (bytes[0] >> 4) as f32,
        (bytes[1] >> 4) as f32,
        (bytes[2] >> 4) as f32,
        (bytes[3] >> 4) as f32,
        (bytes[4] >> 4) as f32,
        (bytes[5] >> 4) as f32,
        (bytes[6] >> 4) as f32,
        (bytes[7] >> 4) as f32,
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

/// Q8_0: 34 bytes per 32 values. scale f16 + 32 i8.
fn matmul_q8_0(
    x: &[f32],
    w: &[u8],
    out: &mut [f32],
    batch: usize,
    n: usize,
    k: usize,
) -> Result<(), BackendError> {
    if k % q8_0::BLOCK_SIZE != 0 {
        return Err(BackendError::InvalidInput {
            op: "MatmulQuant",
            reason: format!("Q8_0 requires K divisible by 32, got K={k}"),
        });
    }
    let blocks_per_row = k / 32;
    let row_bytes = blocks_per_row * 34;

    for b in 0..batch {
        let x_row = &x[b * k..(b + 1) * k];
        let out_row = &mut out[b * n..(b + 1) * n];
        out_row
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, y)| {
                let w_row = &w[i * row_bytes..(i + 1) * row_bytes];
                let mut sum = 0f32;
                for blk in 0..blocks_per_row {
                    let base = blk * 34;
                    let d = half::f16::from_bits(u16::from_le_bytes([
                        w_row[base],
                        w_row[base + 1],
                    ]))
                    .to_f32();
                    // 32 signed bytes → 4 chunks of 8 → 4 f32x8 FMA
                    let mut acc = f32x8::ZERO;
                    for chunk in 0..4 {
                        let off = chunk * 8;
                        let qf = i8_bytes_f32x8(&w_row[base + 2 + off..base + 2 + off + 8]);
                        let xv = f32x8::from(slice8(&x_row[blk * 32 + off..blk * 32 + off + 8]));
                        acc = qf.mul_add(xv, acc);
                    }
                    sum += d * acc.reduce_add();
                }
                *y = sum;
            });
    }
    Ok(())
}

/// Q4_K: 144 bytes per 256 values.
fn matmul_q4_k(
    x: &[f32],
    w: &[u8],
    out: &mut [f32],
    batch: usize,
    n: usize,
    k: usize,
) -> Result<(), BackendError> {
    if k % q4_k::BLOCK_SIZE != 0 {
        return Err(BackendError::InvalidInput {
            op: "MatmulQuant",
            reason: format!("Q4_K requires K divisible by 256, got K={k}"),
        });
    }
    let blocks_per_row = k / 256;
    let row_bytes = blocks_per_row * q4_k::BLOCK_BYTES;

    for b in 0..batch {
        let x_row = &x[b * k..(b + 1) * k];
        let out_row = &mut out[b * n..(b + 1) * n];
        out_row
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, y)| {
                let w_row = &w[i * row_bytes..(i + 1) * row_bytes];
                let mut sum = 0f32;
                for blk in 0..blocks_per_row {
                    let base = blk * 144;
                    let d = half::f16::from_bits(u16::from_le_bytes([
                        w_row[base],
                        w_row[base + 1],
                    ]))
                    .to_f32();
                    let dmin = half::f16::from_bits(u16::from_le_bytes([
                        w_row[base + 2],
                        w_row[base + 3],
                    ]))
                    .to_f32();
                    let scales = &w_row[base + 4..base + 16];
                    let qs = &w_row[base + 16..base + 144];

                    for j in 0..8 {
                        let (s, m) = q4_k::unpack_scale_min_k4_public(j, scales);
                        let d_scaled = d * s as f32;
                        let m_scaled = dmin * m as f32;

                        let pair = j / 2;
                        let is_upper = j % 2;
                        let qs_off = pair * 32;

                        let x_off = blk * 256 + j * 32;
                        // 32 values per sub-block → 4 f32x8 lanes.
                        let mut acc = f32x8::ZERO;
                        let mut x_sum_v = f32x8::ZERO;
                        for chunk in 0..4 {
                            let off = chunk * 8;
                            let b8 = &qs[qs_off + off..qs_off + off + 8];
                            let nibs = if is_upper == 0 {
                                nibbles_low_f32x8(b8)
                            } else {
                                nibbles_high_f32x8(b8)
                            };
                            let xv = f32x8::from(slice8(&x_row[x_off + off..x_off + off + 8]));
                            acc = nibs.mul_add(xv, acc);
                            x_sum_v += xv;
                        }
                        sum += d_scaled * acc.reduce_add() - m_scaled * x_sum_v.reduce_add();
                    }
                }
                *y = sum;
            });
    }
    Ok(())
}

/// Q6_K: 210 bytes per 256 values.
fn matmul_q6_k(
    x: &[f32],
    w: &[u8],
    out: &mut [f32],
    batch: usize,
    n: usize,
    k: usize,
) -> Result<(), BackendError> {
    if k % q6_k::BLOCK_SIZE != 0 {
        return Err(BackendError::InvalidInput {
            op: "MatmulQuant",
            reason: format!("Q6_K requires K divisible by 256, got K={k}"),
        });
    }
    let blocks_per_row = k / 256;
    let row_bytes = blocks_per_row * 210;

    for b in 0..batch {
        let x_row = &x[b * k..(b + 1) * k];
        let out_row = &mut out[b * n..(b + 1) * n];
        out_row
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, y)| {
                let w_row = &w[i * row_bytes..(i + 1) * row_bytes];
                let mut sum = 0f32;
                for blk in 0..blocks_per_row {
                    let base = blk * 210;
                    let ql = &w_row[base..base + 128];
                    let qh = &w_row[base + 128..base + 192];
                    let scales = &w_row[base + 192..base + 208];
                    let d = half::f16::from_bits(u16::from_le_bytes([
                        w_row[base + 208],
                        w_row[base + 209],
                    ]))
                    .to_f32();

                    // Per dequantize_row_q6_K: 2 halves × 4 sets × 32 values.
                    // Strategy: build 32 scalar coefficients (dequantized values)
                    // then SIMD dot-product against the 32 corresponding x values.
                    for h in 0..2 {
                        for i_set in 0..4 {
                            let x_off = blk * 256 + h * 128 + i_set * 32;
                            let ql_base = h * 64 + i_set * 16;
                            let qh_base = h * 32;
                            let shift = (i_set * 2) as u32;
                            let sc_even = scales[h * 8 + i_set * 2] as i8 as f32;
                            let sc_odd = scales[h * 8 + i_set * 2 + 1] as i8 as f32;

                            let mut coeffs = [0f32; 32];
                            for l in 0..16 {
                                let ql_val = ql[ql_base + l] & 0x0F;
                                let qh_val = (qh[qh_base + l] >> shift) & 0x03;
                                let q6 = (ql_val as i32) | ((qh_val as i32) << 4);
                                coeffs[l] = sc_even * (q6 - 32) as f32;
                            }
                            for l in 0..16 {
                                let ql_val = ql[ql_base + l] >> 4;
                                let qh_val = (qh[qh_base + 16 + l] >> shift) & 0x03;
                                let q6 = (ql_val as i32) | ((qh_val as i32) << 4);
                                coeffs[16 + l] = sc_odd * (q6 - 32) as f32;
                            }

                            let mut acc = f32x8::ZERO;
                            for chunk in 0..4 {
                                let off = chunk * 8;
                                let cv = f32x8::from(slice8(&coeffs[off..off + 8]));
                                let xv = f32x8::from(slice8(&x_row[x_off + off..x_off + off + 8]));
                                acc = cv.mul_add(xv, acc);
                            }
                            sum += d * acc.reduce_add();
                        }
                    }
                }
                *y = sum;
            });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::matmul::matmul_f32;
    use crate::cpu::quant::dequantize_to_f32;

    /// Verify fused Q4_0 matmul matches dequantize + f32 matmul.
    #[test]
    fn q4_0_fused_matches_dequant_matmul() {
        // Build a synthetic Q4_0 weight matrix [N=8, K=32] with non-trivial values.
        let n = 8;
        let k = 32;
        let blocks_per_row = k / 32;
        let row_bytes = blocks_per_row * 18;
        let mut w_bytes = vec![0u8; n * row_bytes];
        for i in 0..n {
            let d_bits = half::f16::from_f32(0.1 * (i as f32 + 1.0)).to_bits();
            w_bytes[i * row_bytes] = (d_bits & 0xFF) as u8;
            w_bytes[i * row_bytes + 1] = (d_bits >> 8) as u8;
            for j in 0..16 {
                w_bytes[i * row_bytes + 2 + j] =
                    ((((i + j) as u8) & 0x0F) | ((((i * 2 + j) as u8) & 0x0F) << 4)) as u8;
            }
        }

        let x_data: Vec<f32> = (0..k).map(|i| (i as f32 - 16.0) * 0.1).collect();
        let x = Tensor::from_f32(vec![1, k], x_data);

        // Reference: dequant then f32 matmul.
        let w_f32 = dequantize_to_f32(&w_bytes, DType::Q4_0);
        let w_tensor = Tensor::from_f32(vec![n, k], w_f32);
        let y_ref = matmul_f32(&x, &w_tensor).unwrap().to_f32_vec();

        let y_fused = matmul_quant_f32(&x, &w_bytes, DType::Q4_0, n, k)
            .unwrap()
            .to_f32_vec();
        assert_eq!(y_ref.len(), y_fused.len());
        for (i, (a, b)) in y_ref.iter().zip(y_fused.iter()).enumerate() {
            let diff = (a - b).abs();
            assert!(diff < 1e-5, "out[{i}]: {a} vs {b}, diff={diff}");
        }
    }
}
