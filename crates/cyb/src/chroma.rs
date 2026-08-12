//! The 3×3 chroma grid.
//!
//! Eight fixed overlay slots arranged around a pluggable centre
//! (spacetime). The layout is closed — adding a feature means a new
//! intent particle, never a new slot.

use cybergraph::Particle;

/// One of the nine chroma slot identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChromaId {
    Space,
    Ad,
    Ava,
    Sense,
    Spacetime,
    Sigma,
    Brain,
    Com,
    Time,
}

impl ChromaId {
    pub const ALL: [ChromaId; 9] = [
        ChromaId::Space, ChromaId::Ad, ChromaId::Ava,
        ChromaId::Sense, ChromaId::Spacetime, ChromaId::Sigma,
        ChromaId::Brain, ChromaId::Com, ChromaId::Time,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            ChromaId::Space     => "chroma/space",
            ChromaId::Ad        => "chroma/ad",
            ChromaId::Ava       => "chroma/ava",
            ChromaId::Sense     => "chroma/sense",
            ChromaId::Spacetime => "chroma/spacetime",
            ChromaId::Sigma     => "chroma/sigma",
            ChromaId::Brain     => "chroma/brain",
            ChromaId::Com       => "chroma/com",
            ChromaId::Time      => "chroma/time",
        }
    }

    pub const fn pos(self) -> GridPos {
        match self {
            ChromaId::Space     => GridPos { col: 0, row: 0 },
            ChromaId::Ad        => GridPos { col: 1, row: 0 },
            ChromaId::Ava       => GridPos { col: 2, row: 0 },
            ChromaId::Sense     => GridPos { col: 0, row: 1 },
            ChromaId::Spacetime => GridPos { col: 1, row: 1 },
            ChromaId::Sigma     => GridPos { col: 2, row: 1 },
            ChromaId::Brain     => GridPos { col: 0, row: 2 },
            ChromaId::Com       => GridPos { col: 1, row: 2 },
            ChromaId::Time      => GridPos { col: 2, row: 2 },
        }
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
}
