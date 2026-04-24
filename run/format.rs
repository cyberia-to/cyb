//! .model file reader.
//!
//! Spec: specs/format.md

use crate::core::dtype::DType;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid .model: {0}")]
    Invalid(String),

    #[error("missing field: {0}")]
    MissingField(&'static str),
}

/// Fully parsed .model file.
pub struct ModelFile {
    pub card: String,
    pub config: String,
    pub program: String,
    pub tensors_toml: String,
    pub vocab_toml: String,
    pub chat_toml: String,
    pub eval_toml: String,
    /// Binary weights section — may be mmap-backed for large models.
    pub weights: Vec<u8>,
}

/// Per-tensor metadata from the tensors section.
#[derive(Clone, Debug)]
pub struct TensorMeta {
    pub name: String,
    pub shape: Vec<usize>,
    pub dtype: DType,
    pub offset: u64,
    pub size: u64,
}

/// Read a .model file. Large files are mmap'd; the weights slice is
/// copied out once (for simplicity). Zero-copy mmap access can be
/// added later by exposing a separate reader with &[u8] borrows.
pub fn read_model_file(path: &Path) -> Result<ModelFile, FormatError> {
    let file = std::fs::File::open(path)?;
    let file_len = file.metadata()?.len() as usize;

    // Text header scan: read up to 32 MB to find ~~~weights\n marker.
    // Gemma-4 has ~18 MB of tensor index.
    let scan_limit = file_len.min(32 * 1024 * 1024);
    let mut reader = std::io::BufReader::new(&file);
    let mut header = vec![0u8; scan_limit];
    use std::io::Read;
    reader.read_exact(&mut header)?;

    let marker = b"~~~weights\n";
    let marker_pos = header
        .windows(marker.len())
        .position(|w| w == marker)
        .ok_or_else(|| FormatError::Invalid("no ~~~weights section".into()))?;

    let text_part = std::str::from_utf8(&header[..marker_pos])
        .map_err(|e| FormatError::Invalid(format!("non-utf8 text header: {e}")))?;

    // Extract declared weights size from frontmatter `size = N` under [[files]] for weights.
    // Simple approach: scan for the first `size = <integer>` AFTER "weights" mention.
    let weights_size = parse_weights_size(text_part)?;

    // Parse ~~~sections
    let sections = parse_sections(text_part);

    // Read weights section.
    let weights_start = marker_pos + marker.len();
    let weights_end = (weights_start + weights_size).min(file_len);
    let weights = if file_len > 1_000_000_000 {
        // mmap for large files
        drop(reader);
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        mmap[weights_start..weights_end].to_vec()
    } else if weights_end <= header.len() {
        header[weights_start..weights_end].to_vec()
    } else {
        drop(reader);
        let data = std::fs::read(path)?;
        data[weights_start..weights_end].to_vec()
    };

    Ok(ModelFile {
        card: sections.get("card").cloned().unwrap_or_default(),
        config: sections.get("config").cloned().unwrap_or_default(),
        program: sections.get("program").cloned().unwrap_or_default(),
        tensors_toml: sections.get("tensors").cloned().unwrap_or_default(),
        vocab_toml: sections.get("vocab").cloned().unwrap_or_default(),
        chat_toml: sections.get("chat").cloned().unwrap_or_default(),
        eval_toml: sections.get("eval").cloned().unwrap_or_default(),
        weights,
    })
}

fn parse_weights_size(text: &str) -> Result<usize, FormatError> {
    // Look specifically after `name = "weights"` line for `size = N`.
    let mut after_weights = false;
    for line in text.lines() {
        let t = line.trim();
        if t.contains("name = \"weights\"") {
            after_weights = true;
            continue;
        }
        if after_weights {
            if let Some(rest) = t.strip_prefix("size = ") {
                if let Ok(n) = rest.trim().parse::<usize>() {
                    return Ok(n);
                }
            }
        }
    }
    Err(FormatError::MissingField("weights.size"))
}

fn parse_sections(text: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current_content = String::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("~~~") {
            if let Some(prev) = current_name.take() {
                sections.insert(prev, current_content.clone());
                current_content.clear();
            }
            current_name = Some(name.trim().to_string());
        } else if current_name.is_some() {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }
    if let Some(name) = current_name.take() {
        sections.insert(name, current_content);
    }
    sections
}

/// Parse the tensors section (TOML) into a list of TensorMeta.
pub fn parse_tensors_toml(toml_str: &str) -> Result<Vec<TensorMeta>, FormatError> {
    let value: toml::Value = toml::from_str(toml_str)
        .map_err(|e| FormatError::Invalid(format!("tensors.toml: {e}")))?;
    let table = value
        .as_table()
        .ok_or_else(|| FormatError::Invalid("tensors.toml: root is not table".into()))?;

    let mut out = Vec::with_capacity(table.len());
    for (name, entry) in table {
        let t = entry.as_table().ok_or_else(|| {
            FormatError::Invalid(format!("tensors.toml: {name} is not table"))
        })?;
        let shape: Vec<usize> = t
            .get("shape")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_integer().map(|i| i as usize))
                    .collect()
            })
            .ok_or_else(|| FormatError::Invalid(format!("{name}: missing shape")))?;
        let encoding = t
            .get("encoding")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FormatError::Invalid(format!("{name}: missing encoding")))?;
        let dtype = DType::from_str(encoding)
            .ok_or_else(|| FormatError::Invalid(format!("{name}: unknown encoding {encoding}")))?;
        let offset = t
            .get("offset")
            .and_then(|v| v.as_integer())
            .map(|i| i as u64)
            .unwrap_or(0);
        let size = t
            .get("size")
            .and_then(|v| v.as_integer())
            .map(|i| i as u64)
            .unwrap_or(0);
        out.push(TensorMeta {
            name: name.clone(),
            shape,
            dtype,
            offset,
            size,
        });
    }
    Ok(out)
}

/// Convenience: open .model, parse everything.
pub struct LoadedModel {
    pub file: ModelFile,
    pub tensors: Vec<TensorMeta>,
}

impl LoadedModel {
    pub fn load(path: &Path) -> Result<Self, FormatError> {
        let file = read_model_file(path)?;
        let tensors = parse_tensors_toml(&file.tensors_toml)?;
        Ok(Self { file, tensors })
    }

    /// Get raw bytes for a named tensor.
    pub fn tensor_bytes(&self, name: &str) -> Option<&[u8]> {
        let meta = self.tensors.iter().find(|t| t.name == name)?;
        let start = meta.offset as usize;
        let end = start + meta.size as usize;
        self.file.weights.get(start..end)
    }
}
