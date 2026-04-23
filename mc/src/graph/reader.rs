//! Open and index a `.graph` file: validate frontmatter, locate `~~~` sections,
//! expose them as borrowed slices over the mmap.

use std::collections::HashMap;
use std::path::Path;

use memmap2::Mmap;

use crate::error::{McError, Result};
use crate::graph::frontmatter::{self, FileEntry, Frontmatter};
use crate::graph::record::CyberlinkIter;

pub struct Graph {
    mmap: Mmap,
    frontmatter: Frontmatter,
    sections: HashMap<String, Section>,
}

#[derive(Debug, Clone)]
struct Section {
    start: usize,
    end: usize,
    #[allow(dead_code)]
    entry: FileEntry,
}

impl Graph {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file) }?;

        let (fm_str, body_start) = frontmatter::split(&mmap[..])?;
        let frontmatter = frontmatter::parse(fm_str)?;

        if !frontmatter.cyb.types.iter().any(|t| t == "graph") {
            return Err(McError::InvalidGraph(format!(
                "container types {:?} does not include \"graph\"",
                frontmatter.cyb.types
            )));
        }

        let sections = index_sections(&mmap[..], body_start, &frontmatter.files)?;
        Ok(Self { mmap, frontmatter, sections })
    }

    pub fn name(&self) -> &str {
        &self.frontmatter.cyb.name
    }

    pub fn frontmatter(&self) -> &Frontmatter {
        &self.frontmatter
    }

    pub fn section(&self, name: &str) -> Result<&[u8]> {
        let s = self.sections.get(name).ok_or(McError::MissingSection("section"))?;
        Ok(&self.mmap[s.start..s.end])
    }

    pub fn section_str(&self, name: &str) -> Result<&str> {
        let bytes = self.section(name)?;
        std::str::from_utf8(bytes)
            .map_err(|e| McError::InvalidGraph(format!("section `{name}` not utf-8: {e}")))
    }

    pub fn cyberlinks(&self) -> Result<CyberlinkIter<'_>> {
        let bytes = self.section("cyberlinks")?;
        Ok(CyberlinkIter::new(bytes))
    }

    pub fn config_raw(&self) -> Result<&str> {
        self.section_str("config")
    }
}

fn index_sections(
    bytes: &[u8],
    body_start: usize,
    entries: &[FileEntry],
) -> Result<HashMap<String, Section>> {
    let mut sections = HashMap::new();
    let mut cursor = body_start;

    for entry in entries {
        let header = format!("~~~{}\n", entry.name);
        if cursor + header.len() > bytes.len() || !bytes[cursor..].starts_with(header.as_bytes()) {
            return Err(McError::InvalidGraph(format!(
                "expected `{}` at byte {}",
                header.trim_end(),
                cursor
            )));
        }
        let start = cursor + header.len();

        let end = match entry.size {
            Some(sz) => start + sz as usize,
            None => find_text_end(bytes, start),
        };

        if end > bytes.len() {
            return Err(McError::InvalidGraph(format!(
                "section `{}` extends past EOF (end={}, file={})",
                entry.name,
                end,
                bytes.len()
            )));
        }

        sections.insert(
            entry.name.clone(),
            Section { start, end, entry: entry.clone() },
        );

        cursor = end;
        // Skip a single trailing newline between sections, if present.
        if cursor < bytes.len() && bytes[cursor] == b'\n' {
            cursor += 1;
        }
    }

    Ok(sections)
}

/// Scan forward from `start` for either `\n~~~` (next section) or EOF.
/// Returns the byte offset of the first byte NOT belonging to this text section.
fn find_text_end(bytes: &[u8], start: usize) -> usize {
    let mut j = start;
    while j + 4 <= bytes.len() {
        if &bytes[j..j + 4] == b"\n~~~" {
            return j; // exclude the leading newline of the next section header
        }
        j += 1;
    }
    bytes.len()
}
