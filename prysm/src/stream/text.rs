use bevy::prelude::*;
use cyb_stream::{Chunk, sigil, render};
use super::Molecule;
use crate::theme;

/// `(#, t)` — plain text / particle content
pub struct TextMolecule;

impl Molecule for TextMolecule {
    fn sigil(&self) -> u8 { sigil::HAX }
    fn render(&self) -> u8 { render::TEXT }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let text = String::from_utf8_lossy(&chunk.payload).into_owned();
        commands.spawn((
            Text::new(text),
            TextFont { font_size: theme::BODY, ..default() },
            TextColor(theme::TEXT_PRIMARY),
            Node {
                margin: UiRect::vertical(Val::Px(1.0)),
                ..default()
            },
            ChildOf(parent),
        )).id()
    }
}

/// `(~, t)` — annotation / label / side-info
pub struct AnnotationMolecule;

impl Molecule for AnnotationMolecule {
    fn sigil(&self) -> u8 { sigil::SIG }
    fn render(&self) -> u8 { render::TEXT }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let text = String::from_utf8_lossy(&chunk.payload).into_owned();
        commands.spawn((
            Text::new(text),
            TextFont { font_size: theme::CAPTION, ..default() },
            TextColor(theme::TEXT_DIM),
            Node {
                margin: UiRect::vertical(Val::Px(1.0)),
                ..default()
            },
            ChildOf(parent),
        )).id()
    }
}

/// `(@, t)` — neuron / identity reference
pub struct NeuronMolecule;

impl Molecule for NeuronMolecule {
    fn sigil(&self) -> u8 { sigil::PAT }
    fn render(&self) -> u8 { render::TEXT }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let raw = String::from_utf8_lossy(&chunk.payload).into_owned();
        let label = if raw.starts_with('@') { raw } else { format!("@{raw}") };
        commands.spawn((
            Text::new(label),
            TextFont { font_size: theme::BODY, ..default() },
            TextColor(theme::ACID_BLUE),
            Node {
                margin: UiRect::vertical(Val::Px(1.0)),
                ..default()
            },
            ChildOf(parent),
        )).id()
    }
}
