use std::borrow::Cow;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use wry::{http, Rect, WebView, WebViewBuilder};

use super::WorldState;

pub struct PortalWorldPlugin;

struct PortalWebView {
    webview: WebView,
}

impl Plugin for PortalWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldState::Portal), show_portal)
            .add_systems(OnExit(WorldState::Portal), hide_portal)
            .add_systems(
                Update,
                portal_update.run_if(in_state(WorldState::Portal)),
            );
    }
}

fn show_portal(world: &mut World) {
    if let Some(wv) = world.get_non_send_resource::<PortalWebView>() {
        let _ = wv.webview.set_visible(true);
        update_portal_bounds(world);
        info!("Portal WebView shown (persisted)");
        return;
    }

    create_portal_webview(world);
}

fn portal_dist_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let dist = PathBuf::from(manifest_dir).join("../cyb-portal/dist");
        if dist.exists() {
            return dist.canonicalize().unwrap_or(dist);
        }
        // Fallback for dev: try trunk serve at localhost:8090
        PathBuf::new()
    } else {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default();
        exe_dir.join("cyb-portal")
    }
}

fn mime_from_path(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        "text/javascript"
    } else if path.ends_with(".wasm") {
        "application/wasm"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}

fn create_portal_webview(world: &mut World) {
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
        let dist_dir = portal_dist_dir();

        // If no dist dir exists in dev mode, fall back to trunk serve
        if dist_dir.as_os_str().is_empty() {
            info!("Portal: no dist/, falling back to http://localhost:8090");
            return match WebViewBuilder::new()
                .with_url("http://localhost:8090")
                .with_bounds(Rect {
                    position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                    size: wry::dpi::PhysicalSize::new(inner_size.width, inner_size.height).into(),
                })
                .with_devtools(cfg!(debug_assertions))
                .build_as_child(&**window_wrapper)
            {
                Ok(webview) => Some(webview),
                Err(e) => {
                    warn!("Failed to create Portal WebView: {}", e);
                    None
                }
            };
        }

        // Serve local files via custom protocol (avoids WKWebView file:// restrictions)
        let dist = dist_dir.clone();
        match WebViewBuilder::new()
            .with_custom_protocol("portal".into(), move |_webview_id, request| {
                let uri_path = request.uri().path();
                let path = if uri_path == "/" || uri_path.is_empty() {
                    "index.html".to_string()
                } else {
                    uri_path.trim_start_matches('/').to_string()
                };

                let file_path = dist.join(&path);
                match std::fs::read(&file_path) {
                    Ok(content) => {
                        let mime = mime_from_path(&path);
                        http::Response::builder()
                            .header(http::header::CONTENT_TYPE, mime)
                            .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                            .body(Cow::Owned(content))
                            .unwrap()
                    }
                    Err(_) => {
                        http::Response::builder()
                            .status(404)
                            .header(http::header::CONTENT_TYPE, "text/plain")
                            .body(Cow::Borrowed(b"Not found" as &[u8]))
                            .unwrap()
                    }
                }
            })
            .with_url("portal://localhost/index.html")
            .with_bounds(Rect {
                position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                size: wry::dpi::PhysicalSize::new(inner_size.width, inner_size.height).into(),
            })
            .with_ipc_handler(|msg| {
                info!("IPC from portal: {:?}", msg);
            })
            .with_devtools(cfg!(debug_assertions))
            .build_as_child(&**window_wrapper)
        {
            Ok(webview) => {
                info!("Portal WebView created (custom protocol), dist={}", dist_dir.display());
                Some(webview)
            }
            Err(e) => {
                warn!("Failed to create Portal WebView: {}", e);
                None
            }
        }
    });

    if let Some(webview) = created {
        world.insert_non_send_resource(PortalWebView { webview });
    }
}

fn hide_portal(world: &mut World) {
    if let Some(wv) = world.get_non_send_resource::<PortalWebView>() {
        let _ = wv.webview.set_visible(false);
        info!("Portal WebView hidden (state persisted)");
    }
}

fn portal_update(world: &mut World) {
    update_portal_bounds(world);
}

fn update_portal_bounds(world: &mut World) {
    let Some(wv) = world.remove_non_send_resource::<PortalWebView>() else {
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
