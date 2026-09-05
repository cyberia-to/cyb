//! The particle viewer: tap a node in brain, read the thing itself.
//!
//! brain draws the graph as geometry; this is the page you land on when a
//! node stops being geometry and becomes a question. A tap picks the
//! nearest particle under the finger (same projection the labels use),
//! the camera flies to it (mir's warp), and a fullscreen page opens over
//! the graph: the particle's whole text, its focus, and every axon it
//! hangs from — each one a door to the next particle.
//!
//! Reading is attention, and attention is a link: closing the page casts
//! `particle("brain") -> particle(viewed)` weighted by the seconds spent
//! looking, exactly the rule world transitions follow. The graph learns
//! what you actually read.

use std::time::Instant;

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use mir::bevy::resources::{GpuBuffers, GraphCamera, WarpTarget};
use prysm::theme;

use super::graph::BrainIndex;
use super::{content, identity::Identity, ComInbox, ComSay, SharedCell, WorldState};
use crate::shell::chrome::{ChromeState, CHROME_BOTTOM_H, CHROME_TOP_H};

pub struct ViewerPlugin;

/// The particle currently on the reading stand, and since when.
#[derive(Resource)]
struct Viewed {
    idx: usize,
    hash: [u8; 32],
    since: Instant,
}

#[derive(Component)]
struct ViewerRoot;

#[derive(Component)]
struct ViewerScroll;

#[derive(Component)]
struct CloseButton;

/// A neighbor row: press to walk the axon.
#[derive(Component)]
struct NeighborButton(usize);

/// Mouse press bookkeeping for tap-vs-drag: where and when it went down.
#[derive(Default)]
struct PressState(Option<(Vec2, Instant)>);

impl Plugin for ViewerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (detect_tap, open_viewer, handle_close, handle_neighbors, scroll_viewer)
                .run_if(in_state(WorldState::Graph)),
        )
        .add_systems(OnExit(WorldState::Graph), close_on_leave);

        // `CYB_TAP="512,400@10"` taps that logical point that many seconds
        // in — the scripted finger, same family as CYB_SHOT and CYB_TOUR.
        // It exercises the REAL pick path; only the pointer is synthetic.
        if let Ok(spec) = std::env::var("CYB_TAP") {
            if let Some((xy, at)) = spec.split_once('@') {
                if let (Some((x, y)), Ok(at)) = (
                    xy.split_once(',').and_then(|(a, b)| {
                        Some((a.trim().parse::<f32>().ok()?, b.trim().parse::<f32>().ok()?))
                    }),
                    at.trim().parse::<f32>(),
                ) {
                    app.insert_resource(TapScript { pos: Vec2::new(x, y), at, done: false });
                    app.add_systems(
                        Update,
                        scripted_tap.run_if(in_state(WorldState::Graph)),
                    );
                }
            }
        }
    }
}

/// The scripted finger: one tap, once, at the appointed second.
#[derive(Resource)]
struct TapScript {
    pos: Vec2,
    at: f32,
    done: bool,
}

fn scripted_tap(
    time: Res<Time>,
    mut script: ResMut<TapScript>,
    index: Option<Res<BrainIndex>>,
    gpu: Option<Res<GpuBuffers>>,
    cam: Option<Res<GraphCamera>>,
    warp: Option<ResMut<WarpTarget>>,
    mut commands: Commands,
) {
    if script.done || time.elapsed_secs() < script.at {
        return;
    }
    script.done = true;
    let (Some(index), Some(gpu), Some(cam)) = (index, gpu, cam) else { return };
    let in_view = Vec2::new(script.pos.x, script.pos.y - CHROME_TOP_H);
    let picked = pick(&index, &gpu, &cam, in_view);
    info!("viewer: scripted tap at {in_view:?} -> {picked:?}");
    if let Some(idx) = picked {
        if let Some(mut warp) = warp {
            warp.particle_idx = Some(idx as u32);
        }
        commands.insert_resource(Viewed { idx, hash: index.hashes[idx], since: Instant::now() });
    }
}

/// A tap is a press that neither travelled nor lingered. Everything else
/// belongs to the camera.
const TAP_SLOP_PX: f32 = 8.0;
const TAP_MAX_S: f32 = 0.4;
/// How close (logical px) the tap must land to a particle's center.
const PICK_RADIUS_PX: f32 = 18.0;

fn detect_tap(
    mut press: Local<PressState>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    viewer_open: Option<Res<Viewed>>,
    index: Option<Res<BrainIndex>>,
    gpu: Option<Res<GpuBuffers>>,
    cam: Option<Res<GraphCamera>>,
    warp: Option<ResMut<WarpTarget>>,
    mut commands: Commands,
) {
    // The page over the graph owns the pointer while it is open.
    if viewer_open.is_some() {
        return;
    }
    let (Some(index), Some(gpu), Some(cam)) = (index, gpu, cam) else { return };

    let mut tap: Option<Vec2> = None;

    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(w) = windows.single() {
            press.0 = w.cursor_position().map(|p| (p, Instant::now()));
        }
    }
    if mouse.just_released(MouseButton::Left) {
        if let (Some((start, at)), Ok(w)) = (press.0.take(), windows.single()) {
            if let Some(now) = w.cursor_position() {
                if (now - start).length() < TAP_SLOP_PX
                    && at.elapsed().as_secs_f32() < TAP_MAX_S
                {
                    tap = Some(now);
                }
            }
        }
    }
    for t in touches.iter_just_released() {
        if t.distance().length() < TAP_SLOP_PX {
            tap = Some(t.position());
        }
    }

    let Some(at) = tap else { return };
    // Chrome bands belong to the chrome.
    let [_, lh] = cam.input_viewport;
    if at.y < CHROME_TOP_H || at.y > lh + CHROME_TOP_H {
        return;
    }
    let in_view = Vec2::new(at.x, at.y - CHROME_TOP_H);

    let picked = pick(&index, &gpu, &cam, in_view);
    info!("viewer: tap at {in_view:?} -> {picked:?}");
    if let Some(idx) = picked {
        let hash = index.hashes[idx];
        if let Some(mut warp) = warp {
            // The camera flies to what you chose to read.
            warp.particle_idx = Some(idx as u32);
        }
        commands.insert_resource(Viewed { idx, hash, since: Instant::now() });
    }
}

/// Nearest particle to the tap in screen space — the same projection
/// `place_labels` draws with, plus a distance test. Returns None when the
/// tap lands on empty sky.
fn pick(index: &BrainIndex, gpu: &GpuBuffers, cam: &GraphCamera, at: Vec2) -> Option<usize> {
    let m = cam.view_proj();
    let [lw, lh] = cam.input_viewport;
    let mut best: Option<(usize, f32)> = None;

    for i in 0..index.hashes.len() {
        let base = i * 3;
        if base + 2 >= gpu.pos_cpu.len() {
            break;
        }
        let (x, y, z) = (gpu.pos_cpu[base], gpu.pos_cpu[base + 1], gpu.pos_cpu[base + 2]);
        let w = m[0][3] * x + m[1][3] * y + m[2][3] * z + m[3][3];
        if w <= 0.0 {
            continue;
        }
        let cx = (m[0][0] * x + m[1][0] * y + m[2][0] * z + m[3][0]) / w;
        let cy = (m[0][1] * x + m[1][1] * y + m[2][1] * z + m[3][1]) / w;
        let sx = (cx * 0.5 + 0.5) * lw;
        let sy = (1.0 - (cy * 0.5 + 0.5)) * lh;
        let d = Vec2::new(sx, sy).distance(at);

        // A big node is easier to hit than a small one: its projected
        // radius extends the touch target.
        let rad = gpu.rad_cpu.get(i).copied().unwrap_or(0.0);
        let screen_r = (rad * m[1][1] / w) * (lh * 0.5);
        let reach = PICK_RADIUS_PX.max(screen_r);

        if d < reach && best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

/// Build (or rebuild, when the viewed particle changes) the reading page.
fn open_viewer(
    mut commands: Commands,
    viewed: Option<Res<Viewed>>,
    index: Option<Res<BrainIndex>>,
    gpu: Option<Res<GpuBuffers>>,
    roots: Query<Entity, With<ViewerRoot>>,
) {
    let Some(viewed) = viewed else { return };
    if !viewed.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let (Some(index), Some(gpu)) = (index, gpu) else { return };

    let sidecar = content::load();
    let title = index
        .labels
        .get(viewed.idx)
        .cloned()
        .flatten()
        .unwrap_or_else(|| short_hex(&viewed.hash));
    let body = sidecar.get(&viewed.hash).cloned();

    let root = commands
        .spawn((
            ViewerRoot,
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
            // Above the graph image and its labels, below notices.
            GlobalZIndex(8),
        ))
        .id();

    let page = commands
        .spawn((
            ViewerScroll,
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(theme::MEASURE),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::G * 3.0)),
                row_gap: Val::Px(theme::G * 1.5),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            ChildOf(root),
        ))
        .id();

    // Header: the particle's name, and the way out.
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                column_gap: Val::Px(theme::G),
                ..default()
            },
            ChildOf(page),
        ))
        .id();
    commands.spawn((
        Text::new(title),
        TextFont { font_size: theme::H3, ..default() },
        TextColor(theme::TEXT_PRIMARY),
        Node { max_width: Val::Percent(85.0), ..default() },
        ChildOf(head),
    ));
    let close = commands
        .spawn((
            CloseButton,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G * 0.5)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
            BorderColor::all(theme::BORDER),
            ChildOf(head),
        ))
        .id();
    commands.spawn((
        Text::new("x"),
        TextFont { font_size: theme::BODY, ..default() },
        TextColor(theme::TEXT_DIM),
        ChildOf(close),
    ));

    // The particle's vitals: identity, rank, connectivity.
    let focus = index.focus.get(viewed.idx).copied().unwrap_or(0.0);
    let degree = gpu
        .csr
        .as_ref()
        .map(|csr| neighbors(csr, viewed.idx).count())
        .unwrap_or(0);
    commands.spawn((
        Text::new(format!(
            "{}   focus {:.4}   axons {}",
            short_hex(&viewed.hash),
            focus,
            degree
        )),
        TextFont { font_size: theme::CAPTION, ..default() },
        TextColor(theme::TEXT_DIM),
        ChildOf(page),
    ));

    // The content itself — the point of the page.
    match body {
        Some(text) => {
            commands.spawn((
                Text::new(text),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(theme::TEXT_PRIMARY),
                Node { max_width: Val::Percent(100.0), ..default() },
                ChildOf(page),
            ));
        }
        None => {
            commands.spawn((
                Text::new(
                    "no text aboard for this particle - the hash is known, \
                     the content has not landed here",
                ),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(theme::TEXT_DIM),
                ChildOf(page),
            ));
        }
    }

    // Axons: every neighbor is a door.
    if let Some(csr) = gpu.csr.as_ref() {
        let mut rows: Vec<(usize, f32)> = neighbors(csr, viewed.idx).collect();
        if !rows.is_empty() {
            rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            commands.spawn((
                Text::new("axons"),
                TextFont { font_size: theme::CAPTION, ..default() },
                TextColor(theme::TEXT_DIM),
                Node { margin: UiRect::top(Val::Px(theme::G)), ..default() },
                ChildOf(page),
            ));
            for (n_idx, weight) in rows.into_iter().take(24) {
                let name = index
                    .labels
                    .get(n_idx)
                    .cloned()
                    .flatten()
                    .or_else(|| index.hashes.get(n_idx).map(short_hex))
                    .unwrap_or_default();
                let row = commands
                    .spawn((
                        NeighborButton(n_idx),
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect::axes(
                                Val::Px(theme::G * 1.5),
                                Val::Px(theme::G * 0.75),
                            ),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(theme::DARK_BASE),
                        BorderColor::all(theme::BORDER),
                        ChildOf(page),
                    ))
                    .id();
                commands.spawn((
                    Text::new(name),
                    TextFont { font_size: theme::BODY, ..default() },
                    TextColor(theme::TEXT_PRIMARY),
                    ChildOf(row),
                ));
                commands.spawn((
                    Text::new(format!("{weight:.2}")),
                    TextFont { font_size: theme::CAPTION, ..default() },
                    TextColor(theme::TEXT_DIM),
                    ChildOf(row),
                ));
            }
        }
    }
}

/// Symmetric CSR: one row holds every axon this particle hangs from.
fn neighbors(csr: &mir::graph::Csr, idx: usize) -> impl Iterator<Item = (usize, f32)> + '_ {
    let (a, b) = (
        csr.row_ptr.get(idx).copied().unwrap_or(0) as usize,
        csr.row_ptr.get(idx + 1).copied().unwrap_or(0) as usize,
    );
    (a..b.min(csr.col_idx.len()))
        .map(|e| (csr.col_idx[e] as usize, csr.values.get(e).copied().unwrap_or(0.0)))
}

fn short_hex(hash: &[u8; 32]) -> String {
    let hex: String = hash[..4].iter().map(|b| format!("{b:02x}")).collect();
    format!("{hex}...")
}

/// Close on [x] or Escape; the dwell becomes a link before the page goes.
fn handle_close(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<CloseButton>)>,
    keys: Res<ButtonInput<KeyCode>>,
    chrome: Res<ChromeState>,
    viewed: Option<Res<Viewed>>,
    roots: Query<Entity, With<ViewerRoot>>,
    shared: Option<Res<SharedCell>>,
    who: Option<Res<Identity>>,
    inbox: Option<ResMut<ComInbox>>,
) {
    let Some(viewed) = viewed else { return };
    let pressed = interactions.iter().any(|i| *i == Interaction::Pressed);
    let escaped = keys.just_pressed(KeyCode::Escape) && !chrome.focused;
    if !pressed && !escaped {
        return;
    }
    cast_reading(&viewed, shared, who, inbox);
    for e in &roots {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<Viewed>();
}

/// Walking an axon re-points the page; the leg just finished is cast first.
fn handle_neighbors(
    interactions: Query<(&Interaction, &NeighborButton), Changed<Interaction>>,
    index: Option<Res<BrainIndex>>,
    viewed: Option<ResMut<Viewed>>,
    warp: Option<ResMut<WarpTarget>>,
    shared: Option<Res<SharedCell>>,
    who: Option<Res<Identity>>,
    inbox: Option<ResMut<ComInbox>>,
) {
    let (Some(index), Some(mut viewed)) = (index, viewed) else { return };
    for (i, b) in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        let Some(&hash) = index.hashes.get(b.0) else { continue };
        cast_reading(&viewed, shared, who, inbox);
        viewed.idx = b.0;
        viewed.hash = hash;
        viewed.since = Instant::now();
        if let Some(mut warp) = warp {
            warp.particle_idx = Some(b.0 as u32);
        }
        return; // one door per frame; the borrows above are spent
    }
}

/// Reading time becomes a weighted link, same rule as world dwell.
fn cast_reading(
    viewed: &Viewed,
    shared: Option<Res<SharedCell>>,
    who: Option<Res<Identity>>,
    inbox: Option<ResMut<ComInbox>>,
) {
    let (Some(shared), Some(who)) = (shared, who) else { return };
    let secs = viewed.since.elapsed().as_secs().max(1);
    content::remember("brain");
    let cast = {
        let mut cell = shared.cell.lock().expect("shared cell poisoned");
        cell.cast_weighted(who.neuron, [(content::particle_of("brain"), viewed.hash, secs)])
    };
    match cast {
        Ok(_) => {
            shared.bump();
            if let Some(mut inbox) = inbox {
                inbox.0.push(ComSay::Note(format!("read {} ({secs}s)", short_hex(&viewed.hash))));
            }
        }
        Err(e) => warn!("viewer: cast failed: {e:?}"),
    }
}

/// Leaving brain closes the page the same way [x] does — dwell cast included.
fn close_on_leave(
    mut commands: Commands,
    viewed: Option<Res<Viewed>>,
    roots: Query<Entity, With<ViewerRoot>>,
    shared: Option<Res<SharedCell>>,
    who: Option<Res<Identity>>,
    inbox: Option<ResMut<ComInbox>>,
) {
    if let Some(viewed) = viewed {
        cast_reading(&viewed, shared, who, inbox);
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<Viewed>();
}

/// Wheel or one finger moves the page; logical px, clamped to the overflow
/// (the com scroll lesson: ScrollPosition is logical, ComputedNode is not).
fn scroll_viewer(
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    viewer_open: Option<Res<Viewed>>,
    mut q: Query<(&mut ScrollPosition, &ComputedNode), With<ViewerScroll>>,
) {
    if viewer_open.is_none() {
        wheel.clear();
        return;
    }
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
