use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use wry::{http, Rect, WebViewBuilder};

pub enum WebViewCmd {
    EvalScript(String),
}

/// Send navigation commands to the always-on WebView from any Bevy system.
#[derive(Resource, Clone)]
pub struct WebViewHandle {
    cmd_tx: mpsc::Sender<WebViewCmd>,
}

impl WebViewHandle {
    /// Sync Leptos router to `path` without a full page reload.
    pub fn push_state(&self, path: &str) {
        let js = format!(
            "history.pushState(null,'','{}');window.dispatchEvent(new PopStateEvent('popstate',{{state:history.state}}));",
            path
        );
        let _ = self.cmd_tx.send(WebViewCmd::EvalScript(js));
    }
}

pub struct CommandEvent(pub String);

struct ShellWebView {
    webview:    wry::WebView,
    cmd_rx:     mpsc::Receiver<WebViewCmd>,
    nav_queue:  Arc<Mutex<Vec<String>>>,
    cmd_queue:  Arc<Mutex<Vec<String>>>,
    last_bounds: (u32, u32),
}

pub struct WebViewPlugin;

impl Plugin for WebViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_webview)
            .add_systems(Update, poll_webview)
            .add_systems(Update, update_webview_bounds);
    }
}

fn apps_dist_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::new() // dev: fall back to http://localhost:8090
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default()
            .join("cyb-apps")
    }
}

fn mime_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html",
        Some("js" | "mjs") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn create_webview(world: &mut World) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<WebViewCmd>();
    let nav_queue: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let cmd_queue: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let nav_clone  = Arc::clone(&nav_queue);
    let cmd_clone  = Arc::clone(&cmd_queue);

    let primary_entity = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world);
    let Ok(entity) = primary_entity else { return };

    let maybe_wv = WINIT_WINDOWS.with(|ww| {
        let ww = ww.borrow();
        let Some(win) = ww.get_window(entity) else { return None };
        let scale  = win.scale_factor();
        let phys   = win.inner_size();
        let lw     = (phys.width  as f64 / scale) as u32;
        let lh     = (phys.height as f64 / scale) as u32;
        let dist   = apps_dist_dir();

        let builder = if dist.as_os_str().is_empty() {
            info!("WebView: connecting to http://localhost:8090");
            WebViewBuilder::new().with_url("http://localhost:8090")
        } else {
            let dist2 = dist.clone();
            WebViewBuilder::new()
                .with_custom_protocol("cyb".into(), move |_id, req| {
                    let p = req.uri().path();
                    let rel = if p == "/" || p == "/index.html" {
                        "index.html"
                    } else {
                        p.trim_start_matches('/')
                    };
                    match std::fs::read(dist2.join(rel)) {
                        Ok(bytes) => {
                            let mut r = http::Response::builder()
                                .header(http::header::CONTENT_TYPE, mime_type(rel))
                                .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
                            if rel.ends_with(".html") {
                                r = r.header(
                                    "Content-Security-Policy",
                                    "default-src 'self' cyb: blob: data: 'unsafe-inline' 'unsafe-eval' 'wasm-eval'; connect-src *; font-src *;",
                                );
                            }
                            r.body(Cow::Owned(bytes)).unwrap()
                        }
                        Err(_) => http::Response::builder()
                            .status(404)
                            .body(Cow::Borrowed(b"not found" as &[u8]))
                            .unwrap(),
                    }
                })
                .with_url("cyb://localhost/index.html")
        };

        match builder
            .with_transparent(true)
            .with_initialization_script(
                "document.addEventListener('DOMContentLoaded',function(){
                    window.dispatchEvent(new Event('resize'));
                });"
            )
            .with_bounds(Rect {
                position: wry::dpi::LogicalPosition::new(0, 0).into(),
                size:     wry::dpi::LogicalSize::new(lw, lh).into(),
            })
            .with_ipc_handler(move |msg| {
                let body = msg.body().to_string();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    match v.get("type").and_then(|t| t.as_str()) {
                        Some("navigate") => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                nav_clone.lock().unwrap().push(path.to_string());
                            }
                        }
                        Some("command") => {
                            if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                                cmd_clone.lock().unwrap().push(text.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            })
            .with_devtools(cfg!(debug_assertions))
            .build_as_child(&**win)
        {
            Ok(wv) => {
                info!("WebView created (always-on, transparent)");
                Some(wv)
            }
            Err(e) => {
                warn!("WebView creation failed: {e}");
                None
            }
        }
    });

    if let Some(wv) = maybe_wv {
        world.insert_non_send_resource(ShellWebView { webview: wv, cmd_rx, nav_queue, cmd_queue, last_bounds: (0, 0) });
        world.insert_resource(WebViewHandle { cmd_tx });
    }
}

fn poll_webview(
    shell:          NonSend<ShellWebView>,
    mut next_state: ResMut<NextState<crate::worlds::WorldState>>,
) {
    while let Ok(cmd) = shell.cmd_rx.try_recv() {
        match cmd {
            WebViewCmd::EvalScript(js) => {
                let _ = shell.webview.evaluate_script(&js);
            }
        }
    }
    if let Ok(mut q) = shell.nav_queue.try_lock() {
        for path in q.drain(..) {
            let target = match path.as_str() {
                "/brain"     => Some(crate::worlds::WorldState::Graph),
                "/terminal"  => Some(crate::worlds::WorldState::Terminal),
                "/interface" => Some(crate::worlds::WorldState::Interface),
                _ => None,
            };
            if let Some(t) = target {
                info!("WebView nav → {:?}", t);
                next_state.set(t);
            }
        }
    }
    if let Ok(mut q) = shell.cmd_queue.try_lock() {
        for text in q.drain(..) {
            info!("cmd: {text}");
        }
    }
}

fn update_webview_bounds(world: &mut World) {
    let primary_entity = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world);
    let Ok(entity) = primary_entity else { return };

    let Some(mut shell) = world.remove_non_send_resource::<ShellWebView>() else {
        return;
    };

    WINIT_WINDOWS.with(|ww| {
        let ww = ww.borrow();
        if let Some(win) = ww.get_window(entity) {
            let scale = win.scale_factor();
            let phys  = win.inner_size();
            let lw    = (phys.width  as f64 / scale) as u32;
            let lh    = (phys.height as f64 / scale) as u32;
            if (lw, lh) != shell.last_bounds && lw > 0 && lh > 0 {
                let _ = shell.webview.set_bounds(Rect {
                    position: wry::dpi::LogicalPosition::new(0, 0).into(),
                    size:     wry::dpi::LogicalSize::new(lw, lh).into(),
                });
                let _ = shell.webview.evaluate_script(
                    "window.dispatchEvent(new Event('resize'))"
                );
                shell.last_bounds = (lw, lh);
            }
        }
    });

    world.insert_non_send_resource(shell);
}
