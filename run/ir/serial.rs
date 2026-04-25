//! Graph binary serialization / deserialization.
//!
//! Encodes a [`Graph`] to a compact little-endian binary blob and decodes it
//! back. The blob is stored hex-encoded in the `.model` `~~~graph` text section
//! (see specs/format.md §"graph section").
//!
//! Op tags are the stable u16 values from specs/ir.md §"Op tags".
//! Op payload follows specs/ir.md §"Op payload encoding", extended with
//! the `rope_dim` field for `Rope` (see §"Stateful execution" note).
//!
//! Only the ops used by `transformer_decoder_for_exec` (and the full op table
//! from ir.md) are encoded. Unknown op tags on deserialization return an error
//! rather than silently skipping.
//!
//! Spec: specs/ir.md §"Serialization (binary)"

use super::graph::{Graph, Node};
use crate::core::op::{InterpolateMode, Op, PoolMode, SampleMethod};
use std::io::{self, Cursor, Read};

#[derive(Debug, thiserror::Error)]
pub enum SerialError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("unknown op tag: {0}")]
    UnknownOpTag(u16),
    #[error("truncated: {0}")]
    Truncated(&'static str),
}

// ── write helpers ────────────────────────────────────────────────────────────

fn write_u8(w: &mut Vec<u8>, v: u8) { w.push(v); }
fn write_u16(w: &mut Vec<u8>, v: u16) { w.extend_from_slice(&v.to_le_bytes()); }
fn write_u32(w: &mut Vec<u8>, v: u32) { w.extend_from_slice(&v.to_le_bytes()); }
fn write_i32(w: &mut Vec<u8>, v: i32) { w.extend_from_slice(&v.to_le_bytes()); }
fn write_f32(w: &mut Vec<u8>, v: f32) { w.extend_from_slice(&v.to_le_bytes()); }
fn write_i64(w: &mut Vec<u8>, v: i64) { w.extend_from_slice(&v.to_le_bytes()); }
fn write_string(w: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    write_u32(w, b.len() as u32);
    w.extend_from_slice(b);
}
fn write_strings(w: &mut Vec<u8>, ss: &[String]) {
    write_u32(w, ss.len() as u32);
    for s in ss { write_string(w, s); }
}

// ── read helpers ─────────────────────────────────────────────────────────────

fn read_u8(r: &mut Cursor<&[u8]>) -> Result<u8, SerialError> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).map_err(|_| SerialError::Truncated("u8"))?;
    Ok(b[0])
}
fn read_u16(r: &mut Cursor<&[u8]>) -> Result<u16, SerialError> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b).map_err(|_| SerialError::Truncated("u16"))?;
    Ok(u16::from_le_bytes(b))
}
fn read_u32(r: &mut Cursor<&[u8]>) -> Result<u32, SerialError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|_| SerialError::Truncated("u32"))?;
    Ok(u32::from_le_bytes(b))
}
fn read_i32(r: &mut Cursor<&[u8]>) -> Result<i32, SerialError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|_| SerialError::Truncated("i32"))?;
    Ok(i32::from_le_bytes(b))
}
fn read_f32(r: &mut Cursor<&[u8]>) -> Result<f32, SerialError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|_| SerialError::Truncated("f32"))?;
    Ok(f32::from_le_bytes(b))
}
fn read_i64(r: &mut Cursor<&[u8]>) -> Result<i64, SerialError> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|_| SerialError::Truncated("i64"))?;
    Ok(i64::from_le_bytes(b))
}
fn read_string(r: &mut Cursor<&[u8]>) -> Result<String, SerialError> {
    let len = read_u32(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(|_| SerialError::Truncated("string bytes"))?;
    String::from_utf8(buf).map_err(|e| SerialError::Io(io::Error::new(io::ErrorKind::InvalidData, e)))
}
fn read_strings(r: &mut Cursor<&[u8]>) -> Result<Vec<String>, SerialError> {
    let n = read_u32(r)? as usize;
    (0..n).map(|_| read_string(r)).collect()
}

// ── op tag constants (stable, never reused) ───────────────────────────────────

const TAG_MATMUL: u16 = 0;
const TAG_ADD: u16 = 1;
const TAG_MUL: u16 = 2;
const TAG_SUB: u16 = 3;
const TAG_DIV: u16 = 4;
const TAG_TRANSPOSE: u16 = 5;
const TAG_RESHAPE: u16 = 6;
const TAG_PERMUTE: u16 = 7;
const TAG_CONCAT: u16 = 8;
const TAG_SPLIT: u16 = 9;
const TAG_CHUNK: u16 = 10;
const TAG_CLAMP: u16 = 11;
const TAG_NANTNUM: u16 = 12;
const TAG_SDPA: u16 = 20;
const TAG_SDPA_CROSS: u16 = 21;
const TAG_SDPA_WINDOW: u16 = 22;
const TAG_KV_CACHE: u16 = 23;
const TAG_KV_COMPRESS: u16 = 24;
const TAG_KV_DECOMPRESS: u16 = 25;
const TAG_ROPE: u16 = 26;
const TAG_SINUSOIDAL_EMBED: u16 = 27;
const TAG_RELATIVE_POS_EMBEDDING: u16 = 28;
const TAG_RMS_NORM: u16 = 40;
const TAG_LAYER_NORM: u16 = 41;
const TAG_BATCH_NORM: u16 = 42;
const TAG_GROUP_NORM: u16 = 43;
const TAG_INSTANCE_NORM: u16 = 44;
const TAG_ADA_LN: u16 = 45;
const TAG_SILU: u16 = 60;
const TAG_GELU: u16 = 61;
const TAG_GEGLU: u16 = 62;
const TAG_SWIGLU: u16 = 63;
const TAG_GLU: u16 = 64;
const TAG_RELU: u16 = 65;
const TAG_LEAKY_RELU: u16 = 66;
const TAG_PRELU: u16 = 67;
const TAG_SIGMOID: u16 = 68;
const TAG_TANH: u16 = 69;
const TAG_SOFTMAX: u16 = 70;
const TAG_CONV1D: u16 = 80;
const TAG_CONV2D: u16 = 81;
const TAG_CONV3D: u16 = 82;
const TAG_CONV_TRANSPOSE2D: u16 = 83;
const TAG_CAUSAL_CONV1D: u16 = 84;
const TAG_DEPTHWISE_CONV: u16 = 85;
const TAG_POOL: u16 = 86;
const TAG_INTERPOLATE: u16 = 100;
const TAG_PIXEL_SHUFFLE: u16 = 101;
const TAG_PIXEL_UNSHUFFLE: u16 = 102;
const TAG_PATCH_EMBED: u16 = 103;
const TAG_UNPATCHIFY: u16 = 104;
const TAG_TOKEN_EMBED: u16 = 120;
const TAG_POS_EMBED: u16 = 121;
const TAG_NOISE_SCHEDULE: u16 = 140;
const TAG_FLOW_STEP: u16 = 141;
const TAG_QUANTIZE: u16 = 142;
const TAG_DEQUANTIZE: u16 = 143;
const TAG_SAMPLE: u16 = 144;
const TAG_LORA_APPLY: u16 = 160;
const TAG_KRON: u16 = 161;
const TAG_MATRIX_INVERSE: u16 = 162;
const TAG_FUSED_NORM_MATMUL: u16 = 180;
const TAG_FUSED_SKIP_NORM: u16 = 181;
const TAG_FUSED_SWIGLU: u16 = 182;
const TAG_FLASH_ATTENTION: u16 = 183;
const TAG_ARGMAX: u16 = 199;

// ── op serialization ──────────────────────────────────────────────────────────

fn serialize_op(w: &mut Vec<u8>, op: &Op) {
    match op {
        Op::Matmul          => write_u16(w, TAG_MATMUL),
        Op::Add             => write_u16(w, TAG_ADD),
        Op::Mul             => write_u16(w, TAG_MUL),
        Op::Sub             => write_u16(w, TAG_SUB),
        Op::Div             => write_u16(w, TAG_DIV),
        Op::Transpose { perm } => {
            write_u16(w, TAG_TRANSPOSE);
            write_u32(w, perm.len() as u32);
            for &d in perm { write_u32(w, d as u32); }
        }
        Op::Reshape { shape } => {
            write_u16(w, TAG_RESHAPE);
            write_u32(w, shape.len() as u32);
            for &d in shape { write_i64(w, d); }
        }
        Op::Permute { dims } => {
            write_u16(w, TAG_PERMUTE);
            write_u32(w, dims.len() as u32);
            for &d in dims { write_u32(w, d as u32); }
        }
        Op::Concat { axis } => {
            write_u16(w, TAG_CONCAT);
            write_u32(w, *axis as u32);
        }
        Op::Split { axis, sizes } => {
            write_u16(w, TAG_SPLIT);
            write_u32(w, *axis as u32);
            write_u32(w, sizes.len() as u32);
            for &s in sizes { write_u32(w, s as u32); }
        }
        Op::Chunk { axis, chunks } => {
            write_u16(w, TAG_CHUNK);
            write_u32(w, *axis as u32);
            write_u32(w, *chunks as u32);
        }
        Op::Clamp { min, max } => {
            write_u16(w, TAG_CLAMP);
            write_u8(w, min.is_some() as u8);
            if let Some(v) = min { write_f32(w, *v); }
            write_u8(w, max.is_some() as u8);
            if let Some(v) = max { write_f32(w, *v); }
        }
        Op::NanToNum { nan, posinf, neginf } => {
            write_u16(w, TAG_NANTNUM);
            write_f32(w, *nan);
            write_f32(w, *posinf);
            write_f32(w, *neginf);
        }
        Op::Sdpa { num_heads, kv_heads, head_dim, causal } => {
            write_u16(w, TAG_SDPA);
            write_u32(w, *num_heads);
            write_u32(w, *kv_heads);
            write_u32(w, *head_dim);
            write_u8(w, *causal as u8);
        }
        Op::SdpaCross { num_heads, head_dim } => {
            write_u16(w, TAG_SDPA_CROSS);
            write_u32(w, *num_heads);
            write_u32(w, *head_dim);
        }
        Op::SdpaWindow { num_heads, head_dim, window_size } => {
            write_u16(w, TAG_SDPA_WINDOW);
            write_u32(w, *num_heads);
            write_u32(w, *head_dim);
            write_u32(w, *window_size);
        }
        Op::KvCache => write_u16(w, TAG_KV_CACHE),
        Op::KvCompress { head_dim, bits } => {
            write_u16(w, TAG_KV_COMPRESS);
            write_u32(w, *head_dim);
            write_u32(w, *bits);
        }
        Op::KvDecompress { head_dim, bits } => {
            write_u16(w, TAG_KV_DECOMPRESS);
            write_u32(w, *head_dim);
            write_u32(w, *bits);
        }
        Op::Rope { head_dim, rope_dim, base } => {
            write_u16(w, TAG_ROPE);
            write_u32(w, *head_dim);
            write_u32(w, *rope_dim);
            write_f32(w, *base);
        }
        Op::SinusoidalEmbed { dim } => {
            write_u16(w, TAG_SINUSOIDAL_EMBED);
            write_u32(w, *dim);
        }
        Op::RelativePosEmbedding { num_buckets } => {
            write_u16(w, TAG_RELATIVE_POS_EMBEDDING);
            write_u32(w, *num_buckets);
        }
        Op::RmsNorm { eps } => { write_u16(w, TAG_RMS_NORM); write_f32(w, *eps); }
        Op::LayerNorm { eps } => { write_u16(w, TAG_LAYER_NORM); write_f32(w, *eps); }
        Op::BatchNorm { eps, momentum } => {
            write_u16(w, TAG_BATCH_NORM);
            write_f32(w, *eps);
            write_f32(w, *momentum);
        }
        Op::GroupNorm { num_groups, eps } => {
            write_u16(w, TAG_GROUP_NORM);
            write_u32(w, *num_groups);
            write_f32(w, *eps);
        }
        Op::InstanceNorm { eps } => { write_u16(w, TAG_INSTANCE_NORM); write_f32(w, *eps); }
        Op::AdaLN => write_u16(w, TAG_ADA_LN),
        Op::Silu   => write_u16(w, TAG_SILU),
        Op::Gelu { approximate } => {
            write_u16(w, TAG_GELU);
            write_u8(w, *approximate as u8);
        }
        Op::GeGlu  => write_u16(w, TAG_GEGLU),
        Op::SwiGlu => write_u16(w, TAG_SWIGLU),
        Op::Glu    => write_u16(w, TAG_GLU),
        Op::Relu   => write_u16(w, TAG_RELU),
        Op::LeakyRelu { slope } => { write_u16(w, TAG_LEAKY_RELU); write_f32(w, *slope); }
        Op::PRelu  => write_u16(w, TAG_PRELU),
        Op::Sigmoid => write_u16(w, TAG_SIGMOID),
        Op::Tanh   => write_u16(w, TAG_TANH),
        Op::Softmax { dim } => { write_u16(w, TAG_SOFTMAX); write_i32(w, *dim); }
        Op::Conv1d { kernel, stride, padding, dilation, groups } => {
            write_u16(w, TAG_CONV1D);
            write_u32(w, *kernel); write_u32(w, *stride);
            write_u32(w, *padding); write_u32(w, *dilation); write_u32(w, *groups);
        }
        Op::Conv2d { kernel, stride, padding, dilation, groups } => {
            write_u16(w, TAG_CONV2D);
            write_u32(w, kernel.0); write_u32(w, kernel.1);
            write_u32(w, stride.0); write_u32(w, stride.1);
            write_u32(w, padding.0); write_u32(w, padding.1);
            write_u32(w, dilation.0); write_u32(w, dilation.1);
            write_u32(w, *groups);
        }
        Op::Conv3d { kernel, stride, padding, dilation, groups } => {
            write_u16(w, TAG_CONV3D);
            write_u32(w, kernel.0); write_u32(w, kernel.1); write_u32(w, kernel.2);
            write_u32(w, stride.0); write_u32(w, stride.1); write_u32(w, stride.2);
            write_u32(w, padding.0); write_u32(w, padding.1); write_u32(w, padding.2);
            write_u32(w, dilation.0); write_u32(w, dilation.1); write_u32(w, dilation.2);
            write_u32(w, *groups);
        }
        Op::ConvTranspose2d { kernel, stride, padding } => {
            write_u16(w, TAG_CONV_TRANSPOSE2D);
            write_u32(w, kernel.0); write_u32(w, kernel.1);
            write_u32(w, stride.0); write_u32(w, stride.1);
            write_u32(w, padding.0); write_u32(w, padding.1);
        }
        Op::CausalConv1d { kernel } => { write_u16(w, TAG_CAUSAL_CONV1D); write_u32(w, *kernel); }
        Op::DepthwiseConv { kernel, stride } => {
            write_u16(w, TAG_DEPTHWISE_CONV);
            write_u32(w, *kernel); write_u32(w, *stride);
        }
        Op::Pool { mode, kernel, stride, padding } => {
            write_u16(w, TAG_POOL);
            write_u8(w, match mode { PoolMode::Max => 0, PoolMode::Avg => 1 });
            write_u32(w, kernel.0); write_u32(w, kernel.1);
            write_u32(w, stride.0); write_u32(w, stride.1);
            write_u32(w, padding.0); write_u32(w, padding.1);
        }
        Op::Interpolate { mode, scale } => {
            write_u16(w, TAG_INTERPOLATE);
            write_u8(w, match mode {
                InterpolateMode::Nearest => 0,
                InterpolateMode::Bilinear => 1,
                InterpolateMode::Area => 2,
            });
            write_f32(w, *scale);
        }
        Op::PixelShuffle { upscale_factor }   => { write_u16(w, TAG_PIXEL_SHUFFLE);   write_u32(w, *upscale_factor); }
        Op::PixelUnshuffle { downscale_factor } => { write_u16(w, TAG_PIXEL_UNSHUFFLE); write_u32(w, *downscale_factor); }
        Op::PatchEmbed { patch_size }          => { write_u16(w, TAG_PATCH_EMBED);     write_u32(w, *patch_size); }
        Op::Unpatchify                         => write_u16(w, TAG_UNPATCHIFY),
        Op::TokenEmbed                         => write_u16(w, TAG_TOKEN_EMBED),
        Op::PosEmbed                           => write_u16(w, TAG_POS_EMBED),
        Op::NoiseSchedule                      => write_u16(w, TAG_NOISE_SCHEDULE),
        Op::FlowStep                           => write_u16(w, TAG_FLOW_STEP),
        Op::Quantize { dtype }                 => { write_u16(w, TAG_QUANTIZE); write_u8(w, dtype.tag()); }
        Op::Dequantize                         => write_u16(w, TAG_DEQUANTIZE),
        Op::Sample { method } => {
            write_u16(w, TAG_SAMPLE);
            match method {
                SampleMethod::Greedy                     => { write_u8(w, 0); write_f32(w, 1.0); write_f32(w, 0.0); write_u32(w, 0); }
                SampleMethod::Temperature { temp }       => { write_u8(w, 1); write_f32(w, *temp); write_f32(w, 0.0); write_u32(w, 0); }
                SampleMethod::TopK { k, temp }           => { write_u8(w, 2); write_f32(w, *temp); write_f32(w, 0.0); write_u32(w, *k); }
                SampleMethod::TopP { p, temp }           => { write_u8(w, 3); write_f32(w, *temp); write_f32(w, *p); write_u32(w, 0); }
                SampleMethod::MinP { p, temp }           => { write_u8(w, 4); write_f32(w, *temp); write_f32(w, *p); write_u32(w, 0); }
            }
        }
        Op::LoraApply { rank, alpha } => { write_u16(w, TAG_LORA_APPLY); write_u32(w, *rank); write_f32(w, *alpha); }
        Op::Kron          => write_u16(w, TAG_KRON),
        Op::MatrixInverse => write_u16(w, TAG_MATRIX_INVERSE),
        Op::FusedNormMatmul { eps }  => { write_u16(w, TAG_FUSED_NORM_MATMUL);  write_f32(w, *eps); }
        Op::FusedSkipNorm { eps }    => { write_u16(w, TAG_FUSED_SKIP_NORM);    write_f32(w, *eps); }
        Op::FusedSwiGlu              => write_u16(w, TAG_FUSED_SWIGLU),
        Op::FlashAttention { num_heads, kv_heads, head_dim } => {
            write_u16(w, TAG_FLASH_ATTENTION);
            write_u32(w, *num_heads); write_u32(w, *kv_heads); write_u32(w, *head_dim);
        }
        Op::Argmax { dim } => { write_u16(w, TAG_ARGMAX); write_i32(w, *dim); }
    }
}

// ── op deserialization ────────────────────────────────────────────────────────

fn deserialize_op(r: &mut Cursor<&[u8]>) -> Result<Op, SerialError> {
    use crate::core::dtype::DType;
    let tag = read_u16(r)?;
    Ok(match tag {
        TAG_MATMUL   => Op::Matmul,
        TAG_ADD      => Op::Add,
        TAG_MUL      => Op::Mul,
        TAG_SUB      => Op::Sub,
        TAG_DIV      => Op::Div,
        TAG_TRANSPOSE => {
            let n = read_u32(r)? as usize;
            let perm = (0..n).map(|_| read_u32(r).map(|v| v as usize)).collect::<Result<_, _>>()?;
            Op::Transpose { perm }
        }
        TAG_RESHAPE => {
            let n = read_u32(r)? as usize;
            let shape = (0..n).map(|_| read_i64(r)).collect::<Result<_, _>>()?;
            Op::Reshape { shape }
        }
        TAG_PERMUTE => {
            let n = read_u32(r)? as usize;
            let dims = (0..n).map(|_| read_u32(r).map(|v| v as usize)).collect::<Result<_, _>>()?;
            Op::Permute { dims }
        }
        TAG_CONCAT => { let axis = read_u32(r)? as usize; Op::Concat { axis } }
        TAG_SPLIT  => {
            let axis = read_u32(r)? as usize;
            let n = read_u32(r)? as usize;
            let sizes = (0..n).map(|_| read_u32(r).map(|v| v as usize)).collect::<Result<_, _>>()?;
            Op::Split { axis, sizes }
        }
        TAG_CHUNK => {
            let axis   = read_u32(r)? as usize;
            let chunks = read_u32(r)? as usize;
            Op::Chunk { axis, chunks }
        }
        TAG_CLAMP => {
            let has_min = read_u8(r)? != 0;
            let min = if has_min { Some(read_f32(r)?) } else { None };
            let has_max = read_u8(r)? != 0;
            let max = if has_max { Some(read_f32(r)?) } else { None };
            Op::Clamp { min, max }
        }
        TAG_NANTNUM => {
            let nan = read_f32(r)?; let posinf = read_f32(r)?; let neginf = read_f32(r)?;
            Op::NanToNum { nan, posinf, neginf }
        }
        TAG_SDPA => {
            let num_heads = read_u32(r)?; let kv_heads = read_u32(r)?;
            let head_dim = read_u32(r)?; let causal = read_u8(r)? != 0;
            Op::Sdpa { num_heads, kv_heads, head_dim, causal }
        }
        TAG_SDPA_CROSS => {
            let num_heads = read_u32(r)?; let head_dim = read_u32(r)?;
            Op::SdpaCross { num_heads, head_dim }
        }
        TAG_SDPA_WINDOW => {
            let num_heads = read_u32(r)?; let head_dim = read_u32(r)?; let window_size = read_u32(r)?;
            Op::SdpaWindow { num_heads, head_dim, window_size }
        }
        TAG_KV_CACHE => Op::KvCache,
        TAG_KV_COMPRESS => {
            let head_dim = read_u32(r)?; let bits = read_u32(r)?;
            Op::KvCompress { head_dim, bits }
        }
        TAG_KV_DECOMPRESS => {
            let head_dim = read_u32(r)?; let bits = read_u32(r)?;
            Op::KvDecompress { head_dim, bits }
        }
        TAG_ROPE => {
            let head_dim = read_u32(r)?; let rope_dim = read_u32(r)?; let base = read_f32(r)?;
            Op::Rope { head_dim, rope_dim, base }
        }
        TAG_SINUSOIDAL_EMBED => { let dim = read_u32(r)?; Op::SinusoidalEmbed { dim } }
        TAG_RELATIVE_POS_EMBEDDING => { let num_buckets = read_u32(r)?; Op::RelativePosEmbedding { num_buckets } }
        TAG_RMS_NORM     => { let eps = read_f32(r)?; Op::RmsNorm { eps } }
        TAG_LAYER_NORM   => { let eps = read_f32(r)?; Op::LayerNorm { eps } }
        TAG_BATCH_NORM   => { let eps = read_f32(r)?; let momentum = read_f32(r)?; Op::BatchNorm { eps, momentum } }
        TAG_GROUP_NORM   => { let num_groups = read_u32(r)?; let eps = read_f32(r)?; Op::GroupNorm { num_groups, eps } }
        TAG_INSTANCE_NORM => { let eps = read_f32(r)?; Op::InstanceNorm { eps } }
        TAG_ADA_LN => Op::AdaLN,
        TAG_SILU    => Op::Silu,
        TAG_GELU    => { let approximate = read_u8(r)? != 0; Op::Gelu { approximate } }
        TAG_GEGLU   => Op::GeGlu,
        TAG_SWIGLU  => Op::SwiGlu,
        TAG_GLU     => Op::Glu,
        TAG_RELU    => Op::Relu,
        TAG_LEAKY_RELU => { let slope = read_f32(r)?; Op::LeakyRelu { slope } }
        TAG_PRELU   => Op::PRelu,
        TAG_SIGMOID => Op::Sigmoid,
        TAG_TANH    => Op::Tanh,
        TAG_SOFTMAX => { let dim = read_i32(r)?; Op::Softmax { dim } }
        TAG_CONV1D  => {
            let kernel = read_u32(r)?; let stride = read_u32(r)?;
            let padding = read_u32(r)?; let dilation = read_u32(r)?; let groups = read_u32(r)?;
            Op::Conv1d { kernel, stride, padding, dilation, groups }
        }
        TAG_CONV2D => {
            let kernel = (read_u32(r)?, read_u32(r)?);
            let stride = (read_u32(r)?, read_u32(r)?);
            let padding = (read_u32(r)?, read_u32(r)?);
            let dilation = (read_u32(r)?, read_u32(r)?);
            let groups = read_u32(r)?;
            Op::Conv2d { kernel, stride, padding, dilation, groups }
        }
        TAG_CONV3D => {
            let kernel = (read_u32(r)?, read_u32(r)?, read_u32(r)?);
            let stride = (read_u32(r)?, read_u32(r)?, read_u32(r)?);
            let padding = (read_u32(r)?, read_u32(r)?, read_u32(r)?);
            let dilation = (read_u32(r)?, read_u32(r)?, read_u32(r)?);
            let groups = read_u32(r)?;
            Op::Conv3d { kernel, stride, padding, dilation, groups }
        }
        TAG_CONV_TRANSPOSE2D => {
            let kernel = (read_u32(r)?, read_u32(r)?);
            let stride = (read_u32(r)?, read_u32(r)?);
            let padding = (read_u32(r)?, read_u32(r)?);
            Op::ConvTranspose2d { kernel, stride, padding }
        }
        TAG_CAUSAL_CONV1D   => { let kernel = read_u32(r)?; Op::CausalConv1d { kernel } }
        TAG_DEPTHWISE_CONV  => { let kernel = read_u32(r)?; let stride = read_u32(r)?; Op::DepthwiseConv { kernel, stride } }
        TAG_POOL => {
            let mode = match read_u8(r)? { 1 => PoolMode::Avg, _ => PoolMode::Max };
            let kernel = (read_u32(r)?, read_u32(r)?);
            let stride = (read_u32(r)?, read_u32(r)?);
            let padding = (read_u32(r)?, read_u32(r)?);
            Op::Pool { mode, kernel, stride, padding }
        }
        TAG_INTERPOLATE => {
            let mode = match read_u8(r)? {
                1 => InterpolateMode::Bilinear, 2 => InterpolateMode::Area, _ => InterpolateMode::Nearest,
            };
            let scale = read_f32(r)?;
            Op::Interpolate { mode, scale }
        }
        TAG_PIXEL_SHUFFLE    => { let f = read_u32(r)?; Op::PixelShuffle { upscale_factor: f } }
        TAG_PIXEL_UNSHUFFLE  => { let f = read_u32(r)?; Op::PixelUnshuffle { downscale_factor: f } }
        TAG_PATCH_EMBED      => { let p = read_u32(r)?; Op::PatchEmbed { patch_size: p } }
        TAG_UNPATCHIFY       => Op::Unpatchify,
        TAG_TOKEN_EMBED      => Op::TokenEmbed,
        TAG_POS_EMBED        => Op::PosEmbed,
        TAG_NOISE_SCHEDULE   => Op::NoiseSchedule,
        TAG_FLOW_STEP        => Op::FlowStep,
        TAG_QUANTIZE         => { let tag = read_u8(r)?; Op::Quantize { dtype: DType::from_tag(tag).unwrap_or(DType::F32) } }
        TAG_DEQUANTIZE       => Op::Dequantize,
        TAG_SAMPLE => {
            let method_tag = read_u8(r)?;
            let temp  = read_f32(r)?;
            let p     = read_f32(r)?;
            let k     = read_u32(r)?;
            let method = match method_tag {
                1 => SampleMethod::Temperature { temp },
                2 => SampleMethod::TopK { k, temp },
                3 => SampleMethod::TopP { p, temp },
                4 => SampleMethod::MinP { p, temp },
                _ => SampleMethod::Greedy,
            };
            Op::Sample { method }
        }
        TAG_LORA_APPLY      => { let rank = read_u32(r)?; let alpha = read_f32(r)?; Op::LoraApply { rank, alpha } }
        TAG_KRON             => Op::Kron,
        TAG_MATRIX_INVERSE   => Op::MatrixInverse,
        TAG_FUSED_NORM_MATMUL => { let eps = read_f32(r)?; Op::FusedNormMatmul { eps } }
        TAG_FUSED_SKIP_NORM  => { let eps = read_f32(r)?; Op::FusedSkipNorm { eps } }
        TAG_FUSED_SWIGLU     => Op::FusedSwiGlu,
        TAG_FLASH_ATTENTION  => {
            let num_heads = read_u32(r)?; let kv_heads = read_u32(r)?; let head_dim = read_u32(r)?;
            Op::FlashAttention { num_heads, kv_heads, head_dim }
        }
        TAG_ARGMAX           => { let dim = read_i32(r)?; Op::Argmax { dim } }
        other => return Err(SerialError::UnknownOpTag(other)),
    })
}

// ── node serialization ────────────────────────────────────────────────────────

fn serialize_node(w: &mut Vec<u8>, node: &Node) {
    write_u32(w, node.id as u32);
    serialize_op(w, &node.op);
    write_strings(w, &node.inputs);
    write_strings(w, &node.outputs);
    // backend_hint — 0 = none
    let hint: u8 = match node.backend_hint {
        None => 0,
        Some(super::graph::BackendHint::Cpu) => 1,
        Some(super::graph::BackendHint::WgpuRs) => 2,
        Some(super::graph::BackendHint::Honeycrisp) => 3,
        Some(super::graph::BackendHint::Nox) => 4,
    };
    write_u8(w, hint);
}

fn deserialize_node(r: &mut Cursor<&[u8]>) -> Result<Node, SerialError> {
    use super::graph::BackendHint;
    use std::collections::HashMap;
    let id = read_u32(r)? as usize;
    let op = deserialize_op(r)?;
    let inputs = read_strings(r)?;
    let outputs = read_strings(r)?;
    let hint_byte = read_u8(r)?;
    let backend_hint = match hint_byte {
        1 => Some(BackendHint::Cpu),
        2 => Some(BackendHint::WgpuRs),
        3 => Some(BackendHint::Honeycrisp),
        4 => Some(BackendHint::Nox),
        _ => None,
    };
    Ok(Node { id, op, inputs, outputs, attrs: HashMap::new(), backend_hint })
}

// ── public API ────────────────────────────────────────────────────────────────

/// Serialize a `Graph` to a compact little-endian binary blob.
pub fn serialize(graph: &Graph) -> Vec<u8> {
    let mut w = Vec::new();
    write_u32(&mut w, graph.nodes.len() as u32);
    for node in &graph.nodes {
        serialize_node(&mut w, node);
    }
    w
}

/// Deserialize a `Graph` from bytes produced by [`serialize`].
pub fn deserialize(bytes: &[u8]) -> Result<Graph, SerialError> {
    let mut r = Cursor::new(bytes);
    let num_nodes = read_u32(&mut r)? as usize;
    let mut graph = Graph::new();
    for _ in 0..num_nodes {
        let node = deserialize_node(&mut r)?;
        graph.nodes.push(node);
    }
    Ok(graph)
}

/// Encode binary bytes as lowercase hex string.
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decode lowercase (or uppercase) hex string to bytes.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, SerialError> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(SerialError::Truncated("hex string odd length"));
    }
    (0..s.len() / 2)
        .map(|i| {
            u8::from_str_radix(&s[2 * i..2 * i + 2], 16)
                .map_err(|_| SerialError::Io(io::Error::new(io::ErrorKind::InvalidData, "bad hex")))
        })
        .collect()
}
