//! Scroll position that survives leaving a world and coming back.
//!
//! Worlds hide with Visibility; some still rebuild their tree. This is
//! the memory either way: every PersistScroll node writes its y here,
//! and a freshly spawned one reads it back.

use std::collections::HashMap;

use bevy::prelude::*;

use super::WorldState;
use crate::now::{Now, NowKind};

#[derive(Resource, Default)]
pub struct SavedScroll(HashMap<String, f32>);

#[derive(Component, Clone)]
pub struct PersistScroll(pub &'static str);

pub struct ScrollPlugin;

impl Plugin for ScrollPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SavedScroll>()
            .add_systems(Update, (save, restore));
    }
}

pub fn key_for(world: WorldState, now: Option<&Now>) -> &'static str {
    if let Some(n) = now {
        match n.kind {
            NowKind::Meta => return "particle",
            NowKind::File => return "file",
            NowKind::World => {}
        }
    }
    match world {
        WorldState::Body => "body",
        WorldState::Graph => "brain",
        WorldState::Com => "log",
        WorldState::Robot => "robot",
        WorldState::Sigma => "sigma",
        WorldState::Models => "models",
        WorldState::Vault => "vault",
        WorldState::Memory => "memory",
        WorldState::Oracle => "oracle",
    }
}

fn save(mut saved: ResMut<SavedScroll>, q: Query<(&PersistScroll, &ScrollPosition)>) {
    for (k, pos) in &q {
        saved.0.insert(k.0.to_string(), pos.y);
    }
}

fn restore(
    saved: Res<SavedScroll>,
    mut q: Query<(&PersistScroll, &mut ScrollPosition), Added<PersistScroll>>,
) {
    for (k, mut pos) in &mut q {
        if let Some(&y) = saved.0.get(k.0) {
            pos.y = y;
        }
    }
}
