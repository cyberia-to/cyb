pub mod robot;
pub mod graph;
pub mod sigma;
pub mod com;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldState {
    /// The cybergraph itself, rendered by mir.
    #[default]
    Graph,
    /// The commander's own world: nushell, rune, the prompt.
    Com,
    /// The robot: live-loaded prysm cells (its landing and other pages).
    Robot,
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
