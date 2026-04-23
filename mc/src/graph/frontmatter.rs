//! Parse the `.cyb` three-rule frontmatter: TOML up to the first `~~~` marker.

use crate::error::{McError, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Frontmatter {
    pub cyb: CybMeta,
    #[serde(default, rename = "files")]
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CybMeta {
    pub types: Vec<String>,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub format: String,
    #[serde(default)]
    pub size: Option<u64>,
}

/// Split a `.cyb` byte stream at the first `\n~~~` line.
///
/// Returns the frontmatter as utf-8 and the byte offset of the first `~~~`
/// (so the caller starts section parsing exactly there).
pub fn split(bytes: &[u8]) -> Result<(&str, usize)> {
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if &bytes[i..i + 4] == b"\n~~~" {
            let fm = std::str::from_utf8(&bytes[..i + 1])
                .map_err(|e| McError::InvalidGraph(format!("non-utf8 frontmatter: {e}")))?;
            return Ok((fm, i + 1));
        }
        i += 1;
    }
    Err(McError::InvalidGraph("no `~~~` delimiter found".into()))
}

pub fn parse(s: &str) -> Result<Frontmatter> {
    Ok(toml::from_str(s)?)
}
