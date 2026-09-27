//! Where you were, so back can take you there.
//!
//! Every world switch, particle stand, and spell page pushes a Place.
//! Back pops. The stack is the only history; nothing is inferred.

use bevy::prelude::*;
use file::Particle;

use super::WorldState;
use crate::now::{Now, NowKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub world: WorldState,
    pub kind: NowKind,
    pub hash: Option<[u8; 32]>,
    pub idx: Option<usize>,
    pub spell: bool,
}

impl Place {
    pub fn capture(world: WorldState, now: &Now, spell: bool) -> Self {
        Self {
            world,
            kind: now.kind,
            hash: now.hash.map(|p| *p.as_bytes()),
            idx: now.idx,
            spell,
        }
    }
}

#[derive(Resource, Default)]
pub struct Nav {
    stack: Vec<Place>,
}

impl Nav {
    pub fn push(&mut self, place: Place) {
        if self.stack.last() == Some(&place) {
            return;
        }
        self.stack.push(place);
    }

    pub fn pop(&mut self) -> Option<Place> {
        self.stack.pop()
    }

    pub fn can_back(&self) -> bool {
        !self.stack.is_empty()
    }
}

pub struct NavPlugin;

impl Plugin for NavPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Nav>().add_systems(Update, apply_escape);
    }
}

fn apply_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut nav: ResMut<Nav>,
    mut now: ResMut<Now>,
    world: Res<State<WorldState>>,
    mut next: ResMut<NextState<WorldState>>,
    spells: Query<Entity, With<super::vault::SpellPage>>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::Escape) {
        go_back(&mut nav, &mut now, world.get(), &mut next, &spells, &mut commands);
    }
}

pub fn go_back(
    nav: &mut Nav,
    now: &mut Now,
    here: &WorldState,
    next: &mut NextState<WorldState>,
    spells: &Query<Entity, With<super::vault::SpellPage>>,
    commands: &mut Commands,
) {
    let Some(p) = nav.pop() else {
        now.dismiss();
        return;
    };
    now.kind = p.kind;
    now.hash = p.hash.map(Particle::from_bytes);
    now.idx = p.idx;
    if p.world != *here {
        next.set(p.world);
    }
    if !p.spell {
        for e in spells {
            commands.entity(e).despawn();
        }
    }
}

/// Record `here`, then stand on a particle.
pub fn stand_from(
    nav: &mut Nav,
    now: &mut Now,
    world: WorldState,
    hash: [u8; 32],
    idx: Option<usize>,
    spell: bool,
) {
    nav.push(Place::capture(world, now, spell));
    now.stand(hash, idx);
}
