use crate::shell::window::ShowWindow;
use crate::worlds::WorldState;
use bevy::prelude::*;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};

pub struct HotkeysPlugin;

struct HotkeyManagerRes {
    _manager: GlobalHotKeyManager,
    /// hotkey id -> world, in tab order (Cmd+1 = the main page).
    map: Vec<(u32, WorldState)>,
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
    let worlds = [
        (Code::Digit1, WorldState::Body),
        (Code::Digit2, WorldState::Graph),
        (Code::Digit3, WorldState::Com),
        (Code::Digit4, WorldState::Robot),
        (Code::Digit5, WorldState::Sigma),
        (Code::Digit6, WorldState::Models),
    ];
    let mut map = Vec::new();
    for (code, w) in worlds {
        let hk = HotKey::new(Some(mods), code);
        manager.register(hk).unwrap_or_else(|e| panic!("register {code:?}: {e}"));
        map.push((hk.id(), w));
    }

    world.insert_non_send_resource(HotkeyManagerRes { _manager: manager, map });
}

fn poll_hotkey_events(
    hotkeys: NonSend<HotkeyManagerRes>,
    current_state: Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
    mut show: MessageWriter<ShowWindow>,
) {
    while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
        let Some(&(_, target)) = hotkeys.map.iter().find(|(id, _)| *id == event.id) else {
            continue;
        };

        // The hotkeys are global: they fire with cyb in the background, and
        // reaching for a world is a request to look at it.
        show.write(ShowWindow);
        if *current_state.get() != target {
            info!("Hotkey → {:?}", target);
            next_state.set(target);
        }
    }
}
