pub mod chroma;
pub mod clipboard;
pub mod nav;
pub mod platform;

#[cfg(not(cyb_phone))]
pub mod hotkeys;
#[cfg(not(cyb_phone))]
pub mod tray;
#[cfg(not(cyb_phone))]
pub mod window;
