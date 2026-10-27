//! The 3×3 chroma grid — the shape of the agent, in code.
//!
//! Eight organs that show, arranged around a pluggable centre (spacetime).
//! The slots are named by the organs of `cy/specs/anatomy.md`, the grid by
//! `cy/specs/chroma.md`: cy designs it, cyb paints it. The layout is closed —
//! adding a feature means a new intent particle, never a new slot.
//!
//! ```text
//! now      voice      name
//! sense    spacetime  sigma
//! memory   com        time
//! ```

use cybergraph::Particle;

/// One of the nine chroma slot identifiers, named by the organ it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChromaId {
    /// where — the file the agent stands on (painter's old name: space)
    Now,
    /// says — what the agent answers; soma thinks behind it (old: ad)
    Voice,
    /// who — the neuron's name; drawn as the avatar (old: ava)
    Name,
    /// messages — other neurons, files, robots
    Sense,
    /// the one pluggable centre: a world in cyb, the transcript in a terminal
    Spacetime,
    /// the sum — tokens, balances, the body's work as rows
    Sigma,
    /// map — files with links; drawn as the brain (old: brain)
    Memory,
    /// command — the one input
    Com,
    /// log ← now → plan
    Time,
}

impl ChromaId {
    pub const ALL: [ChromaId; 9] = [
        ChromaId::Now, ChromaId::Voice, ChromaId::Name,
        ChromaId::Sense, ChromaId::Spacetime, ChromaId::Sigma,
        ChromaId::Memory, ChromaId::Com, ChromaId::Time,
    ];

    /// The organ's name — the vocabulary of `cy/specs/anatomy.md`.
    pub const fn organ(self) -> &'static str {
        match self {
            ChromaId::Now => "now",
            ChromaId::Voice => "voice",
            ChromaId::Name => "name",
            ChromaId::Sense => "sense",
            ChromaId::Spacetime => "spacetime",
            ChromaId::Sigma => "sigma",
            ChromaId::Memory => "memory",
            ChromaId::Com => "com",
            ChromaId::Time => "time",
        }
    }

    /// Spacetime is a slot, not an organ: it hosts whatever renders.
    pub const fn is_organ(self) -> bool {
        !matches!(self, ChromaId::Spacetime)
    }

    pub const fn label(self) -> &'static str {
        match self {
            ChromaId::Now => "chroma/now",
            ChromaId::Voice => "chroma/voice",
            ChromaId::Name => "chroma/name",
            ChromaId::Sense => "chroma/sense",
            ChromaId::Spacetime => "chroma/spacetime",
            ChromaId::Sigma => "chroma/sigma",
            ChromaId::Memory => "chroma/memory",
            ChromaId::Com => "chroma/com",
            ChromaId::Time => "chroma/time",
        }
    }

    pub const fn pos(self) -> GridPos {
        match self {
            ChromaId::Now => GridPos { col: 0, row: 0 },
            ChromaId::Voice => GridPos { col: 1, row: 0 },
            ChromaId::Name => GridPos { col: 2, row: 0 },
            ChromaId::Sense => GridPos { col: 0, row: 1 },
            ChromaId::Spacetime => GridPos { col: 1, row: 1 },
            ChromaId::Sigma => GridPos { col: 2, row: 1 },
            ChromaId::Memory => GridPos { col: 0, row: 2 },
            ChromaId::Com => GridPos { col: 1, row: 2 },
            ChromaId::Time => GridPos { col: 2, row: 2 },
        }
    }

    /// The painter's pre-2026-10-07 slot name, for reading old particles and
    /// old pages (`prysm/chroma/specs/*`): space → now, ad → voice,
    /// ava → name, brain → memory; the rest never changed.
    pub fn from_painter_name(name: &str) -> Option<Self> {
        Some(match name {
            "space" | "now" => ChromaId::Now,
            "ad" | "voice" => ChromaId::Voice,
            "ava" | "name" => ChromaId::Name,
            "sense" => ChromaId::Sense,
            "spacetime" => ChromaId::Spacetime,
            "sigma" => ChromaId::Sigma,
            "brain" | "memory" => ChromaId::Memory,
            "com" => ChromaId::Com,
            "time" => ChromaId::Time,
            _ => return None,
        })
    }
}

/// Position in the 3×3 grid. (0,0) is top-left, (2,2) is bottom-right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub col: u8,
    pub row: u8,
}

/// Well-known particle identity for a chroma slot.
///
/// The first bytes spell the chroma's label so the particle is
/// recognisable in hex dumps. When the runtime is wired against the
/// real hemera hash, these should migrate to `hemera::hash(label)`.
pub const fn chroma_particle(id: ChromaId) -> Particle {
    label_particle(id.label().as_bytes())
}

pub(crate) const fn label_particle(label: &[u8]) -> Particle {
    let mut out = [0u8; 32];
    let mut i = 0;
    let max = if label.len() < 32 { label.len() } else { 32 };
    while i < max {
        out[i] = label[i];
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_chroma_has_unique_particle() {
        let mut seen = std::collections::HashSet::new();
        for c in ChromaId::ALL {
            assert!(seen.insert(chroma_particle(c)), "duplicate particle for {:?}", c);
        }
    }

    #[test]
    fn every_chroma_has_unique_grid_pos() {
        let mut seen = std::collections::HashSet::new();
        for c in ChromaId::ALL {
            assert!(seen.insert(c.pos()), "duplicate grid pos for {:?}", c);
        }
    }

    #[test]
    fn eight_organs_around_one_centre() {
        assert_eq!(ChromaId::ALL.iter().filter(|c| c.is_organ()).count(), 8);
        assert_eq!(ChromaId::Spacetime.pos(), GridPos { col: 1, row: 1 });
    }

    #[test]
    fn painter_names_resolve_to_organs() {
        assert_eq!(ChromaId::from_painter_name("space"), Some(ChromaId::Now));
        assert_eq!(ChromaId::from_painter_name("ad"), Some(ChromaId::Voice));
        assert_eq!(ChromaId::from_painter_name("ava"), Some(ChromaId::Name));
        assert_eq!(ChromaId::from_painter_name("brain"), Some(ChromaId::Memory));
        assert_eq!(ChromaId::from_painter_name("face"), None);
        for c in ChromaId::ALL {
            assert_eq!(ChromaId::from_painter_name(c.organ()), Some(c));
        }
    }
}
