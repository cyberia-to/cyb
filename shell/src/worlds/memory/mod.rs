//! memory — the file manager.
//!
//! Same census as brain (particles, axons, graph bytes). Same windowed
//! table + scroll as log. One row per particle the graph actually holds.

use bevy::prelude::*;
use rune_ast::Noun;
use rune_interp::{Host, InterpError};

use super::cell;
use super::graph::{BrainIndex, BrainStats};
use super::windowed;
use super::{WorldState, content};
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H, ContentRoot};

pub struct MemoryWorldPlugin;

#[derive(Component)]
struct MemoryRoot;

#[derive(Component)]
struct MemorySlot;

#[derive(Clone)]
struct Row {
    hash: [u8; 32],
    label: String,
    focus: f32,
    size: usize,
    created: Option<u64>,
}

#[derive(Resource, Default)]
struct MemoryList {
    rows: Vec<Row>,
}

impl Plugin for MemoryWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MemoryList>()
            .add_systems(OnEnter(WorldState::Memory), enter)
            .add_systems(Update, refresh.run_if(in_state(WorldState::Memory)));
    }
}

fn enter(
    mut commands: Commands,
    index: Option<Res<BrainIndex>>,
    stats: Res<BrainStats>,
    mut bodies: ResMut<windowed::TableBodies>,
    mut worlds: Query<(&crate::worlds::WorldUi, &mut Visibility)>,
) {
    let ranked = ranked_rows(index.as_deref());
    if crate::worlds::reveal_world(WorldState::Memory, &mut worlds) {
        commands.insert_resource(MemoryList {
            rows: ranked.clone(),
        });
        bodies.rows.insert("memory", table_rows(&ranked));
        return;
    }
    build_page(commands, index, stats, bodies);
}

fn refresh(
    index: Option<Res<BrainIndex>>,
    stats: Res<BrainStats>,
    roots: Query<Entity, With<MemoryRoot>>,
    mut last: Local<Option<(usize, u32, Option<[u8; 32]>)>>,
    mut list: ResMut<MemoryList>,
    mut bodies: ResMut<windowed::TableBodies>,
) {
    // Never despawn the world root — tearing a live taffy tree panics
    // `invalid SlotMap key` and kills the process. The first Update after
    // OnEnter used to do exactly that.
    if roots.is_empty() {
        return;
    }
    let Some(ref idx) = index else { return };
    let focus_fp = idx.focus.iter().copied().fold(0.0f32, f32::max).to_bits();
    let fp = (idx.hashes.len(), focus_fp, idx.hashes.first().copied());
    if Some(fp) == *last {
        return;
    }
    *last = Some(fp);
    list.rows = ranked_rows(Some(idx));
    bodies.rows.insert("memory", table_rows(&list.rows));
    let _ = stats;
}

fn ranked_rows(index: Option<&BrainIndex>) -> Vec<Row> {
    let meta = content::load_with_meta();
    let Some(index) = index else {
        return Vec::new();
    };
    let mut rows: Vec<Row> = index
        .hashes
        .iter()
        .enumerate()
        .map(|(i, hash)| {
            let m = meta.get(hash);
            let label = m
                .map(|m| m.text.clone())
                .or_else(|| index.labels.get(i).cloned().flatten())
                .unwrap_or_else(|| short_hex(hash));
            Row {
                hash: *hash,
                label,
                focus: index.focus.get(i).copied().unwrap_or(0.0),
                size: m.map(|m| m.text.len()).unwrap_or(0),
                created: m.and_then(|m| m.created),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.focus
            .partial_cmp(&a.focus)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.label.cmp(&b.label))
    });
    rows
}

fn focus_text(f: f32) -> String {
    if f <= 0.0 {
        "0".into()
    } else if f >= 0.001 {
        format!("{f:.3}")
    } else {
        format!("{f:.2e}")
    }
}

fn short_hex(hash: &[u8; 32]) -> String {
    hash[..4].iter().map(|b| format!("{b:02x}")).collect()
}

fn hex32(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn size_text(n: usize) -> String {
    if n == 0 {
        "-".into()
    } else if n < 1024 {
        format!("{n} B")
    } else {
        format!("{:.1} KB", n as f64 / 1024.0)
    }
}

fn date_text(created: Option<u64>) -> String {
    let Some(secs) = created else {
        return "-".into();
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(secs);
    let ago = now.saturating_sub(secs);
    if ago < 60 {
        "now".into()
    } else if ago < 3600 {
        format!("{}m", ago / 60)
    } else if ago < 86_400 {
        format!("{}h", ago / 3600)
    } else if ago < 86_400 * 14 {
        format!("{}d", ago / 86_400)
    } else if ago < 86_400 * 7 * 52 {
        format!("{}w", ago / (86_400 * 7))
    } else {
        format!("{}y", ago / (86_400 * 365))
    }
}

fn table_rows(rows: &[Row]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|r| {
            vec![
                r.label.clone(),
                focus_text(r.focus),
                size_text(r.size),
                date_text(r.created),
                format!("particle:{}", hex32(&r.hash)),
            ]
        })
        .collect()
}

struct MemoryHost {
    particles: String,
    axons: String,
    graph: String,
}

impl Host for MemoryHost {
    fn perform(&mut self, act: u64, args: &Noun, _caps: &Noun) -> Result<Noun, InterpError> {
        if !cell::act_is_query(act) {
            return Ok(Noun::Atom(0));
        }
        match cell::query_name(args).as_str() {
            "particles" => Ok(cell::tape(&self.particles)),
            "axons" => Ok(cell::tape(&self.axons)),
            "graph" => Ok(cell::tape(&self.graph)),
            other => Err(cell::unknown_query(other)),
        }
    }
}

fn build_page(
    mut commands: Commands,
    index: Option<Res<BrainIndex>>,
    stats: Res<BrainStats>,
    mut bodies: ResMut<windowed::TableBodies>,
) {
    let root = commands
        .spawn((
            MemoryRoot,
            crate::worlds::WorldUi(WorldState::Memory),
            ContentRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(CHROME_TOP_H),
                bottom: Val::Px(CHROME_BOTTOM_H),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(prysm::theme::DARK_BASE),
        ))
        .id();

    let scroll = commands
        .spawn((
            crate::worlds::page::scroll_column(),
            ScrollPosition::default(),
            crate::worlds::scroll::PersistScroll("memory"),
            ChildOf(root),
        ))
        .id();

    let slot = commands
        .spawn((
            MemorySlot,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(prysm::theme::G),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(scroll),
        ))
        .id();

    let ranked = ranked_rows(index.as_deref());
    let mut host = MemoryHost {
        particles: cell::exact(stats.particles as u64),
        axons: cell::exact(stats.axons as u64),
        graph: format!("{:.0} KB", stats.graph_bytes as f64 / 1024.0),
    };
    match cell::load("memory").and_then(|src| cell::eval(&src, &mut host)) {
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
    commands.insert_resource(MemoryList {
        rows: ranked.clone(),
    });
    windowed::mount(
        &mut commands,
        slot,
        "memory",
        &["name", "focus", "size", "when"],
        &table_rows(&ranked),
        &mut bodies,
    );
}
