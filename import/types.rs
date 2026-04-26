//! Local importer types. Purposely narrow — just what the loaders +
//! writer need to move bytes from HF files into `.model` files without
//! dragging in the runtime's Graph/Op/etc.
//!
//! DType here mirrors the set we write to `.model` (plus the two legacy
//! GGUF formats Q4_0 and Q4_1 that we dequant-requant to Q4_K on the
//! way in).

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum DType {
    F32,
    F16,
    BF16,
    I8,
    U8,
    Bool,
    Q8_0,
    Q4_0,
    Q4_1,
    Q2_K,
    Q3_K,
    Q4_K,
    Q5_K,
    Q6_K,
    Ternary,
}

impl DType {
    /// Canonical encoding string used in the `.model` tensor index.
    pub fn canonical_encoding(self) -> &'static str {
        match self {
            DType::F32 => "u32",
            DType::F16 => "u16",
            DType::BF16 => "bf16",
            DType::I8 => "i8",
            DType::U8 => "u8",
            DType::Bool => "bool",
            DType::Q8_0 => "q8",
            DType::Q4_0 => "q4",
            DType::Q4_1 => "q4_1",
            DType::Q2_K => "q2k",
            DType::Q3_K => "q3k",
            DType::Q4_K => "q4k",
            DType::Q5_K => "q5k",
            DType::Q6_K => "q6k",
            DType::Ternary => "ternary",
        }
    }
}

/// A single weight tensor as it arrives from a loader.
#[derive(Clone, Debug)]
pub struct Weight {
    pub data: Vec<u8>,
    pub shape: Vec<usize>,
    pub dtype: DType,
    /// GGUF stores 2D weights with ne[0] as the fast dim. True = caller
    /// should transpose into canonical [N, K] layout when writing.
    pub needs_transpose: bool,
}

/// All weights from one source file, keyed by tensor name.
#[derive(Default)]
pub struct Weights {
    pub weights: HashMap<String, Weight>,
    /// Free-form metadata recovered from the source (e.g. GGUF kv store).
    /// Loaders may populate this; writers are free to ignore it.
    pub metadata: HashMap<String, String>,
}

impl Weights {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, w: Weight) {
        self.weights.insert(name.into(), w);
    }

    pub fn len(&self) -> usize {
        self.weights.len()
    }

    pub fn is_empty(&self) -> bool {
        self.weights.is_empty()
    }
}

/// Dequantize raw weight bytes into f32. Covers every format our loaders
/// emit. Unsupported dtypes return an empty vec and log an error — the
/// importer then skips / errors that tensor rather than producing garbage.
///
/// Ported from llm/src/backend/wgpu/model.rs (safetensors_to_f32).
pub fn dequantize_to_f32(data: &[u8], dtype: DType) -> Vec<f32> {
    match dtype {
        DType::F32 => data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        DType::BF16 => data
            .chunks_exact(2)
            .map(|c| {
                let bits = u16::from_le_bytes([c[0], c[1]]);
                f32::from_bits((bits as u32) << 16)
            })
            .collect(),
        DType::F16 => data
            .chunks_exact(2)
            .map(|c| half::f16::from_le_bytes([c[0], c[1]]).to_f32())
            .collect(),
        DType::Q4_0 => data
            .chunks_exact(18)
            .flat_map(|block| {
                let scale = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
                let qs = &block[2..18];
                let mut vals = [0f32; 32];
                for j in 0..16 {
                    let b = qs[j];
                    vals[j] = scale * ((b & 0x0F) as i32 - 8) as f32;
                    vals[j + 16] = scale * ((b >> 4) as i32 - 8) as f32;
                }
                vals
            })
            .collect(),
        DType::Q4_1 => data
            .chunks_exact(20)
            .flat_map(|block| {
                let scale = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
                let min = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
                let qs = &block[4..20];
                let mut vals = [0f32; 32];
                for j in 0..16 {
                    let b = qs[j];
                    vals[j] = scale * (b & 0x0F) as f32 + min;
                    vals[j + 16] = scale * (b >> 4) as f32 + min;
                }
                vals
            })
            .collect(),
        DType::Q8_0 => data
            .chunks_exact(34)
            .flat_map(|block| {
                let scale = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
                let qs = &block[2..34];
                qs.iter().map(move |&q| scale * (q as i8) as f32)
            })
            .collect(),
        DType::U8 | DType::Ternary => data
            .iter()
            .flat_map(|&byte| {
                (0..4).map(move |i| match (byte >> (i * 2)) & 0x3 {
                    0 => -1.0f32,
                    1 => 0.0f32,
                    2 => 1.0f32,
                    _ => 0.0f32,
                })
            })
            .collect(),
        DType::Q4_K => dequantize_q4k(data),
        DType::Q5_K => dequantize_q5k(data),
        DType::Q6_K => dequantize_q6k(data),
        DType::Q3_K => dequantize_q3k(data),
        DType::Q2_K => dequantize_q2k(data),
        _ => {
            log::error!("dequantize_to_f32: unsupported dtype {:?}", dtype);
            Vec::new()
        }
    }
}

fn get_scale_min_k(scales_raw: &[u8], j: usize) -> (u8, u8) {
    if j < 4 {
        (scales_raw[j] & 63, scales_raw[j + 4] & 63)
    } else {
        let sc = (scales_raw[j + 4] & 0xF) | ((scales_raw[j - 4] >> 6) << 4);
        let mn = (scales_raw[j + 4] >> 4) | ((scales_raw[j] >> 6) << 4);
        (sc, mn)
    }
}

fn dequantize_q4k(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(144)
        .flat_map(|block| {
            let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
            let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
            let scales_raw = &block[4..16];
            let qs = &block[16..144];
            let mut vals = [0f32; 256];
            let mut q_ptr = 0usize;
            let mut is = 0usize;
            for grp in 0..4 {
                let (sc1, m1) = get_scale_min_k(scales_raw, is);
                is += 1;
                let d1 = d * sc1 as f32;
                let m1v = dmin * m1 as f32;
                let (sc2, m2) = get_scale_min_k(scales_raw, is);
                is += 1;
                let d2 = d * sc2 as f32;
                let m2v = dmin * m2 as f32;
                let base = grp * 64;
                for l in 0..32 {
                    vals[base + l] = d1 * (qs[q_ptr + l] & 0xF) as f32 - m1v;
                }
                for l in 0..32 {
                    vals[base + 32 + l] = d2 * (qs[q_ptr + l] >> 4) as f32 - m2v;
                }
                q_ptr += 32;
            }
            vals
        })
        .collect()
}

fn dequantize_q5k(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(176)
        .flat_map(|block| {
            let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
            let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
            let scales_raw = &block[4..16];
            let qh = &block[16..48];
            let qs = &block[48..176];
            let mut vals = [0f32; 256];
            let mut q_ptr = 0usize;
            let mut is = 0usize;
            for grp in 0..4 {
                let (sc1, m1) = get_scale_min_k(scales_raw, is);
                is += 1;
                let d1 = d * sc1 as f32;
                let m1v = dmin * m1 as f32;
                let (sc2, m2) = get_scale_min_k(scales_raw, is);
                is += 1;
                let d2 = d * sc2 as f32;
                let m2v = dmin * m2 as f32;
                let base = grp * 64;
                for l in 0..32 {
                    let lo_nib = qs[q_ptr + l] & 0xF;
                    let lo_idx = grp * 64 + l;
                    let lo_qh = (qh[lo_idx / 8] >> (lo_idx % 8)) & 1;
                    vals[base + l] = d1 * (lo_nib | (lo_qh << 4)) as f32 - m1v;
                }
                for l in 0..32 {
                    let hi_nib = qs[q_ptr + l] >> 4;
                    let hi_idx = grp * 64 + 32 + l;
                    let hi_qh = (qh[hi_idx / 8] >> (hi_idx % 8)) & 1;
                    vals[base + 32 + l] = d2 * (hi_nib | (hi_qh << 4)) as f32 - m2v;
                }
                q_ptr += 32;
            }
            vals
        })
        .collect()
}

fn dequantize_q6k(data: &[u8]) -> Vec<f32> {
    // Reference: llama.cpp dequantize_row_q6_K (ggml/k_quants.c). For each
    // 256-value super-block, two halves of 128 values. Per half, l iterates
    // 0..32 producing FOUR outputs (q1..q4) at offsets l+0, l+32, l+64, l+96.
    // The four use different qh-nibble shifts (0/2/4/6) and ql positions.
    // This matches run::backend::cpu::quant::q6_k::dequantize byte-for-byte.
    let n_blocks = data.len() / 210;
    let mut out = vec![0f32; n_blocks * 256];
    for blk in 0..n_blocks {
        let base = blk * 210;
        let ql = &data[base..base + 128];
        let qh = &data[base + 128..base + 192];
        let scales = &data[base + 192..base + 208];
        let d = half::f16::from_le_bytes([data[base + 208], data[base + 209]]).to_f32();
        let y = &mut out[blk * 256..(blk + 1) * 256];

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
                y[y_off + l + 0] = d * scales[sc_off + is] as i8 as f32 * q1 as f32;
                y[y_off + l + 32] = d * scales[sc_off + 2 + is] as i8 as f32 * q2 as f32;
                y[y_off + l + 64] = d * scales[sc_off + 4 + is] as i8 as f32 * q3 as f32;
                y[y_off + l + 96] = d * scales[sc_off + 6 + is] as i8 as f32 * q4 as f32;
            }
        }
    }
    out
}

fn dequantize_q3k(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(110)
        .flat_map(|block| {
            let hmask = &block[0..32];
            let qs = &block[32..96];
            let scales_raw = &block[96..108];
            let d = half::f16::from_le_bytes([block[108], block[109]]).to_f32();
            let get_scale = |j: usize| -> i32 {
                let us = if j < 4 {
                    (scales_raw[j] & 0xF) as u32
                        | ((((scales_raw[8 + (j >> 1)] >> (4 * (j & 1))) & 3) as u32) << 4)
                } else if j < 8 {
                    let jj = j - 4;
                    (scales_raw[4 + jj] & 0xF) as u32
                        | ((((scales_raw[10 + (jj >> 1)] >> (4 * (jj & 1))) & 3) as u32) << 4)
                } else if j < 12 {
                    let jj = j - 8;
                    (scales_raw[jj] >> 4) as u32
                        | ((((scales_raw[8 + (jj >> 1)] >> (4 * (jj & 1) + 2)) & 3) as u32) << 4)
                } else {
                    let jj = j - 12;
                    (scales_raw[4 + jj] >> 4) as u32
                        | ((((scales_raw[10 + (jj >> 1)] >> (4 * (jj & 1) + 2)) & 3) as u32) << 4)
                };
                us as i32 - 32
            };
            let mut vals = [0f32; 256];
            for sb in 0..16 {
                let sc = get_scale(sb) as f32;
                for l in 0..16 {
                    let j = sb * 16 + l;
                    let ql = (qs[j / 4] >> ((j % 4) * 2)) & 3;
                    let hm = (hmask[j / 8] >> (j % 8)) & 1;
                    let q3 = (ql | (hm << 2)) as i32 - 4;
                    vals[j] = d * sc * q3 as f32;
                }
            }
            vals
        })
        .collect()
}

fn dequantize_q2k(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(84)
        .flat_map(|block| {
            let scales = &block[0..16];
            let qs = &block[16..80];
            let d = half::f16::from_le_bytes([block[80], block[81]]).to_f32();
            let dmin = half::f16::from_le_bytes([block[82], block[83]]).to_f32();
            let mut vals = [0f32; 256];
            for sb in 0..16 {
                let sc = (scales[sb] & 0xF) as f32;
                let m = (scales[sb] >> 4) as f32;
                let ds = d * sc;
                let dm = dmin * m;
                for l in 0..16 {
                    let j = sb * 16 + l;
                    let q2 = (qs[j / 4] >> ((j % 4) * 2)) & 3;
                    vals[j] = ds * q2 as f32 - dm;
                }
            }
            vals
        })
        .collect()
}
