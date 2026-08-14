//! cyb — the minimal runtime core for a cyb instance.
//!
//! Contains only what every cyb deployment needs regardless of renderer,
//! hardware, or shell:
//!
//! - the [`ChromaId`] enum and the 3×3 grid layout
//! - well-known particle identities for every chroma and intent
//! - the [`SignalBus`] queue plus helpers to assemble Signals from
//!   individual cyberlinks
//! - money loop (balance / send / receive / settle / pay proofs)
//! - default network endpoints ([`network`]) — **spacepussy-test** after install
//!
//! Depends on `cybergraph` (which transitively brings in `bbg`,
//! `hemera`, `zheng`, `nebu`). Does **not** depend on bevy, wgpu, or
//! any rendering layer — those plug in above this crate.

pub mod cell;
pub mod chroma;
pub mod intent;
pub mod money;
pub mod network;
pub mod sense;
pub mod signal;

pub use cell::Cell;
pub use chroma::{chroma_particle, ChromaId, GridPos};
pub use intent::{
    intent_particle, HINT, IDENTITY, LOCATE, MAP, NOTIFY, OUTPUT, RECORD, RESOURCE, SUBMIT,
    SWITCH_RENDERER,
};
pub use money::{ClockKind, Grade, MoneyError, MoneyEvent, MoneyWallet, PayLeg, PrivateNote};
pub use sense::{money_to_sense, SenseNotify};
pub use signal::{link, SignalBuilder, SignalBus};

pub use cybergraph::{CyberlinkRecord, Signal};
pub use foculus::{
    prove_pay, verify_pay, BoxMoveRecord, FinalityEvidence, PayStatement, RewardClaim,
    SettleReceipt, Tip, TipProver, TipTrust,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
