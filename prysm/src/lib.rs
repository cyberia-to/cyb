//! prysm — cyb's visual composition layer.
//!
//! Atoms (glass, sabers), molecules (commander, button, input), and a
//! shared theme. Renderer-agnostic in spirit; the Bevy plugin here is
//! one concrete binding. New renderers can consume the same component
//! definitions without going through `PrysmPlugin`.
//!
//! Depends on [`cyb_core`] for chroma identities and the signal bus.

pub mod theme;
pub mod atoms;
pub mod molecules;
pub mod stream;

pub use cyb_core as core;
pub use cyb_stream;

pub use theme::{
    G, ACID_BLUE, ACID_GREEN, ACID_RED, ACID_ORANGE, ACID_YELLOW, ACID_INDIGO, ACID_VIOLET,
    SUBTLE, BACKGROUND, MIDGROUND, FOREGROUND,
    H1, H2, H3, BODY, CAPTION, MICRO,
    DARK_BASE, TEXT_PRIMARY, TEXT_DIM,
};
pub use atoms::{GlassDepth, Glass, Saber, glass_bg, saber_h};
pub use molecules::{
    TabItem, Commander, ActiveTab, ButtonPrysm, TextInput, CursorBlink,
    spawn_commander, spawn_button, spawn_input,
    text_input_system, input_focus_system,
};
pub use stream::{
    Molecule, MoleculeRegistry, StreamChannel, StreamScrollback,
    StreamPlugin, register_v0_molecules,
};

use bevy::prelude::*;

pub struct PrysmPlugin;

impl Plugin for PrysmPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(StreamPlugin)
            .init_resource::<ActiveTab>()
            .add_systems(Update, (
                molecules::text_input_system,
                molecules::input_focus_system,
            ));
    }
}
