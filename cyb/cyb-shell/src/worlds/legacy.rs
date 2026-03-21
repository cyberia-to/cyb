use std::borrow::Cow;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use wry::{http, Rect, WebView, WebViewBuilder};

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

fn legacy_build_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        // Dev: use local build/ from project root
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let build = PathBuf::from(manifest_dir).join("../../build");
        if build.exists() {
            return build.canonicalize().unwrap_or(build);
        }
        // Fallback: dev server
        PathBuf::new()
    } else {
        // Release: bundled inside .app
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default();
        exe_dir.join("cyb-web")
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
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".mp3") {
        "audio/mpeg"
    } else if path.ends_with(".ttf") {
        "font/ttf"
    } else if path.ends_with(".woff") || path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
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
        let build_dir = legacy_build_dir();

        // Dev mode: no local build, fall back to dev server
        if build_dir.as_os_str().is_empty() {
            info!("Legacy: no build/, falling back to https://localhost:3001");
            return match WebViewBuilder::new()
                .with_url("https://localhost:3001")
                .with_bounds(Rect {
                    position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                    size: wry::dpi::PhysicalSize::new(inner_size.width, inner_size.height).into(),
                })
                .with_devtools(cfg!(debug_assertions))
                .build_as_child(&**window_wrapper)
            {
                Ok(webview) => Some(webview),
                Err(e) => {
                    warn!("Failed to create Legacy WebView: {}", e);
                    None
                }
            };
        }

        // Serve local build via custom protocol
        let dist = build_dir.clone();
        match WebViewBuilder::new()
            .with_custom_protocol("cyb".into(), move |_webview_id, request| {
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
                        // SPA fallback: serve index.html for client-side routing
                        match std::fs::read(dist.join("index.html")) {
                            Ok(content) => {
                                http::Response::builder()
                                    .header(http::header::CONTENT_TYPE, "text/html")
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
                    }
                }
            })
            .with_url("cyb://localhost/index.html")
            .with_bounds(Rect {
                position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                size: wry::dpi::PhysicalSize::new(inner_size.width, inner_size.height).into(),
            })
            .with_devtools(cfg!(debug_assertions))
            .build_as_child(&**window_wrapper)
        {
            Ok(webview) => {
                info!("Legacy world created (local build), dir={}", build_dir.display());
                Some(webview)
            }
            Err(e) => {
                warn!("Failed to create Legacy WebView: {}", e);
                None
            }
        }
    });

    if let Some(webview) = result {
        // Inject bootstrap data (referral, wallet) into WebView if available
        if let Some(bootstrap_js) = load_bootstrap_script() {
            if let Err(e) = webview.evaluate_script(&bootstrap_js) {
                warn!("Failed to inject bootstrap: {}", e);
            }
        }
        world.insert_non_send_resource(LegacyWebView { webview });
    }
}

/// Load bootstrap.json from app data dir and return JS injection script.
/// bootstrap.json contains {mnemonic, referrer, name} for wallet+referral import.
/// File is deleted after reading (one-time import).
fn load_bootstrap_script() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let bootstrap_path = if cfg!(target_os = "macos") {
        PathBuf::from(&home).join("Library/Application Support/ai.cyb.app/bootstrap.json")
    } else {
        PathBuf::from(&home).join(".local/share/ai.cyb.app/bootstrap.json")
    };

    if !bootstrap_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&bootstrap_path).ok()?;
    // Basic JSON validation: must start with { and end with }
    let trimmed = content.trim();
    if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
        warn!("Invalid bootstrap.json, skipping");
        return None;
    }

    // Delete after reading (one-time import)
    let _ = std::fs::remove_file(&bootstrap_path);
    info!("Bootstrap data loaded and injected");

    Some(format!("window.__CYB_BOOTSTRAP__ = {};", trimmed))
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
