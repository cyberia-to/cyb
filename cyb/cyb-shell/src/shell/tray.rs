use bevy::prelude::*;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::worlds::WorldState;

pub struct TrayPlugin;

struct TrayState {
    _tray: tray_icon::TrayIcon,
    terminal_id: String,
    portal_id: String,
    legacy_id: String,
    interface_id: String,
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

    let item_terminal = MenuItem::new("Terminal (Cmd+1)", true, None);
    let item_portal = MenuItem::new("Portal (Cmd+2)", true, None);
    let item_legacy = MenuItem::new("Legacy (Cmd+3)", true, None);
    let item_interface = MenuItem::new("Interface (Cmd+4)", true, None);
    let separator = PredefinedMenuItem::separator();
    let item_quit = MenuItem::new("Quit", true, None);

    let terminal_id = item_terminal.id().as_ref().to_string();
    let portal_id = item_portal.id().as_ref().to_string();
    let legacy_id = item_legacy.id().as_ref().to_string();
    let interface_id = item_interface.id().as_ref().to_string();
    let quit_id = item_quit.id().as_ref().to_string();

    let _ = menu.append(&item_terminal);
    let _ = menu.append(&item_portal);
    let _ = menu.append(&item_legacy);
    let _ = menu.append(&item_interface);
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
        terminal_id,
        portal_id,
        legacy_id,
        interface_id,
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

        let target = if id == tray.terminal_id {
            WorldState::Terminal
        } else if id == tray.portal_id {
            WorldState::Portal
        } else if id == tray.legacy_id {
            WorldState::Legacy
        } else if id == tray.interface_id {
            WorldState::Interface
        } else {
            continue;
        };

        if *current_state.get() != target {
            info!("Tray: switching to {:?}", target);
            next_state.set(target);
        }
    }
}
