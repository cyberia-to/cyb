//! Cell — a running cyb: a local cybergraph plus the per-neuron chain it heads.
//!
//! This is the whole local loop: assert a cyberlink, apply it to state, read it
//! back. It builds signals through [`SignalBuilder`] and chains them per neuron.
//! Both faces — the GUI shell and the CLI — drive a `Cell`; nothing else.

use std::collections::BTreeMap;

use cybergraph::{ApiError, Cybergraph, NeuronId, Particle, QueryError, QueryOutput};

use crate::signal::SignalBuilder;

/// A running cyb: a local [`Cybergraph`] and the signal chain it heads.
pub struct Cell {
    /// The local graph — the dumb processor this cell drives.
    pub graph: Cybergraph,
    /// Per-neuron chain head: (next step, prev signal hash).
    heads: BTreeMap<NeuronId, (u64, Particle)>,
}

impl Cell {
    /// A fresh cell with an empty graph.
    pub fn new() -> Self {
        Self { graph: Cybergraph::new(), heads: BTreeMap::new() }
    }

    /// Assert a cyberlink `from → to` as `neuron`: build the signal, chain it
    /// onto the neuron's previous one, apply it. Returns the signal's particle.
    pub fn link(
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

impl Default for Cell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_holds_a_link_and_serves_it() {
        let mut cell = Cell::new();
        let (neuron, from, to) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        cell.link(neuron, from, to).expect("link applies");
        assert!(cell.has_particle(&to), "target particle materializes");
        cell.query("?[particle, energy] := particles{particle, energy}").expect("query runs");
    }

    #[test]
    fn links_chain_across_signals() {
        let mut cell = Cell::new();
        let n = [1u8; 32];
        let h0 = cell.link(n, [2u8; 32], [3u8; 32]).unwrap();
        let h1 = cell.link(n, [2u8; 32], [4u8; 32]).unwrap();
        assert_ne!(h0, h1, "chained signals differ");
        assert!(cell.has_particle(&[3u8; 32]) && cell.has_particle(&[4u8; 32]));
    }
}
