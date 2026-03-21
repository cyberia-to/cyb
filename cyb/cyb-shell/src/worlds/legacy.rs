use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use wry::{http, Rect, WebView, WebViewBuilder};

use super::WorldState;

pub struct LegacyWorldPlugin;

pub(crate) struct LegacyWebView {
    pub webview: WebView,
}

/// Marker: WebView creation is pending (window wasn't ready on OnEnter)
#[derive(Resource)]
struct LegacyPendingCreate;

/// Marker: web content has finished loading (React app mounted)
#[derive(Resource)]
pub struct LegacyContentLoaded;

/// Shared flag set by WebView IPC when content is loaded
#[derive(Resource)]
struct ContentLoadedFlag(Arc<AtomicBool>);

impl Plugin for LegacyWorldPlugin {
    fn build(&self, app: &mut App) {
        let flag = Arc::new(AtomicBool::new(false));
        app.insert_resource(ContentLoadedFlag(flag));
        app.insert_resource(LegacyPendingCreate);

        app.add_systems(OnEnter(WorldState::Legacy), show_legacy)
            .add_systems(OnExit(WorldState::Legacy), hide_legacy)
            .add_systems(
                Update,
                preload_legacy_webview.run_if(resource_exists::<LegacyPendingCreate>),
            )
            .add_systems(
                Update,
                poll_content_loaded.run_if(not(resource_exists::<LegacyContentLoaded>)),
            )
            .add_systems(
                Update,
                legacy_update.run_if(in_state(WorldState::Legacy)),
            );
    }
}

fn poll_content_loaded(mut commands: Commands, flag: Res<ContentLoadedFlag>) {
    if flag.0.load(Ordering::Relaxed) {
        commands.insert_resource(LegacyContentLoaded);
        info!("Legacy content loaded (React app ready)");
    }
}

/// Preload WebView in background (runs during Splash and Legacy states)
fn preload_legacy_webview(world: &mut World) {
    if world.get_non_send_resource::<LegacyWebView>().is_some() {
        world.remove_resource::<LegacyPendingCreate>();
        return;
    }
    create_legacy_webview(world);
    if let Some(wv) = world.get_non_send_resource::<LegacyWebView>() {
        // Hide it until Legacy state is entered
        let _ = wv.webview.set_visible(false);
        world.remove_resource::<LegacyPendingCreate>();
        info!("Legacy WebView preloaded (hidden)");
    }
}

fn show_legacy(world: &mut World) {
    if let Some(wv) = world.get_non_send_resource::<LegacyWebView>() {
        let _ = wv.webview.set_visible(true);
        update_legacy_bounds(world);
        info!("Legacy WebView shown");
        return;
    }

    // WebView not preloaded yet — create now
    create_legacy_webview(world);
    if world.get_non_send_resource::<LegacyWebView>().is_none() {
        world.insert_resource(LegacyPendingCreate);
    }
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

/// JS injected into WebView to signal when React app is mounted
const READY_SCRIPT: &str = r#"
    window.addEventListener('load', function() {
        // Wait a tick for React to mount
        setTimeout(function() {
            if (window.ipc) window.ipc.postMessage('ready');
        }, 100);
    });
"#;

fn create_legacy_webview(world: &mut World) {
    let flag = world.resource::<ContentLoadedFlag>().0.clone();

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

        let ipc_flag = flag.clone();
        let ipc_handler = move |msg: wry::http::Request<String>| {
            let body = msg.body().as_str();
            if body == "ready" {
                ipc_flag.store(true, Ordering::Relaxed);
            }
        };

        // Dev mode: no local build, fall back to dev server
        if build_dir.as_os_str().is_empty() {
            info!("Legacy: no build/, falling back to https://localhost:3001");
            let ipc_flag2 = flag.clone();
            let ipc_handler2 = move |msg: wry::http::Request<String>| {
                if msg.body().as_str() == "ready" {
                    ipc_flag2.store(true, Ordering::Relaxed);
                }
            };
            return match WebViewBuilder::new()
                .with_url("https://localhost:3001")
                .with_ipc_handler(ipc_handler2)
                .with_initialization_script(READY_SCRIPT)
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
            .with_ipc_handler(ipc_handler)
            .with_initialization_script(READY_SCRIPT)
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
