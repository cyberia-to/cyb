//! cyb — one app, every platform.
//!
//! Desktop runs `main.rs`, Android enters through [`android_main`] below;
//! both build the identical Bevy app in [`app::build_app`]. The cdylib
//! target exists for the Android JNI load (`libcyb.so`).

pub mod agent;
pub mod app;
pub mod shell;
pub mod worlds;

/// Android entry point. GameActivity loads `libcyb.so` and calls this;
/// bevy_winit picks the `AndroidApp` up from `bevy::android::ANDROID_APP`
/// and drives the same event loop winit runs on desktop.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: bevy::android::android_activity::AndroidApp) {
    let _ = bevy::android::ANDROID_APP.set(android_app);
    app::build_app().run();
}
