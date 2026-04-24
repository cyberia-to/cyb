//! Q6_K — 6-bit K-quant with 256-value superblocks, 16 sub-blocks of 16.
//!
//! Block layout (210 bytes):
//!   ql[128]    low 4 bits per value
//!   qh[64]     high 2 bits per value
//!   scales[16] i8 per-sub-block scales
//!   f16 d      superblock scale
//!
//! Dequant: x = d * scale[j] * (q - 32), where q is the 6-bit reconstructed value.
//!
//! Spec: reference/runtime/quant.md

pub const BLOCK_SIZE: usize = 256;
pub const BLOCK_BYTES: usize = 210;

pub fn dequantize(bytes: &[u8]) -> Vec<f32> {
    assert!(
        bytes.len() % BLOCK_BYTES == 0,
        "Q6_K: byte count {} not multiple of {}",
        bytes.len(),
        BLOCK_BYTES
    );
    let n_blocks = bytes.len() / BLOCK_BYTES;
    let mut out = vec![0f32; n_blocks * BLOCK_SIZE];

    // Reference: llama.cpp dequantize_row_q6_K (ggml/k_quants.c). For each
    // 256-value super-block, two halves of 128 values. Per half, l iterates
    // 0..32 producing FOUR outputs (q1..q4) at offsets l+0, l+32, l+64, l+96.
    // The four use different qh-nibble shifts (0/2/4/6) and ql positions.
    for blk in 0..n_blocks {
        let base = blk * BLOCK_BYTES;
        let ql = &bytes[base..base + 128];
        let qh = &bytes[base + 128..base + 192];
        let scales = &bytes[base + 192..base + 208];
        let d_bits = u16::from_le_bytes([bytes[base + 208], bytes[base + 209]]);
        let d = half::f16::from_bits(d_bits).to_f32();
        let y = &mut out[blk * BLOCK_SIZE..(blk + 1) * BLOCK_SIZE];

        for half in 0..2 {
            let ql_off = half * 64;
            let qh_off = half * 32;
            let sc_off = half * 8;
            let y_off = half * 128;
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
                y[y_off + l + 0] = d * s1 * q1 as f32;
                y[y_off + l + 32] = d * s2 * q2 as f32;
                y[y_off + l + 64] = d * s3 * q3 as f32;
                y[y_off + l + 96] = d * s4 * q4 as f32;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_block_gives_zero_when_d_zero() {
        let block = vec![0u8; BLOCK_BYTES];
        let out = dequantize(&block);
        // d_bits=0 → d=0, so all outputs zero regardless.
        assert_eq!(out.len(), BLOCK_SIZE);
        for v in out {
            assert_eq!(v, 0.0);
        }
    }

    #[test]
    fn d_one_with_zero_q_gives_negative_32_times_scale() {
        // q6=0 → val = d * sc * (0 - 32) = -32 * d * sc
        let mut block = vec![0u8; BLOCK_BYTES];
        // d = 1.0
        let d_bits = half::f16::from_f32(1.0).to_bits();
        block[208] = (d_bits & 0xFF) as u8;
        block[209] = (d_bits >> 8) as u8;
        // All scales = 1
        for i in 192..208 {
            block[i] = 1;
        }
        let out = dequantize(&block);
        for v in out {
            assert!((v - (-32.0)).abs() < 1e-6, "got {v}");
        }
    }

    #[test]
    fn byte_count_validation() {
        let bytes = vec![0u8; BLOCK_BYTES + 1];
        assert!(std::panic::catch_unwind(|| dequantize(&bytes)).is_err());
    }
}
