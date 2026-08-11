pub mod cell;
pub mod graph;
pub mod sigma;
pub mod terminal;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldState {
    #[default]
    Graph,
    Terminal,
    /// Live-loaded prysm cell (the robot landing and other app pages).
    Cell,
    /// Money: balance, send, events, sense (MoneyWallet).
    Sigma,
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
