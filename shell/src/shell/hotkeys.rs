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
    graph_id: u32,
    com_id: u32,
    robot_id: u32,
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
    let hk_com = HotKey::new(Some(mods), Code::Digit2);
    let hk_robot = HotKey::new(Some(mods), Code::Digit3);
    let hk_sigma = HotKey::new(Some(mods), Code::Digit4);

    manager.register(hk_graph).expect("register Cmd+1");
    manager.register(hk_com).expect("register Cmd+2");
    manager.register(hk_robot).expect("register Cmd+3");
    manager.register(hk_sigma).expect("register Cmd+4");

    world.insert_non_send_resource(HotkeyManagerRes {
        _manager: manager,
        graph_id: hk_graph.id(),
        com_id: hk_com.id(),
        robot_id: hk_robot.id(),
        sigma_id: hk_sigma.id(),
    });
}

fn poll_hotkey_events(
    hotkeys: NonSend<HotkeyManagerRes>,
    current_state: Res<State<WorldState>>,
    mut next_state: ResMut<NextState<WorldState>>,
    mut show: MessageWriter<ShowWindow>,
) {
    while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
        let target = if event.id == hotkeys.graph_id {
            WorldState::Graph
        } else if event.id == hotkeys.com_id {
            WorldState::Com
        } else if event.id == hotkeys.robot_id {
            WorldState::Robot
        } else if event.id == hotkeys.sigma_id {
            WorldState::Sigma
        } else {
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
