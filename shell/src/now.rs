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
    /// Previous stands. Back walks this; it never dumps you into brain
    /// just because you tapped another particle.
    stack: Vec<(Particle, Option<usize>, NowKind)>,
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
        let p = Particle::from_bytes(hash);
        if self.kind != NowKind::World {
            if let Some(cur) = self.hash {
                if cur != p {
                    self.stack.push((cur, self.idx, self.kind));
                }
            }
        }
        self.hash = Some(p);
        self.idx = idx;
        self.kind = NowKind::Meta;
    }

    pub fn can_back(&self) -> bool {
        self.kind != NowKind::World || !self.stack.is_empty()
    }

    /// Pop the last stand, or sit down if the stack is empty.
    pub fn back(&mut self) {
        if let Some((h, i, k)) = self.stack.pop() {
            self.hash = Some(h);
            self.idx = i;
            self.kind = k;
        } else {
            self.dismiss();
        }
    }

    pub fn dismiss(&mut self) {
        self.kind = NowKind::World;
        self.hash = None;
        self.idx = None;
        self.stack.clear();
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
            .add_plugins(crate::worlds::file::FilePagePlugin);
    }
}
