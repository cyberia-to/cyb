pub mod graph;
pub mod interface;
pub mod legacy;
pub mod portal;
pub mod sense;
pub mod spells;
pub mod splash;
pub mod terminal;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldState {
    #[default]
    Splash,    // Loading screen → auto-transitions to Spells
    Spells,    // Cmd+1 (neuron identity / mnemonic)
    Graph,     // Cmd+2 (mir graph world — R-1.0)
    Sense,     // Cmd+3 (local inference chat)
    Terminal,  // Cmd+4 (nushell)
    Portal,    // Cmd+5 (Leptos WASM)
    Legacy,    // Cmd+6 (cyb-ts React)
    Interface, // Cmd+7 (Bevy 3D/2D)
}

pub struct WorldsPlugin;

impl Plugin for WorldsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<WorldState>();
    }
}
