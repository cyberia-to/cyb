//! Scroll position that survives leaving a world and coming back.
//!
//! Worlds hide with Visibility; some still rebuild their tree. This is
//! the memory either way: every PersistScroll node writes its y here,
//! and a freshly spawned one reads it back.

use std::collections::HashMap;

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use super::WorldState;
use crate::now::{Now, NowKind};

#[derive(Resource, Default)]
struct Fling(f32);

#[derive(Resource, Default)]
pub struct SavedScroll(HashMap<String, f32>);

#[derive(Component, Clone)]
pub struct PersistScroll(pub &'static str);

pub struct ScrollPlugin;

impl Plugin for ScrollPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SavedScroll>()
            .init_resource::<Fling>()
            .add_systems(Update, (save, restore, drive));
    }
}

/// Wheel + one-finger drag, logical pixels — the same path memory uses.
/// Com's session scroller (`log-session`) is owned by process_scroll
/// (stick-to-bottom). The log *table* is PersistScroll("log") and must move.
fn drive(
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    time: Res<Time>,
    state: Res<State<WorldState>>,
    mut fling: ResMut<Fling>,
    mut q: Query<(
        &PersistScroll,
        &mut ScrollPosition,
        &ComputedNode,
        &InheritedVisibility,
    )>,
) {
    let skip_session = *state.get() == WorldState::Com;
    let dy: f32 = wheel
        .read()
        .map(|e| match e.unit {
            MouseScrollUnit::Line => -e.y * 48.0,
            MouseScrollUnit::Pixel => -e.y,
        })
        .sum();
    let live: Vec<&bevy::input::touch::Touch> = touches.iter().collect();
    let drag = if live.len() == 1 {
        -live[0].delta().y
    } else {
        0.0
    };
    let dt = time.delta_secs().max(1e-4);
    let mut delta = dy + drag;
    if delta != 0.0 {
        let inst = delta / dt;
        fling.0 = (fling.0 * 0.35 + inst * 0.65).clamp(-12_000.0, 12_000.0);
    } else if fling.0.abs() > 80.0 {
        fling.0 *= (-5.5 * dt).exp();
        delta = fling.0 * dt;
    } else {
        fling.0 = 0.0;
    }
    if delta == 0.0 {
        return;
    }
    for (persist, mut pos, cn, vis) in &mut q {
        if skip_session && persist.0 == "log-session" {
            continue;
        }
        if !vis.get() {
            continue;
        }
        let s = cn.inverse_scale_factor();
        let max = (cn.content_size().y * s - cn.size().y * s).max(0.0);
        pos.y = (pos.y + delta).clamp(0.0, max);
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
        WorldState::Chat => "chat",
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
