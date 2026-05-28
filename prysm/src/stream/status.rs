use bevy::prelude::*;
use cyb_stream::{Chunk, sigil, render};
use super::Molecule;
use crate::theme;

/// `(., x)` — end-of-command sentinel.
/// Consumed by `consume_chunks` to record the exit code. Also renders a subtle
/// divider so the user sees where one command's output ends.
pub struct StatusMolecule;

impl Molecule for StatusMolecule {
    fn sigil(&self) -> u8 { sigil::DOT }
    fn render(&self) -> u8 { render::STATUS }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let code = parse_code(&chunk.payload);
        let color = if code == 0 {
            Color::srgba(1.0, 1.0, 1.0, 0.06)
        } else {
            Color::srgba(theme::ACID_RED.to_srgba().red, 0.1, 0.1, 0.3)
        };
        commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(color),
            ChildOf(parent),
        )).id()
    }
}

fn parse_code(payload: &[u8]) -> i32 {
    cyb_stream::read_kv(payload)
        .get("code")
        .and_then(|c| String::from_utf8_lossy(&c.payload).parse().ok())
        .unwrap_or(0)
}
