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

impl Plugin for OracleWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Oracle>()
            .init_resource::<OracleUi>()
            .add_systems(OnEnter(WorldState::Oracle), enter)
            .add_systems(
                Update,
                (poll_list, repaint, handle_row_press, scroll_page)
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
    fn snapshot(&self) -> OracleState {
        self.0.lock().map(|s| s.clone()).unwrap_or_default()
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
                let mut s = slot.lock().expect("oracle state");
                s.busy = false;
                s.version += 1;
                match status {
                    Some(body) => {
                        let field = |key: &str| -> Option<String> {
                            body.lines()
                                .find(|l| l.trim_start().starts_with(key))
                                .and_then(|l| l.split_once(':'))
                                .map(|(_, v)| v.trim().to_string())
                        };
                        s.height = field("height:").and_then(|h| h.parse().ok()).unwrap_or(0);
                        s.root = field("bbg-root:").unwrap_or_default();
                        s.signals = field("signals:").and_then(|n| n.parse().ok()).unwrap_or(0);
                        s.error.clear();
                    }
                    None => s.error = "chain unreachable".into(),
                }
                if let Some(body) = blocks {
                    s.rows = parse_blocks(&body);
                    if let Some(top) = s.rows.first() {
                        s.supply = top.supply;
                    }
                }
            })
            .expect("spawn oracle-blocks");
    }

    fn open_block(&self, url: String, height: u64) {
        let slot = self.0.clone();
        std::thread::Builder::new()
            .name("oracle-block".into())
            .spawn(move || {
                let agent = super::body::networks::agent();
                let out = agent
                    .get(&format!("{url}/block/{height}"))
                    .call()
                    .ok()
                    .and_then(|mut r| r.body_mut().read_to_string().ok());
                let Some(body) = out else { return };
                if let Some(detail) = parse_block(&body) {
                    let mut s = slot.lock().expect("oracle state");
                    s.details.insert(height, detail);
                    s.version += 1;
                }
            })
            .expect("spawn oracle-block");
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

fn parse_block(body: &str) -> Option<BlockDetail> {
    let neuron = field(body, "neuron:")?;
    let lines = body
        .lines()
        .filter(|l| l.starts_with("link ") || l.starts_with("pay "))
        .map(|l| l.to_string())
        .collect();
    Some(BlockDetail { neuron, lines })
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
struct OracleScroll;

#[derive(Component)]
struct RowButton(u64);

fn network_url(hub: &BodyLinkHub) -> Option<String> {
    crate::worlds::sigma::chain::chain_url(&hub.0)
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
    build_page(commands, &oracle.snapshot(), &OracleUi::default());
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
    if oracle.snapshot().busy {
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
    mut seen: Local<u64>,
    roots: Query<Entity, With<OracleRoot>>,
) {
    let snap = oracle.snapshot();
    if snap.version == *seen && !ui.is_changed() {
        return;
    }
    *seen = snap.version;
    for e in &roots {
        commands.entity(e).despawn();
    }
    build_page(commands, &snap, &ui);
}

fn handle_row_press(
    interactions: Query<(&Interaction, &RowButton), Changed<Interaction>>,
    mut ui: ResMut<OracleUi>,
    oracle: Res<Oracle>,
    hub: Option<Res<BodyLinkHub>>,
) {
    for (interaction, row) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let height = row.0;
        if ui.expanded == Some(height) {
            ui.expanded = None;
        } else {
            ui.expanded = Some(height);
            if !oracle.snapshot().details.contains_key(&height) {
                if let Some(hub) = &hub {
                    if let Some(url) = network_url(hub) {
                        oracle.open_block(url, height);
                    }
                }
            }
        }
    }
}

fn short_hex(hex: &str) -> String {
    let total = hex.chars().count();
    if total <= 12 {
        hex.to_string()
    } else {
        let head: String = hex.chars().take(8).collect();
        let tail: String = hex.chars().skip(total - 4).collect();
        format!("{}..{}", head, tail)
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

fn build_page(mut commands: Commands, snap: &OracleState, ui: &OracleUi) {
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
                width: Val::Percent(100.0),
                max_width: Val::Px(theme::MEASURE),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::G * 3.0)),
                row_gap: Val::Px(theme::G),
                overflow: Overflow::scroll_y(),
                ..default()
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

    text(
        &mut commands,
        page,
        "oracle".into(),
        theme::H2,
        theme::TEXT_PRIMARY,
    );

    if snap.height > 0 {
        let root = if snap.root.len() > 12 {
            format!("{}..{}", &snap.root[..8], &snap.root[snap.root.len() - 4..])
        } else {
            snap.root.clone()
        };
        text(
            &mut commands,
            page,
            format!(
                "height {}   supply {}   signals {}   root {}",
                snap.height, snap.supply, snap.signals, root
            ),
            theme::BODY,
            theme::ACID_GREEN,
        );
    }

    if !snap.error.is_empty() {
        text(
            &mut commands,
            page,
            format!("! {}", snap.error),
            theme::CAPTION,
            theme::ACID_RED,
        );
    }
    if snap.rows.is_empty() {
        let msg = if snap.busy {
            "asking the chain...".to_string()
        } else {
            "no recorded blocks yet - oracle only sees blocks finalized \
             since this feature shipped, nothing before is invented"
                .to_string()
        };
        text(&mut commands, page, msg, theme::CAPTION, theme::TEXT_DIM);
        return;
    }

    let header = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G * 0.5)),
                ..default()
            },
            ChildOf(page),
        ))
        .id();
    for h in ["height", "time", "hashrate", "supply"] {
        text(
            &mut commands,
            header,
            h.into(),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
    }

    for (i, row) in snap.rows.iter().enumerate() {
        let r = commands
            .spawn((
                RowButton(row.height),
                Button,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G * 0.6)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::DARK_BASE),
                BorderColor::all(theme::BORDER),
                ChildOf(page),
            ))
            .id();

        let top = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                ChildOf(r),
            ))
            .id();
        text(
            &mut commands,
            top,
            format!("{}", row.height),
            theme::BODY,
            theme::ACID_GREEN,
        );
        text(
            &mut commands,
            top,
            time_text(row.time),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
        text(
            &mut commands,
            top,
            hashrate_text(&snap.rows, i),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
        text(
            &mut commands,
            top,
            format!("{}", row.supply),
            theme::CAPTION,
            theme::TEXT_DIM,
        );

        if ui.expanded == Some(row.height) {
            match snap.details.get(&row.height) {
                Some(d) => {
                    text(
                        &mut commands,
                        r,
                        format!("cast by {}", short_hex(&d.neuron)),
                        theme::CAPTION,
                        theme::TEXT_DIM,
                    );
                    if d.lines.is_empty() {
                        text(
                            &mut commands,
                            r,
                            "no links or transfers in this block".into(),
                            theme::CAPTION,
                            theme::TEXT_DIM,
                        );
                    }
                    for line in &d.lines {
                        text(
                            &mut commands,
                            r,
                            line.clone(),
                            theme::CAPTION,
                            theme::TEXT_PRIMARY,
                        );
                    }
                }
                None => text(
                    &mut commands,
                    r,
                    "loading...".into(),
                    theme::CAPTION,
                    theme::TEXT_DIM,
                ),
            }
        }
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

#[cfg(test)]
mod tests {
    use super::short_hex;

    #[test]
    fn short_hex_passes_through_at_or_under_twelve() {
        assert_eq!(short_hex("abcd1234"), "abcd1234");
        assert_eq!(short_hex("abcd1234efgh"), "abcd1234efgh");
    }

    #[test]
    fn short_hex_truncates_ascii() {
        assert_eq!(short_hex("abcd1234ef567890"), "abcd1234..7890");
    }

    #[test]
    fn short_hex_does_not_panic_on_multibyte_head() {
        // a multi-byte char inside the first 8 bytes: byte-index slicing at
        // `hex[..8]` would land mid-character and panic before this fix.
        let hex = "中234567890abcdef";
        let _ = short_hex(hex);
    }

    #[test]
    fn short_hex_does_not_panic_on_multibyte_tail() {
        // a multi-byte char inside the trailing 4 bytes: byte-index slicing
        // at `hex[hex.len() - 4..]` would land mid-character and panic
        // before this fix. `d.neuron` (oracle/mod.rs:505) is parsed
        // straight out of an untrusted hub HTTP response body.
        let hex = "abcdef1234567890中";
        let _ = short_hex(hex);
    }
}
