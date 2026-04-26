//! Canonical decoders — `cyb/cyb-model` five-encoding set.
//!
//! `u32`, `u16`, `q8`, `q4`, `ternary`. Integer fixed-point throughout;
//! floats are not stored on disk under the canonical spec. These are the
//! reader counterparts to the encoders in `import::quant::canonical`.
//!
//! Decoders are the source of truth for the dequant formulas; matmul and
//! other kernels that fuse dequant must agree byte-for-byte with these.

pub const BLOCK: usize = 32;

/// Decode canonical `u32` (16.16 fixed-point) → f32.
pub fn u32_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| {
            let i = i32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            i as f32 / 65536.0
        })
        .collect()
}

/// Decode canonical `u16` (8.8 fixed-point) → f32.
pub fn u16_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(2)
        .map(|c| {
            let i = i16::from_le_bytes([c[0], c[1]]);
            i as f32 / 256.0
        })
        .collect()
}

/// Decode canonical `q8` → f32. Block layout: [u16 scale (8.8) | 32 × i8].
pub fn q8_to_f32(bytes: &[u8]) -> Vec<f32> {
    let block_bytes = 2 + BLOCK;
    let blocks = bytes.len() / block_bytes;
    let mut out = Vec::with_capacity(blocks * BLOCK);
    for b in 0..blocks {
        let off = b * block_bytes;
        let scale = i16::from_le_bytes([bytes[off], bytes[off + 1]]) as f32 / 256.0;
        for i in 0..BLOCK {
            let q = bytes[off + 2 + i] as i8;
            out.push(q as f32 * scale / 127.0);
        }
    }
    out
}

/// Decode canonical `q4` → f32. Block layout: [u16 scale (8.8) | 16 × packed nibbles].
/// Split nibble layout (matches GGUF Q4_0): byte `i` holds value `i` (low
/// nibble) and value `i + 16` (high nibble).
pub fn q4_to_f32(bytes: &[u8]) -> Vec<f32> {
    let block_bytes = 2 + BLOCK / 2;
    let blocks = bytes.len() / block_bytes;
    let mut out = vec![0f32; blocks * BLOCK];
    for b in 0..blocks {
        let off = b * block_bytes;
        let scale = i16::from_le_bytes([bytes[off], bytes[off + 1]]) as f32 / 256.0;
        let block_start = b * BLOCK;
        for i in 0..(BLOCK / 2) {
            let byte = bytes[off + 2 + i];
            let lo = (byte & 0x0F) as i32 - 8;
            let hi = ((byte >> 4) & 0x0F) as i32 - 8;
            out[block_start + i] = lo as f32 * scale / 8.0;
            out[block_start + i + 16] = hi as f32 * scale / 8.0;
        }
    }
    out
}

/// Decode canonical `ternary` → f32. Layout: 2 bits per value, packed 4-per-byte.
/// Codes: `00 = 0.0`, `01 = +1.0`, `10 = -1.0`. `11` is reserved (decoded as 0).
pub fn ternary_to_f32(bytes: &[u8]) -> Vec<f32> {
    let mut out = Vec::with_capacity(bytes.len() * 4);
    for &byte in bytes {
        for slot in 0..4 {
            let code = (byte >> (slot * 2)) & 0b11;
            let v = match code {
                0b00 => 0.0,
                0b01 => 1.0,
                0b10 => -1.0,
                _ => 0.0,
            };
            out.push(v);
        }
    }
    out
}
