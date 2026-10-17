//! The log page: a rune census + the same windowed table memory uses.
//!
//! Attention casts `world → world` on every tab hop. Those hops are real
//! signals, but they dominate the newest end of the tape (13k `body→sigma`
//! on a lived-in log) so the table looks frozen. They stay in the graph;
//! this page lists the rest.

use bevy::prelude::*;
use rune_ast::Noun;
use rune_interp::{Host, InterpError};

use super::super::{SharedCell, cell, content, identity::Identity, windowed};
use crate::worlds::scroll::PersistScroll;

#[derive(Component)]
pub struct LogSlot;

#[derive(Clone)]
struct LogRow {
    step: String,
    from: String,
    to: String,
    token: String,
    stake: String,
    valence: String,
    from_h: [u8; 32],
    to_h: [u8; 32],
    token_h: [u8; 32],
}

#[derive(Resource, Default)]
pub struct LogFollowTop(pub bool);

struct LogHost {
    signals: String,
    axons: String,
    stake: String,
}

impl Host for LogHost {
    fn perform(&mut self, act: u64, args: &Noun, _caps: &Noun) -> Result<Noun, InterpError> {
        if !cell::act_is_query(act) {
            return Ok(Noun::Atom(0));
        }
        match cell::query_name(args).as_str() {
            "signals" => Ok(cell::tape(&self.signals)),
            "axons" => Ok(cell::tape(&self.axons)),
            "stake" => Ok(cell::tape(&self.stake)),
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

fn is_zero(p: &[u8; 32]) -> bool {
    p.iter().all(|&b| b == 0)
}

fn is_world_label(s: &str) -> bool {
    matches!(
        s,
        "body"
            | "brain"
            | "log"
            | "com"
            | "robot"
            | "sigma"
            | "models"
            | "vault"
            | "memory"
            | "oracle"
    )
}

fn token_of(
    link_token: &[u8; 32],
    network: &[u8; 32],
    texts: &std::collections::HashMap<[u8; 32], String>,
) -> ([u8; 32], String) {
    let p = if !is_zero(link_token) {
        *link_token
    } else if !is_zero(network) {
        *network
    } else {
        [0u8; 32]
    };
    let name = if is_zero(&p) {
        "self".into()
    } else {
        name_of(&p, texts)
    };
    (p, name)
}

fn valence_text(v: i8) -> String {
    if v > 0 {
        format!("+{v}")
    } else {
        v.to_string()
    }
}

fn collect(shared: &SharedCell, _who: [u8; 32]) -> (u64, u64, u64, Vec<LogRow>) {
    let texts = content::load();
    let Ok(cell) = shared.cell.lock() else {
        return (0, 0, 0, Vec::new());
    };
    let n_signals = cell.len() as u64;
    let axons = cell.axons();
    let n_axons = axons.len() as u64;
    let stake: u64 = axons.iter().map(|(_, _, w)| *w).sum();
    let mut rows = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for sig in cell.signals() {
        if sig.links.is_empty() {
            continue;
        }
        for link in &sig.links {
            let from = name_of(&link.from, &texts);
            let to = name_of(&link.to, &texts);
            // Tab hops seed the graph; they are not the log you read.
            if is_world_label(&from) && is_world_label(&to) {
                continue;
            }
            let (token_h, token) = token_of(&link.token, &sig.network, &texts);
            if !seen.insert((sig.neuron, sig.step, link.from, link.to, token_h)) {
                continue;
            }
            rows.push((
                sig.step,
                LogRow {
                    step: sig.step.to_string(),
                    from,
                    to,
                    token,
                    stake: link.amount.to_string(),
                    valence: valence_text(link.valence),
                    from_h: link.from,
                    to_h: link.to,
                    token_h,
                },
            ));
        }
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    let rows = rows.into_iter().map(|(_, r)| r).collect();
    (n_signals, n_axons, stake, rows)
}

fn table_rows(rows: &[LogRow]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|r| {
            vec![
                r.step.clone(),
                r.from.clone(),
                r.to.clone(),
                r.token.clone(),
                r.stake.clone(),
                r.valence.clone(),
                "_".into(),
                particle_ref(&r.from_h),
                particle_ref(&r.to_h),
                particle_ref(&r.token_h),
            ]
        })
        .collect()
}

fn particle_ref(h: &[u8; 32]) -> String {
    if is_zero(h) {
        "_".into()
    } else {
        format!("particle:{}", hex32(h))
    }
}

fn hex32(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn refresh(
    mut commands: Commands,
    shared: Res<SharedCell>,
    who: Res<Identity>,
    slot: Query<Entity, With<LogSlot>>,
    tracks: Query<&windowed::TableTrack>,
    mut scrolls: Query<(&PersistScroll, &mut ScrollPosition)>,
    mut bodies: ResMut<windowed::TableBodies>,
    mut follow: ResMut<LogFollowTop>,
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
    follow.0 = true;
    let mounted = tracks.iter().any(|t| t.key == "log");
    let (signals, axons, stake, rows) = collect(&shared, who.neuron);
    let table = table_rows(&rows);
    bodies.rows.insert("log", table.clone());
    if mounted {
        return;
    }
    for (p, mut pos) in &mut scrolls {
        if p.0 == "log" {
            pos.y = 0.0;
        }
    }
    let mut host = LogHost {
        signals: cell::exact(signals),
        axons: cell::exact(axons),
        stake: cell::exact(stake),
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
    windowed::mount(
        &mut commands,
        slot,
        "log",
        &["step", "from", "to", "token", "stake", "valence"],
        &table,
        &mut bodies,
    );
}
