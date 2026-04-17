//! Q4_K — 4-bit K-quant with 256-value superblocks.
//!
//! Block layout (144 bytes):
//!   f16 d              superblock scale
//!   f16 dmin           superblock min offset
//!   u8  scales[12]     packed 6-bit: 8 sub-block scales + 8 mins
//!   u8  qs[128]        256 nibbles (low nibbles first half per pair of sub-blocks)
//!
//! Dequant: x[i] = d * s[j] * nibble - dmin * m[j], where j = sub-block index.
//!
//! Spec: reference/runtime/quant.md

pub const BLOCK_SIZE: usize = 256;
pub const BLOCK_BYTES: usize = 144;

pub fn dequantize(bytes: &[u8]) -> Vec<f32> {
    assert!(
        bytes.len() % BLOCK_BYTES == 0,
        "Q4_K: byte count {} not multiple of {}",
        bytes.len(),
        BLOCK_BYTES
    );
    let n_blocks = bytes.len() / BLOCK_BYTES;
    let mut out = Vec::with_capacity(n_blocks * BLOCK_SIZE);

    for blk in 0..n_blocks {
        let base = blk * BLOCK_BYTES;
        let d = half::f16::from_bits(u16::from_le_bytes([bytes[base], bytes[base + 1]])).to_f32();
        let dmin =
            half::f16::from_bits(u16::from_le_bytes([bytes[base + 2], bytes[base + 3]])).to_f32();
        let scales = &bytes[base + 4..base + 16];
        let qs = &bytes[base + 16..base + 144];

        // 8 sub-blocks × 32 values.
        for j in 0..8 {
            let (s, m) = unpack_scale_min_k4(j, scales);
            let d_scaled = d * s as f32;
            let m_scaled = dmin * m as f32;

            // Sub-block pair layout: sub-blocks (2k, 2k+1) share a 32-byte qs range.
            // qs range for pair k starts at byte k*32.
            // Within pair: sub-block 2k gets LOW nibbles, 2k+1 gets HIGH nibbles.
            let pair = j / 2;
            let is_upper = j % 2;
            let qs_off = pair * 32;

            for l in 0..32 {
                let byte = qs[qs_off + l];
                let nibble = if is_upper == 0 {
                    byte & 0x0F
                } else {
                    byte >> 4
                } as i32;
                out.push(d_scaled * nibble as f32 - m_scaled);
            }
        }
    }
    out
}

/// Public wrapper for use by Q5_K.
pub fn unpack_scale_min_k4_public(j: usize, scales: &[u8]) -> (u8, u8) {
    unpack_scale_min_k4(j, scales)
}

/// Extract 6-bit scale and min for sub-block j (0..8) from the 12 scale bytes.
fn unpack_scale_min_k4(j: usize, scales: &[u8]) -> (u8, u8) {
    if j < 4 {
        (scales[j] & 0x3F, scales[j + 4] & 0x3F)
    } else {
        let s = (scales[j + 4] & 0x0F) | ((scales[j - 4] >> 6) << 4);
        let m = (scales[j + 4] >> 4) | ((scales[j] >> 6) << 4);
        (s, m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_four_subblocks_give_negative_offset() {
        // Test the simple path: sub-blocks 0..4 use direct packing.
        // nibble=0, d=1.0, dmin=0.5, s=1, m=1 → x = 1*1*0 - 0.5*1 = -0.5
        let mut block = vec![0u8; BLOCK_BYTES];
        let d_bits = half::f16::from_f32(1.0).to_bits();
        let dmin_bits = half::f16::from_f32(0.5).to_bits();
        block[0] = (d_bits & 0xFF) as u8;
        block[1] = (d_bits >> 8) as u8;
        block[2] = (dmin_bits & 0xFF) as u8;
        block[3] = (dmin_bits >> 8) as u8;
        // For j=0..4: scales[j] = 1 (s), scales[j+4] = 1 (m)
        for j in 0..4 {
            block[4 + j] = 1;
            block[4 + j + 4] = 1;
        }

        let out = dequantize(&block);
        assert_eq!(out.len(), 256);
        // Sub-blocks 0..4 = first 128 values, all should be -0.5
        for (i, &v) in out[0..128].iter().enumerate() {
            assert!((v - (-0.5)).abs() < 1e-6, "idx {i}: got {v}");
        }
    }

    #[test]
    fn positive_nibbles_via_scale() {
        let mut block = vec![0u8; BLOCK_BYTES];
        let d_bits = half::f16::from_f32(2.0).to_bits();
        let dmin_bits = half::f16::from_f32(0.0).to_bits();
        block[0] = (d_bits & 0xFF) as u8;
        block[1] = (d_bits >> 8) as u8;
        block[2] = (dmin_bits & 0xFF) as u8;
        block[3] = (dmin_bits >> 8) as u8;
        // Sub-block 0: s=3
        block[4] = 3;
        // dmin=0, so min byte doesn't matter
        // qs[0] = low nibble 5 → first value of sub-block 0 = d*s*5 - 0 = 2*3*5 = 30
        block[16] = 0x05;
        let out = dequantize(&block);
        assert!((out[0] - 30.0).abs() < 1e-6, "got {}", out[0]);
    }

    #[test]
    fn byte_count_validation() {
        let bytes = vec![0u8; BLOCK_BYTES + 1];
        let result = std::panic::catch_unwind(|| dequantize(&bytes));
        assert!(result.is_err());
    }
}
