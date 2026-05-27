use bevy::prelude::*;
use cyb_stream::{Chunk, sigil, render};
use super::Molecule;
use crate::theme;

/// `(., l)` — structured log line.
/// Payload is JSON: `{level, source, message}`
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
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(payload) {
        let level   = v["level"].as_str().unwrap_or("info").to_string();
        let source  = v["source"].as_str().unwrap_or("").to_string();
        let message = v["message"].as_str().unwrap_or("").to_string();
        (level, source, message)
    } else {
        ("info".into(), "".into(), String::from_utf8_lossy(payload).into_owned())
    }
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
