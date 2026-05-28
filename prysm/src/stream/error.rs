use bevy::prelude::*;
use cyb_stream::{Chunk, sigil, render};
use super::Molecule;
use crate::theme;

/// `(!, e)` — typed error with source location.
/// Payload is nested `(=, s)` key-value pairs: level, source, message, span?
pub struct ErrorMolecule;

impl Molecule for ErrorMolecule {
    fn sigil(&self) -> u8 { sigil::ZAP }
    fn render(&self) -> u8 { render::ERROR }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let (level, source, message) = parse_error(&chunk.payload);
        let prefix = match level.as_str() {
            "warn" | "warning" => "⚠ ",
            _ => "✕ ",
        };
        let color = match level.as_str() {
            "warn" | "warning" => theme::ACID_ORANGE,
            _ => theme::ACID_RED,
        };

        let container = commands.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::G)),
                margin: UiRect::vertical(Val::Px(2.0)),
                border: UiRect::left(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(color),
            BackgroundColor(Color::srgba(color.to_srgba().red, 0.05, 0.05, 0.15)),
            ChildOf(parent),
        )).id();

        commands.spawn((
            Text::new(format!("{prefix}{message}")),
            TextFont { font_size: theme::BODY, ..default() },
            TextColor(color),
            ChildOf(container),
        ));

        if !source.is_empty() {
            commands.spawn((
                Text::new(source),
                TextFont { font_size: theme::MICRO, ..default() },
                TextColor(theme::TEXT_DIM),
                ChildOf(container),
            ));
        }

        container
    }
}

fn parse_error(payload: &[u8]) -> (String, String, String) {
    let m = cyb_stream::read_kv(payload);
    let text = |key, default: &str| -> String {
        m.get(key).map(|c| String::from_utf8_lossy(&c.payload).into_owned())
            .unwrap_or_else(|| default.to_string())
    };
    (text("level", "error"), text("source", ""), text("message", "unknown error"))
}
