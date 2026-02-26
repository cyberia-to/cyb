pub mod browser;
pub mod game;
pub mod terminal;
pub mod ui;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldState {
    #[default]
    Terminal, // Cmd+1 (default)
    Browser,  // Cmd+2 (cyb)
    Ui,       // Cmd+3
    Game,     // Cmd+4
}

pub struct WorldsPlugin;

impl Plugin for WorldsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<WorldState>();
    }
}
