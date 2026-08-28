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

/// Who said a line that lands in com's scrollback.
///
/// com is where cyb keeps its record of what happened, so the other worlds
/// talk into it rather than keeping private logs that nobody scrolls back
/// through. Which side a line sits on is the whole distinction: what you asked
/// for on the left, what the machine answered on the right.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Speaker {
    /// An intent — a button pressed, a transfer asked for.
    User,
    /// What came back: an event, a balance, a receipt.
    System,
}

/// Lines waiting to be written into com's scrollback from another world.
///
/// A queue rather than a single slot: one press of `send` produces an intent
/// and then several events, and they must all arrive, in order.
#[derive(Resource, Default)]
pub struct ComInbox(pub Vec<(Speaker, String)>);

impl ComInbox {
    pub fn say(&mut self, who: Speaker, line: impl Into<String>) {
        self.0.push((who, line.into()));
    }
}

pub struct WorldsPlugin;

impl Plugin for WorldsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<WorldState>()
            .init_resource::<PendingShellCmd>()
            .init_resource::<ComInbox>();
    }
}
