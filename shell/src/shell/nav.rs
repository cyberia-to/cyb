//! Tab → world routing.
//!
//! Lives in shell because it knows about `WorldState` — prysm itself
//! is renderer-agnostic and must not couple to specific worlds.
//! Under the chroma architecture this will become a
//! `(com, spacetime, switch-renderer, …)` cyberlink emitted by com.

use bevy::prelude::*;
use prysm::TabItem;
use crate::worlds::WorldState;

pub fn tab_navigation_system(
    tab_q: Query<(&Interaction, &TabItem), Changed<Interaction>>,
    mut next_state: ResMut<NextState<WorldState>>,
) {
    for (interaction, tab) in &tab_q {
        if *interaction == Interaction::Pressed {
            let target = match tab.index {
                0 => WorldState::Graph,
                1 => WorldState::Terminal,
                _ => continue,
            };
            next_state.set(target);
        }
    }
}

pub struct NavPlugin;

impl Plugin for NavPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, tab_navigation_system);
    }
}
