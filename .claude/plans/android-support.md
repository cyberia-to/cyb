# Android: Bevy-native implementation plan

## Principle

Android cyb = desktop cyb. One codebase, one binary, same Bevy app, same worlds, same terminal,
same nushell. Android adds `nu_plugin_android` for hardware APIs. Nothing else differs.

---

## Phase 1 — Bevy entry point on Android

Replace the current `tao + wry` Android stub with Bevy's native Android backend.

### 1.1 Cargo changes

`bevy/Cargo.toml`:
- Add `android-activity` as optional dep (Bevy's Android backend requires it)
- Android feature activates: `bevy/android`, `android-activity`, all nu-* deps (same as desktop
  minus `nu-cli`)
- Remove `dep:tao` and `dep:android_logger` from android feature — Bevy handles the event loop
  and logging

`bevy/Cargo.toml` workspace root:
- Add `android-activity` to workspace deps

### 1.2 Entry point

`bevy/src/lib.rs` — replace current `pub mod android` stub with:

```rust
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    // Identical to main() — all the same plugins
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin { ... }));
    // ... every plugin desktop uses ...
    app.run();
}
```

The `DefaultPlugins` with `bevy_winit` already handles the Android window lifecycle via
`android-activity`. No custom event loop needed.

### 1.3 AndroidManifest.xml

`bevy/gen/android/AndroidManifest.xml`:
- Change activity class from `WryActivity` to `com.google.androidgamesdk.GameActivity`
  (android-activity's GameActivity backend, which Bevy uses by default)
- Add required permissions for later phases: `ACCESS_FINE_LOCATION`, `CAMERA`, `INTERNET`

### 1.4 Delete android/mod.rs

`bevy/src/android/` directory and `lib.rs` reference to it are removed entirely. The tao/wry
Android entry point is gone.

### 1.5 Gradle

`bevy/gen/android/app/build.gradle`:
- Add `android-games-sdk` dependency for GameActivity
- Update CMakeLists / jniLibs to load `libcyb_shell.so` (already there, no change)

### Verification

`make android` cross-compiles and the app launches showing Bevy's splash world on device.

---

## Phase 2 — Terminal world on Android

`worlds/terminal.rs` runs **unchanged**. sugarloaf uses wgpu → Android Vulkan/GLES. alacritty_terminal
is pure Rust. nushell is pure Rust.

### 2.1 Android environment init

`bevy/assets/nu-config/android.nu` — new file, sourced after `env.nu` on Android only:

```nushell
# Android-specific environment setup
$env.PATH = ($env.PATH | prepend ["/system/bin" "/system/xbin" $"($env.HOME)/bin"])
$env.TERM = "xterm-256color"
```

`init_nushell_engine()` in `terminal.rs`:
- Detect Android via `#[cfg(target_os = "android")]`
- Set `$HOME` to `android_activity::AndroidApp::internal_data_path()` (passed in at startup via
  a resource)
- Source `android.nu` after config.nu

### 2.2 Soft keyboard

Bevy's `Window::ime_enabled = true` triggers Android soft keyboard. The terminal's `OnEnter`
system sets this. Existing `process_keyboard_input` handles `KeyboardInput` events from Android
IME unchanged — Bevy normalizes them.

Touch tap on terminal surface → `Window::ime_position` → soft keyboard appears. No extra code.

### Verification

Terminal world launches on Android, nushell prompt appears, commands like `ls /system/bin`,
`ps`, `curl https://...` work. toybox external commands work via `$PATH`.

---

## Phase 3 — `nu_plugin_android`

New workspace crate: `cyb/nu_plugin_android/`

### Architecture

```
nushell engine
  └─ registers plugin at startup (Android-only)
       └─ nu_plugin_android (Rust crate)
            └─ JNI bridge
                 └─ Android APIs (Java/Kotlin via JavaVM)
```

The plugin is not a separate process (standard nushell plugin protocol) — it's registered inline
as an `InProcessPlugin` to avoid the process-spawn overhead on mobile.

### Commands

| Command | Android API | Permission |
|---|---|---|
| `android gps [--watch]` | LocationManager / Fused Location | ACCESS_FINE_LOCATION |
| `android camera [--front]` | CameraX | CAMERA |
| `android sensors list` | SensorManager | none |
| `android sensors read <type>` | SensorManager | none |
| `android intent --action <a> --data <d>` | Intent | none |
| `android clipboard get` | ClipboardManager | none |
| `android clipboard set <text>` | ClipboardManager | none |
| `android notify --title <t> --body <b>` | NotificationManager | POST_NOTIFICATIONS |
| `android contacts` | ContentResolver | READ_CONTACTS |
| `android wifi` | WifiManager | ACCESS_WIFI_STATE |
| `android battery` | BatteryManager | none |

### Implementation notes

- `jni` crate (already in Android ecosystem) for JVM calls from Rust
- `android-activity` gives access to `JavaVM` — pass it to plugin at init
- Each command blocks on JVM call, returns structured `Value` (record/table)
- `android gps --watch` streams records via `PipelineData::ListStream`

### Registration

`terminal.rs` → `init_nushell_engine()`:

```rust
#[cfg(target_os = "android")]
{
    let plugin = nu_plugin_android::AndroidPlugin::new(jvm.clone());
    engine_state.add_plugin(Box::new(plugin));
}
```

### Verification

```nushell
android battery          # → {level: 87, charging: true, ...}
android gps              # → {lat: 55.75, lon: 37.61, accuracy: 4.2}
android sensors list     # → table of available sensors
android clipboard get    # → "text from clipboard"
```

---

## File change summary

| File | Change |
|---|---|
| `bevy/Cargo.toml` | android feature: swap tao/wry for bevy/android + nu-* deps |
| `bevy/src/lib.rs` | replace `pub mod android` with `android_main` fn |
| `bevy/src/android/` | delete entirely |
| `bevy/gen/android/AndroidManifest.xml` | GameActivity, add permissions |
| `bevy/gen/android/app/build.gradle` | GameActivity dep |
| `bevy/assets/nu-config/android.nu` | new: Android PATH/HOME setup |
| `bevy/src/worlds/terminal.rs` | add Android HOME init (cfg-gated, ~5 lines) |
| `cyb/nu_plugin_android/` | new crate |
| `cyb/Cargo.toml` | add nu_plugin_android to workspace |
| `cyb/CLAUDE.md` | one binary rule (done) |

---

## Effort estimate

| Phase | Effort |
|---|---|
| Phase 1 — Bevy entry point | 2 days |
| Phase 2 — Terminal on Android | 1 day |
| Phase 3 — nu_plugin_android | 4–5 days |
| **Total** | **7–8 days** |
