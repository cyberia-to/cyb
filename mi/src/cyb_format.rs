//! `.model` file writer.
//!
//! The `.model` format is seven named text sections plus a trailing
//! binary `~~~weights` blob. TOML frontmatter lists each section and the
//! declared weight-blob size so readers can mmap it efficiently.
//!
//! This is the writer half of the format only — `mr/src/format.rs` owns
//! the reader path for the live runtime.

use std::io::{self, Write};
use std::path::Path;

/// Pack the seven sections + weights into a single `.model` file at
/// `output_path`. All text sections are UTF-8; `weights` is whatever
/// the tensor index (`tensors_toml`) claims the layout to be.
#[allow(clippy::too_many_arguments)]
pub fn write_model_file(
    output_path: &Path,
    name: &str,
    card: &str,
    config: &str,
    program: &str,
    program_format: &str,
    tensors_toml: &str,
    vocab: &str,
    eval: &str,
    weights: &[u8],
) -> io::Result<()> {
    let mut f = std::fs::File::create(output_path)?;

    // --- TOML frontmatter ---
    writeln!(f, "[cyb]")?;
    writeln!(f, "types = [\"model\"]")?;
    writeln!(f, "name = \"{name}\"")?;
    writeln!(f)?;
    for (section, format) in [
        ("card", "md"),
        ("config", "toml"),
        ("program", program_format),
        ("tensors", "toml"),
        ("vocab", "toml"),
        ("eval", "toml"),
    ] {
        writeln!(f, "[[files]]")?;
        writeln!(f, "name = \"{section}\"")?;
        writeln!(f, "format = \"{format}\"")?;
        writeln!(f)?;
    }
    writeln!(f, "[[files]]")?;
    writeln!(f, "name = \"weights\"")?;
    writeln!(f, "format = \"tensors\"")?;
    writeln!(f, "size = {}", weights.len())?;

    // --- Named text sections ---
    for (marker, body) in [
        ("~~~card", card),
        ("~~~config", config),
        ("~~~program", program),
        ("~~~tensors", tensors_toml),
        ("~~~vocab", vocab),
        ("~~~eval", eval),
    ] {
        writeln!(f, "{marker}")?;
        f.write_all(body.as_bytes())?;
        if !body.ends_with('\n') {
            writeln!(f)?;
        }
    }

    // --- Binary weights ---
    writeln!(f, "~~~weights")?;
    f.write_all(weights)?;

    Ok(())
}
