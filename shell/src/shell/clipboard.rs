//! Clipboard read shared by every paste-capable surface.
//!
//! arboard opens the system pasteboard on demand — the handle is
//! created per call rather than cached, which sidesteps the
//! NSPasteboard-on-the-wrong-thread footgun on macOS.

pub fn read_clipboard() -> Result<String, arboard::Error> {
    let mut clip = arboard::Clipboard::new()?;
    clip.get_text()
}
