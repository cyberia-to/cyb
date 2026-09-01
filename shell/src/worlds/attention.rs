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
//! The unit is seconds, floor one: a glance still happened. This is the
//! smallest version of "time on page as a link parameter" — worlds are the
//! only pages cyb has today; when robot grows real pages, the same cast
//! works per page.

use std::time::Instant;

use bevy::prelude::*;
use bevy::state::state::StateTransitionEvent;

use super::{content, identity::Identity, ComInbox, ComSay, SharedCell, WorldState};

pub struct AttentionPlugin;

/// Where the attention currently rests, and since when.
#[derive(Resource)]
struct Dwell {
    world: WorldState,
    since: Instant,
}

impl Plugin for AttentionPlugin {
    fn build(&self, app: &mut App) {
        let world = *app
            .world()
            .resource::<State<WorldState>>()
            .get();
        app.insert_resource(Dwell {
            world,
            since: Instant::now(),
        })
        .add_systems(Update, observe_transitions);

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
                        "brain" | "graph" => WorldState::Graph,
                        "log" | "com" => WorldState::Com,
                        "robot" => WorldState::Robot,
                        "sigma" => WorldState::Sigma,
                        "models" => WorldState::Models,
                        _ => return None,
                    };
                    Some((world, secs.parse().ok()?))
                })
                .collect();
            if !stops.is_empty() {
                app.insert_resource(Tour { stops, at: 0, wait: 0.0 });
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

fn run_tour(
    time: Res<Time>,
    mut tour: ResMut<Tour>,
    mut next: ResMut<NextState<WorldState>>,
) {
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
        WorldState::Graph => "brain",
        WorldState::Com => "log",
        WorldState::Robot => "robot",
        WorldState::Sigma => "sigma",
        WorldState::Models => "models",
    }
}

fn observe_transitions(
    mut transitions: MessageReader<StateTransitionEvent<WorldState>>,
    mut dwell: ResMut<Dwell>,
    shared: Res<SharedCell>,
    who: Res<Identity>,
    mut inbox: ResMut<ComInbox>,
) {
    for t in transitions.read() {
        let (Some(exited), Some(entered)) = (t.exited, t.entered) else {
            continue;
        };
        if exited == entered {
            continue;
        }
        // Seconds of attention the old world actually received.
        let secs = dwell.since.elapsed().as_secs().max(1);
        dwell.world = entered;
        dwell.since = Instant::now();

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
                inbox.0.push(ComSay::Note(format!("-> {to}  ({from} {secs}s)")));
            }
            Err(e) => warn!("attention: cast failed: {e:?}"),
        }
    }
}
