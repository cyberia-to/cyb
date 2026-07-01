//! cyb-core — the minimal runtime core for a cyb instance.
//!
//! Contains only what every cyb deployment needs regardless of renderer,
//! hardware, or shell:
//!
//! - the [`ChromaId`] enum and the 3×3 grid layout
//! - well-known particle identities for every chroma and intent
//! - the [`SignalBus`] queue plus helpers to assemble Signals from
//!   individual cyberlinks
//!
//! Depends on `cybergraph` (which transitively brings in `bbg`,
//! `hemera`, `zheng`, `nebu`). Does **not** depend on bevy, wgpu, or
//! any rendering layer — those plug in above this crate.

pub mod cell;
pub mod chroma;
pub mod intent;
pub mod signal;

pub use cell::Cell;
pub use chroma::{ChromaId, GridPos, chroma_particle};
pub use intent::{
    intent_particle,
    SUBMIT, SWITCH_RENDERER, OUTPUT, HINT, NOTIFY,
    IDENTITY, RESOURCE, LOCATE, MAP, RECORD,
};
pub use signal::{SignalBuilder, SignalBus, link};

pub use cybergraph::{CyberlinkRecord, Signal};
