use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use wry::{Rect, WebView, WebViewBuilder};

use super::WorldState;

pub struct LegacyWorldPlugin;

pub(crate) struct LegacyWebView {
    pub webview: WebView,
}

impl Plugin for LegacyWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldState::Legacy), show_legacy)
            .add_systems(OnExit(WorldState::Legacy), hide_legacy)
            .add_systems(
                Update,
                legacy_update.run_if(in_state(WorldState::Legacy)),
            );
    }
}

fn show_legacy(world: &mut World) {
    if let Some(wv) = world.get_non_send_resource::<LegacyWebView>() {
        let _ = wv.webview.set_visible(true);
        update_legacy_bounds(world);
        info!("Legacy WebView shown (persisted)");
        return;
    }

    create_legacy_webview(world);
}

fn create_legacy_webview(world: &mut World) {
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

        let url = if cfg!(debug_assertions) {
            "https://localhost:3001".to_string()
        } else {
            "https://cyb.ai".to_string()
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
                info!("Legacy world created, loading {}", url);
                Some(webview)
            }
            Err(e) => {
                warn!("Failed to create Legacy WebView: {}", e);
                None
            }
        }
    });

    if let Some(webview) = result {
        world.insert_non_send_resource(LegacyWebView { webview });
    }
}

fn hide_legacy(world: &mut World) {
    if let Some(wv) = world.get_non_send_resource::<LegacyWebView>() {
        let _ = wv.webview.set_visible(false);
        info!("Legacy WebView hidden (state persisted)");
    }
}

fn legacy_update(world: &mut World) {
    update_legacy_bounds(world);
}

fn update_legacy_bounds(world: &mut World) {
    let Some(wv) = world.remove_non_send_resource::<LegacyWebView>() else {
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
