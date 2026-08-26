use bevy::prelude::*;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::shell::window::ShowWindow;
use crate::worlds::WorldState;

pub struct TrayPlugin;

struct TrayState {
    _tray:       tray_icon::TrayIcon,
    show_id:     String,
    graph_id:    String,
    terminal_id: String,
    cell_id:     String,
    sigma_id:    String,
    quit_id:     String,
}

impl Plugin for TrayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_tray)
            .add_systems(Update, poll_tray_events);
    }
}

fn create_tray(world: &mut World) {
    let menu = Menu::new();

    let item_show     = MenuItem::new("Show cyb", true, None);
    let item_graph    = MenuItem::new("Graph (Cmd+1)", true, None);
    let item_terminal = MenuItem::new("Terminal (Cmd+2)", true, None);
    let item_cell     = MenuItem::new("Landing (Cmd+3)", true, None);
    let item_sigma    = MenuItem::new("Sigma (Cmd+4)", true, None);
    let item_quit     = MenuItem::new("Quit cyb", true, None);

    let show_id     = item_show.id().as_ref().to_string();
    let graph_id    = item_graph.id().as_ref().to_string();
    let terminal_id = item_terminal.id().as_ref().to_string();
    let cell_id     = item_cell.id().as_ref().to_string();
    let sigma_id    = item_sigma.id().as_ref().to_string();
    let quit_id     = item_quit.id().as_ref().to_string();

    let _ = menu.append(&item_show);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&item_graph);
    let _ = menu.append(&item_terminal);
    let _ = menu.append(&item_cell);
    let _ = menu.append(&item_sigma);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&item_quit);

    let icon = Icon::from_rgba(eye_glyph_rgba(), GLYPH_PX, GLYPH_PX)
        .expect("Failed to create tray icon");

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("cyb")
        .with_icon(icon)
        // macOS renders a template image from its alpha channel alone, so the
        // glyph tracks the menu bar's light/dark theme and highlights on click.
        .with_icon_as_template(true)
        .build()
        .expect("Failed to create tray icon");

    world.insert_non_send_resource(TrayState {
        _tray: tray,
        show_id,
        graph_id,
        terminal_id,
        cell_id,
        sigma_id,
        quit_id,
    });
}

// ── the glyph ────────────────────────────────────────────────────────────────

/// 36px = 2× the 18pt height a macOS status item draws at, so the glyph is
/// pixel-exact on retina.
const GLYPH_PX: u32 = 36;
/// Supersampling factor per axis — 16 coverage samples per pixel.
const SAMPLES: u32 = 4;

// The eye is the intersection of two discs of radius LID_R centred at ±LID_C on
// the vertical axis, in a coordinate square spanning [-1, 1]. The pair
// (half-width 0.94, half-height 0.54) inverts to these two constants.
const LID_R: f32 = 1.0881;
const LID_C: f32 = 0.5481;
/// Lid stroke: the ring between the eye and the same shape shrunk by this much.
const LID_STROKE: f32 = 0.13;
const PUPIL_R: f32 = 0.30;

/// The cyb eye, drawn as alpha coverage: lid outline plus a solid pupil.
/// Colour is irrelevant — a template image carries shape in alpha only.
fn eye_glyph_rgba() -> Vec<u8> {
    let mut rgba = Vec::with_capacity((GLYPH_PX * GLYPH_PX * 4) as usize);
    for y in 0..GLYPH_PX {
        for x in 0..GLYPH_PX {
            rgba.extend_from_slice(&[0, 0, 0, coverage(x, y)]);
        }
    }
    rgba
}

/// Fraction of this pixel the glyph covers, as 0..=255.
fn coverage(px: u32, py: u32) -> u8 {
    let mut hits = 0u32;
    for sy in 0..SAMPLES {
        for sx in 0..SAMPLES {
            let x = to_unit(px, sx);
            let y = to_unit(py, sy);
            let on_lid = in_eye(x, y, LID_R) && !in_eye(x, y, LID_R - LID_STROKE);
            let on_pupil = x * x + y * y <= PUPIL_R * PUPIL_R;
            if on_lid || on_pupil {
                hits += 1;
            }
        }
    }
    ((hits * 255) / (SAMPLES * SAMPLES)) as u8
}

/// Pixel + subsample index → the [-1, 1] coordinate square.
fn to_unit(pixel: u32, sample: u32) -> f32 {
    let offset = (sample as f32 + 0.5) / SAMPLES as f32;
    (pixel as f32 + offset) / GLYPH_PX as f32 * 2.0 - 1.0
}

fn in_eye(x: f32, y: f32, r: f32) -> bool {
    let upper = x * x + (y - LID_C) * (y - LID_C);
    let lower = x * x + (y + LID_C) * (y + LID_C);
    upper <= r * r && lower <= r * r
}

// ── events ───────────────────────────────────────────────────────────────────

fn poll_tray_events(
    tray:           NonSend<TrayState>,
    current_state:  Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
    mut show:       MessageWriter<ShowWindow>,
    mut exit:       MessageWriter<AppExit>,
) {
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        let id = event.id.as_ref();

        if id == tray.quit_id {
            exit.write(AppExit::Success);
            return;
        }

        if id == tray.show_id {
            show.write(ShowWindow);
            continue;
        }

        let target = if id == tray.graph_id {
            WorldState::Graph
        } else if id == tray.terminal_id {
            WorldState::Terminal
        } else if id == tray.cell_id {
            WorldState::Cell
        } else if id == tray.sigma_id {
            WorldState::Sigma
        } else {
            continue;
        };

        // Picking a world from the tray is also a request to look at it.
        show.write(ShowWindow);
        if *current_state.get() != target {
            next_state.set(target);
        }
    }
}
