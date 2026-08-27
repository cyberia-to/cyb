//! What the surrounding OS imposes on the app: safe margins and the IME.
//!
//! Desktop windows own their whole client area and carry a hardware keyboard,
//! so both are inert there. On Android the window runs edge to edge under the
//! status bar and the gesture pill, and text input needs the soft keyboard
//! asked for by name.

use bevy::prelude::*;

/// Screen margins the system reserves, in logical pixels. Chrome adds these to
/// its own padding so nothing it draws lands under a system bar.
#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub struct SafeArea {
    pub top: f32,
    pub bottom: f32,
}

/// Set by whoever wants text: the soft keyboard follows this each frame.
///
/// `text` is the IME's own buffer. GameActivity routes the soft keyboard
/// through GameTextInput rather than key events, so on Android this — not
/// `KeyboardInput` — is where typing arrives. Whoever wants the text reads it
/// and may clear it; setting `wanted` false hides the keyboard and resets it.
#[derive(Resource, Default)]
pub struct SoftInput {
    pub wanted: bool,
    pub text: String,
    shown: bool,
}

pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SafeArea>()
            .init_resource::<SoftInput>()
            .add_systems(Update, (track_safe_area, drive_soft_input));
    }
}

/// System bar insets in physical pixels, packed top<<16 | bottom, written by
/// `MainActivity`'s window-insets listener.
#[cfg(target_os = "android")]
static SYSTEM_INSETS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// How far the soft keyboard reaches up the screen, physical pixels.
#[cfg(target_os = "android")]
static IME_INSET: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Called from Kotlin on every insets change. Both values are physical pixels
/// and comfortably under 16 bits on any real display.
///
/// # Safety
/// Invoked by the JVM with the standard JNI prologue; the two pointers are
/// unused, and the payload is a pair of plain integers.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_ai_cyb_app_MainActivity_nativeSetInsets(
    _env: *mut core::ffi::c_void,
    _this: *mut core::ffi::c_void,
    top: i32,
    bottom: i32,
    ime: i32,
) {
    let packed = ((top.clamp(0, 0xffff) as u32) << 16) | (bottom.clamp(0, 0xffff) as u32);
    SYSTEM_INSETS.store(packed, std::sync::atomic::Ordering::Relaxed);
    IME_INSET.store(ime.max(0) as u32, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(target_os = "android")]
fn track_safe_area(mut safe: ResMut<SafeArea>, windows: Query<&Window>) {
    let Ok(window) = windows.single() else { return };
    let packed = SYSTEM_INSETS.load(std::sync::atomic::Ordering::Relaxed);
    let scale = window.scale_factor().max(1.0);
    // The keyboard is a bottom inset like any other: while it is up the
    // chrome rides above it instead of hiding underneath.
    let ime = IME_INSET.load(std::sync::atomic::Ordering::Relaxed);
    let next = SafeArea {
        top: (packed >> 16) as f32 / scale,
        bottom: (packed & 0xffff).max(ime) as f32 / scale,
    };
    if *safe != next {
        info!("platform: safe area top {:.0} bottom {:.0}", next.top, next.bottom);
        *safe = next;
    }
}

#[cfg(not(target_os = "android"))]
fn track_safe_area(_safe: ResMut<SafeArea>, _windows: Query<&Window>) {}

#[cfg(target_os = "android")]
fn drive_soft_input(mut input: ResMut<SoftInput>, mut hz_set: Local<bool>) {
    let Some(app) = bevy::android::ANDROID_APP.get() else { return };

    // Ask the compositor for the panel's fast mode once the surface exists.
    // Without this the adaptive display idles at 60 Hz and vsync caps the
    // app there no matter how cheap the frame is. Resolved via dlsym: the
    // symbol lives in libnativewindow.so (API 30+), which the API-24 link
    // sysroot does not carry.
    if !*hz_set {
        if let Some(window) = app.native_window() {
            unsafe {
                let lib = libc::dlopen(c"libnativewindow.so".as_ptr(), libc::RTLD_NOW);
                let sym = if lib.is_null() { std::ptr::null_mut() }
                          else { libc::dlsym(lib, c"ANativeWindow_setFrameRate".as_ptr()) };
                if sym.is_null() {
                    warn!("platform: ANativeWindow_setFrameRate unavailable");
                } else {
                    let set_rate: extern "C" fn(*mut core::ffi::c_void, f32, i8) -> i32 =
                        std::mem::transmute(sym);
                    let rc = set_rate(window.ptr().as_ptr().cast(), 120.0, 0);
                    info!("platform: requested 120 Hz (rc {rc})");
                }
            }
            *hz_set = true;
        }
    }

    if input.wanted != input.shown {
        if input.wanted {
            // Start from an empty IME buffer so the previous line does not
            // reappear under the cursor.
            app.set_text_input_state(android_activity::input::TextInputState {
                text: String::new(),
                selection: android_activity::input::TextSpan { start: 0, end: 0 },
                compose_region: None,
            });
            input.text.clear();
            app.show_soft_input(true);
        } else {
            app.hide_soft_input(false);
            input.text.clear();
        }
        input.shown = input.wanted;
    }

    if input.shown {
        let state = app.text_input_state();
        if state.text != input.text {
            debug!("platform: ime text {:?}", state.text);
            input.text = state.text;
        }
    }
}

#[cfg(not(target_os = "android"))]
fn drive_soft_input(mut input: ResMut<SoftInput>) {
    // Desktop has a keyboard already; keep the state honest so the commander
    // does not think it is waiting on one.
    input.shown = input.wanted;
}
