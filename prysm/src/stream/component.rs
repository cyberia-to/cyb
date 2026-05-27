use bevy::prelude::*;
use cyb_stream::{Chunk, sigil, render, decode_nested};
use super::Molecule;
use crate::theme;

/// `(|, c)` — composition container. Renders nested chunks in a column.
pub struct ComponentMolecule;

impl Molecule for ComponentMolecule {
    fn sigil(&self) -> u8 { sigil::BAR }
    fn render(&self) -> u8 { render::COMPONENT }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        spawn_container(commands, parent, chunk, false)
    }
}

/// `(/, c)` — scope / breadcrumb container. Same layout, slightly different chrome.
pub struct ScopeMolecule;

impl Molecule for ScopeMolecule {
    fn sigil(&self) -> u8 { sigil::FAS }
    fn render(&self) -> u8 { render::COMPONENT }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        spawn_container(commands, parent, chunk, true)
    }
}

fn spawn_container(commands: &mut Commands, parent: Entity, chunk: &Chunk, is_scope: bool) -> Entity {
    let container = commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(100.0),
            padding: if is_scope {
                UiRect::all(Val::Px(0.0))
            } else {
                UiRect::vertical(Val::Px(theme::G * 0.5))
            },
            ..default()
        },
        ChildOf(parent),
    )).id();

    // Parse and spawn nested chunks using a local registry lookup.
    // Note: we can't access the registry resource here (no SystemParam available in Molecule::spawn).
    // v0: spawn inner chunks as plain text; a future pass wires the registry through.
    let inner = decode_nested(&chunk.payload);
    for inner_chunk in inner {
        // Inline: handle the most common nested types directly.
        spawn_inner(commands, container, &inner_chunk);
    }

    container
}

/// Minimal inline renderer for nested chunks inside a component.
/// Handles the types that actually appear in composition payloads.
pub fn spawn_inner(commands: &mut Commands, parent: Entity, chunk: &Chunk) {
    use cyb_stream::{render as R, sigil as S};
    match (chunk.sigil, chunk.render) {
        (_, R::TEXT) => {
            let text = String::from_utf8_lossy(&chunk.payload).into_owned();
            let color = if chunk.sigil == S::PAT {
                theme::ACID_BLUE
            } else if chunk.sigil == S::SIG {
                theme::TEXT_DIM
            } else {
                theme::TEXT_PRIMARY
            };
            commands.spawn((
                Text::new(text),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(color),
                ChildOf(parent),
            ));
        }
        (_, R::COMPONENT) => {
            // Recursive — but v0 only goes one level deep to avoid stack issues
            let inner = decode_nested(&chunk.payload);
            for c in inner {
                spawn_inner(commands, parent, &c);
            }
        }
        (_, R::ERROR) => {
            let msg = String::from_utf8_lossy(&chunk.payload).into_owned();
            commands.spawn((
                Text::new(format!("✕ {msg}")),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(theme::ACID_RED),
                ChildOf(parent),
            ));
        }
        _ => {
            // Unknown inner chunk — show as dim metadata
            let label = format!(
                "[{} {}]",
                chunk.sigil as char, chunk.render as char
            );
            commands.spawn((
                Text::new(label),
                TextFont { font_size: theme::MICRO, ..default() },
                TextColor(theme::TEXT_DIM),
                ChildOf(parent),
            ));
        }
    }
}
