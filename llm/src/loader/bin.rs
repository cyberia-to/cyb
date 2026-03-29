//! Binary format loader — simple raw weight files
//!
//! Supports two sub-formats:
//!
//! 1. **Raw tensor dump**: headerless file of f32 values, shape inferred from file size
//!    (used for single-tensor weight files, e.g. fasttext vectors)
//!
//! 2. **Fasttext .bin format**:
//!    [4 bytes: magic u32 = 0x46543200 or dimension check]
//!    [4 bytes: vocab_size (i32)]
//!    [4 bytes: hidden_size (i32)]
//!    [vocab_size * hidden_size * 4 bytes: f32 weight data row-major]
//!
//! Returns a Graph IR with weights.

use std::path::Path;

use crate::ir::{DType, Graph, TensorMeta, WeightData};

/// Fasttext magic bytes (optional — some versions omit it)
const FASTTEXT_MAGIC: u32 = 0x46543200; // "FT2\0" in LE

/// Load a .bin file into Graph IR
///
/// Attempts fasttext-style header detection first (vocab_size + hidden_size prefix).
/// Falls back to treating the entire file as raw f32 embedding data.
pub fn load_bin(path: &Path) -> Result<Graph, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mmap = unsafe {
        memmap2::Mmap::map(&file)
            .map_err(|e| format!("Cannot mmap {}: {e}", path.display()))?
    };

    if mmap.len() < 12 {
        return Err("File too small for .bin format".to_string());
    }

    // Try reading first 4 bytes as potential magic or vocab_size
    let first_u32 = read_u32_le(&mmap[0..4]);

    let (vocab_size, hidden_size, data_offset) = if first_u32 == FASTTEXT_MAGIC {
        // Fasttext v2 format with magic
        // [4 magic] [4 vocab] [4 hidden] [data...]
        if mmap.len() < 16 {
            return Err("File too small for fasttext v2 .bin".to_string());
        }
        let vocab = read_i32_le(&mmap[4..8]) as usize;
        let hidden = read_i32_le(&mmap[8..12]) as usize;
        log::info!("Fasttext v2 .bin: vocab={vocab}, hidden={hidden}");
        (vocab, hidden, 12)
    } else {
        // Try fasttext v1: [4 vocab_size] [4 hidden_size] [data...]
        let maybe_vocab = read_i32_le(&mmap[0..4]) as usize;
        let maybe_hidden = read_i32_le(&mmap[4..8]) as usize;
        let expected_data = maybe_vocab * maybe_hidden * 4;
        let remaining = mmap.len() - 8;

        if maybe_vocab > 0
            && maybe_vocab < 10_000_000
            && maybe_hidden > 0
            && maybe_hidden < 100_000
            && expected_data > 0
            && remaining >= expected_data
        {
            log::info!("Fasttext v1 .bin: vocab={maybe_vocab}, hidden={maybe_hidden}");
            (maybe_vocab, maybe_hidden, 8)
        } else {
            // Raw f32 dump — guess shape as [N, 1] vector
            if mmap.len() % 4 != 0 {
                return Err("Raw .bin file size not multiple of 4 (not f32 aligned)".to_string());
            }
            let num_elements = mmap.len() / 4;
            log::info!("Raw f32 .bin: {num_elements} elements");

            let mut graph = Graph::new();
            let shape = vec![num_elements];
            let data = mmap.to_vec();
            graph.add_tensor(
                "weight".to_string(),
                TensorMeta::weight(shape.clone(), DType::F32),
            );
            graph.add_weight(
                "weight".to_string(),
                WeightData {
                    data,
                    shape,
                    dtype: DType::F32,
                },
            );
            return Ok(graph);
        }
    };

    // Read embedding matrix [vocab_size, hidden_size] as f32
    let data_bytes = vocab_size * hidden_size * 4;
    let data_end = data_offset + data_bytes;
    if data_end > mmap.len() {
        return Err(format!(
            "Embedding data out of bounds: need {} bytes at offset {}, file is {} bytes",
            data_bytes, data_offset, mmap.len(),
        ));
    }

    let raw_data = mmap[data_offset..data_end].to_vec();
    let shape = vec![vocab_size, hidden_size];

    let mut graph = Graph::new();
    graph.add_tensor(
        "embeddings.weight".to_string(),
        TensorMeta::weight(shape.clone(), DType::F32),
    );
    graph.add_weight(
        "embeddings.weight".to_string(),
        WeightData {
            data: raw_data,
            shape,
            dtype: DType::F32,
        },
    );

    log::info!(
        "Bin loaded: [{vocab_size}, {hidden_size}] embeddings from {}",
        path.display(),
    );

    Ok(graph)
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_i32_le(bytes: &[u8]) -> i32 {
    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
