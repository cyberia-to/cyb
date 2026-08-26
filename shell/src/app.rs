//! The one cyb App — every platform builds it here.
//!
//! Desktop (`main.rs`) and Android (`lib.rs`'s `android_main`) call
//! [`build_app`] and get the same Bevy app: same worlds, same chrome, same
//! nav. Only the surfaces that name a platform differ — global hotkeys, the
//! tray, and the hide-on-close window contract exist where a desktop does.

use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::renderer::{RenderDevice, RenderQueue};

use crate::{agent, shell, worlds};

/// Exposes the render device/queue on the main world so worlds can allocate
/// GPU resources without reaching into the render sub-app.
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

pub fn build_app() -> App {
    let mut app = App::new();

    #[allow(unused_mut)]
    let mut window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "cyb".into(),
            resolution: (1280u32, 800u32).into(),
            ..default()
        }),
        ..default()
    };

    // The robot outlives its window: closing hides, the tray brings it back,
    // only tray → Quit ends the process. See `shell/window.rs`. On Android
    // the OS owns the app lifecycle, so the defaults stand.
    #[cfg(not(target_os = "android"))]
    {
        window_plugin.close_when_requested = false;
        window_plugin.exit_condition = bevy::window::ExitCondition::DontExit;
    }

    app.add_plugins(
        DefaultPlugins.set(window_plugin).set(bevy::render::RenderPlugin {
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
    .add_plugins(shell::nav::NavPlugin)
    .add_plugins(mir::bevy::GraphWorldPlugin)
    .add_plugins(worlds::graph::GraphBridgePlugin)
    .add_plugins(worlds::terminal::TerminalWorldPlugin)
    .add_plugins(worlds::cell::CellWorldPlugin)
    .add_plugins(worlds::sigma::SigmaWorldPlugin)
    .add_plugins(agent::AgentPlugin);

    #[cfg(not(target_os = "android"))]
    app.add_plugins(shell::hotkeys::HotkeysPlugin)
        .add_plugins(shell::window::WindowLifecyclePlugin)
        .add_plugins(shell::tray::TrayPlugin);

    app
}
