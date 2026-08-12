//! The signal bus and helpers to assemble Signals from cyberlinks.
//!
//! A signal is an atomic bundle of cyberlinks signed by one neuron.
//! Inside a single cyb process the bus is just a FIFO queue — the
//! `proof` field stays `None` because local signals do not need a
//! STARK. When a neuron commits the signal on-chain, `proof` becomes
//! `Some(zheng::Proof)` and the same value rides the network.
//!
//! Bevy/wgpu wrappers live in higher layers. This module is plain
//! Rust so the core can run in a CLI, on a server, on Android, or
//! inside tests.

use std::sync::Mutex;

use cybergraph::{CyberlinkRecord, NeuronId, Particle, Signal, SELF_NETWORK};

/// Build a single cyberlink record. Local-only signals carry
/// `neuron = [0; 32]` and `height = 0`; they are filled in when the
/// owning neuron actually commits to the chain.
pub fn link(
    neuron: NeuronId,
    from: Particle,
    to: Particle,
    token: Particle,
    amount: u64,
    valence: i8,
) -> CyberlinkRecord {
    CyberlinkRecord { neuron, from, to, token, amount, valence, height: 0 }
}

/// Fluent builder for assembling a multi-cyberlink signal.
pub struct SignalBuilder {
    neuron: NeuronId,
    links: Vec<CyberlinkRecord>,
}

impl SignalBuilder {
    pub fn new(neuron: NeuronId) -> Self {
        Self { neuron, links: Vec::new() }
    }

    pub fn link(
        mut self,
        from: Particle,
        to: Particle,
        token: Particle,
        amount: u64,
        valence: i8,
    ) -> Self {
        self.links.push(link(self.neuron, from, to, token, amount, valence));
        self
    }

    pub fn build(self) -> Signal {
        Signal {
            neuron: self.neuron,
            network: SELF_NETWORK,
            links: self.links,
            delta_pi: Vec::new(),
            box_moves: Vec::new(),
            prev: [0u8; 32],
            step: 0,
            height: 0,
            proof: None,
        }
    }
}

/// Thread-safe FIFO of signals. Single bus per cyb process.
///
/// Chromas push with [`SignalBus::publish`]. A consumer (typically a
/// renderer or another chroma) drains with [`SignalBus::drain`] and
/// filters on `link.to`.
pub struct SignalBus {
    inner: Mutex<Vec<Signal>>,
}

impl SignalBus {
    pub fn new() -> Self {
        Self { inner: Mutex::new(Vec::new()) }
    }

    pub fn publish(&self, signal: Signal) {
        self.inner.lock().expect("signal bus poisoned").push(signal);
    }

    /// Take every queued signal, leaving the bus empty.
    pub fn drain(&self) -> Vec<Signal> {
        let mut guard = self.inner.lock().expect("signal bus poisoned");
        std::mem::take(&mut *guard)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().map(|g| g.is_empty()).unwrap_or(true)
    }
}

impl Default for SignalBus {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chroma::{ChromaId, chroma_particle};
    use crate::intent::SUBMIT;

    #[test]
    fn builder_assembles_signal() {
        let neuron: NeuronId = [7u8; 32];
        let sig = SignalBuilder::new(neuron)
            .link(chroma_particle(ChromaId::Com),
                  chroma_particle(ChromaId::Spacetime),
                  SUBMIT, 1, 1)
            .build();
        assert_eq!(sig.links.len(), 1);
        assert_eq!(sig.neuron, neuron);
        assert!(sig.proof.is_none());
    }

    #[test]
    fn bus_round_trips_signals() {
        let bus = SignalBus::new();
        assert!(bus.is_empty());
        bus.publish(SignalBuilder::new([0u8; 32]).build());
        bus.publish(SignalBuilder::new([0u8; 32]).build());
        assert_eq!(bus.drain().len(), 2);
        assert!(bus.is_empty());
    }
}
