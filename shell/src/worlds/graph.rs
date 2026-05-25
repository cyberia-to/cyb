use std::sync::Arc;
use bevy::prelude::*;

use mir::graph::{Csr, ParticleIndex, Cyberlink};
use mir::bevy::resources::GraphWorldConfig;
use mir::bevy::world::GraphWorldState;

use super::WorldState;
use crate::prysm::molecules::spawn_commander;

#[derive(Component)]
struct GraphUIMarker;

pub struct GraphBridgePlugin;

impl Plugin for GraphBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, insert_graph_config)
            .add_systems(OnEnter(WorldState::Graph), show_graph_ui)
            .add_systems(OnExit(WorldState::Graph),  hide_graph_ui)
            .add_systems(Update, sync_graph_state);
    }
}

fn show_graph_ui(mut commands: Commands) {
    commands
        .spawn((
            GraphUIMarker,
            Node {
                width:            Val::Percent(100.0),
                height:           Val::Percent(100.0),
                flex_direction:   FlexDirection::Column,
                justify_content:  JustifyContent::FlexEnd,
                position_type:    PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|root| {
            spawn_commander(root, 1, &["spells", "graph", "sense"]);
        });
}

fn hide_graph_ui(mut commands: Commands, q: Query<Entity, With<GraphUIMarker>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn insert_graph_config(mut commands: Commands) {
    commands.insert_resource(GraphWorldConfig {
        graph: Arc::new(build_synthetic_csr()),
    });
}

fn sync_graph_state(
    world_state:     Res<State<WorldState>>,
    graph_state_cur: Res<State<GraphWorldState>>,
    mut graph_state: ResMut<NextState<GraphWorldState>>,
) {
    let target = if *world_state.get() == WorldState::Graph {
        GraphWorldState::Active
    } else {
        GraphWorldState::Inactive
    };
    if *graph_state_cur.get() != target {
        graph_state.set(target);
    }
}

pub fn build_synthetic_csr() -> Csr {
    fn hash(i: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[31] = i;
        h
    }

    fn link(from: u8, to: u8, amount: u128) -> Cyberlink {
        Cyberlink {
            neuron:  [0u8; 32],
            from:    hash(from),
            to:      hash(to),
            token:   0,
            amount,
            valence: 1,
            block:   1,
        }
    }

    let edges: Vec<Cyberlink> = vec![
        link(0,  2,  100), link(0,  3,  100), link(0,  4,  100),
        link(0,  5,   80), link(0,  14,  80),
        link(3,  4,   90), link(3,  15,  70), link(4,  2,   80),
        link(5,  14,  90), link(5,  6,   80),
        link(6,  7,   90), link(6,  8,   80), link(6,  5,   70),
        link(7,  11,  80), link(7,  12,  80),
        link(8,  9,   90), link(9,  6,   70), link(9,  8,   70),
        link(10, 4,   80), link(11, 12,  70),
        link(12, 7,   80), link(13, 2,   90),
        link(1,  16,  80), link(1,  17,  70), link(1,  8,   70),
        link(16, 1,   60), link(17, 7,   70), link(18, 11,  60),
    ];

    let vocab = ParticleIndex::build(edges.iter().copied());
    Csr::build(edges.into_iter(), &vocab)
}
