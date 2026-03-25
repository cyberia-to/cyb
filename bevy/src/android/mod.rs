use log::info;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::{WebViewBuilder, WebViewBuilderExtAndroid};

// JNI bindings required by wry and tao on Android.
// Package: ai.cyb.app → domain=ai_cyb, package=app
const _: () = {
    tao::android_binding!(ai_cyb, app, WryActivity, wry::android_setup, _start_app);
    wry::android_binding!(ai_cyb, app);
};

/// Android entry point — called by tao's android_binding macro
fn _start_app() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("cyb"),
    );

    info!("cyb Android starting");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("cyb")
        .build(&event_loop)
        .expect("Failed to create window");

    let _webview = WebViewBuilder::new()
        .with_asset_loader("cyb".into())
        .with_url("https://cyb.assets/index.html")
        .with_devtools(true)
        .build(&window)
        .expect("Failed to create WebView");

    info!("cyb WebView created");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

#[allow(dead_code)]
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
