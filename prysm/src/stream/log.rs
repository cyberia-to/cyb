use bevy::prelude::*;
use cyb_stream::{Chunk, sigil, render};
use super::Molecule;
use crate::theme;

/// `(., l)` — structured log line.
/// Payload is nested `(=, s)` key-value pairs: level, source, message
pub struct LogMolecule;

impl Molecule for LogMolecule {
    fn sigil(&self) -> u8 { sigil::DOT }
    fn render(&self) -> u8 { render::LOG }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let (level, source, message) = parse_log(&chunk.payload);
        let (prefix, color) = level_style(&level);
        let full = if source.is_empty() {
            format!("{prefix} {message}")
        } else {
            format!("{prefix} [{source}] {message}")
        };
        commands.spawn((
            Text::new(full),
            TextFont { font_size: theme::MICRO, ..default() },
            TextColor(color),
            Node { margin: UiRect::vertical(Val::Px(0.5)), ..default() },
            ChildOf(parent),
        )).id()
    }
}

fn parse_log(payload: &[u8]) -> (String, String, String) {
    let m = cyb_stream::read_kv(payload);
    let text = |key, default: &str| -> String {
        m.get(key).map(|c| String::from_utf8_lossy(&c.payload).into_owned())
            .unwrap_or_else(|| default.to_string())
    };
    (text("level", "info"), text("source", ""), text("message", ""))
}

fn level_style(level: &str) -> (&'static str, Color) {
    match level {
        "error" | "err"  => ("E", theme::ACID_RED),
        "warn"  | "warning" => ("W", theme::ACID_ORANGE),
        "debug" | "dbg"  => ("D", theme::TEXT_DIM),
        "trace"          => ("T", Color::srgba(0.3, 0.3, 0.35, 1.0)),
        _                => ("I", theme::TEXT_DIM),
    }
}
