//! cyb-stream consumer for prysm — renders typed chunks as native Bevy widgets.
//!
//! Add `StreamPlugin` to your Bevy app. Create a `StreamChannel` and inject it as a resource.
//! Feed chunks via the sender; the `consume_chunks` system drains them each frame into the
//! active `StreamScrollback` entity.

pub mod text;
pub mod error;
pub mod log;
pub mod status;
pub mod progress;
pub mod action;
pub mod component;
pub mod table;

use std::collections::HashMap;
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender, unbounded};
use cyb_stream::{Chunk, ChunkId};

// ── Molecule trait ────────────────────────────────────────────────────────────

/// A prysm molecule handles one `(sigil, render)` chunk type.
///
/// Spawn creates the Bevy widget hierarchy under `parent`.
/// Update refreshes an existing entity in place (default: no-op; molecule may respawn itself).
pub trait Molecule: Send + Sync {
    fn sigil(&self) -> u8;
    fn render(&self) -> u8;

    /// Spawn a new widget for `chunk` as a child of `parent`. Returns the root entity.
    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity;

    /// Update an existing entity from a new chunk (for live-updating chunks like progress).
    /// Default: do nothing (the old widget remains until it's coalesced by the scrollback).
    fn update(&self, _commands: &mut Commands, _entity: Entity, _chunk: &Chunk) {}
}

// ── MoleculeRegistry ─────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct MoleculeRegistry {
    map: HashMap<(u8, u8), Box<dyn Molecule>>,
}

impl MoleculeRegistry {
    pub fn register(&mut self, molecule: impl Molecule + 'static) {
        self.map.insert((molecule.sigil(), molecule.render()), Box::new(molecule));
    }

    pub fn get(&self, sigil: u8, render: u8) -> Option<&dyn Molecule> {
        self.map.get(&(sigil, render)).map(|b| b.as_ref())
    }
}

// ── StreamChannel ─────────────────────────────────────────────────────────────

/// Inject this as a resource so the terminal world can push chunks.
#[derive(Resource, Clone)]
pub struct StreamChannel {
    pub tx: Sender<Chunk>,
    pub rx: Receiver<Chunk>,
}

impl Default for StreamChannel {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self { tx, rx }
    }
}

// ── StreamScrollback ──────────────────────────────────────────────────────────

/// Vertical scrollable list of rendered chunks. One per terminal session.
#[derive(Component, Default)]
pub struct StreamScrollback {
    /// Appended in order; (ChunkId, Entity) for chunks with ids (for in-place update).
    pub id_map: HashMap<ChunkId, Entity>,
    /// Last exit code seen on a (., x) status chunk.
    pub last_status: Option<i32>,
}

// ── FallbackMolecule ──────────────────────────────────────────────────────────

/// Used when no registered molecule handles (sigil, render).
pub struct FallbackMolecule {
    pub sigil: u8,
    pub render: u8,
}

impl Molecule for FallbackMolecule {
    fn sigil(&self) -> u8 { self.sigil }
    fn render(&self) -> u8 { self.render }
    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let label = format!(
            "[?{} {}] {}",
            self.sigil as char,
            self.render as char,
            String::from_utf8_lossy(&chunk.payload).chars().take(60).collect::<String>()
        );
        let e = commands.spawn((
            Text::new(label),
            TextFont { font_size: crate::theme::MICRO, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.5, 1.0)),
            ChildOf(parent),
        )).id();
        e
    }
}

// ── consume_chunks system ─────────────────────────────────────────────────────

pub fn consume_chunks(
    channel:  Res<StreamChannel>,
    registry: Res<MoleculeRegistry>,
    mut scrollback_q: Query<(Entity, &mut StreamScrollback)>,
    mut commands: Commands,
) {
    let Ok((scrollback_entity, mut scrollback)) = scrollback_q.single_mut() else {
        return;
    };

    for chunk in channel.rx.try_iter() {
        // Status chunk: record exit code, don't render anything visible
        if chunk.sigil == cyb_stream::sigil::DOT && chunk.render == cyb_stream::render::STATUS {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&chunk.payload) {
                scrollback.last_status = v["code"].as_i64().map(|c| c as i32);
            }
            continue;
        }

        // If chunk has an id and we've seen it, update in place
        if let Some(id) = chunk.id {
            if let Some(&existing) = scrollback.id_map.get(&id) {
                if let Some(mol) = registry.get(chunk.sigil, chunk.render) {
                    mol.update(&mut commands, existing, &chunk);
                }
                continue;
            }
        }

        // Spawn new molecule
        let entity = if let Some(mol) = registry.get(chunk.sigil, chunk.render) {
            mol.spawn(&mut commands, scrollback_entity, &chunk)
        } else {
            FallbackMolecule { sigil: chunk.sigil, render: chunk.render }
                .spawn(&mut commands, scrollback_entity, &chunk)
        };

        if let Some(id) = chunk.id {
            scrollback.id_map.insert(id, entity);
        }
    }
}

// ── StreamPlugin ──────────────────────────────────────────────────────────────

pub struct StreamPlugin;

impl Plugin for StreamPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoleculeRegistry>()
            .init_resource::<StreamChannel>()
            .add_systems(Update, consume_chunks);
    }
}

/// Register all built-in v0 molecules.
pub fn register_v0_molecules(registry: &mut MoleculeRegistry) {
    registry.register(text::TextMolecule);
    registry.register(text::AnnotationMolecule);
    registry.register(text::NeuronMolecule);
    registry.register(error::ErrorMolecule);
    registry.register(log::LogMolecule);
    registry.register(progress::ProgressMolecule);
    registry.register(action::ActionMolecule);
    registry.register(component::ComponentMolecule);
    registry.register(component::ScopeMolecule);
    registry.register(table::TableMolecule);
}
