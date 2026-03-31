//! GGML loader — whisper.cpp model format
//!
//! Format:
//! [4 bytes: magic 0x67676d6c]
//! [model hyperparams (11 x u32)]
//! [mel filters: n_mel x n_fft x f32]
//! [tokenizer vocab: n_vocab entries]
//! [tensor data: n_dims + name_len + type + dims[] + name + aligned data]

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use crate::ir::{DType, Graph, TensorMeta, WeightData};

const GGML_MAGIC: u32 = 0x67676d6c;

/// Whisper model hyperparameters
#[derive(Debug, Clone)]
pub struct WhisperHparams {
    pub n_vocab: u32,
    pub n_audio_ctx: u32,
    pub n_audio_state: u32,
    pub n_audio_head: u32,
    pub n_audio_layer: u32,
    pub n_text_ctx: u32,
    pub n_text_state: u32,
    pub n_text_head: u32,
    pub n_text_layer: u32,
    pub n_mels: u32,
    pub ftype: u32,
}

/// Mel filterbank extracted from the GGML file
#[derive(Debug, Clone)]
pub struct MelFilters {
    /// Number of mel bins (typically 80)
    pub n_mel: usize,
    /// Number of FFT bins (typically 201 = n_fft/2 + 1)
    pub n_fft: usize,
    /// Filter coefficients: [n_mel * n_fft] row-major
    pub data: Vec<f32>,
}

/// Full result of loading a GGML whisper model
pub struct GgmlWhisperData {
    pub graph: Graph,
    pub hparams: WhisperHparams,
    pub mel_filters: MelFilters,
    pub vocab: Vec<String>,
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, String> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_u32: {e}"))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f32(cursor: &mut Cursor<&[u8]>) -> Result<f32, String> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).map_err(|e| format!("read_f32: {e}"))?;
    Ok(f32::from_le_bytes(buf))
}

fn ggml_type_to_dtype(ttype: u32) -> DType {
    match ttype {
        0 => DType::F32,
        1 => DType::F16,
        2 => DType::Q4,     // GGML_TYPE_Q4_0
        3 => DType::Q4_1,   // GGML_TYPE_Q4_1
        8 => DType::Q8,     // GGML_TYPE_Q8_0
        _ => {
            log::warn!("Unknown GGML tensor type {ttype}, defaulting to F32");
            DType::F32
        }
    }
}

/// Byte size per block for each GGML type
#[allow(dead_code)]
fn ggml_type_element_size(ttype: u32) -> usize {
    match ttype {
        0 => 4,   // f32
        1 => 2,   // f16
        2 => 18,  // q4_0: 32 elements = 2 (scale) + 16 (data)
        3 => 20,  // q4_1: 32 elements = 2+2 (scale+min) + 16 (data)
        8 => 34,  // q8_0: 32 elements = 2 (scale) + 32 (data)
        _ => 4,
    }
}

/// Number of elements per quantization block
#[allow(dead_code)]
fn ggml_type_block_size(ttype: u32) -> usize {
    match ttype {
        0 | 1 => 1,    // f32, f16: 1 element per "block"
        2 | 3 | 8 => 32, // quantized: 32 elements per block
        _ => 1,
    }
}

/// Load a GGML whisper model, returning only the Graph (backward compat)
pub fn load_ggml(path: &Path) -> Result<Graph, String> {
    let data = load_ggml_full(path)?;
    Ok(data.graph)
}

/// Load a GGML whisper model with all data: graph, hparams, mel filters, vocab
pub fn load_ggml_full(path: &Path) -> Result<GgmlWhisperData, String> {
    let file_data = std::fs::read(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;

    let mut cursor = Cursor::new(file_data.as_slice());

    // Magic
    let magic = read_u32(&mut cursor)?;
    if magic != GGML_MAGIC {
        return Err(format!("Not a GGML file: magic=0x{magic:08x}"));
    }

    // Hyperparams
    let hparams = WhisperHparams {
        n_vocab: read_u32(&mut cursor)?,
        n_audio_ctx: read_u32(&mut cursor)?,
        n_audio_state: read_u32(&mut cursor)?,
        n_audio_head: read_u32(&mut cursor)?,
        n_audio_layer: read_u32(&mut cursor)?,
        n_text_ctx: read_u32(&mut cursor)?,
        n_text_state: read_u32(&mut cursor)?,
        n_text_head: read_u32(&mut cursor)?,
        n_text_layer: read_u32(&mut cursor)?,
        n_mels: read_u32(&mut cursor)?,
        ftype: read_u32(&mut cursor)?,
    };

    log::info!("GGML whisper: vocab={}, audio_state={}, audio_heads={}, audio_layers={}, text_state={}, text_heads={}, text_layers={}, mels={}, ftype={}",
        hparams.n_vocab, hparams.n_audio_state, hparams.n_audio_head, hparams.n_audio_layer,
        hparams.n_text_state, hparams.n_text_head, hparams.n_text_layer, hparams.n_mels, hparams.ftype);

    // Mel filters: n_mel x n_fft
    let filters_n_mel = read_u32(&mut cursor)? as usize;
    let filters_n_fft = read_u32(&mut cursor)? as usize;
    let n_filter_values = filters_n_mel * filters_n_fft;
    let mut mel_data = Vec::with_capacity(n_filter_values);
    for _ in 0..n_filter_values {
        mel_data.push(read_f32(&mut cursor)?);
    }
    log::info!("Loaded mel filters: {filters_n_mel}x{filters_n_fft}");

    let mel_filters = MelFilters {
        n_mel: filters_n_mel,
        n_fft: filters_n_fft,
        data: mel_data,
    };

    // Tokenizer vocab
    let n_vocab_tokenizer = read_u32(&mut cursor)? as usize;
    let mut vocab = Vec::with_capacity(n_vocab_tokenizer);
    for _ in 0..n_vocab_tokenizer {
        let word_len = read_u32(&mut cursor)? as usize;
        if word_len > 1024 * 1024 {
            return Err(format!("Vocab word too long: {word_len}"));
        }
        let mut word_buf = vec![0u8; word_len];
        cursor.read_exact(&mut word_buf)
            .map_err(|e| format!("Read vocab word: {e}"))?;
        vocab.push(String::from_utf8_lossy(&word_buf).to_string());
    }
    log::info!("Loaded {n_vocab_tokenizer} tokenizer vocab entries (model vocab={})", hparams.n_vocab);

    // Tensors
    let mut graph = Graph::new();
    let mut tensor_count = 0;

    while (cursor.position() as usize) < file_data.len() - 12 {
        let tensor_start = cursor.position();
        // Read tensor header
        let n_dims = match read_u32(&mut cursor) {
            Ok(v) if v <= 4 => v as usize,
            Ok(v) => { log::debug!("Bad n_dims={v} at offset {tensor_start}"); break; }
            _ => break,
        };
        let name_len = match read_u32(&mut cursor) {
            Ok(v) if v < 256 => v as usize,
            Ok(v) => { log::debug!("Bad name_len={v} at offset {}", cursor.position() - 4); break; }
            _ => break,
        };
        let ttype = read_u32(&mut cursor)?;

        let mut dims = Vec::with_capacity(n_dims);
        for _ in 0..n_dims {
            dims.push(read_u32(&mut cursor)? as usize);
        }

        let mut name_buf = vec![0u8; name_len];
        cursor.read_exact(&mut name_buf)
            .map_err(|e| format!("Read tensor name: {e}"))?;
        let name = String::from_utf8_lossy(&name_buf).to_string();

        // GGML format: data starts immediately after name (no alignment)
        // (Unlike GGUF which aligns to 32 bytes)

        // Calculate data size
        let num_elements: usize = if dims.is_empty() { 1 } else { dims.iter().product() };
        let byte_size = match ttype {
            0 => num_elements * 4,          // f32
            1 => num_elements * 2,          // f16
            2 => (num_elements / 32) * 18,  // q4_0: 32 elements per 18 bytes
            3 => (num_elements / 32) * 20,  // q4_1: 32 elements per 20 bytes
            8 => (num_elements / 32) * 34,  // q8_0: 32 elements per 34 bytes
            _ => num_elements * 4,          // unknown: assume f32
        };

        let data_start = cursor.position() as usize;
        if data_start + byte_size > file_data.len() {
            log::warn!("Tensor {name} data out of bounds, stopping");
            break;
        }

        let raw_data = file_data[data_start..data_start + byte_size].to_vec();
        cursor.seek(SeekFrom::Current(byte_size as i64))
            .map_err(|e| format!("Skip tensor data: {e}"))?;

        let dtype = ggml_type_to_dtype(ttype);

        graph.add_tensor(
            name.clone(),
            TensorMeta::weight(dims.clone(), dtype),
        );
        graph.add_weight(name, WeightData { data: raw_data, shape: dims, dtype, needs_transpose: false });
        tensor_count += 1;
    }

    log::info!("GGML loaded: {tensor_count} tensors from {}", path.display());

    Ok(GgmlWhisperData {
        graph,
        hparams,
        mel_filters,
        vocab,
    })
}
