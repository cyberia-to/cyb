//! Virtual table: a fixed pool of lanes over a tall track.
//!
//! Scroll moves `ScrollPosition` only. Lane texts recycle when the viewport
//! nears the edge of the pool, so 36k rows never become 36k entities and
//! a flick does not rebuild the tree.

use std::collections::HashMap;

use bevy::prelude::*;
use prysm::molecules::action::ActionButton;
use prysm::theme;

use crate::now::Now;
use crate::worlds::WorldState;
use crate::worlds::nav::Nav;

pub const ROW_H: f32 = 40.0;
const POOL: usize = 80;
const OVERSCAN: usize = 16;
/// Census + header sitting above the track.
pub const HEAD_H: f32 = 100.0;

#[derive(Resource, Default)]
pub struct TableBodies {
    pub rows: HashMap<&'static str, Vec<Vec<String>>>,
}

#[derive(Component)]
pub struct TableTrack {
    pub key: &'static str,
    pub n: usize,
    pub start: usize,
    pub pool_n: usize,
    pub head: f32,
}

#[derive(Component)]
pub(crate) struct TablePool;

#[derive(Component)]
pub(crate) struct TablePadTop;

#[derive(Component)]
pub(crate) struct TablePadBot;

#[derive(Component)]
pub(crate) struct PoolLane {
    key: &'static str,
    i: usize,
}

#[derive(Component)]
pub(crate) struct PoolText {
    key: &'static str,
    lane: usize,
    col: usize,
}

#[derive(Component)]
pub(crate) struct PoolHit {
    key: &'static str,
    lane: usize,
    col: usize,
}

/// Mount header + a tall track with a recycled pool. Call once per page.
pub fn mount(
    commands: &mut Commands,
    parent: Entity,
    key: &'static str,
    headers: &[&str],
    rows: &[Vec<String>],
    bodies: &mut TableBodies,
) {
    bodies.rows.insert(key, rows.to_vec());
    let n = rows.len();
    spawn_header(commands, parent, headers);
    let pool_n = POOL.min(n.max(1));
    let track = commands
        .spawn((
            TableTrack {
                key,
                n,
                start: 0,
                pool_n,
                head: HEAD_H,
            },
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        TablePadTop,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(0.0),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(track),
    ));
    let pool = commands
        .spawn((
            TablePool,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(track),
        ))
        .id();
    commands.spawn((
        TablePadBot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(n.saturating_sub(pool_n) as f32 * ROW_H),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(track),
    ));
    let cols = headers.len().max(1);
    let tracks = tracks_for(cols, headers);
    let end = end_align(headers);
    for i in 0..pool_n {
        spawn_lane(
            commands,
            pool,
            key,
            i,
            cols,
            &tracks,
            &end,
            rows.get(i).map(Vec::as_slice),
        );
    }
}

fn tracks_for(n: usize, headers: &[&str]) -> Vec<f32> {
    match n {
        0 | 1 => vec![100.0],
        2 => vec![62.0, 38.0],
        3 => vec![50.0, 25.0, 25.0],
        4 if headers.first() == Some(&"from") => vec![38.0, 38.0, 12.0, 12.0],
        4 => vec![54.0, 14.0, 18.0, 14.0],
        6 if headers.first() == Some(&"step") => vec![10.0, 26.0, 26.0, 16.0, 12.0, 10.0],
        n => {
            let rest = 56.0 / (n as f32 - 1.0);
            let mut t = vec![44.0];
            t.extend(std::iter::repeat(rest).take(n - 1));
            t
        }
    }
}

fn end_align(headers: &[&str]) -> Vec<bool> {
    headers.iter().copied().map(is_end_col).collect()
}

fn is_end_col(header: &str) -> bool {
    matches!(
        header,
        "step" | "stake" | "valence" | "focus" | "size" | "when"
    )
}

fn cell_box(width: f32, end: bool) -> Node {
    Node {
        width: Val::Percent(width),
        min_width: Val::Px(0.0),
        flex_shrink: 0.0,
        overflow: Overflow::clip(),
        flex_direction: FlexDirection::Row,
        justify_content: if end {
            JustifyContent::FlexEnd
        } else {
            JustifyContent::FlexStart
        },
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_header(commands: &mut Commands, parent: Entity, headers: &[&str]) {
    let tracks = tracks_for(headers.len().max(1), headers);
    let end = end_align(headers);
    let lane = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                width: Val::Percent(100.0),
                height: Val::Px(32.0),
                min_height: Val::Px(32.0),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(Val::Px(theme::G * 1.5)),
                border: UiRect::bottom(Val::Px(1.0)),
                overflow: Overflow::clip(),
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(theme::BORDER),
            ChildOf(parent),
        ))
        .id();
    for (i, h) in headers.iter().enumerate() {
        commands
            .spawn((
                cell_box(tracks[i], end.get(i).copied().unwrap_or(false)),
                ChildOf(lane),
            ))
            .with_children(|cell| {
                cell.spawn((
                    Text::new(*h),
                    TextFont {
                        font_size: theme::MICRO,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                    TextLayout::new_with_no_wrap(),
                ));
            });
    }
}

fn spawn_lane(
    commands: &mut Commands,
    pool: Entity,
    key: &'static str,
    i: usize,
    cols: usize,
    tracks: &[f32],
    end: &[bool],
    row: Option<&[String]>,
) {
    let (visible, targets) = row.map(split_row).unwrap_or_default();
    let lane = commands
        .spawn((
            PoolLane { key, i },
            Node {
                flex_direction: FlexDirection::Row,
                width: Val::Percent(100.0),
                height: Val::Px(ROW_H),
                min_height: Val::Px(ROW_H),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(Val::Px(theme::G * 1.5)),
                overflow: Overflow::clip(),
                align_items: AlignItems::Center,
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(zebra(i)),
            ChildOf(pool),
        ))
        .id();
    for c in 0..cols {
        let text = visible.get(c).cloned().unwrap_or_default();
        let target = targets.get(c).cloned().flatten();
        let mut e = commands.spawn((
            cell_box(
                tracks.get(c).copied().unwrap_or(10.0),
                end.get(c).copied().unwrap_or(false),
            ),
            Button,
            ActionButton {
                label: text.clone(),
                target_ref: target.unwrap_or_default(),
            },
            PoolHit {
                key,
                lane: i,
                col: c,
            },
            ChildOf(lane),
        ));
        e.with_children(|cell| {
            cell.spawn((
                PoolText {
                    key,
                    lane: i,
                    col: c,
                },
                Text::new(text),
                TextFont {
                    font_size: if c == 0 { theme::BODY } else { theme::CAPTION },
                    ..default()
                },
                TextColor(if c == 0 {
                    theme::TEXT_PRIMARY
                } else {
                    theme::TEXT_DIM
                }),
                TextLayout::new_with_no_wrap(),
            ));
        });
    }
}

fn zebra(i: usize) -> Color {
    Color::srgba(0.13, 0.92, 0.51, if i % 2 == 0 { 0.06 } else { 0.14 })
}

fn is_target(c: &str) -> bool {
    c == "_"
        || c.starts_with("particle:")
        || c.starts_with("cyb://")
        || c.starts_with("model:")
        || c.starts_with("fetch:")
        || c.starts_with("vault:")
        || c.starts_with("work:")
        || c.starts_with("block:")
}

fn split_row(row: &[String]) -> (Vec<String>, Vec<Option<String>>) {
    let mut cells: Vec<String> = row.to_vec();
    let mut targets = Vec::new();
    while cells.last().is_some_and(|c| is_target(c)) {
        targets.push(cells.pop().unwrap());
    }
    targets.reverse();
    let targets: Vec<Option<String>> = targets
        .into_iter()
        .map(|t| if t == "_" { None } else { Some(t) })
        .collect();
    (cells, targets)
}

/// How many pixels of census+header sit above the track in the same scroller.
pub(crate) fn measure_head(
    mut tracks: Query<(Entity, &ChildOf, &mut TableTrack)>,
    children: Query<&Children>,
    nodes: Query<&ComputedNode>,
) {
    for (e, parent, mut track) in &mut tracks {
        let Ok(kids) = children.get(parent.parent()) else {
            continue;
        };
        let mut h = 0.0;
        for k in kids.iter() {
            if k == e {
                break;
            }
            if let Ok(n) = nodes.get(k) {
                h += n.size().y * n.inverse_scale_factor();
            }
        }
        if (track.head - h).abs() > 1.0 {
            track.head = h;
        }
    }
}

/// Recycle pool texts when the viewport walks off the overscan. Layout of
/// the track stays put — only ScrollPosition moves between recycles.
pub(crate) fn recycle(
    bodies: Res<TableBodies>,
    persist: Query<(&crate::worlds::scroll::PersistScroll, &ScrollPosition)>,
    vis: Query<&InheritedVisibility>,
    mut tracks: Query<(Entity, &mut TableTrack)>,
    mut pads_top: Query<(&ChildOf, &mut Node), (With<TablePadTop>, Without<TablePadBot>)>,
    mut pads_bot: Query<(&ChildOf, &mut Node), (With<TablePadBot>, Without<TablePadTop>)>,
    mut lanes: Query<(&PoolLane, &mut BorderColor)>,
    mut texts: Query<(&PoolText, &mut Text)>,
    mut hits: Query<(&PoolHit, &mut ActionButton)>,
) {
    for (track_e, mut track) in &mut tracks {
        if vis.get(track_e).ok().is_some_and(|v| !v.get()) {
            continue;
        }
        let Some(rows) = bodies.rows.get(track.key) else {
            continue;
        };
        let n = rows.len();
        track.n = n;
        if n == 0 {
            continue;
        }
        let y = persist
            .iter()
            .find(|(p, _)| p.0 == track.key)
            .map(|(_, s)| s.y)
            .unwrap_or(0.0);
        let view = ((y - track.head).max(0.0) / ROW_H).floor() as usize;
        let view = view.min(n.saturating_sub(1));
        let pool_n = track.pool_n.max(1);
        let new_start = view
            .saturating_sub(OVERSCAN)
            .min(n.saturating_sub(pool_n.max(1)).min(n.saturating_sub(1)));
        let force = bodies.is_changed();
        if !force && new_start == track.start {
            continue;
        }
        track.start = new_start;
        let top_h = new_start as f32 * ROW_H;
        let bot_h = n.saturating_sub(new_start + pool_n) as f32 * ROW_H;
        for (parent, mut node) in &mut pads_top {
            if parent.parent() == track_e {
                node.height = Val::Px(top_h);
            }
        }
        for (parent, mut node) in &mut pads_bot {
            if parent.parent() == track_e {
                node.height = Val::Px(bot_h);
            }
        }
        let key = track.key;
        for (lane, mut border) in &mut lanes {
            if lane.key == key {
                *border = BorderColor::all(zebra(new_start + lane.i));
            }
        }
        for (pt, mut text) in &mut texts {
            if pt.key != key {
                continue;
            }
            let idx = new_start + pt.lane;
            let (visible, _) = rows.get(idx).map(|r| split_row(r)).unwrap_or_default();
            let want = visible.get(pt.col).cloned().unwrap_or_default();
            if text.0 != want {
                *text = Text::new(want);
            }
        }
        for (hit, mut btn) in &mut hits {
            if hit.key != key {
                continue;
            }
            let idx = new_start + hit.lane;
            let (_, targets) = rows.get(idx).map(|r| split_row(r)).unwrap_or_default();
            let tgt = targets.get(hit.col).cloned().flatten().unwrap_or_default();
            if btn.target_ref != tgt {
                btn.target_ref = tgt;
            }
        }
    }
}

pub(crate) fn click_particle(
    q: Query<(&Interaction, &ActionButton), Changed<Interaction>>,
    mut now: ResMut<Now>,
    mut nav: ResMut<Nav>,
    world: Res<State<WorldState>>,
    index: Option<Res<super::graph::BrainIndex>>,
    warp: Option<ResMut<mir::bevy::resources::WarpTarget>>,
) {
    for (i, b) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        let Some(hex) = b.target_ref.strip_prefix("particle:") else {
            continue;
        };
        let Some(p) = file::Particle::from_hex(hex) else {
            continue;
        };
        let hash = *p.as_bytes();
        let idx = index
            .as_deref()
            .and_then(|ix| ix.hashes.iter().position(|h| *h == hash));
        if let Some(mut warp) = warp {
            if let Some(idx) = idx {
                warp.particle_idx = Some(idx as u32);
            }
        }
        super::nav::stand_from(&mut nav, &mut now, *world.get(), hash, idx, false);
        return;
    }
}
