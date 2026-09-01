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
    // Android never sets HOME, and everything durable in cyb — the graph
    // log, the mnemonic, the content store, the models — lives under
    // `~/cyb` and `~/llm`. Unset, those paths resolved to `/` and every
    // write silently failed: the cell ran ephemeral and the identity was
    // re-minted each launch. The app's internal data dir is the body's own
    // writable root; making it HOME makes every desktop path true here too.
    if let Some(data) = android_app.internal_data_path() {
        // SAFETY: first thing in the process, before any thread reads env.
        unsafe { std::env::set_var("HOME", &data) };
    }
    let _ = bevy::android::ANDROID_APP.set(android_app);
    app::build_app().run();
}
