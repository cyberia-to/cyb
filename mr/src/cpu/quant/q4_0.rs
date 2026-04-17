//! Q4_0 — 4-bit symmetric, 32-value blocks.
//!
//! Block layout (18 bytes):
//!   f16 d              (2 bytes — scale)
//!   u8  qs[16]         (16 bytes — 32 nibbles)
//!
//! Low nibble of byte j (j=0..16) stores x[j]. High nibble stores x[j+16].
//!
//! Dequant: x[i] = d * (nibble - 8).
//!
//! Spec: reference/runtime/quant.md

pub const BLOCK_SIZE: usize = 32;
pub const BLOCK_BYTES: usize = 18;

pub fn dequantize(bytes: &[u8]) -> Vec<f32> {
    assert!(
        bytes.len() % BLOCK_BYTES == 0,
        "Q4_0: byte count {} not multiple of {}",
        bytes.len(),
        BLOCK_BYTES
    );
    let n_blocks = bytes.len() / BLOCK_BYTES;
    let mut out = Vec::with_capacity(n_blocks * BLOCK_SIZE);

    for blk in 0..n_blocks {
        let base = blk * BLOCK_BYTES;
        let d_bits = u16::from_le_bytes([bytes[base], bytes[base + 1]]);
        let d = half::f16::from_bits(d_bits).to_f32();
        let qs = &bytes[base + 2..base + 18];
        // First half: low nibbles
        for j in 0..16 {
            out.push(d * ((qs[j] & 0x0F) as i32 - 8) as f32);
        }
        // Second half: high nibbles
        for j in 0..16 {
            out.push(d * ((qs[j] >> 4) as i32 - 8) as f32);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_nibbles_give_negative_8_times_d() {
        // nibble 0 → -8, so x = -8d
        let d = 1.0f32;
        let d_bits = half::f16::from_f32(d).to_bits();
        let mut block = vec![0u8; 18];
        block[0] = (d_bits & 0xFF) as u8;
        block[1] = (d_bits >> 8) as u8;
        // qs all zero = all nibbles 0 = all values -8
        let out = dequantize(&block);
        assert_eq!(out.len(), 32);
        for v in out {
            assert!((v - (-8.0)).abs() < 1e-6);
        }
    }

    #[test]
    fn single_nibble_positive() {
        let d = 0.5f32;
        let d_bits = half::f16::from_f32(d).to_bits();
        let mut block = vec![0u8; 18];
        block[0] = (d_bits & 0xFF) as u8;
        block[1] = (d_bits >> 8) as u8;
        // qs[0] = 0x9F: low nibble=15, high nibble=9
        block[2] = 0x9F;
        let out = dequantize(&block);
        // out[0] = d * (15-8) = 0.5 * 7 = 3.5
        assert!((out[0] - 3.5).abs() < 1e-6);
        // out[16] = d * (9-8) = 0.5 * 1 = 0.5
        assert!((out[16] - 0.5).abs() < 1e-6);
    }
}
