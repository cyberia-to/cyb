mod agent;
mod shell;
mod worlds;

use bevy::prelude::*;
use agent::AgentPlugin;
use shell::hotkeys::HotkeysPlugin;
use shell::tray::TrayPlugin;
use worlds::WorldsPlugin;
use worlds::browser::BrowserWorldPlugin;
use worlds::game::GameWorldPlugin;
use worlds::terminal::TerminalWorldPlugin;
use worlds::ui::UiWorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "cyb".into(),
                resolution: (1280u32, 800u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WorldsPlugin)
        .add_plugins(HotkeysPlugin)
        .add_plugins(GameWorldPlugin)
        .add_plugins(BrowserWorldPlugin)
        .add_plugins(UiWorldPlugin)
        .add_plugins(TerminalWorldPlugin)
        .add_plugins(AgentPlugin)
        .add_plugins(TrayPlugin)
        .run();
}
