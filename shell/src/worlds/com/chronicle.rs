//! The log page: a rune census + numbers table over the signal chain.
//!
//! com keeps the nushell engine and the live session scrollback. This is
//! the chronicle those commands write into — `~/cyb/graph.log` as a table.

use bevy::prelude::*;
use rune_ast::Noun;
use rune_interp::{Host, InterpError};

use super::super::{SharedCell, cell, content, identity::Identity};

const ROW_CAP: usize = 200;

#[derive(Component)]
pub struct LogSlot;

struct LogHost {
    signals: String,
    links: String,
    weight: String,
    rows: Noun,
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
            "table-body" => Ok(self.rows.clone()),
            other => Err(cell::unknown_query(other)),
        }
    }
}

struct Row {
    step: u64,
    n: usize,
    wt: u64,
    label: String,
    ours: bool,
}

fn label_of(sig: &cyb_core::Signal, texts: &std::collections::HashMap<[u8; 32], String>) -> String {
    let Some(first) = sig.links.first() else {
        return format!("step {}", sig.step);
    };
    let name = |p: &[u8; 32]| {
        texts
            .get(p)
            .cloned()
            .unwrap_or_else(|| file::Particle::from_bytes(*p).short_hex())
    };
    let a = name(&first.from);
    let b = name(&first.to);
    if sig.links.len() == 1 {
        format!("{a} → {b}")
    } else {
        format!("{a} → {b} +{}", sig.links.len() - 1)
    }
}

fn collect(shared: &SharedCell, who: [u8; 32]) -> (u64, u64, u64, Vec<Row>) {
    let texts = content::load();
    let Ok(cell) = shared.cell.lock() else {
        return (0, 0, 0, Vec::new());
    };
    let mut links = 0u64;
    let mut weight = 0u64;
    let mut rows = Vec::new();
    for sig in cell.signals() {
        let n = sig.links.len();
        let wt: u64 = sig.links.iter().map(|l| l.amount).sum();
        links += n as u64;
        weight += wt;
        rows.push(Row {
            step: sig.step,
            n,
            wt,
            label: label_of(sig, &texts),
            ours: sig.neuron == who,
        });
    }
    let signals = rows.len() as u64;
    rows.sort_by(|a, b| b.ours.cmp(&a.ours).then(b.step.cmp(&a.step)));
    rows.truncate(ROW_CAP);
    (signals, links, weight, rows)
}

fn chunks_of(shared: &SharedCell, who: [u8; 32]) -> Result<Vec<tade::Chunk>, String> {
    let (signals, links, weight, rows) = collect(shared, who);
    let body = cell::list(
        rows.iter()
            .map(|r| {
                cell::row(&[
                    &r.label,
                    &r.step.to_string(),
                    &r.n.to_string(),
                    &cell::compact(r.wt),
                ])
            })
            .collect(),
    );
    let mut host = LogHost {
        signals: cell::compact(signals),
        links: cell::compact(links),
        weight: cell::compact(weight),
        rows: body,
    };
    cell::eval(&cell::load("log")?, &mut host)
}

/// Rebuild the chronicle when the cell moves. The slot stays put so the
/// live session under it is not torn down.
pub fn refresh(
    mut commands: Commands,
    shared: Res<SharedCell>,
    who: Res<Identity>,
    slot: Query<Entity, With<LogSlot>>,
    children: Query<&Children>,
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
    match chunks_of(&shared, who.neuron) {
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
}
