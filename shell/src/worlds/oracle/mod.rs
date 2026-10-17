//! oracle — the block explorer.
//!
//! A standard chain explorer's table: height, time, hashrate, and the pi
//! supply as of that block, newest first. Tap a row to see what actually
//! happened in it — the links and transfers its one signal carried. Talks
//! to the first configured network's own `GET /blocks` / `GET /block/<h>`
//! (soft3/crate/src/node.rs) — local bookkeeping the node keeps for this
//! page alone, never part of consensus.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use prysm::theme;

use super::WorldState;
use super::body::BodyLinkHub;
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H, ContentRoot};

pub struct OracleWorldPlugin;

struct OracleHost {
    height: String,
    supply: String,
    signals: String,
    rows: rune_ast::Noun,
}

impl rune_interp::Host for OracleHost {
    fn perform(
        &mut self,
        act: u64,
        args: &rune_ast::Noun,
        _caps: &rune_ast::Noun,
    ) -> Result<rune_ast::Noun, rune_interp::InterpError> {
        if !super::cell::act_is_query(act) {
            return Ok(rune_ast::Noun::Atom(0));
        }
        match super::cell::query_name(args).as_str() {
            "height" => Ok(super::cell::tape(&self.height)),
            "supply" => Ok(super::cell::tape(&self.supply)),
            "signals" => Ok(super::cell::tape(&self.signals)),
            "table-body" => Ok(self.rows.clone()),
            other => Err(super::cell::unknown_query(other)),
        }
    }
}

impl Plugin for OracleWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Oracle>()
            .init_resource::<OracleUi>()
            .init_resource::<OpenBlock>()
            .add_systems(OnEnter(WorldState::Oracle), enter)
            .add_systems(OnExit(WorldState::Oracle), hide_block_page)
            .add_systems(
                Update,
                (
                    handle_table_press,
                    fill_block_page,
                    poll_list,
                    repaint,
                    scroll_page,
                )
                    .run_if(in_state(WorldState::Oracle)),
            );
    }
}

#[derive(Clone, Debug, Default)]
struct BlockRow {
    height: u64,
    time: u64,
    supply: u64,
    weight: u64,
}

/// The expanded row shows only what the table header does not already:
/// who cast the block's signal and its links/transfers.
#[derive(Clone, Debug, Default)]
struct BlockDetail {
    neuron: String,
    /// Already human-readable — one line per link/transfer, straight from
    /// the node's own text.
    lines: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct OracleState {
    height: u64,
    supply: u64,
    root: String,
    signals: u64,
    rows: Vec<BlockRow>,
    details: HashMap<u64, BlockDetail>,
    error: String,
    busy: bool,
    version: u64,
}

/// Shared with the background fetch threads, same shape as sigma's
/// `ChainMoney` — threads write, the page reads a snapshot.
#[derive(Resource, Clone, Default)]
struct Oracle(Arc<Mutex<OracleState>>);

impl Oracle {
    fn try_list(&self) -> Option<OracleState> {
        let s = self.0.try_lock().ok()?;
        Some(OracleState {
            height: s.height,
            supply: s.supply,
            root: s.root.clone(),
            signals: s.signals,
            rows: s.rows.clone(),
            details: HashMap::new(),
            error: s.error.clone(),
            busy: s.busy,
            version: s.version,
        })
    }

    fn refresh_list(&self, url: String) {
        let slot = self.0.clone();
        if let Ok(mut s) = slot.lock() {
            s.busy = true;
            s.version += 1;
        }
        std::thread::Builder::new()
            .name("oracle-blocks".into())
            .spawn(move || {
                let agent = super::body::networks::agent_raw();
                let status = agent
                    .get(&format!("{url}/status"))
                    .call()
                    .ok()
                    .and_then(|mut r| {
                        r.status()
                            .is_success()
                            .then(|| r.body_mut().read_to_string().ok())
                            .flatten()
                    });
                let blocks = agent
                    .get(&format!("{url}/blocks?limit=50"))
                    .call()
                    .ok()
                    .and_then(|mut r| {
                        r.status()
                            .is_success()
                            .then(|| r.body_mut().read_to_string().ok())
                            .flatten()
                    });
                let mut height = 0u64;
                let mut root = String::new();
                let mut signals = 0u64;
                let mut error = String::new();
                match status {
                    Some(body) => {
                        height = field(&body, "height:")
                            .and_then(|h| h.parse().ok())
                            .unwrap_or(0);
                        root = field(&body, "bbg-root:").unwrap_or_default();
                        signals = field(&body, "signals:")
                            .and_then(|n| n.parse().ok())
                            .unwrap_or(0);
                    }
                    None => error = "chain unreachable".into(),
                }
                let mut rows = blocks.as_deref().map(parse_blocks).unwrap_or_default();
                let mut supply = rows.first().map(|r| r.supply).unwrap_or(0);
                if rows.is_empty() && height > 0 {
                    rows.push(BlockRow {
                        height,
                        time: 0,
                        supply,
                        weight: 0,
                    });
                } else if let Some(top) = rows.first() {
                    supply = top.supply;
                }
                let mut s = slot.lock().expect("oracle state");
                s.busy = false;
                s.version += 1;
                s.height = height;
                s.root = root;
                s.signals = signals;
                s.error = error;
                s.rows = rows;
                s.supply = supply;
            })
            .expect("spawn oracle-blocks");
    }

    fn open_block(&self, _url: String, _height: u64) {
        // Never GET /block/{h}. A lived-in height is megabytes of links;
        // ureq .call() waited on the body and froze the shell. The overlay
        // shows the list row we already have.
    }
}

fn parse_blocks(body: &str) -> Vec<BlockRow> {
    body.lines()
        .filter(|l| !l.starts_with('-') && !l.starts_with("particle:"))
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            Some(BlockRow {
                height: it.next()?.parse().ok()?,
                time: it.next()?.parse().ok()?,
                supply: it.next()?.parse().ok()?,
                weight: it.next()?.parse().ok()?,
            })
        })
        .collect()
}

fn field(body: &str, key: &str) -> Option<String> {
    body.lines()
        .find(|l| l.trim_start().starts_with(key))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string())
}

/// A lived-in block can carry tens of thousands of links. Reading the
/// whole body OOMs the process; keep a page's worth of both bytes and lines.
const BLOCK_BODY_CAP: usize = 8 * 1024;
const BLOCK_LINE_CAP: usize = 16;

fn read_capped(body: &mut ureq::Body, cap: usize) -> Option<String> {
    use std::io::Read;
    let mut reader = body.as_reader();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if buf.len() >= cap {
            break;
        }
        let n = reader.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        let take = n.min(cap - buf.len());
        buf.extend_from_slice(&chunk[..take]);
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn parse_block(body: &str) -> BlockDetail {
    let neuron = field(body, "neuron:").unwrap_or_default();
    let mut total = 0usize;
    let mut lines: Vec<String> = Vec::new();
    for l in body
        .lines()
        .filter(|l| l.starts_with("link ") || l.starts_with("pay "))
    {
        total += 1;
        if lines.len() < BLOCK_LINE_CAP {
            lines.push(l.to_string());
        }
    }
    if total > lines.len() {
        lines.push(format!("… {} more", total - lines.len()));
    }
    if neuron.is_empty() && lines.is_empty() {
        return BlockDetail {
            neuron,
            lines: vec!["no links or transfers".into()],
        };
    }
    BlockDetail { neuron, lines }
}

/// Which row (by height) is expanded — purely local UI state, never
/// shared with the fetch threads.
#[derive(Resource, Default)]
struct OracleUi {
    expanded: Option<u64>,
}

#[derive(Component)]
struct OracleRoot;

#[derive(Component)]
pub struct BlockPage(u64);

#[derive(Resource, Default)]
pub struct OpenBlock(pub Option<u64>);

#[derive(Component)]
struct OracleScroll;

#[derive(Component)]
struct RowButton(u64);

fn network_url(hub: &BodyLinkHub) -> Option<String> {
    crate::worlds::sigma::chain::chain_url(&hub.0)
}

fn hide_block_page(mut q: Query<&mut Visibility, With<BlockPage>>) {
    for mut vis in &mut q {
        *vis = Visibility::Hidden;
    }
}

fn enter(
    oracle: Res<Oracle>,
    hub: Option<Res<BodyLinkHub>>,
    commands: Commands,
    mut worlds: Query<(&crate::worlds::WorldUi, &mut Visibility)>,
) {
    if let Some(hub) = &hub {
        if let Some(url) = network_url(hub) {
            oracle.refresh_list(url);
        }
    }
    if crate::worlds::reveal_world(WorldState::Oracle, &mut worlds) {
        return;
    }
    build_page(
        commands,
        &oracle.try_list().unwrap_or_default(),
        &OracleUi::default(),
    );
}

/// A slow background refresh — block history does not change under you
/// the way a live balance does; this just keeps new blocks appearing.
fn poll_list(
    time: Res<Time>,
    mut wait: Local<f32>,
    oracle: Res<Oracle>,
    hub: Option<Res<BodyLinkHub>>,
) {
    *wait += time.delta_secs();
    if *wait < 12.0 {
        return;
    }
    *wait = 0.0;
    if oracle.0.try_lock().ok().map(|s| s.busy).unwrap_or(true) {
        return;
    }
    if let Some(hub) = &hub {
        if let Some(url) = network_url(hub) {
            oracle.refresh_list(url);
        }
    }
}

fn repaint(
    mut commands: Commands,
    oracle: Res<Oracle>,
    ui: Res<OracleUi>,
    open: Res<OpenBlock>,
    mut seen: Local<u64>,
    roots: Query<Entity, With<OracleRoot>>,
) {
    if open.0.is_some() {
        return;
    }
    let Some(snap) = oracle.try_list() else {
        return;
    };
    if snap.version == *seen && !ui.is_changed() {
        return;
    }
    *seen = snap.version;
    if !roots.is_empty() {
        return;
    }
    build_page(commands, &snap, &ui);
}

fn handle_table_press(
    q: Query<(&Interaction, &prysm::molecules::action::ActionButton), Changed<Interaction>>,
    mut open: ResMut<OpenBlock>,
    mut nav: ResMut<crate::worlds::nav::Nav>,
    now: Res<crate::now::Now>,
    world: Res<State<WorldState>>,
) {
    for (i, b) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        let Some(h) = b.target_ref.strip_prefix("block:") else {
            continue;
        };
        let Ok(height) = h.parse::<u64>() else {
            continue;
        };
        nav.push(crate::worlds::nav::Place::capture(
            *world.get(),
            &now,
            false,
        ));
        open.0 = Some(height);
    }
}

fn fill_block_page(
    mut commands: Commands,
    open: Res<OpenBlock>,
    oracle: Res<Oracle>,
    mut pages: Query<(&BlockPage, &mut Visibility)>,
) {
    let Some(h) = open.0 else {
        for (_, mut vis) in &mut pages {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
        return;
    };
    let mut have = false;
    for (page, mut vis) in &mut pages {
        if page.0 == h {
            have = true;
            if *vis != Visibility::Visible {
                *vis = Visibility::Visible;
            }
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
    if !have {
        let row = oracle
            .0
            .try_lock()
            .ok()
            .and_then(|s| s.rows.iter().find(|r| r.height == h).cloned());
        spawn_overlay(&mut commands, h, row.as_ref());
    }
}

fn spawn_overlay(commands: &mut Commands, h: u64, row: Option<&BlockRow>) {
    let root = commands
        .spawn((
            BlockPage(h),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(CHROME_TOP_H),
                bottom: Val::Px(CHROME_BOTTOM_H),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::G * 3.0)),
                row_gap: Val::Px(theme::G),
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
            GlobalZIndex(12),
        ))
        .id();
    let line = |commands: &mut Commands, parent: Entity, s: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(s),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            ChildOf(parent),
        ));
    };
    line(
        commands,
        root,
        format!("block {h}"),
        theme::H2,
        theme::ACID_GREEN,
    );
    if let Some(r) = row {
        line(
            commands,
            root,
            format!(
                "{}   supply {}   wt {}",
                time_text(r.time),
                r.supply,
                r.weight
            ),
            theme::BODY,
            theme::TEXT_PRIMARY,
        );
    }
}

fn short_hex(hex: &str) -> String {
    if hex.len() <= 12 {
        hex.to_string()
    } else {
        format!("{}..{}", &hex[..8], &hex[hex.len() - 4..])
    }
}

fn hashrate_text(rows: &[BlockRow], i: usize) -> String {
    let row = &rows[i];
    if row.weight == 0 {
        return "-".into();
    }
    // rows are newest-first: the previous block in wall-clock time is the
    // NEXT entry in this slice.
    let Some(prev) = rows.get(i + 1) else {
        return "-".into();
    };
    let dt = row.time.saturating_sub(prev.time).max(1);
    format!("{:.2}/s", row.weight as f64 / dt as f64)
}

fn time_text(secs: u64) -> String {
    if secs == 0 {
        return "-".into();
    }
    chrono::DateTime::from_timestamp(secs as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "-".into())
}

fn build_page(mut commands: Commands, snap: &OracleState, _ui: &OracleUi) {
    let root = commands
        .spawn((
            OracleRoot,
            crate::worlds::WorldUi(WorldState::Oracle),
            ContentRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(CHROME_TOP_H),
                bottom: Val::Px(CHROME_BOTTOM_H),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
        ))
        .id();

    let page = commands
        .spawn((
            OracleScroll,
            Node {
                height: Val::Percent(100.0),
                ..super::page::scroll_column()
            },
            ScrollPosition::default(),
            ChildOf(root),
        ))
        .id();

    let text = |commands: &mut Commands, parent: Entity, s: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(s),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            ChildOf(parent),
        ));
    };

    if !snap.error.is_empty() {
        text(
            &mut commands,
            page,
            format!("! {}", snap.error),
            theme::CAPTION,
            theme::ACID_RED,
        );
    }

    let body = super::cell::list(
        snap.rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                super::cell::row(&[
                    &row.height.to_string(),
                    &time_text(row.time),
                    &row.supply.to_string(),
                    &hashrate_text(&snap.rows, i),
                    &format!("block:{}", row.height),
                ])
            })
            .collect(),
    );
    let mut host = OracleHost {
        height: super::cell::exact(snap.height),
        supply: super::cell::exact(snap.supply),
        signals: super::cell::exact(snap.signals),
        rows: body,
    };
    match super::cell::load("oracle").and_then(|src| super::cell::eval(&src, &mut host)) {
        Ok(chunks) => super::cell::dispatch_page(&mut commands, page, &chunks),
        Err(e) => text(&mut commands, page, e, theme::CAPTION, theme::ACID_RED),
    }
}

fn scroll_page(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    touches: Res<Touches>,
    mut q: Query<(&mut ScrollPosition, &ComputedNode), With<OracleScroll>>,
) {
    let mut dy: f32 = wheel.read().map(|e| -e.y * 40.0).sum();
    let live: Vec<&bevy::input::touch::Touch> = touches.iter().collect();
    if live.len() == 1 {
        dy -= live[0].delta().y;
    }
    if dy == 0.0 {
        return;
    }
    for (mut pos, computed) in &mut q {
        let content = computed.content_size().y * computed.inverse_scale_factor();
        let view = computed.size().y * computed.inverse_scale_factor();
        let max = (content - view).max(0.0);
        pos.y = (pos.y + dy).clamp(0.0, max);
    }
}
