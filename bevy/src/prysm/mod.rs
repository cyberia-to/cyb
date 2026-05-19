pub mod theme;
pub mod atoms;
pub mod molecules;

pub use theme::{
    G, ACID_BLUE, ACID_GREEN, ACID_RED, ACID_ORANGE, ACID_YELLOW, ACID_INDIGO, ACID_VIOLET,
    SUBTLE, BACKGROUND, MIDGROUND, FOREGROUND,
    H1, H2, H3, BODY, CAPTION, MICRO,
    DARK_BASE, TEXT_PRIMARY, TEXT_DIM,
};
pub use atoms::{GlassDepth, Glass, Saber, glass_bg, saber_h};
pub use molecules::{TabItem, Commander, ActiveTab, ButtonPrysm, TextInput, CursorBlink,
    spawn_commander, spawn_button, spawn_input, text_input_system};

use bevy::prelude::*;

pub struct PrysmPlugin;

impl Plugin for PrysmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveTab>()
            .add_systems(Update, molecules::text_input_system);
    }
}
