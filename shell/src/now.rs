//! Now — the particle the robot stands on.
//!
//! Not a world. Not a tab. The cursor. Space chroma will wear this;
//! spacetime shows particle meta or a file when we stand.

use bevy::prelude::*;
use particle::{File, Particle};
use spark;

use crate::worlds::WorldState;
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
    /// Stand on this particle. Glide to file if a spark resolves.
    pub fn stand(&mut self, hash: [u8; 32], idx: Option<usize>) {
        let p = Particle::from_bytes(hash);
        self.hash = Some(p);
        self.idx = idx;
        self.kind = match load_file(hash) {
            Some(f) if spark::resolve(&f).is_some() => NowKind::File,
            Some(_) | None => NowKind::Meta,
        };
    }
}

pub fn load_file(hash: [u8; 32]) -> Option<File> {
    let text = content::load().remove(&hash)?;
    Some(File::bind(Particle::from_bytes(hash), text.into_bytes()))
}

pub struct NowPlugin;

impl Plugin for NowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Now>()
            .add_plugins(crate::worlds::particle_page::ParticlePagePlugin)
            .add_plugins(crate::worlds::file::FilePagePlugin);
    }
}
