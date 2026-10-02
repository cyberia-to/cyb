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
            .add_systems(
                Update,
                (track_safe_area, sync_android_window, drive_soft_input),
            );
    }
}

/// System bar insets in physical pixels, packed top<<16 | bottom, written by
/// `MainActivity`'s window-insets listener.
#[cfg(target_os = "android")]
static SYSTEM_INSETS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// How far the soft keyboard reaches up the screen, physical pixels.
#[cfg(target_os = "android")]
static IME_INSET: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// `DisplayMetrics.density * 1000`. 0 until Kotlin has spoken.
#[cfg(target_os = "android")]
static DISPLAY_DENSITY_MILLI: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

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

/// # Safety
/// Same JNI prologue as [`Java_ai_cyb_app_MainActivity_nativeSetInsets`].
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_ai_cyb_app_MainActivity_nativeSetDensity(
    _env: *mut core::ffi::c_void,
    _this: *mut core::ffi::c_void,
    milli: i32,
) {
    DISPLAY_DENSITY_MILLI.store(milli.max(1000) as u32, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(target_os = "android")]
fn track_safe_area(mut safe: ResMut<SafeArea>, windows: Query<&Window>, input: Res<SoftInput>) {
    let Ok(window) = windows.single() else {
        return;
    };
    let packed = SYSTEM_INSETS.load(std::sync::atomic::Ordering::Relaxed);
    let scale = window.scale_factor().max(1.0);
    // IME only counts while the commander wants it. Counting it always made
    // the bottom inset twitch and the whole UI relayout every frame.
    let ime = if input.wanted {
        IME_INSET.load(std::sync::atomic::Ordering::Relaxed)
    } else {
        0
    };
    let next = SafeArea {
        top: ((packed >> 16) as f32 / scale).round(),
        bottom: ((packed & 0xffff).max(ime) as f32 / scale).round(),
    };
    if (safe.top - next.top).abs() >= 1.0 || (safe.bottom - next.bottom).abs() >= 1.0 {
        info!(
            "platform: safe area top {:.0} bottom {:.0}",
            next.top, next.bottom
        );
        *safe = next;
    }
}

#[cfg(not(target_os = "android"))]
fn track_safe_area(_safe: ResMut<SafeArea>, _windows: Query<&Window>) {}

/// GameActivity starts at the desktop WindowPlugin size (1280×800). Until the
/// Bevy window matches the real surface, layout is a 760px column in a
/// landscape box, touch hits miss the tab strip, and resize fights flicker.
#[cfg(target_os = "android")]
fn sync_android_window(mut windows: Query<&mut Window>) {
    let Some(app) = bevy::android::ANDROID_APP.get() else {
        return;
    };
    let Some(nw) = app.native_window() else {
        return;
    };
    let w = nw.width() as u32;
    let h = nw.height() as u32;
    if w < 2 || h < 2 {
        return;
    }
    let milli = DISPLAY_DENSITY_MILLI.load(std::sync::atomic::Ordering::Relaxed);
    let density = if milli >= 1000 {
        milli as f32 / 1000.0
    } else {
        (w as f32 / 412.0).clamp(1.0, 4.0)
    };
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let res = &mut window.resolution;
    let (cw, ch) = (res.physical_width(), res.physical_height());
    // Native window size wobbles by a pixel on some frames; chasing it
    // rebuilds the swapchain and the graph image — the whole screen flashes.
    if cw.abs_diff(w) > 2 || ch.abs_diff(h) > 2 {
        info!("platform: surface {w}x{h} density {density:.2}");
        res.set_physical_resolution(w, h);
    }
    let current = res.scale_factor_override().unwrap_or(0.0);
    if (current - density).abs() > 0.05 {
        info!(
            "platform: scale {density:.2} logical {:.0}x{:.0}",
            w as f32 / density,
            h as f32 / density
        );
        res.set_scale_factor_override(Some(density));
    }
}

#[cfg(not(target_os = "android"))]
fn sync_android_window(_windows: Query<&mut Window>) {}

#[cfg(target_os = "android")]
fn drive_soft_input(mut input: ResMut<SoftInput>, mut hz_set: Local<bool>) {
    let Some(app) = bevy::android::ANDROID_APP.get() else {
        return;
    };

    // Ask the compositor for the panel's fast mode once the surface exists.
    // Without this the adaptive display idles at 60 Hz and vsync caps the
    // app there no matter how cheap the frame is. Resolved via dlsym: the
    // symbol lives in libnativewindow.so (API 30+), which the API-24 link
    // sysroot does not carry.
    if !*hz_set {
        if let Some(window) = app.native_window() {
            unsafe {
                let lib = libc::dlopen(c"libnativewindow.so".as_ptr(), libc::RTLD_NOW);
                let sym = if lib.is_null() {
                    std::ptr::null_mut()
                } else {
                    libc::dlsym(lib, c"ANativeWindow_setFrameRate".as_ptr())
                };
                if sym.is_null() {
                    warn!("platform: ANativeWindow_setFrameRate unavailable");
                } else {
                    let set_rate: extern "C" fn(*mut core::ffi::c_void, f32, i8) -> i32 =
                        std::mem::transmute(sym);
                    // Paint+UI is ~32 fps on this Pixel. Asking 120 made the
                    // LTPO panel hunt and the whole screen strobe. 60 with
                    // FIXED_SOURCE (1) locks the rate to what we can fill.
                    let rc = set_rate(window.ptr().as_ptr().cast(), 60.0, 1);
                    info!("platform: requested 60 Hz fixed (rc {rc})");
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
