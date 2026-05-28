use bevy::prelude::*;
use cyb_stream::{Chunk, sigil, render, decode_nested};
use super::Molecule;
use crate::theme;

/// `(#, T)` — 2D grid of records.
///
/// Payload layout (from `cyb_stream::table_chunk`):
///   schema row: `(/, s)` containing `(~, t)` per header
///   data rows:  `(:, s)` each containing one cell chunk per column
pub struct TableMolecule;

#[derive(Component)]
pub struct TableRoot;

impl Molecule for TableMolecule {
    fn sigil(&self) -> u8 { sigil::HAX }
    fn render(&self) -> u8 { render::TABLE }

    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity {
        let rows = decode_nested(&chunk.payload);
        let (headers, data_rows) = split_table_rows(&rows);

        let root = commands.spawn((
            TableRoot,
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            ChildOf(parent),
        )).id();

        // Header row
        if !headers.is_empty() {
            let header_row = commands.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                ChildOf(root),
            )).id();

            for h in &headers {
                spawn_cell(commands, header_row, h, true);
            }
        }

        // Data rows
        for (i, row) in data_rows.iter().enumerate() {
            let bg = if i % 2 == 0 {
                Color::NONE
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.03)
            };
            let data_row = commands.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(bg),
                ChildOf(root),
            )).id();

            for cell in row {
                let text = String::from_utf8_lossy(&cell.payload).into_owned();
                spawn_cell(commands, data_row, &text, false);
            }
        }

        root
    }
}

fn spawn_cell(commands: &mut Commands, row: Entity, text: &str, is_header: bool) {
    let (size, color) = if is_header {
        (theme::CAPTION, Color::srgba(0.75, 0.75, 0.80, 1.0))
    } else {
        (theme::BODY, theme::TEXT_PRIMARY)
    };
    commands.spawn((
        Text::new(text.to_string()),
        TextFont { font_size: size, ..default() },
        TextColor(color),
        Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            overflow: Overflow::clip(),
            ..default()
        },
        ChildOf(row),
    ));
}

fn split_table_rows(rows: &[Chunk]) -> (Vec<String>, Vec<Vec<Chunk>>) {
    let mut headers = Vec::new();
    let mut data_rows = Vec::new();

    for row in rows {
        if row.sigil == sigil::FAS && row.render == render::STRUCT {
            // Schema row — collect header labels from `(~, t)` inner chunks
            let inner = decode_nested(&row.payload);
            for c in inner {
                let label = String::from_utf8_lossy(&c.payload).into_owned();
                headers.push(label);
            }
        } else if row.sigil == sigil::COL && row.render == render::STRUCT {
            // Data row
            let cells = decode_nested(&row.payload);
            data_rows.push(cells);
        }
    }

    (headers, data_rows)
}
