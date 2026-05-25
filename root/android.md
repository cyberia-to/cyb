# android

## principle

android cyb = desktop cyb.

one codebase. one binary. same Bevy app, same worlds, same terminal, same nushell engine.
android adds `nu_plugin_android` which exposes hardware APIs (GPS, camera, sensors, etc.)
that desktop doesn't have. that is the only difference.

## one binary rule

WebView is only for web content (Legacy, Portal worlds). all other rendering — terminal, graph,
3D, UI — is pure Rust via Bevy + wgpu. no JS bundles, no external runtimes embedded in the binary.

## why nushell on android is powerful

desktop terminal agents can read files, run HTTP, process data. android terminal agents can do
all of that AND access the physical world:

```nushell
android gps              # location
android camera           # see the world
android sensors          # feel the environment
android contacts         # know who you know
android ble scan         # talk to IoT
android battery          # device state
android wifi             # network environment
```

android doesn't limit the terminal — it extends it.

## external commands on android

android ships `toybox` at `/system/bin` and `/system/xbin`. nushell external commands work:

```nushell
ls /system/bin       # works
ps                   # works
curl https://...     # works
grep  find  sed  awk  cat  cp  mv  rm  mkdir  ping  nc  stat  tar  gzip
```

nushell scripting, pipelines, HTTP, file I/O — identical to desktop. the sandbox only means
no system package manager (no apt/brew). app-local binaries can go in `$HOME/bin`.

## implementation plan

### phase 1 — bevy entry point (2 days)

replace current `tao + wry` android stub (`bevy/src/android/mod.rs`) with Bevy's native
android backend via `android-activity` crate.

**`bevy/src/lib.rs`:**
```rust
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    App::new()
        .add_plugins(DefaultPlugins...)
        .add_plugins(TerminalWorldPlugin)
        .add_plugins(GraphWorldPlugin)
        // ... identical to main()
        .run();
}
```

**`bevy/Cargo.toml`** android feature:
```toml
android = [
    "bevy/android",
    "dep:android-activity",
    "dep:nu-protocol",
    "dep:nu-engine",
    "dep:nu-parser",
    "dep:nu-command",
    "dep:nu-cmd-lang",
    "dep:nu-std",
    "dep:nu-utils",
]
```

**`bevy/gen/android/AndroidManifest.xml`:** switch activity to `GameActivity` (android-activity
backend). add permissions: `ACCESS_FINE_LOCATION`, `CAMERA`, `INTERNET`, `POST_NOTIFICATIONS`.

delete `bevy/src/android/` entirely — tao/wry entry point is gone.

### phase 2 — terminal world (1 day)

`worlds/terminal.rs` runs **unchanged**. sugarloaf uses wgpu → Android Vulkan. alacritty_terminal
and nushell are pure Rust.

**`bevy/assets/nu-config/android.nu`** (new, sourced after `env.nu` on Android):
```nushell
$env.PATH = ($env.PATH | prepend ["/system/bin" "/system/xbin" $"($env.HOME)/bin"])
$env.TERM = "xterm-256color"
```

**`terminal.rs`** `init_nushell_engine()` — 5 lines cfg-gated on `target_os = "android"`:
set `$HOME` to `AndroidApp::internal_data_path()`, source `android.nu`.

android soft keyboard: `Window::ime_enabled = true` in terminal `OnEnter`. Bevy normalizes
Android IME events into `KeyboardInput` — existing input handler works unchanged.

### phase 3 — nu_plugin_android (4–5 days)

new workspace crate `cyb/nu_plugin_android/` — JNI bridge registered as an inline plugin
(no subprocess overhead).

| command | android API | permission |
|---|---|---|
| `android gps [--watch]` | LocationManager | ACCESS_FINE_LOCATION |
| `android camera [--front]` | CameraX | CAMERA |
| `android sensors list` | SensorManager | — |
| `android sensors read <type>` | SensorManager | — |
| `android intent --action <a> --data <d>` | Intent | — |
| `android clipboard get/set` | ClipboardManager | — |
| `android notify --title --body` | NotificationManager | POST_NOTIFICATIONS |
| `android contacts` | ContentResolver | READ_CONTACTS |
| `android wifi` | WifiManager | ACCESS_WIFI_STATE |
| `android battery` | BatteryManager | — |

uses `jni` crate for JVM calls. `android-activity` provides `JavaVM`. plugin registered in
`init_nushell_engine()` behind `#[cfg(target_os = "android")]`.

## files changed

| file | change |
|---|---|
| `bevy/Cargo.toml` | android feature: bevy/android + android-activity + nu-* |
| `bevy/src/lib.rs` | `android_main` fn replacing `pub mod android` |
| `bevy/src/android/` | deleted |
| `bevy/gen/android/AndroidManifest.xml` | GameActivity, permissions |
| `bevy/gen/android/app/build.gradle` | GameActivity dep |
| `bevy/assets/nu-config/android.nu` | new: Android PATH/HOME |
| `bevy/src/worlds/terminal.rs` | ~5 lines Android HOME init |
| `cyb/nu_plugin_android/` | new crate |
| `cyb/Cargo.toml` | add nu_plugin_android to workspace |
