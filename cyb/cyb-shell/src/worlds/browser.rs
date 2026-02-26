use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use wry::{Rect, WebView, WebViewBuilder};

use super::WorldState;

pub struct BrowserWorldPlugin;

pub(crate) struct WryWebView {
    pub webview: WebView,
}

#[derive(Resource, Default)]
struct BrowserCreated(bool);

impl Plugin for BrowserWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrowserCreated>()
            .add_systems(OnExit(WorldState::Browser), destroy_webview)
            .add_systems(
                Update,
                browser_update.run_if(in_state(WorldState::Browser)),
            )
            .add_systems(
                Update,
                toggle_devtools.run_if(in_state(WorldState::Browser)),
            );
    }
}

fn browser_update(world: &mut World) {
    if !world.resource::<BrowserCreated>().0 {
        // Create webview on first frame
        let primary_entity = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world);
        let Ok(entity) = primary_entity else { return };

        let created = WINIT_WINDOWS.with(|ww| {
            let ww = ww.borrow();
            let Some(window_wrapper) = ww.get_window(entity) else {
                return None;
            };

            let inner_size = window_wrapper.inner_size();

            let url = if cfg!(debug_assertions) {
                "https://localhost:3001"
            } else {
                "https://cyb.ai"
            };

            match WebViewBuilder::new()
                .with_url(url)
                .with_bounds(Rect {
                    position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                    size: wry::dpi::PhysicalSize::new(inner_size.width, inner_size.height).into(),
                })
                .with_ipc_handler(|msg| {
                    info!("IPC from webview: {:?}", msg);
                })
                .with_devtools(cfg!(debug_assertions))
                .build_as_child(&**window_wrapper)
            {
                Ok(webview) => {
                    info!("Browser WebView created, loading {}", url);
                    Some(webview)
                }
                Err(e) => {
                    warn!("Failed to create browser WebView: {}", e);
                    None
                }
            }
        });

        if let Some(webview) = created {
            world.insert_non_send_resource(WryWebView { webview });
        }
        world.resource_mut::<BrowserCreated>().0 = true;
        return;
    }

    // Update webview bounds
    let Some(wv) = world.remove_non_send_resource::<WryWebView>() else {
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

/// Toggle devtools with F12 or Cmd+Option+I (debug builds only)
fn toggle_devtools(
    wv: Option<NonSend<WryWebView>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !cfg!(debug_assertions) {
        return;
    }
    let Some(wv) = wv else { return };

    let f12 = keys.just_pressed(KeyCode::F12);

    let cmd_opt_i = keys.just_pressed(KeyCode::KeyI) && {
        #[cfg(target_os = "macos")]
        let modifier = keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight);
        #[cfg(not(target_os = "macos"))]
        let modifier = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        modifier && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    };

    if f12 || cmd_opt_i {
        info!("Toggling devtools");
        if wv.webview.is_devtools_open() {
            wv.webview.close_devtools();
        } else {
            wv.webview.open_devtools();
        }
    }
}

fn destroy_webview(world: &mut World) {
    world.remove_non_send_resource::<WryWebView>();
    world.resource_mut::<BrowserCreated>().0 = false;
    info!("Browser WebView destroyed");
}
