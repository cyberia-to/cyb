use bevy::prelude::*;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use crate::worlds::WorldState;

pub struct HotkeysPlugin;

struct HotkeyManagerRes {
    _manager: GlobalHotKeyManager,
    ui_id: u32,
    game_id: u32,
    terminal_id: u32,
    browser_id: u32,
}

impl Plugin for HotkeysPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register_hotkeys)
            .add_systems(Update, poll_hotkey_events);
    }
}

fn register_hotkeys(world: &mut World) {
    let manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");

    // Use Cmd+1..4 on macOS (Super = Cmd key)
    let mods = Modifiers::SUPER;
    let hk_ui = HotKey::new(Some(mods), Code::Digit1);
    let hk_game = HotKey::new(Some(mods), Code::Digit2);
    let hk_terminal = HotKey::new(Some(mods), Code::Digit3);
    let hk_browser = HotKey::new(Some(mods), Code::Digit4);

    manager.register(hk_ui).expect("register Cmd+1");
    manager.register(hk_game).expect("register Cmd+2");
    manager.register(hk_terminal).expect("register Cmd+3");
    manager.register(hk_browser).expect("register Cmd+4");

    world.insert_non_send_resource(HotkeyManagerRes {
        _manager: manager,
        ui_id: hk_ui.id(),
        game_id: hk_game.id(),
        terminal_id: hk_terminal.id(),
        browser_id: hk_browser.id(),
    });
}

fn poll_hotkey_events(
    hotkeys: NonSend<HotkeyManagerRes>,
    current_state: Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
) {
    while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
        let target = if event.id == hotkeys.ui_id {
            WorldState::Ui
        } else if event.id == hotkeys.game_id {
            WorldState::Game
        } else if event.id == hotkeys.terminal_id {
            WorldState::Terminal
        } else if event.id == hotkeys.browser_id {
            WorldState::Browser
        } else {
            continue;
        };

        if *current_state.get() != target {
            info!("Hotkey: switching to {:?}", target);
            next_state.set(target);
        }
    }
}
