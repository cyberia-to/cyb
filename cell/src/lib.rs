//! cell — the unit cyb runs.
//!
//! A cell holds a slice of the cybergraph and the loop that turns a neuron's
//! signals into proven state. This first cut is the local loop only: assert a
//! cyberlink, apply it to local state, serve it back. No peers, no consensus,
//! no face — just "cyb can hold a graph and prove a change to it."

use std::collections::BTreeMap;

pub use cybergraph::{
    ApiError, CyberlinkRecord, Cybergraph, NeuronId, Particle, QueryError, QueryOutput, Signal,
    SELF_NETWORK,
};

/// A cyb cell: a local [`Cybergraph`] plus the per-neuron signal chain it heads.
pub struct Cell {
    /// The local graph — the dumb processor over bbg this cell drives.
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
    /// onto the neuron's previous one, and apply it to local state. Returns the
    /// signal's particle (its hash).
    pub fn link(
        &mut self,
        neuron: NeuronId,
        from: Particle,
        to: Particle,
    ) -> Result<Particle, ApiError> {
        let (step, prev) = self.heads.get(&neuron).copied().unwrap_or((0, [0u8; 32]));
        let signal = Signal {
            neuron,
            network: SELF_NETWORK,
            links: vec![CyberlinkRecord {
                neuron,
                from,
                to,
                token: [0u8; 32],
                amount: 1,
                valence: 1,
                height: 0,
            }],
            delta_pi: vec![],
            prev,
            step,
            height: 0,
            proof: None,
        };
        let id = signal.hash();
        self.graph.link(signal)?;
        self.heads.insert(neuron, (step + 1, id));
        Ok(id)
    }

    /// Read the graph with an inf query.
    pub fn query(&self, script: &str) -> Result<QueryOutput, QueryError> {
        self.graph.query(script)
    }

    /// Does a particle exist in this cell's local state?
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

    /// The whole next step in one assertion: a cell holds a link and serves it.
    #[test]
    fn cell_holds_a_link_and_serves_it() {
        let mut cell = Cell::new();
        let neuron = [1u8; 32];
        let from = [2u8; 32];
        let to = [3u8; 32];

        cell.link(neuron, from, to).expect("link applies to local state");

        assert!(cell.has_particle(&to), "target particle materializes after the link");

        // the public query path runs over the local graph
        cell.query("?[particle, energy] := particles{particle, energy}").expect("query runs");
    }

    /// A neuron's signals chain — distinct hashes, both applied.
    #[test]
    fn links_chain_across_signals() {
        let mut cell = Cell::new();
        let n = [1u8; 32];
        let h0 = cell.link(n, [2u8; 32], [3u8; 32]).unwrap();
        let h1 = cell.link(n, [2u8; 32], [4u8; 32]).unwrap();
        assert_ne!(h0, h1, "chained signals have distinct hashes");
        assert!(cell.has_particle(&[3u8; 32]) && cell.has_particle(&[4u8; 32]));
    }
}
