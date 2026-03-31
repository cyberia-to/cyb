//! `.cyb` — universal model format for cyb-llm runtime.
//!
//! One file = config + graph IR + quantized weights.
//! Replaces GGUF, safetensors, ONNX with a single format
//! designed for the cyb-llm Graph IR.
//!
//! ## File layout
//!
//! ```text
//! [header]         magic + version + flags + section offsets
//! [config]         TOML string (model_type, architecture, etc.)
//! [graph]          serialized IR nodes (optional)
//! [tensor index]   per-tensor metadata (name, shape, dtype, offset)
//! [tensor data]    raw weight bytes, aligned for mmap
//! ```

use crate::ir::{
    Attrs, AttrValue, BackendHint, DType, Dim, Graph, Node, Op, Residency,
    Shape, TensorMeta, WeightData,
};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::Path;

// ── Constants ────────────────────────────────────────────────────

const CYB_MAGIC: [u8; 4] = *b"CYB\x01";
const CYB_VERSION: u32 = 1;
const ALIGNMENT: usize = 64; // tensor data alignment for mmap/GPU

// Flags
const FLAG_HAS_GRAPH: u32 = 1 << 0;
const FLAG_HAS_WEIGHTS: u32 = 1 << 1;
const FLAG_HAS_CONFIG: u32 = 1 << 2;

// Section IDs
const SECTION_CONFIG: u32 = 1;
const SECTION_GRAPH: u32 = 2;
const SECTION_TENSOR_INDEX: u32 = 3;
const SECTION_TENSOR_DATA: u32 = 4;

// ── Writer ───────────────────────────────────────────────────────

/// Write a Graph + config to a .cyb file
pub fn write_cyb(
    path: &Path,
    graph: &Graph,
    config_toml: &str,
    include_graph: bool,
) -> io::Result<()> {
    let mut f = io::BufWriter::new(std::fs::File::create(path)?);

    let has_weights = !graph.weights.is_empty();
    let has_graph = include_graph && !graph.nodes.is_empty();
    let mut flags = FLAG_HAS_CONFIG;
    if has_graph {
        flags |= FLAG_HAS_GRAPH;
    }
    if has_weights {
        flags |= FLAG_HAS_WEIGHTS;
    }

    // Count sections
    let mut section_count = 1u32; // config always present
    if has_graph {
        section_count += 1;
    }
    if has_weights {
        section_count += 2; // index + data
    }

    // ── Serialize sections to buffers first ──
    let config_bytes = config_toml.as_bytes();
    let graph_bytes = if has_graph {
        serialize_graph_nodes(&graph.nodes)?
    } else {
        Vec::new()
    };
    let (index_bytes, data_bytes) = if has_weights {
        serialize_weights(&graph.weights)?
    } else {
        (Vec::new(), Vec::new())
    };

    // ── Compute offsets ──
    // Header: magic(4) + version(4) + flags(4) + section_count(4) = 16
    // Section table: section_count * (id:4 + offset:8 + size:8) = section_count * 20
    let header_size = 16 + (section_count as usize) * 20;
    let header_padded = align(header_size);

    let mut offset = header_padded;
    let config_offset = offset;
    offset += align(config_bytes.len());

    let graph_offset = offset;
    if has_graph {
        offset += align(graph_bytes.len());
    }

    let index_offset = offset;
    if has_weights {
        offset += align(index_bytes.len());
    }

    let data_offset = offset;
    // data_bytes doesn't need trailing alignment

    // ── Write header ──
    f.write_all(&CYB_MAGIC)?;
    f.write_all(&CYB_VERSION.to_le_bytes())?;
    f.write_all(&flags.to_le_bytes())?;
    f.write_all(&section_count.to_le_bytes())?;

    // ── Write section table ──
    write_section_entry(&mut f, SECTION_CONFIG, config_offset as u64, config_bytes.len() as u64)?;
    if has_graph {
        write_section_entry(&mut f, SECTION_GRAPH, graph_offset as u64, graph_bytes.len() as u64)?;
    }
    if has_weights {
        write_section_entry(&mut f, SECTION_TENSOR_INDEX, index_offset as u64, index_bytes.len() as u64)?;
        write_section_entry(&mut f, SECTION_TENSOR_DATA, data_offset as u64, data_bytes.len() as u64)?;
    }

    // ── Pad header ──
    pad_to(&mut f, header_size, header_padded)?;

    // ── Write sections ──
    f.write_all(config_bytes)?;
    pad_to(&mut f, config_bytes.len(), align(config_bytes.len()))?;

    if has_graph {
        f.write_all(&graph_bytes)?;
        pad_to(&mut f, graph_bytes.len(), align(graph_bytes.len()))?;
    }

    if has_weights {
        f.write_all(&index_bytes)?;
        pad_to(&mut f, index_bytes.len(), align(index_bytes.len()))?;

        f.write_all(&data_bytes)?;
    }

    f.flush()?;
    Ok(())
}

fn write_section_entry(w: &mut impl Write, id: u32, offset: u64, size: u64) -> io::Result<()> {
    w.write_all(&id.to_le_bytes())?;
    w.write_all(&offset.to_le_bytes())?;
    w.write_all(&size.to_le_bytes())?;
    Ok(())
}

fn align(n: usize) -> usize {
    (n + ALIGNMENT - 1) / ALIGNMENT * ALIGNMENT
}

fn pad_to(w: &mut impl Write, current: usize, target: usize) -> io::Result<()> {
    if target > current {
        w.write_all(&vec![0u8; target - current])?;
    }
    Ok(())
}

// ── Graph serialization ──────────────────────────────────────────

fn serialize_graph_nodes(nodes: &[Node]) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    write_u64(&mut buf, nodes.len() as u64)?;

    for node in nodes {
        write_u32(&mut buf, node.id as u32)?;
        serialize_op(&mut buf, &node.op)?;

        // inputs
        write_u32(&mut buf, node.inputs.len() as u32)?;
        for inp in &node.inputs {
            write_string(&mut buf, inp)?;
        }

        // outputs
        write_u32(&mut buf, node.outputs.len() as u32)?;
        for out in &node.outputs {
            write_string(&mut buf, out)?;
        }

        // attrs
        serialize_attrs(&mut buf, &node.attrs)?;

        // backend hint
        match &node.backend_hint {
            None => buf.push(0),
            Some(BackendHint::Metal) => buf.push(1),
            Some(BackendHint::Ane) => buf.push(2),
            Some(BackendHint::Cuda) => buf.push(3),
            Some(BackendHint::Wgpu) => buf.push(4),
            Some(BackendHint::Cpu) => buf.push(5),
        }
    }
    Ok(buf)
}

fn serialize_op(buf: &mut Vec<u8>, op: &Op) -> io::Result<()> {
    match op {
        // Linear algebra (0-13)
        Op::Matmul => buf.push(0),
        Op::Add => buf.push(1),
        Op::Mul => buf.push(2),
        Op::Sub => buf.push(3),
        Op::Div => buf.push(4),
        Op::Transpose { perm } => {
            buf.push(5);
            write_u32(buf, perm.len() as u32)?;
            for &p in perm { write_u32(buf, p as u32)?; }
        }
        Op::Reshape { shape } => {
            buf.push(6);
            write_u32(buf, shape.len() as u32)?;
            for &s in shape { write_i64(buf, s)?; }
        }
        Op::Permute { dims } => {
            buf.push(7);
            write_u32(buf, dims.len() as u32)?;
            for &d in dims { write_u32(buf, d as u32)?; }
        }
        Op::Concat { axis } => { buf.push(8); write_u32(buf, *axis as u32)?; }
        Op::Split { axis, sizes } => {
            buf.push(9);
            write_u32(buf, *axis as u32)?;
            write_u32(buf, sizes.len() as u32)?;
            for &s in sizes { write_u32(buf, s as u32)?; }
        }
        Op::Chunk { axis, chunks } => { buf.push(10); write_u32(buf, *axis as u32)?; write_u32(buf, *chunks as u32)?; }
        Op::Clamp { min, max } => {
            buf.push(11);
            write_opt_f32(buf, *min)?;
            write_opt_f32(buf, *max)?;
        }
        Op::NanToNum { nan, posinf, neginf } => {
            buf.push(12);
            write_f32(buf, *nan)?; write_f32(buf, *posinf)?; write_f32(buf, *neginf)?;
        }
        Op::TokenEmbed => buf.push(13),
        Op::PosEmbed => buf.push(14),
        Op::Argmax => buf.push(15),

        // Attention (16-22)
        Op::Sdpa { num_heads, kv_heads, head_dim, causal } => {
            buf.push(16);
            write_u32(buf, *num_heads)?; write_u32(buf, *kv_heads)?;
            write_u32(buf, *head_dim)?; buf.push(*causal as u8);
        }
        Op::SdpaCross { num_heads, head_dim } => {
            buf.push(17); write_u32(buf, *num_heads)?; write_u32(buf, *head_dim)?;
        }
        Op::SdpaWindow { num_heads, head_dim, window_size } => {
            buf.push(18); write_u32(buf, *num_heads)?; write_u32(buf, *head_dim)?; write_u32(buf, *window_size)?;
        }
        Op::KvCache => buf.push(19),
        Op::Rope { head_dim, base } => { buf.push(20); write_u32(buf, *head_dim)?; write_f32(buf, *base)?; }
        Op::SinusoidalEmbed { dim } => { buf.push(21); write_u32(buf, *dim)?; }
        Op::RelativePosEmbedding { num_buckets } => { buf.push(22); write_u32(buf, *num_buckets)?; }

        // Normalization (23-28)
        Op::RmsNorm { eps } => { buf.push(23); write_f32(buf, *eps)?; }
        Op::LayerNorm { eps } => { buf.push(24); write_f32(buf, *eps)?; }
        Op::BatchNorm { eps, momentum } => { buf.push(25); write_f32(buf, *eps)?; write_f32(buf, *momentum)?; }
        Op::GroupNorm { num_groups, eps } => { buf.push(26); write_u32(buf, *num_groups)?; write_f32(buf, *eps)?; }
        Op::InstanceNorm { eps } => { buf.push(27); write_f32(buf, *eps)?; }
        Op::AdaLN => buf.push(28),

        // Activation (29-39)
        Op::Silu => buf.push(29),
        Op::Gelu { approximate } => { buf.push(30); buf.push(*approximate as u8); }
        Op::GeGlu => buf.push(31),
        Op::SwiGlu => buf.push(32),
        Op::Glu => buf.push(33),
        Op::Relu => buf.push(34),
        Op::LeakyRelu { slope } => { buf.push(35); write_f32(buf, *slope)?; }
        Op::PRelu => buf.push(36),
        Op::Sigmoid => buf.push(37),
        Op::Tanh => buf.push(38),
        Op::Softmax { dim } => { buf.push(39); write_i32(buf, *dim)?; }

        // Convolution (40-46)
        Op::Conv1d { kernel, stride, padding, groups } => {
            buf.push(40);
            write_u32(buf, *kernel)?; write_u32(buf, *stride)?;
            write_u32(buf, *padding)?; write_u32(buf, *groups)?;
        }
        Op::Conv2d { kernel, stride, padding, groups } => {
            buf.push(41);
            write_u32(buf, kernel.0)?; write_u32(buf, kernel.1)?;
            write_u32(buf, stride.0)?; write_u32(buf, stride.1)?;
            write_u32(buf, padding.0)?; write_u32(buf, padding.1)?;
            write_u32(buf, *groups)?;
        }
        Op::Conv3d { kernel, stride, padding, groups } => {
            buf.push(42);
            write_u32(buf, kernel.0)?; write_u32(buf, kernel.1)?; write_u32(buf, kernel.2)?;
            write_u32(buf, stride.0)?; write_u32(buf, stride.1)?; write_u32(buf, stride.2)?;
            write_u32(buf, padding.0)?; write_u32(buf, padding.1)?; write_u32(buf, padding.2)?;
            write_u32(buf, *groups)?;
        }
        Op::ConvTranspose2d { kernel, stride, padding } => {
            buf.push(43);
            write_u32(buf, kernel.0)?; write_u32(buf, kernel.1)?;
            write_u32(buf, stride.0)?; write_u32(buf, stride.1)?;
            write_u32(buf, padding.0)?; write_u32(buf, padding.1)?;
        }
        Op::CausalConv1d { kernel } => { buf.push(44); write_u32(buf, *kernel)?; }
        Op::DepthwiseConv { kernel, stride } => { buf.push(45); write_u32(buf, *kernel)?; write_u32(buf, *stride)?; }
        Op::Pool { mode, kernel } => {
            buf.push(46);
            buf.push(match mode {
                crate::ir::PoolMode::Max => 0,
                crate::ir::PoolMode::Avg => 1,
            });
            write_u32(buf, kernel.0)?; write_u32(buf, kernel.1)?;
        }

        // Spatial (47-51)
        Op::Interpolate { mode, scale } => {
            buf.push(47);
            buf.push(match mode {
                crate::ir::InterpolateMode::Nearest => 0,
                crate::ir::InterpolateMode::Bilinear => 1,
                crate::ir::InterpolateMode::Area => 2,
            });
            write_f32(buf, *scale)?;
        }
        Op::PixelShuffle { upscale_factor } => { buf.push(48); write_u32(buf, *upscale_factor)?; }
        Op::PixelUnshuffle { downscale_factor } => { buf.push(49); write_u32(buf, *downscale_factor)?; }
        Op::PatchEmbed { patch_size } => { buf.push(50); write_u32(buf, *patch_size)?; }
        Op::Unpatchify => buf.push(51),

        // Special (52-56)
        Op::NoiseSchedule => buf.push(52),
        Op::FlowStep => buf.push(53),
        Op::Quantize { dtype } => { buf.push(54); buf.push(dtype_to_u8(dtype)); }
        Op::Dequantize => buf.push(55),
        Op::Sample { method } => {
            buf.push(56);
            match method {
                crate::ir::SampleMethod::Greedy => buf.push(0),
                crate::ir::SampleMethod::TopP(p) => { buf.push(1); write_f32(buf, *p)?; }
                crate::ir::SampleMethod::TopK(k) => { buf.push(2); write_u32(buf, *k)?; }
                crate::ir::SampleMethod::TopKTopP(k, p) => { buf.push(3); write_u32(buf, *k)?; write_f32(buf, *p)?; }
            }
        }

        // Adapter (57-59)
        Op::LoraApply { rank, alpha } => { buf.push(57); write_u32(buf, *rank)?; write_f32(buf, *alpha)?; }
        Op::Kron => buf.push(58),
        Op::MatrixInverse => buf.push(59),

        // Fused (60-63)
        Op::FusedNormMatmul { eps } => { buf.push(60); write_f32(buf, *eps)?; }
        Op::FusedSkipNorm { eps } => { buf.push(61); write_f32(buf, *eps)?; }
        Op::FusedSwiGlu => buf.push(62),
        Op::FlashAttention { num_heads, kv_heads, head_dim } => {
            buf.push(63); write_u32(buf, *num_heads)?; write_u32(buf, *kv_heads)?; write_u32(buf, *head_dim)?;
        }
    }
    Ok(())
}

fn serialize_attrs(buf: &mut Vec<u8>, attrs: &Attrs) -> io::Result<()> {
    write_u32(buf, attrs.len() as u32)?;
    for (key, val) in attrs {
        write_string(buf, key)?;
        match val {
            AttrValue::Int(v) => { buf.push(0); write_i64(buf, *v)?; }
            AttrValue::Float(v) => { buf.push(1); write_f32(buf, *v)?; }
            AttrValue::String(v) => { buf.push(2); write_string(buf, v)?; }
            AttrValue::Ints(v) => {
                buf.push(3);
                write_u32(buf, v.len() as u32)?;
                for &i in v { write_i64(buf, i)?; }
            }
            AttrValue::Floats(v) => {
                buf.push(4);
                write_u32(buf, v.len() as u32)?;
                for &f in v { write_f32(buf, f)?; }
            }
            AttrValue::Bool(v) => { buf.push(5); buf.push(*v as u8); }
        }
    }
    Ok(())
}

// ── Weight serialization ─────────────────────────────────────────

fn serialize_weights(weights: &HashMap<String, WeightData>) -> io::Result<(Vec<u8>, Vec<u8>)> {
    // Sort by name for reproducibility
    let mut names: Vec<&String> = weights.keys().collect();
    names.sort();

    let mut index = Vec::new();
    let mut data = Vec::new();

    write_u64(&mut index, names.len() as u64)?;

    for name in &names {
        let w = &weights[*name];
        write_string(&mut index, name)?;

        // shape
        write_u32(&mut index, w.shape.len() as u32)?;
        for &dim in &w.shape {
            write_u64(&mut index, dim as u64)?;
        }

        // dtype
        index.push(dtype_to_u8(&w.dtype));

        // offset + size in data section
        let data_offset = data.len() as u64;
        let data_size = w.data.len() as u64;
        write_u64(&mut index, data_offset)?;
        write_u64(&mut index, data_size)?;

        // append weight data, aligned
        data.extend_from_slice(&w.data);
        let remainder = data.len() % ALIGNMENT;
        if remainder != 0 {
            data.extend(std::iter::repeat(0u8).take(ALIGNMENT - remainder));
        }
    }

    Ok((index, data))
}

// ── Reader ───────────────────────────────────────────────────────

/// Read a .cyb file into a Graph + config string
pub fn read_cyb(path: &Path) -> io::Result<(Graph, String)> {
    let file_data = std::fs::read(path)?;
    if file_data.len() < 16 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too small"));
    }

    // ── Header ──
    if file_data[..4] != CYB_MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad magic"));
    }
    let version = read_u32(&file_data, 4);
    if version != CYB_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported version {version}"),
        ));
    }
    let flags = read_u32(&file_data, 8);
    let section_count = read_u32(&file_data, 12) as usize;

    // ── Section table ──
    let mut config_section = (0usize, 0usize);
    let mut graph_section = (0usize, 0usize);
    let mut index_section = (0usize, 0usize);
    let mut data_section = (0usize, 0usize);

    for i in 0..section_count {
        let base = 16 + i * 20;
        let id = read_u32(&file_data, base);
        let offset = read_u64(&file_data, base + 4) as usize;
        let size = read_u64(&file_data, base + 12) as usize;
        match id {
            SECTION_CONFIG => config_section = (offset, size),
            SECTION_GRAPH => graph_section = (offset, size),
            SECTION_TENSOR_INDEX => index_section = (offset, size),
            SECTION_TENSOR_DATA => data_section = (offset, size),
            _ => {} // skip unknown sections
        }
    }

    // ── Config ──
    let config = if config_section.1 > 0 {
        std::str::from_utf8(&file_data[config_section.0..config_section.0 + config_section.1])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .to_string()
    } else {
        String::new()
    };

    // ── Graph nodes ──
    let nodes = if flags & FLAG_HAS_GRAPH != 0 && graph_section.1 > 0 {
        let graph_data = &file_data[graph_section.0..graph_section.0 + graph_section.1];
        deserialize_graph_nodes(graph_data)?
    } else {
        Vec::new()
    };

    // ── Weights ──
    let weights = if flags & FLAG_HAS_WEIGHTS != 0 && index_section.1 > 0 {
        let idx = &file_data[index_section.0..index_section.0 + index_section.1];
        let dat = &file_data[data_section.0..];
        deserialize_weights(idx, dat)?
    } else {
        HashMap::new()
    };

    let graph = Graph {
        nodes,
        tensors: HashMap::new(), // rebuilt by runtime from weights + config
        weights,
    };

    Ok((graph, config))
}

// ── Graph deserialization ────────────────────────────────────────

fn deserialize_graph_nodes(data: &[u8]) -> io::Result<Vec<Node>> {
    let mut pos = 0;
    let count = read_u64_at(data, &mut pos) as usize;
    let mut nodes = Vec::with_capacity(count);

    for _ in 0..count {
        let id = read_u32_at(data, &mut pos) as usize;
        let op = deserialize_op(data, &mut pos)?;

        let n_inputs = read_u32_at(data, &mut pos) as usize;
        let mut inputs = Vec::with_capacity(n_inputs);
        for _ in 0..n_inputs {
            inputs.push(read_string_at(data, &mut pos)?);
        }

        let n_outputs = read_u32_at(data, &mut pos) as usize;
        let mut outputs = Vec::with_capacity(n_outputs);
        for _ in 0..n_outputs {
            outputs.push(read_string_at(data, &mut pos)?);
        }

        let attrs = deserialize_attrs(data, &mut pos)?;

        let hint_byte = data[pos];
        pos += 1;
        let backend_hint = match hint_byte {
            0 => None,
            1 => Some(BackendHint::Metal),
            2 => Some(BackendHint::Ane),
            3 => Some(BackendHint::Cuda),
            4 => Some(BackendHint::Wgpu),
            5 => Some(BackendHint::Cpu),
            _ => None,
        };

        nodes.push(Node {
            id,
            op,
            inputs,
            outputs,
            attrs,
            backend_hint,
        });
    }
    Ok(nodes)
}

fn deserialize_op(data: &[u8], pos: &mut usize) -> io::Result<Op> {
    let tag = data[*pos];
    *pos += 1;

    Ok(match tag {
        0 => Op::Matmul,
        1 => Op::Add,
        2 => Op::Mul,
        3 => Op::Sub,
        4 => Op::Div,
        5 => {
            let n = read_u32_at(data, pos) as usize;
            let perm = (0..n).map(|_| read_u32_at(data, pos) as usize).collect();
            Op::Transpose { perm }
        }
        6 => {
            let n = read_u32_at(data, pos) as usize;
            let shape = (0..n).map(|_| read_i64_at(data, pos)).collect();
            Op::Reshape { shape }
        }
        7 => {
            let n = read_u32_at(data, pos) as usize;
            let dims = (0..n).map(|_| read_u32_at(data, pos) as usize).collect();
            Op::Permute { dims }
        }
        8 => Op::Concat { axis: read_u32_at(data, pos) as usize },
        9 => {
            let axis = read_u32_at(data, pos) as usize;
            let n = read_u32_at(data, pos) as usize;
            let sizes = (0..n).map(|_| read_u32_at(data, pos) as usize).collect();
            Op::Split { axis, sizes }
        }
        10 => Op::Chunk { axis: read_u32_at(data, pos) as usize, chunks: read_u32_at(data, pos) as usize },
        11 => Op::Clamp { min: read_opt_f32_at(data, pos), max: read_opt_f32_at(data, pos) },
        12 => Op::NanToNum { nan: read_f32_at(data, pos), posinf: read_f32_at(data, pos), neginf: read_f32_at(data, pos) },
        13 => Op::TokenEmbed,
        14 => Op::PosEmbed,
        15 => Op::Argmax,

        16 => Op::Sdpa {
            num_heads: read_u32_at(data, pos),
            kv_heads: read_u32_at(data, pos),
            head_dim: read_u32_at(data, pos),
            causal: { let b = data[*pos]; *pos += 1; b != 0 },
        },
        17 => Op::SdpaCross { num_heads: read_u32_at(data, pos), head_dim: read_u32_at(data, pos) },
        18 => Op::SdpaWindow { num_heads: read_u32_at(data, pos), head_dim: read_u32_at(data, pos), window_size: read_u32_at(data, pos) },
        19 => Op::KvCache,
        20 => Op::Rope { head_dim: read_u32_at(data, pos), base: read_f32_at(data, pos) },
        21 => Op::SinusoidalEmbed { dim: read_u32_at(data, pos) },
        22 => Op::RelativePosEmbedding { num_buckets: read_u32_at(data, pos) },

        23 => Op::RmsNorm { eps: read_f32_at(data, pos) },
        24 => Op::LayerNorm { eps: read_f32_at(data, pos) },
        25 => Op::BatchNorm { eps: read_f32_at(data, pos), momentum: read_f32_at(data, pos) },
        26 => Op::GroupNorm { num_groups: read_u32_at(data, pos), eps: read_f32_at(data, pos) },
        27 => Op::InstanceNorm { eps: read_f32_at(data, pos) },
        28 => Op::AdaLN,

        29 => Op::Silu,
        30 => Op::Gelu { approximate: { let b = data[*pos]; *pos += 1; b != 0 } },
        31 => Op::GeGlu,
        32 => Op::SwiGlu,
        33 => Op::Glu,
        34 => Op::Relu,
        35 => Op::LeakyRelu { slope: read_f32_at(data, pos) },
        36 => Op::PRelu,
        37 => Op::Sigmoid,
        38 => Op::Tanh,
        39 => Op::Softmax { dim: read_i32_at(data, pos) },

        40 => Op::Conv1d { kernel: read_u32_at(data, pos), stride: read_u32_at(data, pos), padding: read_u32_at(data, pos), groups: read_u32_at(data, pos) },
        41 => Op::Conv2d {
            kernel: (read_u32_at(data, pos), read_u32_at(data, pos)),
            stride: (read_u32_at(data, pos), read_u32_at(data, pos)),
            padding: (read_u32_at(data, pos), read_u32_at(data, pos)),
            groups: read_u32_at(data, pos),
        },
        42 => Op::Conv3d {
            kernel: (read_u32_at(data, pos), read_u32_at(data, pos), read_u32_at(data, pos)),
            stride: (read_u32_at(data, pos), read_u32_at(data, pos), read_u32_at(data, pos)),
            padding: (read_u32_at(data, pos), read_u32_at(data, pos), read_u32_at(data, pos)),
            groups: read_u32_at(data, pos),
        },
        43 => Op::ConvTranspose2d {
            kernel: (read_u32_at(data, pos), read_u32_at(data, pos)),
            stride: (read_u32_at(data, pos), read_u32_at(data, pos)),
            padding: (read_u32_at(data, pos), read_u32_at(data, pos)),
        },
        44 => Op::CausalConv1d { kernel: read_u32_at(data, pos) },
        45 => Op::DepthwiseConv { kernel: read_u32_at(data, pos), stride: read_u32_at(data, pos) },
        46 => {
            let mode = match data[*pos] { 0 => crate::ir::PoolMode::Max, _ => crate::ir::PoolMode::Avg };
            *pos += 1;
            Op::Pool { mode, kernel: (read_u32_at(data, pos), read_u32_at(data, pos)) }
        }

        47 => {
            let mode = match data[*pos] {
                0 => crate::ir::InterpolateMode::Nearest,
                1 => crate::ir::InterpolateMode::Bilinear,
                _ => crate::ir::InterpolateMode::Area,
            };
            *pos += 1;
            Op::Interpolate { mode, scale: read_f32_at(data, pos) }
        }
        48 => Op::PixelShuffle { upscale_factor: read_u32_at(data, pos) },
        49 => Op::PixelUnshuffle { downscale_factor: read_u32_at(data, pos) },
        50 => Op::PatchEmbed { patch_size: read_u32_at(data, pos) },
        51 => Op::Unpatchify,

        52 => Op::NoiseSchedule,
        53 => Op::FlowStep,
        54 => Op::Quantize { dtype: u8_to_dtype(data[*pos]).unwrap_or(DType::F16) },
        55 => Op::Dequantize,
        56 => {
            let method_tag = data[*pos]; *pos += 1;
            let method = match method_tag {
                0 => crate::ir::SampleMethod::Greedy,
                1 => crate::ir::SampleMethod::TopP(read_f32_at(data, pos)),
                2 => crate::ir::SampleMethod::TopK(read_u32_at(data, pos)),
                3 => crate::ir::SampleMethod::TopKTopP(read_u32_at(data, pos), read_f32_at(data, pos)),
                _ => crate::ir::SampleMethod::Greedy,
            };
            Op::Sample { method }
        }

        57 => Op::LoraApply { rank: read_u32_at(data, pos), alpha: read_f32_at(data, pos) },
        58 => Op::Kron,
        59 => Op::MatrixInverse,

        60 => Op::FusedNormMatmul { eps: read_f32_at(data, pos) },
        61 => Op::FusedSkipNorm { eps: read_f32_at(data, pos) },
        62 => Op::FusedSwiGlu,
        63 => Op::FlashAttention { num_heads: read_u32_at(data, pos), kv_heads: read_u32_at(data, pos), head_dim: read_u32_at(data, pos) },

        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown op tag {tag}"))),
    })
}

fn deserialize_attrs(data: &[u8], pos: &mut usize) -> io::Result<Attrs> {
    let count = read_u32_at(data, pos) as usize;
    let mut attrs = HashMap::with_capacity(count);
    for _ in 0..count {
        let key = read_string_at(data, pos)?;
        let tag = data[*pos]; *pos += 1;
        let val = match tag {
            0 => AttrValue::Int(read_i64_at(data, pos)),
            1 => AttrValue::Float(read_f32_at(data, pos)),
            2 => AttrValue::String(read_string_at(data, pos)?),
            3 => {
                let n = read_u32_at(data, pos) as usize;
                AttrValue::Ints((0..n).map(|_| read_i64_at(data, pos)).collect())
            }
            4 => {
                let n = read_u32_at(data, pos) as usize;
                AttrValue::Floats((0..n).map(|_| read_f32_at(data, pos)).collect())
            }
            5 => AttrValue::Bool({ let b = data[*pos]; *pos += 1; b != 0 }),
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown attr tag {tag}"))),
        };
        attrs.insert(key, val);
    }
    Ok(attrs)
}

// ── Weight deserialization ───────────────────────────────────────

fn deserialize_weights(index: &[u8], data: &[u8]) -> io::Result<HashMap<String, WeightData>> {
    let mut pos = 0;
    let count = read_u64_at(index, &mut pos) as usize;
    let mut weights = HashMap::with_capacity(count);

    for _ in 0..count {
        let name = read_string_at(index, &mut pos)?;
        let n_dims = read_u32_at(index, &mut pos) as usize;
        let shape: Vec<usize> = (0..n_dims).map(|_| read_u64_at(index, &mut pos) as usize).collect();
        let dtype = u8_to_dtype(index[pos]).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, format!("unknown dtype {}", index[pos]))
        })?;
        pos += 1;
        let data_offset = read_u64_at(index, &mut pos) as usize;
        let data_size = read_u64_at(index, &mut pos) as usize;

        let weight_data = data.get(data_offset..data_offset + data_size)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "weight data out of bounds"))?;

        weights.insert(name, WeightData {
            data: weight_data.to_vec(),
            shape,
            dtype,
        });
    }
    Ok(weights)
}

// ── DType mapping ────────────────────────────────────────────────

fn dtype_to_u8(dtype: &DType) -> u8 {
    match dtype {
        DType::F32 => 0,
        DType::F16 => 1,
        DType::BF16 => 2,
        DType::I8 => 3,
        DType::U8 => 4,
        DType::Bool => 5,
        DType::Q8 => 6,
        DType::Q4 => 7,
        DType::Q4_1 => 8,
        DType::Ternary => 9,
        DType::Q2_K => 10,
        DType::Q3_K => 11,
        DType::Q4_K => 12,
        DType::Q5_K => 13,
        DType::Q6_K => 14,
    }
}

fn u8_to_dtype(b: u8) -> Option<DType> {
    Some(match b {
        0 => DType::F32,
        1 => DType::F16,
        2 => DType::BF16,
        3 => DType::I8,
        4 => DType::U8,
        5 => DType::Bool,
        6 => DType::Q8,
        7 => DType::Q4,
        8 => DType::Q4_1,
        9 => DType::Ternary,
        10 => DType::Q2_K,
        11 => DType::Q3_K,
        12 => DType::Q4_K,
        13 => DType::Q5_K,
        14 => DType::Q6_K,
        _ => return None,
    })
}

// ── Primitive read/write ─────────────────────────────────────────

fn write_u32(buf: &mut Vec<u8>, v: u32) -> io::Result<()> { buf.extend_from_slice(&v.to_le_bytes()); Ok(()) }
fn write_u64(buf: &mut Vec<u8>, v: u64) -> io::Result<()> { buf.extend_from_slice(&v.to_le_bytes()); Ok(()) }
fn write_i32(buf: &mut Vec<u8>, v: i32) -> io::Result<()> { buf.extend_from_slice(&v.to_le_bytes()); Ok(()) }
fn write_i64(buf: &mut Vec<u8>, v: i64) -> io::Result<()> { buf.extend_from_slice(&v.to_le_bytes()); Ok(()) }
fn write_f32(buf: &mut Vec<u8>, v: f32) -> io::Result<()> { buf.extend_from_slice(&v.to_le_bytes()); Ok(()) }

fn write_opt_f32(buf: &mut Vec<u8>, v: Option<f32>) -> io::Result<()> {
    match v {
        Some(f) => { buf.push(1); write_f32(buf, f)?; }
        None => buf.push(0),
    }
    Ok(())
}

fn write_string(buf: &mut Vec<u8>, s: &str) -> io::Result<()> {
    write_u32(buf, s.len() as u32)?;
    buf.extend_from_slice(s.as_bytes());
    Ok(())
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap_or([0; 4]))
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap_or([0; 8]))
}

fn read_u32_at(data: &[u8], pos: &mut usize) -> u32 {
    let v = read_u32(data, *pos);
    *pos += 4;
    v
}

fn read_u64_at(data: &[u8], pos: &mut usize) -> u64 {
    let v = read_u64(data, *pos);
    *pos += 8;
    v
}

fn read_i32_at(data: &[u8], pos: &mut usize) -> i32 {
    let v = i32::from_le_bytes(data[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
    *pos += 4;
    v
}

fn read_i64_at(data: &[u8], pos: &mut usize) -> i64 {
    let v = i64::from_le_bytes(data[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
    *pos += 8;
    v
}

fn read_f32_at(data: &[u8], pos: &mut usize) -> f32 {
    let v = f32::from_le_bytes(data[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
    *pos += 4;
    v
}

fn read_opt_f32_at(data: &[u8], pos: &mut usize) -> Option<f32> {
    let tag = data[*pos]; *pos += 1;
    if tag != 0 { Some(read_f32_at(data, pos)) } else { None }
}

fn read_string_at(data: &[u8], pos: &mut usize) -> io::Result<String> {
    let len = read_u32_at(data, pos) as usize;
    let s = std::str::from_utf8(&data[*pos..*pos + len])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        .to_string();
    *pos += len;
    Ok(s)
}
