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
    let mut out = Vec::with_capacity(n_blocks * BLOCK_SIZE);

    for blk in 0..n_blocks {
        let base = blk * BLOCK_BYTES;
        let ql = &bytes[base..base + 128];
        let qh = &bytes[base + 128..base + 192];
        let scales = &bytes[base + 192..base + 208];
        let d_bits = u16::from_le_bytes([bytes[base + 208], bytes[base + 209]]);
        let d = half::f16::from_bits(d_bits).to_f32();

        // 2 halves of 128 values each. Each half has 8 sub-blocks of 16.
        // llama.cpp layout: for each half h (0,1):
        //   for each set i (0,1,2,3) of 32 values:
        //     for each l in 0..32:
        //       idx = h*128 + i*32 + l
        //       ql_lo = ql[h*64 + i*16 + (l % 16)] (low nibble for l<16, high for l>=16)
        //       qh_bits = qh[h*32 + l] shifted by (i*2)
        //       q6 = (ql_lo & 0xF) | ((qh_bits & 0x3) << 4)
        //       sc_idx = h*8 + i*2 + l/16
        //       val = d * scales[sc_idx] * (q6 - 32)
        //
        // Reference: llama.cpp dequantize_row_q6_K (k-quants.c).
        for h in 0..2 {
            for i in 0..4 {
                for l in 0..32 {
                    let ql_idx = h * 64 + i * 16 + (l % 16);
                    let qh_idx = h * 32 + l;
                    let ql_val = if l < 16 {
                        ql[ql_idx] & 0x0F
                    } else {
                        ql[ql_idx] >> 4
                    };
                    let qh_val = (qh[qh_idx] >> (i * 2)) & 0x03;
                    let q6 = (ql_val as i32) | ((qh_val as i32) << 4);
                    let sc_idx = h * 8 + i * 2 + l / 16;
                    let sc = scales[sc_idx] as i8 as f32;
                    out.push(d * sc * (q6 - 32) as f32);
                }
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
