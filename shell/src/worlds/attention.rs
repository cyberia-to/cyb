//! Attention is a link: where you go, and how long you stayed.
//!
//! Every world switch casts one weighted cyberlink,
//! `particle(world you left) → particle(world you entered)`, with the
//! seconds you dwelt in the old world as the link's amount. That one rule
//! does three jobs at once:
//!
//! - the log stops being blind to navigation — every transition is a line
//!   in the record, because it is a signal on the chain like everything else;
//! - the graph seeds itself from use. A fresh cyb needs no synthetic demo
//!   constellation: by the time you first open brain, your own movement has
//!   already drawn something true;
//! - the weights are the raw material for focus. mir's layout already pulls
//!   proportionally to link amount, so the worlds you live in drift together
//!   on screen today — and tru's ranking has honest attention data to chew
//!   on the day it lands.
//!
//! Seconds count only while the window is visible and focused. Closing to
//! the tray, or another app in front, pauses the clock — overnight in the
//! tray must not become 28 000 seconds of "attention".

use std::time::Instant;

use bevy::prelude::*;
use bevy::state::state::StateTransitionEvent;
use bevy::window::PrimaryWindow;

use super::{ComInbox, ComSay, SharedCell, WorldState, content, identity::Identity};

pub struct AttentionPlugin;

/// Where attention rests, and how much visible focused time it has accrued.
#[derive(Resource)]
struct Dwell {
    world: WorldState,
    /// Whole seconds already banked while the window was live.
    accrued: u64,
    /// `Some` only while the window is visible and focused.
    running_since: Option<Instant>,
}

impl Plugin for AttentionPlugin {
    fn build(&self, app: &mut App) {
        let world = *app.world().resource::<State<WorldState>>().get();
        app.insert_resource(Dwell {
            world,
            accrued: 0,
            running_since: Some(Instant::now()),
        })
        .add_systems(Update, (gate_dwell, observe_transitions).chain());

        // `CYB_TOUR="log:3,brain:5,sigma:2,log:4"` walks the worlds on a
        // timer — the scripted stand-in for a hand on the tabs. It exists to
        // prove attention casting end to end (each hop should log a note and
        // weight a link), and it doubles as a demo: run it once and brain
        // shows your itinerary as a graph.
        if let Ok(tour) = std::env::var("CYB_TOUR") {
            let stops: Vec<(WorldState, f32)> = tour
                .split(',')
                .filter_map(|s| {
                    let (name, secs) = s.trim().split_once(':')?;
                    let world = match name {
                        "body" => WorldState::Body,
                        "brain" | "graph" => WorldState::Graph,
                        "log" | "com" => WorldState::Com,
                        "chat" => WorldState::Chat,
                        "robot" => WorldState::Robot,
                        "sigma" => WorldState::Sigma,
                        "models" => WorldState::Models,
                        "vault" => WorldState::Vault,
                        _ => return None,
                    };
                    Some((world, secs.parse().ok()?))
                })
                .collect();
            if !stops.is_empty() {
                app.insert_resource(Tour {
                    stops,
                    at: 0,
                    wait: 0.0,
                });
                app.add_systems(Update, run_tour);
            }
        }
    }
}

#[derive(Resource)]
struct Tour {
    stops: Vec<(WorldState, f32)>,
    at: usize,
    wait: f32,
}

fn run_tour(time: Res<Time>, mut tour: ResMut<Tour>, mut next: ResMut<NextState<WorldState>>) {
    if tour.at >= tour.stops.len() {
        return;
    }
    tour.wait += time.delta_secs();
    let (world, hold) = tour.stops[tour.at];
    if tour.wait >= hold {
        tour.wait = 0.0;
        tour.at += 1;
        next.set(world);
    }
}

/// The display name a world's particle is minted under. Stable and
/// human-readable on purpose: these particles are labels in brain, and the
/// same name from two cybs is the same particle — attention is comparable.
pub fn world_name(w: WorldState) -> &'static str {
    match w {
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

fn window_live(window: &Window) -> bool {
    window.visible && window.focused
}

/// Pause the clock when the window is gone or not in front. Resume on show.
fn gate_dwell(window: Query<&Window, With<PrimaryWindow>>, mut dwell: ResMut<Dwell>) {
    let Ok(window) = window.single() else {
        return;
    };
    let live = window_live(window);
    match (live, dwell.running_since) {
        (false, Some(since)) => {
            dwell.accrued = dwell.accrued.saturating_add(since.elapsed().as_secs());
            dwell.running_since = None;
        }
        (true, None) => {
            dwell.running_since = Some(Instant::now());
        }
        _ => {}
    }
}

fn observe_transitions(
    mut transitions: MessageReader<StateTransitionEvent<WorldState>>,
    mut dwell: ResMut<Dwell>,
    shared: Res<SharedCell>,
    who: Res<Identity>,
    mut inbox: ResMut<ComInbox>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let live = window.single().ok().is_some_and(window_live);
    for t in transitions.read() {
        let (Some(exited), Some(entered)) = (t.exited, t.entered) else {
            continue;
        };
        if exited == entered {
            continue;
        }
        let mut secs = dwell.accrued;
        if let Some(since) = dwell.running_since {
            secs = secs.saturating_add(since.elapsed().as_secs());
        }
        dwell.world = entered;
        dwell.accrued = 0;
        dwell.running_since = if live { Some(Instant::now()) } else { None };
        // A glance while looking still counts as one second. A hop while
        // the window was closed casts nothing.
        if secs == 0 {
            if !live {
                continue;
            }
            secs = 1;
        }

        let from = world_name(exited);
        let to = world_name(entered);
        content::remember(from);
        content::remember(to);

        let cast = {
            let mut cell = shared.cell.lock().expect("shared cell poisoned");
            cell.cast_weighted(
                who.neuron,
                [(content::particle_of(from), content::particle_of(to), secs)],
            )
        };
        match cast {
            Ok(_) => {
                shared.bump();
                inbox
                    .0
                    .push(ComSay::Note(format!("-> {to}  ({from} {secs}s)")));
            }
            Err(e) => warn!("attention: cast failed: {e:?}"),
        }
    }
}
