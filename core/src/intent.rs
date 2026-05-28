//! Well-known intent particles.
//!
//! An intent is the *token* field of a cyberlink — the kind of message
//! moving from one chroma to another. Intents are particles, not enum
//! variants: minting a new intent does not require recompiling cyb-core.
//!
//! These constants cover the canonical chroma-to-chroma intents named
//! in `cyb/root/chroma.md`. Custom intents can be minted at runtime via
//! [`intent_particle`].

use cybergraph::Particle;
use crate::chroma::label_particle;

pub const SUBMIT:          Particle = label_particle(b"intent/submit");
pub const SWITCH_RENDERER: Particle = label_particle(b"intent/switch-renderer");
pub const OUTPUT:          Particle = label_particle(b"intent/output");
pub const HINT:            Particle = label_particle(b"intent/hint");
pub const NOTIFY:          Particle = label_particle(b"intent/notify");
pub const IDENTITY:        Particle = label_particle(b"intent/identity");
pub const RESOURCE:        Particle = label_particle(b"intent/resource");
pub const LOCATE:          Particle = label_particle(b"intent/locate");
pub const MAP:             Particle = label_particle(b"intent/map");
pub const RECORD:          Particle = label_particle(b"intent/record");

/// Mint a particle for a custom intent. Pass `"submit"`, `"my-app/foo"`
/// — the result is the well-known particle for that name.
pub const fn intent_particle(name: &[u8]) -> Particle {
    // Reserve a small prefix so custom intents do not collide with the
    // built-in `intent/...` namespace by accident.
    let mut out = [0u8; 32];
    let prefix = b"intent/";
    let mut i = 0;
    while i < prefix.len() && i < 32 {
        out[i] = prefix[i];
        i += 1;
    }
    let mut j = 0;
    while j < name.len() && i < 32 {
        out[i] = name[j];
        i += 1;
        j += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_intents_are_unique() {
        let intents = [SUBMIT, SWITCH_RENDERER, OUTPUT, HINT, NOTIFY,
                       IDENTITY, RESOURCE, LOCATE, MAP, RECORD];
        let mut seen = std::collections::HashSet::new();
        for i in intents {
            assert!(seen.insert(i), "duplicate intent particle");
        }
    }

    #[test]
    fn custom_intent_matches_builtin_naming() {
        assert_eq!(intent_particle(b"submit"), SUBMIT);
        assert_eq!(intent_particle(b"hint"), HINT);
    }
}
