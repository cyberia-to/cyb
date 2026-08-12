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
//!
//! Depends on `cybergraph` (which transitively brings in `bbg`,
//! `hemera`, `zheng`, `nebu`). Does **not** depend on bevy, wgpu, or
//! any rendering layer — those plug in above this crate.

pub mod cell;
pub mod chroma;
pub mod intent;
pub mod money;
pub mod sense;
pub mod signal;

pub use cell::Cell;
pub use chroma::{ChromaId, GridPos, chroma_particle};
pub use intent::{
    HINT, IDENTITY, LOCATE, MAP, NOTIFY, OUTPUT, RECORD, RESOURCE, SUBMIT, SWITCH_RENDERER,
    intent_particle,
};
pub use money::{ClockKind, Grade, MoneyError, MoneyEvent, MoneyWallet, PayLeg, PrivateNote};
pub use sense::{SenseNotify, money_to_sense};
pub use signal::{SignalBuilder, SignalBus, link};

pub use cybergraph::{CyberlinkRecord, Signal};
pub use foculus::{
    BoxMoveRecord, FinalityEvidence, PayStatement, RewardClaim, SettleReceipt, Tip, TipProver,
    TipTrust, prove_pay, verify_pay,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
