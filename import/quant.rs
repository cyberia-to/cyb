//! Re-quantization at import time.
//!
//! Two encoders live here:
//!
//! 1. **GGUF K-quant encoders** (legacy path, today's writer):
//!    `f32_to_q4k` re-quants legacy Q4_0 / Q4_1 sources up to Q4_K so
//!    the runtime kernels target K-quants only.
//!
//! 2. **Canonical encoders** (alignment target — see
//!    `.claude/plans/canonical-format-alignment.md`):
//!    fixed five-encoding set defined by `cyb/cyb-model`:
//!    `u32`, `u16`, `q8`, `q4`, `ternary`. These are integer
//!    fixed-point encodings; floats are banned from canonical
//!    `.model` weights.

// ── Legacy path (today's writer) ─────────────────────────────────────────────

/// Quantize `weights` (an `[N, K]` matrix in row-major f32) into Q4_K.
///
/// Q4_K layout per 256-value superblock (144 bytes):
///   f16 d + f16 dmin + 12 bytes packed scales/mins + 128 bytes of nibbles.
pub fn f32_to_q4k(weights: &[f32], n: usize, k: usize) -> Vec<u8> {
    let super_blocks = k / 256;
    let mut out = vec![0u8; n * super_blocks * 144];
    for row in 0..n {
        for sb in 0..super_blocks {
            let base = row * k + sb * 256;
            let dst = (row * super_blocks + sb) * 144;
            let block_vals = &weights[base..base + 256];
            let mut scales_arr = [0u8; 12];
            let mut qs = [0u8; 128];
            let mut sub_scales = [0.0f32; 8];
            let mut sub_mins = [0.0f32; 8];
            for j in 0..8 {
                let sub = &block_vals[j * 32..(j + 1) * 32];
                let max_val = sub.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let min_val = sub.iter().cloned().fold(f32::INFINITY, f32::min);
                let eff_min = min_val.min(0.0);
                sub_mins[j] = -eff_min;
                sub_scales[j] = if max_val > eff_min {
                    (max_val - eff_min) / 15.0
                } else {
                    0.0
                };
            }
            let max_scale = sub_scales.iter().cloned().fold(0.0f32, f32::max);
            let max_min = sub_mins.iter().cloned().fold(0.0f32, f32::max);
            let d = max_scale / 63.0;
            let dmin = max_min / 63.0;
            let inv_d = if max_scale > 0.0 { 1.0 / d } else { 0.0 };
            let inv_dmin = if max_min > 0.0 { 1.0 / dmin } else { 0.0 };
            out[dst..dst + 2].copy_from_slice(&half::f16::from_f32(d).to_le_bytes());
            out[dst + 2..dst + 4].copy_from_slice(&half::f16::from_f32(dmin).to_le_bytes());
            for j in 0..8 {
                let sc = (sub_scales[j] * inv_d).round().min(63.0).max(0.0) as u8;
                let m = (sub_mins[j] * inv_dmin).round().min(63.0).max(0.0) as u8;
                if j < 4 {
                    scales_arr[j] = (scales_arr[j] & 0xC0) | (sc & 63);
                    scales_arr[j + 4] = (scales_arr[j + 4] & 0xC0) | (m & 63);
                } else {
                    scales_arr[j + 4] = (scales_arr[j + 4] & 0xF0) | (sc & 0xF);
                    scales_arr[j - 4] = (scales_arr[j - 4] & 0x3F) | ((sc >> 4) << 6);
                    scales_arr[j + 4] = (scales_arr[j + 4] & 0x0F) | ((m & 0xF) << 4);
                    scales_arr[j] = (scales_arr[j] & 0x3F) | ((m >> 4) << 6);
                }
            }
            out[dst + 4..dst + 16].copy_from_slice(&scales_arr);
            for grp in 0..4 {
                let inv_sc1 = if sub_scales[grp * 2] > 0.0 {
                    1.0 / sub_scales[grp * 2]
                } else {
                    0.0
                };
                let inv_sc2 = if sub_scales[grp * 2 + 1] > 0.0 {
                    1.0 / sub_scales[grp * 2 + 1]
                } else {
                    0.0
                };
                for l in 0..32 {
                    let q1 = ((block_vals[grp * 64 + l] + sub_mins[grp * 2]) * inv_sc1)
                        .round()
                        .min(15.0)
                        .max(0.0) as u8;
                    let q2 = ((block_vals[grp * 64 + 32 + l] + sub_mins[grp * 2 + 1]) * inv_sc2)
                        .round()
                        .min(15.0)
                        .max(0.0) as u8;
                    qs[grp * 32 + l] = (q1 & 0xF) | ((q2 & 0xF) << 4);
                }
            }
            out[dst + 16..dst + 144].copy_from_slice(&qs);
        }
    }
    out
}

// ── Canonical encoders (cyb/cyb-model) ───────────────────────────────────────
//
// All canonical encodings are integer fixed-point. The runtime decodes the
// bytes back to f32 via the matching decoder in `run::backend::cpu::canonical`.

pub mod canonical {
    /// Block size for q4 / q8 canonical encodings (32 values per block).
    pub const BLOCK: usize = 32;

    /// Encode f32 as canonical `u32` (16.16 fixed-point).
    ///
    /// Encoded value: `clamp(round(f * 65536), i32::MIN, i32::MAX)` reinterpreted
    /// as little-endian `u32`. Reconstruction: `i32 / 65536.0`.
    ///
    /// Range: ±32_767.99998. Resolution: 1/65536 ≈ 1.5e-5. Used for the
    /// canonical-spec "full precision" slot (norms, biases).
    pub fn f32_to_u32(values: &[f32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(values.len() * 4);
        for &v in values {
            let scaled = (v * 65536.0).round();
            let clamped = scaled.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
            out.extend_from_slice(&clamped.to_le_bytes());
        }
        out
    }

    /// Encode f32 as canonical `u16` (8.8 fixed-point).
    ///
    /// Encoded: `clamp(round(f * 256), i16::MIN, i16::MAX)` as little-endian
    /// `u16`. Reconstruction: `i16 / 256.0`.
    ///
    /// Range: ±127.996. Resolution: 1/256 ≈ 3.9e-3. Used for the canonical-
    /// spec "half precision" slot (smaller f16-source weights). Saturates
    /// silently for |f| > 128 — caller must ensure inputs fit.
    pub fn f32_to_u16(values: &[f32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(values.len() * 2);
        for &v in values {
            let scaled = (v * 256.0).round();
            let clamped = scaled.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            out.extend_from_slice(&clamped.to_le_bytes());
        }
        out
    }

    /// Encode f32 as canonical `q8` (32-value blocks, u16 scale + 32 i8s).
    ///
    /// Per block (34 bytes): scale stored as canonical `u16` (8.8), then 32
    /// signed int8s. Dequant: `int8[i] * scale / 127`. Block scale chosen so
    /// the largest |value| lands at i8 = 127.
    pub fn f32_to_q8(values: &[f32]) -> Vec<u8> {
        assert!(
            values.len() % BLOCK == 0,
            "f32_to_q8: input length {} must be multiple of {}",
            values.len(),
            BLOCK
        );
        let blocks = values.len() / BLOCK;
        let mut out = vec![0u8; blocks * (2 + BLOCK)];
        for b in 0..blocks {
            let block = &values[b * BLOCK..(b + 1) * BLOCK];
            let amax = block
                .iter()
                .fold(0.0f32, |acc, &v| acc.max(v.abs()));
            let dst = b * (2 + BLOCK);
            // Scale is the f32 quantization step; encoded into the u16 slot.
            let scale = amax;
            let u16_scale = (scale * 256.0).round().clamp(0.0, i16::MAX as f32) as i16;
            out[dst..dst + 2].copy_from_slice(&u16_scale.to_le_bytes());
            let inv = if scale > 0.0 { 127.0 / scale } else { 0.0 };
            for (i, &v) in block.iter().enumerate() {
                let q = (v * inv).round().clamp(-127.0, 127.0) as i8;
                out[dst + 2 + i] = q as u8;
            }
        }
        out
    }

    /// Encode f32 as canonical `q4` (32-value blocks, u16 scale + 16 packed nibbles).
    ///
    /// Per block (18 bytes): scale stored as canonical `u16` (8.8), then 32
    /// 4-bit nibbles packed two-per-byte (low nibble first). Dequant:
    /// `(nibble - 8) * scale / 8`. Range covered by one block: roughly
    /// `[-scale, +scale]`.
    pub fn f32_to_q4(values: &[f32]) -> Vec<u8> {
        assert!(
            values.len() % BLOCK == 0,
            "f32_to_q4: input length {} must be multiple of {}",
            values.len(),
            BLOCK
        );
        let blocks = values.len() / BLOCK;
        let mut out = vec![0u8; blocks * (2 + BLOCK / 2)];
        for b in 0..blocks {
            let block = &values[b * BLOCK..(b + 1) * BLOCK];
            let amax = block
                .iter()
                .fold(0.0f32, |acc, &v| acc.max(v.abs()));
            let dst = b * (2 + BLOCK / 2);
            // scale represents amax → nibble offset 7 (=  +max of the signed range)
            let scale = amax;
            let u16_scale = (scale * 256.0).round().clamp(0.0, i16::MAX as f32) as i16;
            out[dst..dst + 2].copy_from_slice(&u16_scale.to_le_bytes());
            let inv = if scale > 0.0 { 8.0 / scale } else { 0.0 };
            for i in 0..(BLOCK / 2) {
                let lo = (block[2 * i] * inv).round().clamp(-8.0, 7.0) as i32;
                let hi = (block[2 * i + 1] * inv).round().clamp(-8.0, 7.0) as i32;
                let n_lo = ((lo + 8) & 0xF) as u8;
                let n_hi = ((hi + 8) & 0xF) as u8;
                out[dst + 2 + i] = n_lo | (n_hi << 4);
            }
        }
        out
    }

    /// Encode f32 as canonical `ternary` (2 bits per value, packed 4-per-byte).
    ///
    /// Encoding: `00 = 0`, `01 = +1`, `10 = -1`, `11 = reserved`. 32 values per
    /// 8 bytes. Inputs not in {-1, 0, +1} are quantized to the nearest of those
    /// three by sign threshold (|v| < 0.5 → 0; v >= 0.5 → +1; v <= -0.5 → -1).
    pub fn f32_to_ternary(values: &[f32]) -> Vec<u8> {
        assert!(
            values.len() % BLOCK == 0,
            "f32_to_ternary: input length {} must be multiple of {}",
            values.len(),
            BLOCK
        );
        let blocks = values.len() / BLOCK;
        let mut out = vec![0u8; blocks * 8];
        for b in 0..blocks {
            let block = &values[b * BLOCK..(b + 1) * BLOCK];
            let dst = b * 8;
            for byte_idx in 0..8 {
                let mut byte = 0u8;
                for slot in 0..4 {
                    let v = block[byte_idx * 4 + slot];
                    let code: u8 = if v >= 0.5 {
                        0b01
                    } else if v <= -0.5 {
                        0b10
                    } else {
                        0b00
                    };
                    byte |= code << (slot * 2);
                }
                out[dst + byte_idx] = byte;
            }
        }
        out
    }
}
