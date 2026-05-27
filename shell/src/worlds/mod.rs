pub mod graph;
pub mod terminal;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldState {
    #[default]
    Graph,
    Terminal,
}

/// Shell command forwarded from the commander bar to nushell.
#[derive(Resource, Default)]
pub struct PendingShellCmd(pub Option<String>);

pub struct WorldsPlugin;

impl Plugin for WorldsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<WorldState>()
            .init_resource::<PendingShellCmd>();
    }
}
