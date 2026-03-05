mod agent;
mod shell;
mod worlds;

use bevy::prelude::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::RenderApp;
use agent::AgentPlugin;
use shell::hotkeys::HotkeysPlugin;
use shell::tray::TrayPlugin;
use worlds::WorldsPlugin;
use worlds::interface::InterfaceWorldPlugin;
use worlds::legacy::LegacyWorldPlugin;
use worlds::portal::PortalWorldPlugin;
use worlds::terminal::TerminalWorldPlugin;

/// Clones Bevy's GPU resources (Device, Queue) into the main world
/// so non-render systems (like Terminal) can use them.
struct GpuBridgePlugin;

impl Plugin for GpuBridgePlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let (device, queue) = {
            let render_app = app.sub_app(RenderApp);
            let device = render_app.world().resource::<RenderDevice>().clone();
            let queue = render_app.world().resource::<RenderQueue>().clone();
            (device, queue)
        };
        app.insert_resource(device);
        app.insert_resource(queue);
    }
}

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
        .add_plugins(GpuBridgePlugin)
        .add_plugins(WorldsPlugin)
        .add_plugins(HotkeysPlugin)
        .add_plugins(InterfaceWorldPlugin)
        .add_plugins(PortalWorldPlugin)
        .add_plugins(LegacyWorldPlugin)
        .add_plugins(TerminalWorldPlugin)
        .add_plugins(AgentPlugin)
        .add_plugins(TrayPlugin)
        .run();
}
