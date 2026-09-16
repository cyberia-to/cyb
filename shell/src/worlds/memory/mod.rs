//! memory — the file manager for particles.
//!
//! Every particle this body holds text for, one row each, ranked by the
//! same tru φ* focus brain draws by (`super::graph::BrainIndex` — one
//! computation, two consumers), with its size in bytes and when it was
//! last remembered. Tap a row to read it: the exact fullscreen page brain
//! opens on a tap, landed on without ever touching the graph itself.

use bevy::prelude::*;
use mir::bevy::resources::WarpTarget;
use prysm::theme;

use super::graph::BrainIndex;
use super::{SharedCell, WorldState, content};
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H, ContentRoot};

pub struct MemoryWorldPlugin;

#[derive(Component)]
struct MemoryRoot;

#[derive(Component)]
struct MemoryScroll;

/// Tap the row: open the same particle brain would show for this node.
#[derive(Component)]
struct OpenRow {
    idx: usize,
    hash: [u8; 32],
}

impl Plugin for MemoryWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldState::Memory), enter)
            .add_systems(
                Update,
                (refresh_on_index, finger).run_if(in_state(WorldState::Memory)),
            );
    }
}

fn enter(
    commands: Commands,
    index: Option<Res<BrainIndex>>,
    shared: Res<SharedCell>,
    mut worlds: Query<(&crate::worlds::WorldUi, &mut Visibility)>,
) {
    if crate::worlds::reveal_world(WorldState::Memory, &mut worlds) {
        return;
    }
    build_page(commands, index, shared);
}

/// Rebuild whenever the graph's own index moves — which includes the very
/// first frames after a `CYB_WORLD=memory` boot, where OnEnter fired
/// before graph's Startup pass had computed anything: the page was built
/// against an empty index and would otherwise stay empty forever.
fn refresh_on_index(
    mut commands: Commands,
    index: Option<Res<BrainIndex>>,
    shared: Res<SharedCell>,
    roots: Query<Entity, With<MemoryRoot>>,
    mut last: Local<Option<(usize, Option<[u8; 32]>, Option<[u8; 32]>)>>,
) {
    let Some(ref idx) = index else { return };
    // Focus floats change every tri-kernel run; tearing the list down
    // for that is the flash when you surf memory ↔ brain. Rebuild only
    // when the set of particles actually moved.
    let fp = (
        idx.hashes.len(),
        idx.hashes.first().copied(),
        idx.hashes.last().copied(),
    );
    if last.is_none() && !roots.is_empty() {
        *last = Some(fp);
        return;
    }
    if Some(fp) == *last && !roots.is_empty() {
        return;
    }
    if !idx.is_changed() && !roots.is_empty() {
        *last = Some(fp);
        return;
    }
    *last = Some(fp);
    if roots.is_empty() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    build_page(commands, index, shared);
}

/// One ranked row: hash, label, focus, byte size, and when it was last
/// remembered — `None` when the store never carried a `created` stamp for
/// this particle (the graph's own label-only guesses, or lines soma-kernel
/// wrote before the field existed).
struct Row {
    idx: usize,
    hash: [u8; 32],
    label: String,
    focus: f32,
    size: usize,
    created: Option<u64>,
}

fn ranked_rows(index: &BrainIndex) -> Vec<Row> {
    let meta = content::load_with_meta();
    let mut rows: Vec<Row> = index
        .hashes
        .iter()
        .enumerate()
        .map(|(idx, hash)| {
            let m = meta.get(hash);
            let label = m
                .map(|m| m.text.clone())
                .or_else(|| index.labels.get(idx).cloned().flatten())
                .unwrap_or_else(|| short_hex(hash));
            Row {
                idx,
                hash: *hash,
                label,
                focus: index.focus.get(idx).copied().unwrap_or(0.0),
                size: m.map(|m| m.text.len()).unwrap_or(0),
                created: m.and_then(|m| m.created),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.focus
            .partial_cmp(&a.focus)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn short_hex(hash: &[u8; 32]) -> String {
    hash[..4].iter().map(|b| format!("{b:02x}")).collect()
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
    match created {
        None => "-".into(),
        Some(secs) => chrono::DateTime::from_timestamp(secs as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".into()),
    }
}

fn compact(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}

fn bytes_text(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1} MB", n as f64 / 1e6)
    } else if n >= 1000 {
        format!("{:.1} KB", n as f64 / 1e3)
    } else {
        format!("{n} B")
    }
}

fn spawn_stat(commands: &mut Commands, parent: Entity, value: String, caption: &'static str) {
    let (fill, hair) = prysm::glass(prysm::GlassDepth::Foreground);
    let card = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(Val::Px(theme::G), Val::Px(theme::G * 1.5)),
                border: UiRect::all(Val::Px(1.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            fill,
            hair,
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        Text::new(value),
        TextFont {
            font_size: theme::H2,
            ..default()
        },
        TextColor(theme::ACID_GREEN),
        ChildOf(card),
    ));
    commands.spawn((
        Text::new(caption),
        TextFont {
            font_size: theme::MICRO,
            ..default()
        },
        TextColor(theme::TEXT_DIM),
        ChildOf(card),
    ));
}

fn build_page(mut commands: Commands, index: Option<Res<BrainIndex>>, shared: Res<SharedCell>) {
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
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
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

    let Some(index) = index else {
        text(
            &mut commands,
            root,
            "the graph has not opened yet".into(),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
        return;
    };

    let rows = ranked_rows(&index);
    let bytes: u64 = rows.iter().map(|r| r.size as u64).sum();
    let particles = rows.len() as u64;
    let links = shared
        .cell
        .lock()
        .map(|c| c.axons().len() as u64)
        .unwrap_or(0);

    let stats = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(theme::MEASURE),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(theme::G),
                padding: UiRect::new(
                    Val::Px(theme::G * 3.0),
                    Val::Px(theme::G * 1.5),
                    Val::Px(theme::G * 3.0),
                    Val::Px(theme::G * 2.0),
                ),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    spawn_stat(&mut commands, stats, compact(particles), "particles");
    spawn_stat(&mut commands, stats, bytes_text(bytes), "bytes");
    spawn_stat(&mut commands, stats, compact(links), "links");

    let page = commands
        .spawn((
            MemoryScroll,
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(theme::MEASURE),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                padding: UiRect::new(
                    Val::Px(theme::G * 3.0),
                    Val::Px(theme::G * 3.0),
                    Val::Px(0.0),
                    Val::Px(theme::G * 3.0),
                ),
                row_gap: Val::Px(theme::G),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            ChildOf(root),
        ))
        .id();

    if rows.is_empty() {
        text(
            &mut commands,
            page,
            "nothing remembered yet".into(),
            theme::BODY,
            theme::TEXT_DIM,
        );
        return;
    }

    for row in &rows {
        let r = commands
            .spawn((
                OpenRow {
                    idx: row.idx,
                    hash: row.hash,
                },
                Button,
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G)),
                    border: UiRect::all(Val::Px(1.0)),
                    column_gap: Val::Px(theme::G),
                    ..default()
                },
                BackgroundColor(theme::DARK_BASE),
                BorderColor::all(theme::BORDER),
                ChildOf(page),
            ))
            .id();

        let left = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(theme::G * 1.5),
                    ..default()
                },
                ChildOf(r),
            ))
            .id();
        let mut label: String = row.label.chars().take(40).collect();
        if row.label.chars().count() > 40 {
            label.push_str("..");
        }
        text(&mut commands, left, label, theme::BODY, theme::TEXT_PRIMARY);
        text(
            &mut commands,
            left,
            format!("focus {:.3}", row.focus),
            theme::CAPTION,
            theme::TEXT_DIM,
        );

        let right = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(theme::G * 1.5),
                    ..default()
                },
                ChildOf(r),
            ))
            .id();
        text(
            &mut commands,
            right,
            size_text(row.size),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
        text(
            &mut commands,
            right,
            date_text(row.created),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
    }
}

/// A tap is a press that did not travel. Drag is scroll. Opening used to
/// fire on Button-Pressed, so the first finger-down jumped to brain and
/// the list never moved.
const TAP_SLOP_PX: f32 = 12.0;
/// Finger delta multiplier — the list should outrun the thumb a little.
const DRAG_GAIN: f32 = 1.75;
const FRICTION: f32 = 3.6;
const FLING_MIN: f32 = 90.0;
const V_MAX: f32 = 14_000.0;

struct Gesture {
    start: Vec2,
    last: Vec2,
    scrolling: bool,
    row: Option<(usize, [u8; 32])>,
}

#[derive(Default)]
struct Fling {
    v: f32,
}

fn extents(computed: &ComputedNode) -> f32 {
    let content = computed.content_size().y * computed.inverse_scale_factor();
    let view = computed.size().y * computed.inverse_scale_factor();
    (content - view).max(0.0)
}

fn nudge(pos: &mut f32, dy: f32, max: f32) {
    let next = *pos + dy;
    *pos = if next < 0.0 {
        next * 0.38
    } else if next > max {
        max + (next - max) * 0.38
    } else {
        next
    };
}

fn spring_back(pos: &mut f32, v: &mut f32, max: f32, dt: f32) {
    if *pos < 0.0 {
        *pos += (0.0 - *pos) * (1.0 - (-18.0 * dt).exp());
        *v *= 0.35;
        if pos.abs() < 0.5 {
            *pos = 0.0;
        }
    } else if *pos > max {
        *pos += (max - *pos) * (1.0 - (-18.0 * dt).exp());
        *v *= 0.35;
        if (*pos - max).abs() < 0.5 {
            *pos = max;
        }
    }
}

fn finger(
    mut g: Local<Option<Gesture>>,
    mut fling: Local<Fling>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
    windows: Query<&Window>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    rows: Query<(&Interaction, &OpenRow)>,
    mut scroll: Query<(&mut ScrollPosition, &ComputedNode), With<MemoryScroll>>,
    mut commands: Commands,
    mut next: ResMut<NextState<WorldState>>,
    warp: Option<ResMut<WarpTarget>>,
) {
    let dt = time.delta_secs().max(1.0 / 240.0);
    let mut dy: f32 = wheel.read().map(|e| -e.y * 72.0).sum();

    let Ok(window) = windows.single() else {
        return;
    };
    let mouse_pos = window.cursor_position();
    let down_pos = touches.iter().next().map(|t| t.position()).or_else(|| {
        mouse
            .pressed(MouseButton::Left)
            .then_some(())
            .and_then(|_| mouse_pos)
    });
    let released_pos = touches
        .iter_just_released()
        .next()
        .map(|t| t.position())
        .or_else(|| {
            mouse
                .just_released(MouseButton::Left)
                .then_some(())
                .and_then(|_| mouse_pos)
        });

    let dragging = down_pos.is_some();

    if let Some(pos) = down_pos {
        if g.is_none() {
            let row = rows
                .iter()
                .find_map(|(i, r)| (*i == Interaction::Pressed).then_some((r.idx, r.hash)));
            *g = Some(Gesture {
                start: pos,
                last: pos,
                scrolling: false,
                row,
            });
        }
        if let Some(g) = g.as_mut() {
            let delta = pos - g.last;
            g.last = pos;
            if (pos - g.start).length() > TAP_SLOP_PX {
                g.scrolling = true;
            }
            if g.scrolling {
                let step = -delta.y * DRAG_GAIN;
                dy += step;
                let inst = step / dt;
                fling.v = (fling.v * 0.45 + inst * 0.55).clamp(-V_MAX, V_MAX);
            }
        }
    }

    if dragging {
        if dy != 0.0 {
            for (mut pos, computed) in &mut scroll {
                let max = extents(computed);
                nudge(&mut pos.y, dy, max);
            }
        }
    } else {
        if dy != 0.0 {
            fling.v += dy / dt;
        }
        if fling.v.abs() > FLING_MIN {
            let step = fling.v * dt;
            fling.v *= (-FRICTION * dt).exp();
            for (mut pos, computed) in &mut scroll {
                let max = extents(computed);
                nudge(&mut pos.y, step, max);
                spring_back(&mut pos.y, &mut fling.v, max, dt);
            }
        } else {
            fling.v = 0.0;
            for (mut pos, computed) in &mut scroll {
                let max = extents(computed);
                let mut v = 0.0;
                spring_back(&mut pos.y, &mut v, max, dt);
            }
        }
    }

    if let Some(pos) = released_pos {
        let Some(g) = g.take() else { return };
        if g.scrolling || (pos - g.start).length() > TAP_SLOP_PX {
            if fling.v.abs() < FLING_MIN {
                fling.v = 0.0;
            }
            return;
        }
        fling.v = 0.0;
        let Some((idx, hash)) = g.row.or_else(|| {
            rows.iter().find_map(|(i, r)| {
                (*i == Interaction::Pressed || *i == Interaction::Hovered)
                    .then_some((r.idx, r.hash))
            })
        }) else {
            return;
        };
        if let Some(mut warp) = warp {
            warp.particle_idx = Some(idx as u32);
        }
        super::viewer::open(&mut commands, idx, hash);
        next.set(WorldState::Graph);
    } else if down_pos.is_none() {
        *g = None;
    }
}
