use bevy::prelude::*;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::worlds::WorldState;

pub struct TrayPlugin;

struct TrayState {
    _tray: tray_icon::TrayIcon,
    ui_id: String,
    game_id: String,
    terminal_id: String,
    browser_id: String,
    quit_id: String,
}

impl Plugin for TrayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_tray)
            .add_systems(Update, poll_tray_events);
    }
}

fn create_tray(world: &mut World) {
    let menu = Menu::new();

    let item_ui = MenuItem::new("UI (Cmd+1)", true, None);
    let item_game = MenuItem::new("Game (Cmd+2)", true, None);
    let item_terminal = MenuItem::new("Terminal (Cmd+3)", true, None);
    let item_browser = MenuItem::new("Browser (Cmd+4)", true, None);
    let separator = PredefinedMenuItem::separator();
    let item_quit = MenuItem::new("Quit", true, None);

    let ui_id = item_ui.id().as_ref().to_string();
    let game_id = item_game.id().as_ref().to_string();
    let terminal_id = item_terminal.id().as_ref().to_string();
    let browser_id = item_browser.id().as_ref().to_string();
    let quit_id = item_quit.id().as_ref().to_string();

    let _ = menu.append(&item_ui);
    let _ = menu.append(&item_game);
    let _ = menu.append(&item_terminal);
    let _ = menu.append(&item_browser);
    let _ = menu.append(&separator);
    let _ = menu.append(&item_quit);

    // 16x16 white square as placeholder icon
    let icon_rgba = vec![255u8; 16 * 16 * 4];
    let icon = Icon::from_rgba(icon_rgba, 16, 16).expect("Failed to create tray icon");

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("cyb")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");

    world.insert_non_send_resource(TrayState {
        _tray: tray,
        ui_id,
        game_id,
        terminal_id,
        browser_id,
        quit_id,
    });

    info!("Tray icon created");
}

fn poll_tray_events(
    tray: NonSend<TrayState>,
    current_state: Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
    mut exit: MessageWriter<AppExit>,
) {
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        let id = event.id.as_ref();

        if id == tray.quit_id {
            info!("Tray: quit requested");
            exit.write(AppExit::Success);
            return;
        }

        let target = if id == tray.ui_id {
            WorldState::Ui
        } else if id == tray.game_id {
            WorldState::Game
        } else if id == tray.terminal_id {
            WorldState::Terminal
        } else if id == tray.browser_id {
            WorldState::Browser
        } else {
            continue;
        };

        if *current_state.get() != target {
            info!("Tray: switching to {:?}", target);
            next_state.set(target);
        }
    }
}
