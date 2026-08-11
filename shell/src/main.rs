mod agent;
mod shell;
mod worlds;

use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::renderer::{RenderDevice, RenderQueue};

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
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "cyb".into(),
                        resolution: (1280u32, 800u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::render::RenderPlugin {
                    render_creation: bevy::render::settings::RenderCreation::Automatic(
                        bevy::render::settings::WgpuSettings { ..default() },
                    ),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(bevy::color::Color::BLACK))
        .add_plugins(GpuBridgePlugin)
        .add_plugins(prysm::PrysmPlugin)
        .add_plugins(worlds::WorldsPlugin)
        .add_plugins(shell::chrome::ChromePlugin)
        .add_plugins(shell::hotkeys::HotkeysPlugin)
        .add_plugins(shell::nav::NavPlugin)
        .add_plugins(mir::bevy::GraphWorldPlugin)
        .add_plugins(worlds::graph::GraphBridgePlugin)
        .add_plugins(worlds::terminal::TerminalWorldPlugin)
        .add_plugins(worlds::cell::CellWorldPlugin)
        .add_plugins(worlds::sigma::SigmaWorldPlugin)
        .add_plugins(agent::AgentPlugin)
        .add_plugins(shell::tray::TrayPlugin)
        .run();
}
