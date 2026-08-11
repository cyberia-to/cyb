use crate::worlds::WorldState;
use bevy::prelude::*;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};

pub struct HotkeysPlugin;

struct HotkeyManagerRes {
    _manager: GlobalHotKeyManager,
    graph_id: u32,
    terminal_id: u32,
    cell_id: u32,
    sigma_id: u32,
}

impl Plugin for HotkeysPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register_hotkeys)
            .add_systems(Update, poll_hotkey_events);
    }
}

fn register_hotkeys(world: &mut World) {
    let manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");

    let mods = Modifiers::SUPER;
    let hk_graph = HotKey::new(Some(mods), Code::Digit1);
    let hk_terminal = HotKey::new(Some(mods), Code::Digit2);
    let hk_cell = HotKey::new(Some(mods), Code::Digit3);
    let hk_sigma = HotKey::new(Some(mods), Code::Digit4);

    manager.register(hk_graph).expect("register Cmd+1");
    manager.register(hk_terminal).expect("register Cmd+2");
    manager.register(hk_cell).expect("register Cmd+3");
    manager.register(hk_sigma).expect("register Cmd+4");

    world.insert_non_send_resource(HotkeyManagerRes {
        _manager: manager,
        graph_id: hk_graph.id(),
        terminal_id: hk_terminal.id(),
        cell_id: hk_cell.id(),
        sigma_id: hk_sigma.id(),
    });
}

fn poll_hotkey_events(
    hotkeys: NonSend<HotkeyManagerRes>,
    current_state: Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
) {
    while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
        let target = if event.id == hotkeys.graph_id {
            WorldState::Graph
        } else if event.id == hotkeys.terminal_id {
            WorldState::Terminal
        } else if event.id == hotkeys.cell_id {
            WorldState::Cell
        } else if event.id == hotkeys.sigma_id {
            WorldState::Sigma
        } else {
            continue;
        };

        if *current_state.get() != target {
            info!("Hotkey → {:?}", target);
            next_state.set(target);
        }
    }
}
