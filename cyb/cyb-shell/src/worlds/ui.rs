use std::process::{Child, Command};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use wry::{Rect, WebView, WebViewBuilder};

use super::WorldState;

pub struct UiWorldPlugin;

struct DioxusProcess {
    child: Child,
}

struct UiWebView {
    webview: WebView,
}

#[derive(Resource, Default)]
struct UiCreated(bool);

impl Plugin for UiWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiCreated>()
            .add_systems(OnExit(WorldState::Ui), destroy_ui_world)
            .add_systems(
                Update,
                ui_update.run_if(in_state(WorldState::Ui)),
            );
    }
}

fn ui_update(world: &mut World) {
    if !world.resource::<UiCreated>().0 {
        let primary_entity = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world);
        let Ok(entity) = primary_entity else { return };

        let result = WINIT_WINDOWS.with(|ww| {
            let ww = ww.borrow();
            let Some(window_wrapper) = ww.get_window(entity) else {
                return None;
            };

            let inner_size = window_wrapper.inner_size();

            let (url, child_process) = if cfg!(debug_assertions) {
                match Command::new("dx")
                    .args(["serve", "--port", "8080"])
                    .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../cyb-ui")
                    .spawn()
                {
                    Ok(child) => {
                        info!("Spawned dx serve (pid {})", child.id());
                        ("http://localhost:8080".to_string(), Some(child))
                    }
                    Err(e) => {
                        warn!("Failed to spawn dx serve: {}. Loading fallback.", e);
                        ("about:blank".to_string(), None)
                    }
                }
            } else {
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                    .unwrap_or_default();
                let ui_bin = exe_dir.join("cyb-ui");
                match Command::new(&ui_bin).arg("--port").arg("8080").spawn() {
                    Ok(child) => {
                        info!("Spawned cyb-ui (pid {})", child.id());
                        ("http://localhost:8080".to_string(), Some(child))
                    }
                    Err(e) => {
                        warn!("Failed to spawn cyb-ui at {:?}: {}", ui_bin, e);
                        ("about:blank".to_string(), None)
                    }
                }
            };

            match WebViewBuilder::new()
                .with_url(&url)
                .with_bounds(Rect {
                    position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                    size: wry::dpi::PhysicalSize::new(inner_size.width, inner_size.height).into(),
                })
                .with_devtools(cfg!(debug_assertions))
                .build_as_child(&**window_wrapper)
            {
                Ok(webview) => {
                    info!("UI world created, loading {}", url);
                    Some((webview, child_process))
                }
                Err(e) => {
                    warn!("Failed to create UI WebView: {}", e);
                    None
                }
            }
        });

        if let Some((webview, child_process)) = result {
            world.insert_non_send_resource(UiWebView { webview });
            if let Some(child) = child_process {
                world.insert_non_send_resource(DioxusProcess { child });
            }
        }
        world.resource_mut::<UiCreated>().0 = true;
        return;
    }

    // Update bounds
    let Some(wv) = world.remove_non_send_resource::<UiWebView>() else {
        return;
    };

    let primary_entity = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world);
    if let Ok(entity) = primary_entity {
        WINIT_WINDOWS.with(|ww| {
            let ww = ww.borrow();
            if let Some(window_wrapper) = ww.get_window(entity) {
                let size = window_wrapper.inner_size();
                let _ = wv.webview.set_bounds(Rect {
                    position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                    size: wry::dpi::PhysicalSize::new(size.width, size.height).into(),
                });
            }
        });
    }

    world.insert_non_send_resource(wv);
}

fn destroy_ui_world(world: &mut World) {
    world.remove_non_send_resource::<UiWebView>();

    if let Some(mut proc) = world.remove_non_send_resource::<DioxusProcess>() {
        info!("Killing Dioxus process (pid {})", proc.child.id());
        let _ = proc.child.kill();
        let _ = proc.child.wait();
    }

    world.resource_mut::<UiCreated>().0 = false;
    info!("UI world destroyed");
}
