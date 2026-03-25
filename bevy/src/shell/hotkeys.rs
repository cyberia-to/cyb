use bevy::prelude::*;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use crate::worlds::WorldState;

pub struct HotkeysPlugin;

struct HotkeyManagerRes {
    _manager: GlobalHotKeyManager,
    terminal_id: u32,
    portal_id: u32,
    legacy_id: u32,
    interface_id: u32,
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
    let hk_terminal = HotKey::new(Some(mods), Code::Digit1);
    let hk_portal = HotKey::new(Some(mods), Code::Digit2);
    let hk_legacy = HotKey::new(Some(mods), Code::Digit3);
    let hk_interface = HotKey::new(Some(mods), Code::Digit4);

    manager.register(hk_terminal).expect("register Cmd+1");
    manager.register(hk_portal).expect("register Cmd+2");
    manager.register(hk_legacy).expect("register Cmd+3");
    manager.register(hk_interface).expect("register Cmd+4");

    world.insert_non_send_resource(HotkeyManagerRes {
        _manager: manager,
        terminal_id: hk_terminal.id(),
        portal_id: hk_portal.id(),
        legacy_id: hk_legacy.id(),
        interface_id: hk_interface.id(),
    });
}

fn poll_hotkey_events(
    hotkeys: NonSend<HotkeyManagerRes>,
    current_state: Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
) {
    while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
        let target = if event.id == hotkeys.terminal_id {
            WorldState::Terminal
        } else if event.id == hotkeys.portal_id {
            WorldState::Portal
        } else if event.id == hotkeys.legacy_id {
            WorldState::Legacy
        } else if event.id == hotkeys.interface_id {
            WorldState::Interface
        } else {
            continue;
        };

        if *current_state.get() != target {
            info!("Hotkey: switching from {:?} to {:?}", current_state.get(), target);
            next_state.set(target);
        }
    }
}
