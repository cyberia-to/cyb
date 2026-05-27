use bevy::prelude::*;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::worlds::WorldState;

pub struct TrayPlugin;

struct TrayState {
    _tray:       tray_icon::TrayIcon,
    graph_id:    String,
    terminal_id: String,
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

    let item_graph    = MenuItem::new("Graph (Cmd+1)", true, None);
    let item_terminal = MenuItem::new("Terminal (Cmd+2)", true, None);
    let separator     = PredefinedMenuItem::separator();
    let item_quit     = MenuItem::new("Quit", true, None);

    let graph_id    = item_graph.id().as_ref().to_string();
    let terminal_id = item_terminal.id().as_ref().to_string();
    let quit_id     = item_quit.id().as_ref().to_string();

    let _ = menu.append(&item_graph);
    let _ = menu.append(&item_terminal);
    let _ = menu.append(&separator);
    let _ = menu.append(&item_quit);

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
        graph_id,
        terminal_id,
        quit_id,
    });
}

fn poll_tray_events(
    tray:           NonSend<TrayState>,
    current_state:  Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
    mut exit:       MessageWriter<AppExit>,
) {
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        let id = event.id.as_ref();

        if id == tray.quit_id {
            exit.write(AppExit::Success);
            return;
        }

        let target = if id == tray.graph_id {
            WorldState::Graph
        } else if id == tray.terminal_id {
            WorldState::Terminal
        } else {
            continue;
        };

        if *current_state.get() != target {
            next_state.set(target);
        }
    }
}
