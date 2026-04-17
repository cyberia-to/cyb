//! Q8_0 — 8-bit symmetric, 32-value blocks.
//!
//! Block layout (34 bytes):
//!   f16 d              (2 bytes)
//!   i8  qs[32]         (32 bytes)
//!
//! Dequant: x[i] = d * qs[i].
//!
//! Spec: reference/runtime/quant.md

pub const BLOCK_SIZE: usize = 32;
pub const BLOCK_BYTES: usize = 34;

pub fn dequantize(bytes: &[u8]) -> Vec<f32> {
    assert!(
        bytes.len() % BLOCK_BYTES == 0,
        "Q8_0: byte count {} not multiple of {}",
        bytes.len(),
        BLOCK_BYTES
    );
    let n_blocks = bytes.len() / BLOCK_BYTES;
    let mut out = Vec::with_capacity(n_blocks * BLOCK_SIZE);
    for blk in 0..n_blocks {
        let base = blk * BLOCK_BYTES;
        let d_bits = u16::from_le_bytes([bytes[base], bytes[base + 1]]);
        let d = half::f16::from_bits(d_bits).to_f32();
        for j in 0..32 {
            let q = bytes[base + 2 + j] as i8;
            out.push(d * q as f32);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_d_and_q() {
        let d = 1.0f32;
        let d_bits = half::f16::from_f32(d).to_bits();
        let mut block = vec![0u8; 34];
        block[0] = (d_bits & 0xFF) as u8;
        block[1] = (d_bits >> 8) as u8;
        // Counting qs = 0..32
        for j in 0..32 {
            block[2 + j] = j as u8;
        }
        let out = dequantize(&block);
        for j in 0..32 {
            assert!((out[j] - j as f32).abs() < 1e-6);
        }
    }

    #[test]
    fn negative_values() {
        let d = 1.0f32;
        let d_bits = half::f16::from_f32(d).to_bits();
        let mut block = vec![0u8; 34];
        block[0] = (d_bits & 0xFF) as u8;
        block[1] = (d_bits >> 8) as u8;
        block[2] = (-5i8) as u8;
        let out = dequantize(&block);
        assert!((out[0] - (-5.0)).abs() < 1e-6);
    }
}
