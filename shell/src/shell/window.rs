//! Window lifecycle — the window is a view onto the robot, not the robot itself.
//!
//! Closing the window hides it; the process keeps running behind the tray icon,
//! so a stray Cmd+W or a misdirected click while switching apps costs a redraw
//! rather than the session. The process ends on explicit intent only: tray →
//! Quit, which writes `AppExit` (see `shell/tray.rs`).

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowCloseRequested};

/// Request to bring the primary window back on screen. Written by the tray.
#[derive(Message)]
pub struct ShowWindow;

pub struct WindowLifecyclePlugin;

/// Frames left to wait before focusing a freshly shown window. winit applies
/// `focus_window` before `set_visible` within one frame, so focusing on the
/// same frame as the show is a no-op — it has to land on a later frame.
#[derive(Resource, Default)]
struct FocusIn(u8);

const FOCUS_DELAY_FRAMES: u8 = 2;

impl Plugin for WindowLifecyclePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ShowWindow>()
            .init_resource::<FocusIn>()
            .add_systems(Update, (hide_on_close, show_on_request, focus_when_due).chain());
    }
}

fn hide_on_close(
    mut closed: MessageReader<WindowCloseRequested>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if closed.read().next().is_none() {
        return;
    }
    closed.clear();
    if window.visible {
        window.visible = false;
        info!("Window hidden — cyb keeps running in the tray");
    }
}

fn show_on_request(
    mut show: MessageReader<ShowWindow>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut focus_in: ResMut<FocusIn>,
) {
    if show.read().next().is_none() {
        return;
    }
    show.clear();
    if !window.visible {
        window.visible = true;
    }
    focus_in.0 = FOCUS_DELAY_FRAMES;
}

fn focus_when_due(
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut focus_in: ResMut<FocusIn>,
) {
    if focus_in.0 == 0 {
        return;
    }
    focus_in.0 -= 1;
    if focus_in.0 == 0 && window.visible {
        window.focused = true;
    }
}
