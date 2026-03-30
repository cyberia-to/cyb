//! Weight storage, checkpoint load/save, and weight transposition

use crate::config::ModelConfig;
use std::io::{Read, Seek, SeekFrom};

/// Per-layer weight set
pub struct LayerWeights {
    pub wq: Vec<f32>,
    pub wk: Vec<f32>,
    pub wv: Vec<f32>,
    pub wo: Vec<f32>,
    pub w1: Vec<f32>,
    pub w2: Vec<f32>,
    pub w3: Vec<f32>,
    pub rms_att: Vec<f32>,
    pub rms_ffn: Vec<f32>,
    pub q_norm: Vec<f32>,
    pub k_norm: Vec<f32>,
}

impl LayerWeights {
    pub fn alloc(cfg: &ModelConfig) -> Self {
        LayerWeights {
            wq: vec![0.0; cfg.wq_size()],
            wk: vec![0.0; cfg.wk_size()],
            wv: vec![0.0; cfg.wv_size()],
            wo: vec![0.0; cfg.wo_size()],
            w1: vec![0.0; cfg.w1_size()],
            w2: vec![0.0; cfg.w2_size()],
            w3: vec![0.0; cfg.w3_size()],
            rms_att: vec![0.0; cfg.dim],
            rms_ffn: vec![0.0; cfg.dim],
            q_norm: vec![1.0; cfg.hd],
            k_norm: vec![1.0; cfg.hd],
        }
    }
}

/// KV cache for inference decode
pub struct KVCache {
    pub k_cache: Vec<f32>, // [kv_dim, seq]
    pub v_cache: Vec<f32>, // [kv_dim, seq]
}

impl KVCache {
    pub fn alloc(cfg: &ModelConfig) -> Self {
        KVCache {
            k_cache: vec![0.0; cfg.kv_dim * cfg.seq],
            v_cache: vec![0.0; cfg.kv_dim * cfg.seq],
        }
    }
}

/// Checkpoint header (88 bytes, matches C struct CkptHdr)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CkptHeader {
    pub magic: u32,
    pub version: u32,
    pub step: i32,
    pub total_steps: i32,
    pub n_layers: i32,
    pub vocab_size: i32,
    pub dim: i32,
    pub hidden_dim: i32,
    pub n_heads: i32,
    pub seq_len: i32,
    pub lr: f32,
    pub loss: f32,
    pub cum_compile: f64,
    pub cum_train: f64,
    pub cum_wall: f64,
    pub cum_steps: i32,
    pub cum_batches: i32,
    pub adam_t: i32,
    pub kv_heads: i32,
    pub head_dim: i32,
    pub q_dim: i32,
}

const CKPT_MAGIC: u32 = 0x424C5A54;

/// Load checkpoint weights (skip Adam state). Returns (layers, rms_final, embed, header).
pub fn load_checkpoint(
    path: &str,
    cfg: &ModelConfig,
) -> Result<(Vec<LayerWeights>, Vec<f32>, Vec<f32>, CkptHeader), String> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("Cannot open {}: {}", path, e))?;

    // Read header
    let mut hdr_bytes = [0u8; std::mem::size_of::<CkptHeader>()];
    f.read_exact(&mut hdr_bytes)
        .map_err(|e| format!("Read header: {}", e))?;
    let hdr: CkptHeader = unsafe { std::ptr::read(hdr_bytes.as_ptr() as *const CkptHeader) };

    if hdr.magic != CKPT_MAGIC {
        return Err(format!("Bad magic: {:#x}", hdr.magic));
    }
    if hdr.version != 4 && hdr.version != 5 {
        return Err(format!("Bad version: {}", hdr.version));
    }
    if hdr.n_layers as usize != cfg.n_layers || hdr.dim as usize != cfg.dim {
        return Err("Model config mismatch".into());
    }

    let v5 = hdr.version >= 5;
    let mut adam_skip: usize = 2
        * (cfg.wq_size()
            + cfg.wk_size()
            + cfg.wv_size()
            + cfg.wo_size()
            + cfg.w1_size()
            + cfg.w2_size()
            + cfg.w3_size()
            + cfg.dim
            + cfg.dim);
    if v5 {
        adam_skip += 2 * (cfg.hd + cfg.hd);
    }
    adam_skip *= 4; // f32 bytes

    let mut layers = Vec::with_capacity(cfg.n_layers);
    for _ in 0..cfg.n_layers {
        let mut lw = LayerWeights::alloc(cfg);
        read_f32(&mut f, &mut lw.wq)?;
        read_f32(&mut f, &mut lw.wk)?;
        read_f32(&mut f, &mut lw.wv)?;
        read_f32(&mut f, &mut lw.wo)?;
        read_f32(&mut f, &mut lw.w1)?;
        read_f32(&mut f, &mut lw.w2)?;
        read_f32(&mut f, &mut lw.w3)?;
        read_f32(&mut f, &mut lw.rms_att)?;
        read_f32(&mut f, &mut lw.rms_ffn)?;
        if v5 {
            read_f32(&mut f, &mut lw.q_norm)?;
            read_f32(&mut f, &mut lw.k_norm)?;
        }
        f.seek(SeekFrom::Current(adam_skip as i64))
            .map_err(|e| e.to_string())?;
        layers.push(lw);
    }

    let mut rms_final = vec![0.0f32; cfg.dim];
    read_f32(&mut f, &mut rms_final)?;
    // Skip rms_final Adam state
    f.seek(SeekFrom::Current((2 * cfg.dim * 4) as i64))
        .map_err(|e| e.to_string())?;

    let mut embed = vec![0.0f32; cfg.vocab * cfg.dim];
    read_f32(&mut f, &mut embed)?;

    Ok((layers, rms_final, embed, hdr))
}

fn read_f32(f: &mut std::fs::File, buf: &mut [f32]) -> Result<(), String> {
    let bytes =
        unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, buf.len() * 4) };
    f.read_exact(bytes).map_err(|e| format!("Read: {}", e))
}

/// Transpose weight matrix: dst[c*rows+r] = src[r*cols+c]
pub fn transpose(dst: &mut [f32], src: &[f32], rows: usize, cols: usize) {
    for r in 0..rows {
        for c in 0..cols {
            dst[c * rows + r] = src[r * cols + c];
        }
    }
}
