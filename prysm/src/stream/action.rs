use bevy::prelude::*;
use cyb_stream::{Chunk, sigil, render, decode_nested};
use super::Molecule;
use crate::{theme, atoms::{glass_bg, GlassDepth}};

/// `(!, c)` — button / action component.
/// Payload is nested chunks: label `(~, t)` and optional ref `(#, t)`.
pub struct ActionMolecule;

/// Marks a button spawned from an action chunk. Carries the ref payload for the event.
#[derive(Component)]
pub struct ActionButton {
    pub label: String,
    pub target_ref: String,
}

impl Molecule for ActionMolecule {
    fn sigil(&self) -> u8 { sigil::ZAP }
    fn render(&self) -> u8 { render::COMPONENT }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let (label, target_ref) = parse_action(&chunk.payload);

        commands.spawn((
            ActionButton { label: label.clone(), target_ref },
            Button,
            Node {
                padding: UiRect::axes(Val::Px(theme::G * 2.0), Val::Px(theme::G * 0.75)),
                margin: UiRect::vertical(Val::Px(2.0)),
                align_self: AlignSelf::FlexStart,
                ..default()
            },
            glass_bg(GlassDepth::Midground),
            ChildOf(parent),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(theme::TEXT_PRIMARY),
            ));
        })
        .id()
    }
}

fn parse_action(payload: &[u8]) -> (String, String) {
    let chunks = decode_nested(payload);
    let mut label = String::new();
    let mut target_ref = String::new();
    for c in chunks {
        if c.sigil == sigil::SIG && c.render == render::TEXT {
            label = String::from_utf8_lossy(&c.payload).into_owned();
        } else if c.sigil == sigil::HAX && c.render == render::TEXT {
            target_ref = String::from_utf8_lossy(&c.payload).into_owned();
        }
    }
    if label.is_empty() { label = "action".into(); }
    (label, target_ref)
}
