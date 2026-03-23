// Library target: only used on Android (cdylib for JNI).
// On desktop, the binary target (main.rs) is the entry point.

#[cfg(feature = "android")]
pub mod android;
