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
    let mut sum = 0f32;
    for b in 0..blocks {
        let block = &w_bytes[b * 18..(b + 1) * 18];
        let x_block = &x[b * 32..(b + 1) * 32];
        let d_bits = u16::from_le_bytes([block[0], block[1]]);
        let d = half::f16::from_bits(d_bits).to_f32();
        let qs = &block[2..18];

        // Low nibbles → first 16, high → last 16.
        // Accumulate in f32 without allocating the 32-value dequantized array.
        let mut acc = 0f32;
        for j in 0..16 {
            let lo = ((qs[j] & 0x0F) as i32 - 8) as f32;
            let hi = ((qs[j] >> 4) as i32 - 8) as f32;
            acc += x_block[j] * lo + x_block[j + 16] * hi;
        }
        sum += d * acc;
    }
    sum
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
                    let mut acc = 0f32;
                    for j in 0..32 {
                        let q = w_row[base + 2 + j] as i8;
                        acc += x_row[blk * 32 + j] * q as f32;
                    }
                    sum += d * acc;
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
                        let mut acc = 0f32;
                        let mut x_sum = 0f32;
                        for l in 0..32 {
                            let byte = qs[qs_off + l];
                            let nibble = if is_upper == 0 {
                                byte & 0x0F
                            } else {
                                byte >> 4
                            } as i32;
                            let xv = x_row[x_off + l];
                            acc += xv * nibble as f32;
                            x_sum += xv;
                        }
                        sum += d_scaled * acc - m_scaled * x_sum;
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
                    for h in 0..2 {
                        for i_set in 0..4 {
                            let x_off = blk * 256 + h * 128 + i_set * 32;
                            let mut acc_even = 0f32;
                            let mut acc_odd = 0f32;
                            for l in 0..32 {
                                let ql_idx = h * 64 + i_set * 16 + (l % 16);
                                let qh_idx = h * 32 + l;
                                let ql_val = if l < 16 {
                                    ql[ql_idx] & 0x0F
                                } else {
                                    ql[ql_idx] >> 4
                                };
                                let qh_val = (qh[qh_idx] >> (i_set * 2)) & 0x03;
                                let q6 = (ql_val as i32) | ((qh_val as i32) << 4);
                                let sc_idx = h * 8 + i_set * 2 + l / 16;
                                let sc = scales[sc_idx] as i8 as f32;
                                let val = sc * (q6 - 32) as f32;
                                if l < 16 {
                                    acc_even += x_row[x_off + l] * val;
                                } else {
                                    acc_odd += x_row[x_off + l] * val;
                                }
                            }
                            sum += d * (acc_even + acc_odd);
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
