//! Cell — a running cyb: a local cybergraph plus the per-neuron signal chains
//! it heads.
//!
//! Durable by default, and event-sourced. Every change is a *signal* — an
//! atomic bundle of cyberlinks a neuron heads onto its chain. Signals are the
//! source of truth; the bbg state is derived by replaying them. A cell persists
//! each applied signal as a **tape frame** (the same wire form the sync layer
//! gossips) to an append-only log, and replays that log on open, so its graph
//! survives restart.
//!
//! Durability across *time* (replay a log) and across *space* (apply a peer's
//! batch) are the same mechanism: feed signals to [`Cybergraph::link`], which
//! dedups them through the per-neuron [`SignalChain`]. Re-applying a signal the
//! cell already holds is a harmless no-op (the chain rejects it as
//! equivocation) — so replay and gossip are both idempotent and the grow-only
//! cyberlink graph converges.
//!
//! Use [`Cell::open`] for a durable cell (the default any real cyb uses) and
//! [`Cell::ephemeral`] for an in-memory one (tests, throwaway runs).

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use cybergraph::{
    ApiError, ChainError, Cybergraph, NeuronId, Particle, QueryError, QueryOutput, Signal,
};
use foculus::{decode_signal_frame, decode_signals, encode_signal_frame};

use crate::signal::SignalBuilder;

/// A running cyb: a local [`Cybergraph`] and the durable log of the signals it
/// has applied.
pub struct Cell {
    /// The local graph — the dumb processor this cell drives. Its
    /// `chains` field *is* the cell's chain bookkeeping; the cell keeps none of
    /// its own.
    pub graph: Cybergraph,
    /// Append-only log of applied signal frames. `None` for an ephemeral cell.
    log: Option<File>,
}

impl Cell {
    /// An in-memory cell — forgets everything on drop. For tests and throwaway
    /// runs; a real cyb uses [`Cell::open`].
    pub fn ephemeral() -> Self {
        Self { graph: Cybergraph::new(), log: None }
    }

    /// A durable cell backed by the signal log at `path`. Replays the log to
    /// rebuild state, then appends new signals to it. The graph survives restart.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                fs::create_dir_all(dir)?;
            }
        }

        let mut cell = Self::ephemeral();
        if let Ok(mut f) = File::open(path) {
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes)?;
            // replay: signals are the source of truth, so this rebuilds
            // identical state. Apply straight to the graph — the log already
            // holds these frames, so don't re-log them.
            for sig in decode_signals(&bytes) {
                let _ = cell.graph.link(sig);
            }
        }
        cell.log = Some(OpenOptions::new().create(true).append(true).open(path)?);
        Ok(cell)
    }

    /// The next `(step, prev)` for `neuron`'s chain — the tip. Derived from the
    /// graph's [`SignalChain`], so the cell holds no chain state of its own.
    fn tip(&self, neuron: &NeuronId) -> (u64, Particle) {
        match self.graph.chains.get(neuron) {
            Some(chain) if !chain.entries.is_empty() => {
                let step = chain.entries.len() as u64;
                let prev = chain.entries[&(step - 1)].hash();
                (step, prev)
            }
            _ => (0, [0u8; 32]),
        }
    }

    /// Apply a fully-formed signal to the graph and, on success, persist its
    /// tape frame. Returns the signal's particle. The single choke point every
    /// change flows through — local links, replay, and gossip alike.
    fn commit(&mut self, sig: Signal) -> Result<Particle, ApiError> {
        let id = sig.hash();
        let frame = encode_signal_frame(&sig);
        self.graph.link(sig)?;
        if let Some(log) = self.log.as_mut() {
            let _ = log.write_all(&frame).and_then(|_| log.flush());
        }
        Ok(id)
    }

    /// Cast a sentence: apply an ordered batch of cyberlinks as **one atomic
    /// signal** on `neuron`'s chain. They land together or not at all — half a
    /// sentence never exists (the signal is the utterance boundary). Built at the
    /// chain tip, applied, and persisted (if durable). Returns the signal's
    /// particle.
    pub fn cast(
        &mut self,
        neuron: NeuronId,
        links: impl IntoIterator<Item = (Particle, Particle)>,
    ) -> Result<Particle, ApiError> {
        let (step, prev) = self.tip(&neuron);
        let mut builder = SignalBuilder::new(neuron);
        for (from, to) in links {
            builder = builder.link(from, to, [0u8; 32], 1, 1);
        }
        let mut sig = builder.build();
        sig.step = step;
        sig.prev = prev;
        self.commit(sig)
    }

    /// Assert a single cyberlink `from → to` as `neuron` — a one-link
    /// [`Cell::cast`].
    pub fn link(
        &mut self,
        neuron: NeuronId,
        from: Particle,
        to: Particle,
    ) -> Result<Particle, ApiError> {
        self.cast(neuron, [(from, to)])
    }

    /// Apply one signal frame received from a peer — the same tape frame this
    /// cell would emit. A frame the cell already holds dedups to a no-op via the
    /// SignalChain, so gossiping a signal lands it in the durable graph exactly
    /// like a local one. Returns the signal's particle, or `None` if the frame
    /// was malformed or already held.
    pub fn receive_frame(&mut self, bytes: &[u8]) -> Option<Particle> {
        let sig = decode_signal_frame(bytes)?;
        match self.commit(sig) {
            Ok(id) => Some(id),
            // already held (equivocation) — benign, the graph is unchanged
            Err(ApiError::SyncRejected(ChainError::Equivocation)) => None,
            Err(_) => None,
        }
    }

    /// Every signal this cell holds, as a concatenated batch of tape frames in
    /// per-neuron step order — what it would ship a peer for anti-entropy.
    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for chain in self.graph.chains.values() {
            for sig in chain.entries.values() {
                out.extend_from_slice(&encode_signal_frame(sig));
            }
        }
        out
    }

    /// Absorb a peer's batch of signal frames. Applies each in order; signals
    /// already held (equivocation) or not-yet-applicable (a gap in the chain)
    /// are skipped. Returns how many new signals were applied. Both cells
    /// converge to the union of their signals — eventual consistency for the
    /// grow-only cyberlink graph.
    pub fn absorb(&mut self, bytes: &[u8]) -> usize {
        let mut applied = 0;
        for sig in decode_signals(bytes) {
            if self.commit(sig).is_ok() {
                applied += 1;
            }
        }
        applied
    }

    /// The signals this cell holds, grouped by neuron and ordered by step — the
    /// event log, the source of truth from which the graph state is derived.
    pub fn signals(&self) -> Vec<&Signal> {
        self.graph.chains.values().flat_map(|chain| chain.entries.values()).collect()
    }

    /// How many signals the cell has applied across all chains.
    pub fn len(&self) -> usize {
        self.graph.chains.values().map(|c| c.entries.len()).sum()
    }

    /// Whether the cell holds no signals yet.
    pub fn is_empty(&self) -> bool {
        self.graph.chains.values().all(|c| c.entries.is_empty())
    }

    /// Read the graph with an inf query.
    pub fn query(&self, script: &str) -> Result<QueryOutput, QueryError> {
        self.graph.query(script)
    }

    /// How many particles the cell holds.
    pub fn particles(&self) -> usize {
        self.graph.bbg.state.particles.len()
    }

    /// The graph's nodes: every particle that appears as a cyberlink endpoint,
    /// paired with its energy. Derived from the axons (edges), so a pure *source*
    /// still shows up — energy only flows to a link's target, so a source has no
    /// energy record of its own, but it is a node nonetheless.
    ///
    /// This is the honest node set. The raw `particles` table also holds
    /// axon-particles (`H(from,to)`, carrying weight not energy); those are edges,
    /// not nodes, and are reported by [`Cell::axons`].
    pub fn nodes(&self) -> Vec<(Particle, u64)> {
        let st = &self.graph.bbg.state;
        let mut set = std::collections::BTreeSet::new();
        for (from, to) in st.axon_edges.values() {
            set.insert(*from);
            set.insert(*to);
        }
        set.into_iter().map(|p| (p, st.particles.get(&p).map_or(0, |r| r.energy))).collect()
    }

    /// The graph's axons (edges): `(from, to, weight)`, one per linked pair.
    pub fn axons(&self) -> Vec<(Particle, Particle, u64)> {
        let st = &self.graph.bbg.state;
        st.axon_edges
            .iter()
            .map(|(aid, (from, to))| (*from, *to, st.particles.get(aid).map_or(0, |r| r.weight)))
            .collect()
    }

    /// Does a particle exist in this cell's state?
    pub fn has_particle(&self, p: &Particle) -> bool {
        self.graph.bbg.state.particles.contains_key(p)
    }
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
    fn frame_replicates_a_link_across_cells() {
        let (n, from, to) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        // cell A applies a link; its snapshot is the frame it would emit.
        let mut a = Cell::ephemeral();
        a.link(n, from, to).unwrap();
        let frame = a.snapshot();
        // cell B receives it over the "wire" and applies it.
        let mut b = Cell::ephemeral();
        assert_eq!(b.absorb(&frame), 1, "one new signal applied");
        assert!(b.has_particle(&to), "the linked particle replicated onto B");
    }

    #[test]
    fn gossip_is_idempotent_and_converges() {
        // Two cells, two distinct neurons, each with its own links.
        let (na, nb) = ([0xAAu8; 32], [0xBBu8; 32]);
        let mut a = Cell::ephemeral();
        let mut b = Cell::ephemeral();
        a.link(na, [1u8; 32], [2u8; 32]).unwrap();
        a.link(na, [1u8; 32], [3u8; 32]).unwrap();
        b.link(nb, [4u8; 32], [5u8; 32]).unwrap();

        // exchange snapshots both ways
        let (sa, sb) = (a.snapshot(), b.snapshot());
        assert_eq!(b.absorb(&sa), 2, "B applies A's two signals");
        assert_eq!(a.absorb(&sb), 1, "A applies B's one signal");

        // both converge to the union
        for p in [[2u8; 32], [3u8; 32], [5u8; 32]] {
            assert!(a.has_particle(&p) && b.has_particle(&p), "converged on {p:?}");
        }

        // re-applying the same snapshot is a pure no-op — the SignalChain dedups
        assert_eq!(b.absorb(&sa), 0, "no double-counting on re-gossip");
        assert_eq!(a.absorb(&sb), 0, "no double-counting on re-gossip");
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
