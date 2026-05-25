use bevy::prelude::*;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use crate::worlds::WorldState;

pub struct HotkeysPlugin;

struct HotkeyManagerRes {
    _manager:    GlobalHotKeyManager,
    spells_id:   u32,
    graph_id:    u32,
    sense_id:    u32,
    terminal_id: u32,
    portal_id:   u32,
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
    let hk_spells    = HotKey::new(Some(mods), Code::Digit1);
    let hk_graph     = HotKey::new(Some(mods), Code::Digit2);
    let hk_sense     = HotKey::new(Some(mods), Code::Digit3);
    let hk_terminal  = HotKey::new(Some(mods), Code::Digit4);
    let hk_portal    = HotKey::new(Some(mods), Code::Digit5);
    let hk_interface = HotKey::new(Some(mods), Code::Digit6);

    manager.register(hk_spells).expect("register Cmd+1");
    manager.register(hk_graph).expect("register Cmd+2");
    manager.register(hk_sense).expect("register Cmd+3");
    manager.register(hk_terminal).expect("register Cmd+4");
    manager.register(hk_portal).expect("register Cmd+5");
    manager.register(hk_interface).expect("register Cmd+6");

    world.insert_non_send_resource(HotkeyManagerRes {
        _manager:    manager,
        spells_id:   hk_spells.id(),
        graph_id:    hk_graph.id(),
        sense_id:    hk_sense.id(),
        terminal_id: hk_terminal.id(),
        portal_id:   hk_portal.id(),
        interface_id: hk_interface.id(),
    });
}

fn poll_hotkey_events(
    hotkeys:       NonSend<HotkeyManagerRes>,
    current_state: Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
) {
    while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
        let target = if event.id == hotkeys.spells_id {
            WorldState::Spells
        } else if event.id == hotkeys.graph_id {
            WorldState::Graph
        } else if event.id == hotkeys.sense_id {
            WorldState::Sense
        } else if event.id == hotkeys.terminal_id {
            WorldState::Terminal
        } else if event.id == hotkeys.portal_id {
            WorldState::Portal
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
