//! Clipboard read shared by every paste-capable surface.
//!
//! arboard opens the system pasteboard on demand — the handle is
//! created per call rather than cached, which sidesteps the
//! NSPasteboard-on-the-wrong-thread footgun on macOS. arboard has no
//! Android backend; there the read reports empty until a JNI-backed
//! clipboard lands with `nu_plugin_android` (android-support.md P3).

#[cfg(not(cyb_phone))]
pub fn read_clipboard() -> Result<String, arboard::Error> {
    let mut clip = arboard::Clipboard::new()?;
    clip.get_text()
}

#[cfg(cyb_phone)]
pub fn read_clipboard() -> Result<String, std::convert::Infallible> {
    Ok(String::new())
}

#[cfg(not(cyb_phone))]
pub fn write_clipboard(text: &str) -> Result<(), arboard::Error> {
    let mut clip = arboard::Clipboard::new()?;
    clip.set_text(text.to_string())
}

#[cfg(cyb_phone)]
pub fn write_clipboard(_text: &str) -> Result<(), std::convert::Infallible> {
    Ok(())
}
