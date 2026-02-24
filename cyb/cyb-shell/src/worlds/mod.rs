pub mod browser;
pub mod game;
pub mod terminal;
pub mod ui;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldState {
    Ui,       // Cmd+1
    Game,     // Cmd+2
    Terminal, // Cmd+3
    #[default]
    Browser,  // Cmd+4 (default: loads cyb-ts)
}

pub struct WorldsPlugin;

impl Plugin for WorldsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<WorldState>();
    }
}
