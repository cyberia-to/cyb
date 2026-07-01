//! Cell — a running cyb: a local cybergraph plus the per-neuron chain it heads.
//!
//! Durable by default. A cell persists the links it applies to an append-only
//! log and replays them on open, so its graph survives restart. Signals are
//! deterministic, so replaying `(neuron, from, to)` rebuilds identical state —
//! event sourcing, the links are the source of truth, bbg state is derived.
//!
//! Use [`Cell::open`] for a durable cell (the default any real cyb uses) and
//! [`Cell::ephemeral`] for an in-memory one (tests, throwaway runs).

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use cybergraph::{ApiError, Cybergraph, NeuronId, Particle, QueryError, QueryOutput};

use crate::signal::SignalBuilder;

/// A running cyb: a local [`Cybergraph`] and the signal chain it heads.
pub struct Cell {
    /// The local graph — the dumb processor this cell drives.
    pub graph: Cybergraph,
    /// Per-neuron chain head: (next step, prev signal hash).
    heads: BTreeMap<NeuronId, (u64, Particle)>,
    /// Append-only link log. `None` for an ephemeral cell.
    log: Option<File>,
}

impl Cell {
    /// An in-memory cell — forgets everything on drop. For tests and throwaway
    /// runs; a real cyb uses [`Cell::open`].
    pub fn ephemeral() -> Self {
        Self { graph: Cybergraph::new(), heads: BTreeMap::new(), log: None }
    }

    /// A durable cell backed by the link log at `path`. Replays the log to
    /// rebuild state, then appends new links to it. The graph survives restart.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                fs::create_dir_all(dir)?;
            }
        }

        let mut cell = Self::ephemeral();
        if let Ok(f) = File::open(path) {
            for line in BufReader::new(f).lines() {
                if let Some((neuron, from, to)) = parse(&line?) {
                    // replay: deterministic, so rebuilds identical state
                    let _ = cell.apply(neuron, from, to);
                }
            }
        }
        cell.log = Some(OpenOptions::new().create(true).append(true).open(path)?);
        Ok(cell)
    }

    /// Build + chain + apply a signal for one cyberlink. No logging.
    fn apply(
        &mut self,
        neuron: NeuronId,
        from: Particle,
        to: Particle,
    ) -> Result<Particle, ApiError> {
        let mut sig = SignalBuilder::new(neuron).link(from, to, [0u8; 32], 1, 1).build();
        let (step, prev) = self.heads.get(&neuron).copied().unwrap_or((0, [0u8; 32]));
        sig.prev = prev;
        sig.step = step;
        let id = sig.hash();
        self.graph.link(sig)?;
        self.heads.insert(neuron, (step + 1, id));
        Ok(id)
    }

    /// Assert a cyberlink `from → to` as `neuron`, apply it, and persist it to
    /// the log (if durable). Returns the signal's particle.
    pub fn link(
        &mut self,
        neuron: NeuronId,
        from: Particle,
        to: Particle,
    ) -> Result<Particle, ApiError> {
        let id = self.apply(neuron, from, to)?;
        if let Some(log) = self.log.as_mut() {
            let line = format!("{} {} {}\n", hex(&neuron), hex(&from), hex(&to));
            let _ = log.write_all(line.as_bytes()).and_then(|_| log.flush());
        }
        Ok(id)
    }

    /// Read the graph with an inf query.
    pub fn query(&self, script: &str) -> Result<QueryOutput, QueryError> {
        self.graph.query(script)
    }

    /// How many particles the cell holds.
    pub fn particles(&self) -> usize {
        self.graph.bbg.state.particles.len()
    }

    /// Does a particle exist in this cell's state?
    pub fn has_particle(&self, p: &Particle) -> bool {
        self.graph.bbg.state.particles.contains_key(p)
    }
}

fn hex(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Parse one log line: `neuron_hex from_hex to_hex`.
fn parse(line: &str) -> Option<(NeuronId, Particle, Particle)> {
    let mut it = line.split_whitespace();
    Some((unhex(it.next()?)?, unhex(it.next()?)?, unhex(it.next()?)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_holds_a_link_and_serves_it() {
        let mut cell = Cell::ephemeral();
        let (neuron, from, to) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        cell.link(neuron, from, to).expect("link applies");
        assert!(cell.has_particle(&to), "target particle materializes");
        cell.query("?[particle, energy] := particles{particle, energy}").expect("query runs");
    }

    #[test]
    fn links_chain_across_signals() {
        let mut cell = Cell::ephemeral();
        let n = [1u8; 32];
        let h0 = cell.link(n, [2u8; 32], [3u8; 32]).unwrap();
        let h1 = cell.link(n, [2u8; 32], [4u8; 32]).unwrap();
        assert_ne!(h0, h1, "chained signals differ");
        assert!(cell.has_particle(&[3u8; 32]) && cell.has_particle(&[4u8; 32]));
    }

    #[test]
    fn durable_survives_reopen() {
        let path = std::env::temp_dir().join("cyb-durable-test.log");
        let _ = fs::remove_file(&path);

        let (n, from, to) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        {
            let mut cell = Cell::open(&path).expect("open");
            cell.link(n, from, to).expect("link");
            assert!(cell.has_particle(&to));
        } // dropped — file remains

        {
            let cell = Cell::open(&path).expect("reopen");
            assert!(cell.has_particle(&to), "particle survived restart via replay");
        }

        let _ = fs::remove_file(&path);
    }
}
