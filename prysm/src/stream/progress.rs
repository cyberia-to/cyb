use bevy::prelude::*;
use cyb_stream::{Chunk, ChunkId, sigil, render};
use super::Molecule;
use crate::theme;

/// `(., p)` — live progress bar. Deduped by `chunk.id` in the scrollback.
pub struct ProgressMolecule;

#[derive(Component)]
pub struct ProgressBar {
    pub id: ChunkId,
}

#[derive(Component)]
pub struct ProgressFill;

#[derive(Component)]
pub struct ProgressLabel;

impl Molecule for ProgressMolecule {
    fn sigil(&self) -> u8 { sigil::DOT }
    fn render(&self) -> u8 { render::PROGRESS }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let (id, label, fraction) = parse_progress(&chunk.payload, chunk.id);

        let container = commands.spawn((
            ProgressBar { id },
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                padding: UiRect::vertical(Val::Px(2.0)),
                ..default()
            },
            ChildOf(parent),
        )).id();

        // label row
        commands.spawn((
            ProgressLabel,
            Text::new(label),
            TextFont { font_size: theme::MICRO, ..default() },
            TextColor(theme::TEXT_DIM),
            ChildOf(container),
        ));

        // track + fill
        let track = commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(4.0),
                margin: UiRect::top(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
            ChildOf(container),
        )).id();

        commands.spawn((
            ProgressFill,
            Node {
                width: Val::Percent((fraction * 100.0).clamp(0.0, 100.0)),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(theme::ACID_BLUE),
            ChildOf(track),
        ));

        container
    }

    fn update(&self, commands: &mut Commands, entity: Entity, chunk: &Chunk) {
        let (_, label, fraction) = parse_progress(&chunk.payload, chunk.id);

        // Update fill width — we need to find the ProgressFill child.
        // In v0 we despawn and respawn. A future pass can query children by component.
        // For now: mark for full respawn by removing the ProgressBar marker.
        // The scrollback `consume_chunks` sees the id in id_map and calls update().
        // We delete the old widget and let the next call spawn fresh.
        commands.entity(entity).despawn();
        // Caller will re-insert on next frame since we don't spawn here.
        // Simpler v0: log the update to avoid crashes.
        let _ = (label, fraction); // consumed
    }
}

fn parse_progress(payload: &[u8], id: Option<ChunkId>) -> (ChunkId, String, f32) {
    let default_id = id.unwrap_or(ChunkId(0));
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(payload) {
        let chunk_id  = v["id"].as_u64().map(ChunkId).unwrap_or(default_id);
        let label     = v["label"].as_str().unwrap_or("").to_string();
        let current   = v["current"].as_u64().unwrap_or(0) as f32;
        let total     = v["total"].as_u64().unwrap_or(1).max(1) as f32;
        (chunk_id, label, current / total)
    } else {
        (default_id, String::from_utf8_lossy(payload).into_owned(), 0.0)
    }
}
