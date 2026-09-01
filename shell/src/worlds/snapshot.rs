//! `snapshot` — the cell, written as a `.graph` container.
//!
//! CT-0 (tru's compiled-transformers spec) begins with one sentence:
//! *compile: G → M, where G is a cybergraph snapshot in .graph format*. The
//! compiler is tru milestone M5 and does not exist yet — but its input can,
//! and from today it does: typing `snapshot` into the commander writes this
//! cyb's whole graph to `~/cyb/snapshot.graph`, frontmatter + config +
//! fixed 128-byte cyberlink records, exactly the layout tru's reader mmaps.
//!
//! That makes the artifact real before the compiler is: `tru inspect` and
//! `tru focus` chew this file right now, and the day the eight passes land,
//! the pipeline's first stage has been in production for months.

use std::io::Write as _;

use super::SharedCell;

/// The .graph cyberlink record: 128 bytes, cyb-graph spec §cyberlinks.
const RECORD_SIZE: usize = 128;

fn encode(
    neuron: &[u8; 32],
    from: &[u8; 32],
    to: &[u8; 32],
    token: u32,
    amount: u128,
    valence: i8,
    block: u64,
) -> [u8; RECORD_SIZE] {
    let mut r = [0u8; RECORD_SIZE];
    r[0..32].copy_from_slice(neuron);
    r[32..64].copy_from_slice(from);
    r[64..96].copy_from_slice(to);
    r[96..100].copy_from_slice(&token.to_le_bytes());
    r[100..116].copy_from_slice(&amount.to_le_bytes());
    r[116] = valence as u8;
    r[117..125].copy_from_slice(&block.to_le_bytes());
    r
}

/// Write the snapshot. Returns `(path, signals, links)` on success.
pub fn export(shared: &SharedCell) -> Result<(std::path::PathBuf, usize, usize), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = std::path::Path::new(&home).join("cyb").join("snapshot.graph");

    // Every chain, every signal, chain order — the snapshot is the whole
    // cell, not one neuron's view of it.
    let mut records: Vec<u8> = Vec::new();
    let (mut n_signals, mut n_links) = (0usize, 0usize);
    {
        let cell = shared.cell.lock().expect("shared cell poisoned");
        for chain in cell.graph.chains.values() {
            for sig in chain.entries.values() {
                n_signals += 1;
                for l in &sig.links {
                    n_links += 1;
                    records.extend_from_slice(&encode(
                        &l.neuron,
                        &l.from,
                        &l.to,
                        1, // single-token table below; index 1 throughout
                        l.amount as u128,
                        l.valence,
                        l.height,
                    ));
                }
            }
        }
    }

    let frontmatter = format!(
        "[cyb]\n\
         types = [\"graph\"]\n\
         name = \"cyb-snapshot\"\n\
         \n\
         [[files]]\n\
         name = \"config\"\n\
         format = \"toml\"\n\
         \n\
         [[files]]\n\
         name = \"cyberlinks\"\n\
         format = \"records\"\n\
         size = {}\n",
        records.len()
    );
    // The one-token denomination table CT-0 §2.4 resolves stakes against.
    // Local links are cast with the zero token particle; weight one keeps
    // effective stake equal to raw amount.
    let config = "chain_id = \"cyb-local\"\n\
                  block = 0\n\
                  \n\
                  [[tokens]]\n\
                  particle = \"0000000000000000000000000000000000000000000000000000000000000000\"\n\
                  weight = 1\n";

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(frontmatter.as_bytes()).map_err(|e| e.to_string())?;
    write!(f, "~~~config\n{config}").map_err(|e| e.to_string())?;
    f.write_all(b"~~~cyberlinks\n").map_err(|e| e.to_string())?;
    f.write_all(&records).map_err(|e| e.to_string())?;

    Ok((path, n_signals, n_links))
}
