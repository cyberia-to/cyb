//! Body proving tickets: one SpMV step over this cyb's own graph, checked
//! by recomputation.
//!
//! A ticket claims `y = A·x` where `A` is the public graph (axons as
//! weighted edges) and `x` is drawn from public data alone — the beacon
//! seed and the ticket number. Nothing in the statement is secret, so
//! there is no witness to prove: any sound proof in today's public profile
//! (zheng certificate v3) would disclose `y` and check the same `nnz`
//! products the recomputation does, at `8·n` more bytes. A ticket
//! therefore carries the graph commitment and a digest of `y`, and the
//! verifier recomputes — the same choice foculus makes for φ*
//! (`foculus::phi`). This replaces zheng's legacy `prove_spmv` /
//! `verify_spmv`, whose HyperNova fold the verifier never checked.
//!
//! Sound, deterministic, not succinct: verifying costs one SpMV, the same
//! as producing. A succinct ticket needs the SpMV as a step relation under
//! accumulation (proof-system repair, phase 3).

use cyber_hemera::Hasher;
use nebu::Goldilocks as F;

const GRAPH_DOMAIN: &[u8] = b"cyb-ticket-graph-v1";
const OUTPUT_DOMAIN: &[u8] = b"cyb-ticket-output-v1";

/// Bytes of a [`Ticket`]: graph 32 ‖ seed 8 ‖ number 8 ‖ output 32.
pub const TICKET_BYTES: usize = 80;

/// The public matrix a ticket is computed over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketGraph {
    pub n: usize,
    /// (row, column, weight), every index below `n`.
    pub edges: Vec<(usize, usize, F)>,
}

/// One ticket: the statement `digest(A·x(seed, number)) = output` over the
/// graph with commitment `graph`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub graph: [u8; 32],
    pub seed: u64,
    pub number: u64,
    pub output: [u8; 32],
}

impl Ticket {
    pub fn to_bytes(&self) -> [u8; TICKET_BYTES] {
        let mut out = [0u8; TICKET_BYTES];
        out[..32].copy_from_slice(&self.graph);
        out[32..40].copy_from_slice(&self.seed.to_le_bytes());
        out[40..48].copy_from_slice(&self.number.to_le_bytes());
        out[48..].copy_from_slice(&self.output);
        out
    }
}

fn digest(domain: &[u8], body: impl FnOnce(&mut Hasher)) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(&(domain.len() as u64).to_le_bytes());
    h.update(domain);
    body(&mut h);
    *h.finalize().as_bytes()
}

impl TicketGraph {
    /// The local cybergraph as a matrix: axon endpoints become row/column
    /// indices in first-seen order, amounts ride as weights (at least 1).
    /// A cyb that has not lived yet computes over a seeded ring of two.
    pub fn from_axons(axons: &[([u8; 32], [u8; 32], u64)]) -> Self {
        let mut index = std::collections::HashMap::<[u8; 32], usize>::new();
        let mut id = |p: [u8; 32]| {
            let next = index.len();
            *index.entry(p).or_insert(next)
        };
        let edges: Vec<_> = axons
            .iter()
            .map(|(from, to, amount)| (id(*from), id(*to), F::new((*amount).max(1))))
            .collect();
        let n = index.len().max(2);
        if edges.is_empty() {
            let ring = (0..n)
                .map(|i| (i, (i + 1) % n, F::new(splitmix64(i as u64) | 1)))
                .collect();
            return Self { n, edges: ring };
        }
        Self { n, edges }
    }

    /// Well-formed: nonempty and every edge in range.
    pub fn valid(&self) -> bool {
        self.n > 0 && self.edges.iter().all(|&(r, c, _)| r < self.n && c < self.n)
    }

    /// Hemera commitment to `n` and the ordered edge list.
    pub fn commitment(&self) -> [u8; 32] {
        digest(GRAPH_DOMAIN, |h| {
            h.update(&(self.n as u64).to_le_bytes());
            h.update(&(self.edges.len() as u64).to_le_bytes());
            for &(r, c, w) in &self.edges {
                h.update(&(r as u64).to_le_bytes());
                h.update(&(c as u64).to_le_bytes());
                h.update(&w.as_u64().to_le_bytes());
            }
        })
    }

    /// `A·x`; `None` if the graph is malformed or `x` has the wrong length.
    pub fn spmv(&self, x: &[F]) -> Option<Vec<F>> {
        if !self.valid() || x.len() != self.n {
            return None;
        }
        let mut y = vec![F::ZERO; self.n];
        for &(r, c, w) in &self.edges {
            y[r] += w * x[c];
        }
        Some(y)
    }
}

/// A graph with its commitment computed once: a body proves many tickets
/// over one snapshot, a verifier checks many against one graph.
#[derive(Clone, Debug)]
pub struct CommittedGraph {
    graph: TicketGraph,
    commitment: [u8; 32],
}

impl CommittedGraph {
    pub fn new(graph: TicketGraph) -> Self {
        let commitment = graph.commitment();
        Self { graph, commitment }
    }
    pub fn graph(&self) -> &TicketGraph {
        &self.graph
    }
    pub fn commitment(&self) -> [u8; 32] {
        self.commitment
    }
}

/// The public input vector of ticket `number` under beacon `seed`.
pub fn challenge(seed: u64, number: u64, n: usize) -> Vec<F> {
    (0..n)
        .map(|i| F::new(splitmix64(number.wrapping_mul(0x9e37) ^ seed ^ i as u64)))
        .collect()
}

fn output_digest(y: &[F]) -> [u8; 32] {
    digest(OUTPUT_DOMAIN, |h| {
        h.update(&(y.len() as u64).to_le_bytes());
        for v in y {
            h.update(&v.as_u64().to_le_bytes());
        }
    })
}

/// Compute ticket `number` over `graph` at beacon `seed`.
pub fn prove(graph: &CommittedGraph, seed: u64, number: u64) -> Option<Ticket> {
    let g = &graph.graph;
    let y = g.spmv(&challenge(seed, number, g.n))?;
    Some(Ticket {
        graph: graph.commitment,
        seed,
        number,
        output: output_digest(&y),
    })
}

/// Recompute and compare: the graph must match the ticket's commitment
/// and `A·x(seed, number)` must hash to the claimed output.
pub fn verify(ticket: &Ticket, graph: &CommittedGraph) -> bool {
    ticket.graph == graph.commitment
        && prove(graph, ticket.seed, ticket.number).is_some_and(|t| t == *ticket)
}

/// One counted ticket in `TICKET_DIFFICULTY` computed ones. Calibrated so a
/// counted ticket keeps the cost the retired zheng SpMV ticket had: 97.6 ms
/// legacy against 0.58 ms recomputed on the default body graph (473 nodes,
/// M4 Max, cyb#1407), ratio ≈ 170 — the subsidy a body earns per minute,
/// and so PUSSY issuance on the test chain, stays where it was.
pub const TICKET_DIFFICULTY: u64 = 170;

/// Whether a ticket counts: its output digest, read as a little-endian u64,
/// falls below `u64::MAX / TICKET_DIFFICULTY`. Public and recomputable.
pub fn counts(ticket: &Ticket) -> bool {
    let mut w = [0u8; 8];
    w.copy_from_slice(&ticket.output[..8]);
    u64::from_le_bytes(w) < u64::MAX / TICKET_DIFFICULTY
}

/// splitmix64 — the standard seed scrambler; deterministic vectors.
pub fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[cfg(test)]
#[path = "ticket_tests.rs"]
mod tests;
