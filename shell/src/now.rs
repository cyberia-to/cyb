//! Now — the particle the robot stands on.
//!
//! Not a world. Not a tab. The cursor. Space chroma will wear this;
//! spacetime shows particle meta or a file when we stand.

use bevy::prelude::*;
use file::{File, Particle};

use crate::worlds::content;

/// The particle underfoot, and whether spacetime is showing its meta,
/// its file, or the ordinary world.
#[derive(Resource, Debug, Default)]
pub struct Now {
    pub hash: Option<Particle>,
    pub idx: Option<usize>,
    pub kind: NowKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NowKind {
    #[default]
    World,
    Meta,
    File,
}

impl Now {
    /// Stand on this particle's meta page. Never dump the file onto the
    /// UI thread — a text spark of a lived-in particle is megabytes of
    /// Bevy `Text` and hangs the process. The file page is a later door.
    pub fn stand(&mut self, hash: [u8; 32], idx: Option<usize>) {
        self.hash = Some(Particle::from_bytes(hash));
        self.idx = idx;
        self.kind = NowKind::Meta;
    }

    pub fn dismiss(&mut self) {
        self.kind = NowKind::World;
        self.hash = None;
        self.idx = None;
    }
}

pub fn load_file(hash: [u8; 32]) -> Option<File> {
    let text = content::lookup(&hash)?;
    Some(File::bind(Particle::from_bytes(hash), text.into_bytes()))
}

pub struct NowPlugin;

impl Plugin for NowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Now>()
            .add_plugins(crate::worlds::particle_page::ParticlePagePlugin)
            .add_plugins(crate::worlds::file::FilePagePlugin)
            .add_systems(Update, dismiss_overlay);
    }
}

/// A tap that did not hit a button, while standing on a particle, sits down.
fn dismiss_overlay(
    mut now: ResMut<Now>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    buttons: Query<&Interaction, With<Button>>,
    mut press: Local<Option<Vec2>>,
) {
    if now.kind == NowKind::World {
        *press = None;
        return;
    }
    let pos = windows.single().ok().and_then(|w| w.cursor_position());
    if mouse.just_pressed(MouseButton::Left) {
        *press = pos;
    }
    for t in touches.iter_just_pressed() {
        *press = Some(t.position());
    }
    let released =
        mouse.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if !released {
        return;
    }
    let Some(start) = press.take() else {
        return;
    };
    let end = pos
        .or_else(|| touches.iter_just_released().next().map(|t| t.position()))
        .unwrap_or(start);
    if (end - start).length() > 22.0 {
        return;
    }
    if buttons.iter().any(|i| *i == Interaction::Pressed) {
        return;
    }
    now.dismiss();
}
