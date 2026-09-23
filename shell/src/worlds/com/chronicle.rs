//! The log page: a rune census + a windowed numbers table over the chain.
//!
//! Every signal is in the list. Only a window of rows is spawned — a
//! lived-in graph.log is tens of thousands of signals, and Bevy UI cannot
//! hold them all as entities. Spacers keep the scroll length honest.

use bevy::prelude::*;
use rune_ast::Noun;
use rune_interp::{Host, InterpError};

use super::super::{SharedCell, cell, content, identity::Identity};

const ROW_H: f32 = 34.0;
const WINDOW: usize = 64;
const HEAD_H: f32 = 108.0;

#[derive(Component)]
pub struct LogSlot;

#[derive(Component)]
pub struct LogWindow;

#[derive(Component)]
struct LogSpacerTop;

#[derive(Component)]
struct LogSpacerBot;

#[derive(Clone)]
struct LogRow {
    from: String,
    to: String,
    step: String,
    wt: String,
}

#[derive(Resource, Default)]
pub struct Chronicle {
    rows: Vec<LogRow>,
}

struct LogHost {
    signals: String,
    links: String,
    weight: String,
}

impl Host for LogHost {
    fn perform(&mut self, act: u64, args: &Noun, _caps: &Noun) -> Result<Noun, InterpError> {
        if !cell::act_is_query(act) {
            return Ok(Noun::Atom(0));
        }
        match cell::query_name(args).as_str() {
            "signals" => Ok(cell::tape(&self.signals)),
            "links" => Ok(cell::tape(&self.links)),
            "weight" => Ok(cell::tape(&self.weight)),
            other => Err(cell::unknown_query(other)),
        }
    }
}

fn name_of(p: &[u8; 32], texts: &std::collections::HashMap<[u8; 32], String>) -> String {
    texts
        .get(p)
        .cloned()
        .unwrap_or_else(|| file::Particle::from_bytes(*p).short_hex())
}

fn collect(shared: &SharedCell, who: [u8; 32]) -> (u64, u64, u64, Vec<LogRow>) {
    let texts = content::load();
    let Ok(cell) = shared.cell.lock() else {
        return (0, 0, 0, Vec::new());
    };
    let mut n_signals = 0u64;
    let mut n_links = 0u64;
    let mut weight = 0u64;
    let mut rows = Vec::new();
    // One row per cyberlink — a signal with three links used to hide two
    // of them behind "+2". Newest of this neuron first, then everyone else.
    for sig in cell.signals() {
        n_signals += 1;
        if sig.links.is_empty() {
            rows.push((
                sig.neuron == who,
                sig.step,
                LogRow {
                    from: "—".into(),
                    to: "—".into(),
                    step: sig.step.to_string(),
                    wt: "0".into(),
                },
            ));
            continue;
        }
        for link in &sig.links {
            n_links += 1;
            weight += link.amount;
            rows.push((
                sig.neuron == who,
                sig.step,
                LogRow {
                    from: name_of(&link.from, &texts),
                    to: name_of(&link.to, &texts),
                    step: sig.step.to_string(),
                    wt: link.amount.to_string(),
                },
            ));
        }
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let rows = rows.into_iter().map(|(_, _, r)| r).collect();
    (n_signals, n_links, weight, rows)
}

/// Rebuild the chronicle when the cell moves. The slot stays put so the
/// live session under it is not torn down.
pub fn refresh(
    mut commands: Commands,
    shared: Res<SharedCell>,
    who: Res<Identity>,
    slot: Query<Entity, With<LogSlot>>,
    children: Query<&Children>,
    mut chronicle: ResMut<Chronicle>,
    mut last: Local<Option<u64>>,
) {
    let Ok(slot) = slot.single() else {
        return;
    };
    let v = shared.version();
    if Some(v) == *last {
        return;
    }
    *last = Some(v);
    if let Ok(kids) = children.get(slot) {
        for c in kids.iter() {
            commands.entity(c).despawn();
        }
    }
    let (signals, links, weight, rows) = collect(&shared, who.neuron);
    chronicle.rows = rows;
    let mut host = LogHost {
        signals: cell::exact(signals),
        links: cell::exact(links),
        weight: cell::exact(weight),
    };
    match cell::load("log").and_then(|src| cell::eval(&src, &mut host)) {
        Ok(chunks) => cell::dispatch_page(&mut commands, slot, &chunks),
        Err(e) => {
            commands.spawn((
                Text::new(e),
                TextFont {
                    font_size: prysm::theme::CAPTION,
                    ..default()
                },
                TextColor(prysm::theme::ACID_RED),
                ChildOf(slot),
            ));
        }
    }
    spawn_table(&mut commands, slot, &chronicle.rows, 0);
}

fn spawn_table(commands: &mut Commands, slot: Entity, rows: &[LogRow], start: usize) {
    let n = rows.len();
    let start = start.min(n);
    let end = (start + WINDOW).min(n);
    let window = commands
        .spawn((
            LogWindow,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(slot),
        ))
        .id();
    commands.spawn((
        LogSpacerTop,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(start as f32 * ROW_H),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(window),
    ));
    let slice = &rows[start..end];
    let mut host = WindowHost {
        rows: cell::list(
            slice
                .iter()
                .map(|r| cell::row(&[&r.from, &r.to, &r.step, &r.wt]))
                .collect(),
        ),
    };
    if let Ok(chunks) = cell::eval(
        r#"table(row("from","to","step","wt"), query("table-body"))"#,
        &mut host,
    ) {
        cell::dispatch_page(commands, window, &chunks);
    }
    commands.spawn((
        LogSpacerBot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px((n.saturating_sub(end)) as f32 * ROW_H),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(window),
    ));
}

struct WindowHost {
    rows: Noun,
}

impl Host for WindowHost {
    fn perform(&mut self, act: u64, args: &Noun, _caps: &Noun) -> Result<Noun, InterpError> {
        if cell::act_is_query(act) && cell::query_name(args) == "table-body" {
            return Ok(self.rows.clone());
        }
        Ok(Noun::Atom(0))
    }
}

/// Slide the spawned window so the spacers keep every signal in reach.
pub fn slide_window(
    mut commands: Commands,
    chronicle: Res<Chronicle>,
    slot: Query<(Entity, &ChildOf), With<LogSlot>>,
    window: Query<Entity, With<LogWindow>>,
    scroll: Query<&ScrollPosition>,
    mut last: Local<Option<usize>>,
) {
    let Ok((slot, parent)) = slot.single() else {
        return;
    };
    let n = chronicle.rows.len();
    if n == 0 {
        return;
    }
    let y = scroll.get(parent.parent()).map(|p| p.y).unwrap_or(0.0);
    let start = ((y - HEAD_H).max(0.0) / ROW_H).floor() as usize;
    let start = start.min(n.saturating_sub(1));
    if Some(start) == *last && !chronicle.is_changed() {
        return;
    }
    *last = Some(start);
    for e in &window {
        commands.entity(e).despawn();
    }
    spawn_table(&mut commands, slot, &chronicle.rows, start);
}
