//! cyb — one app, every platform.
//!
//! Desktop runs `main.rs`, Android enters through [`android_main`] below;
//! both build the identical Bevy app in [`app::build_app`]. The cdylib
//! target exists for the Android JNI load (`libcyb.so`).

pub mod agent;
pub mod app;
pub mod now;
pub mod shell;
pub mod worlds;

/// Android entry point. GameActivity loads `libcyb.so` and calls this;
/// bevy_winit picks the `AndroidApp` up from `bevy::android::ANDROID_APP`
/// and drives the same event loop winit runs on desktop.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: bevy::android::android_activity::AndroidApp) {
    // Android never sets HOME, and everything durable in cyb — the graph
    // log, the spell, the content store, the models — lives under
    // `~/cyb` and `~/llm`. Unset, those paths resolved to `/` and every
    // write silently failed: the cell ran ephemeral and the identity was
    // re-minted each launch. The app's internal data dir is the body's own
    // writable root; making it HOME makes every desktop path true here too.
    if let Some(data) = android_app.internal_data_path() {
        // SAFETY: first thing in the process, before any thread reads env.
        unsafe { std::env::set_var("HOME", &data) };
        let llm = data.join("llm");
        let _ = std::fs::create_dir_all(&llm);
        let _ = std::fs::create_dir_all(data.join("cyb"));
        // Public files/llm (adb push, share sheet) sits on FUSE. exists()
        // can be true while mmap — how .model loads — fails. Copy into
        // internal so soma actually reads the weights.
        if let Some(ext) = android_app.external_data_path() {
            ingest_llm_dir(&ext.join("llm"), &llm);
        }
        // A leftover pointer at a path we cannot open would pin soma on
        // a dead file and skip ~/llm. Drop it.
        let chosen = data.join("cyb").join("model");
        if let Ok(p) = std::fs::read_to_string(&chosen) {
            let p = std::path::PathBuf::from(p.trim());
            if std::fs::File::open(&p).is_err() {
                let _ = std::fs::remove_file(&chosen);
                eprintln!("cyb: dropped unreadable model pointer {}", p.display());
            }
        }
    }
    // GameActivity can call this again in the same process (new surface,
    // activity recreated). winit allows one EventLoop per process; parking
    // the new thread left a black splash forever because the first loop
    // had already lost the window. Die so Android starts a clean process.
    if bevy::android::ANDROID_APP.get().is_some() {
        eprintln!("cyb: android_main re-entry — exiting for a clean EventLoop");
        std::process::exit(0);
    }
    let _ = bevy::android::ANDROID_APP.set(android_app);
    app::build_app().run();
}

/// Copy `*.model` from `from` into `into` when the destination is missing
/// or a different size. Unreadable sources (adb-pushed FUSE files) are skipped.
#[cfg(target_os = "android")]
fn ingest_llm_dir(from: &std::path::Path, into: &std::path::Path) {
    let Ok(rd) = std::fs::read_dir(from) else {
        return;
    };
    for e in rd.flatten() {
        let src = e.path();
        if !src.extension().is_some_and(|x| x == "model") {
            continue;
        }
        let Some(name) = src.file_name() else {
            continue;
        };
        let dest = into.join(name);
        let src_len = e.metadata().map(|m| m.len()).unwrap_or(0);
        let dest_len = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        if dest_len == src_len && dest_len > 0 {
            continue;
        }
        match std::fs::copy(&src, &dest) {
            Ok(_) => eprintln!("cyb: ingested {} → {}", src.display(), dest.display()),
            Err(err) => eprintln!("cyb: skip {}: {err}", src.display()),
        }
    }
}
