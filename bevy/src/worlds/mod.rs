pub mod interface;
pub mod legacy;
pub mod portal;
pub mod splash;
pub mod terminal;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldState {
    #[default]
    Splash,    // Loading screen → auto-transitions to Legacy
    Terminal,  // Cmd+1 (nushell)
    Portal,    // Cmd+2 (Leptos WASM)
    Legacy,    // Cmd+3 (cyb-ts React)
    Interface, // Cmd+4 (Bevy 3D/2D)
}

pub struct WorldsPlugin;

impl Plugin for WorldsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<WorldState>();
    }
}
