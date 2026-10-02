//! Stamp the build so the chrome can say which cyb is running.
//!
//! `CYB_VERSION` = the RELEASE version first (`v0.12.1` — the number on
//! the GitHub release), then the short git hash (+ `*` when the tree is
//! dirty) and the build minute. Two questions, one chip: "which release
//! is this?" and "is the window I am looking at the build I just made?"

use std::process::Command;

fn main() {
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "dev".into());
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    // Build minute, UTC, from `date` — good enough to tell two builds apart.
    let stamp = Command::new("date")
        .args(["-u", "+%m.%d %H:%M"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let mark = if dirty { "*" } else { "" };
    // The release number comes from the manifest cargo is building — the
    // same one `make ship` bumps and tags, so the chip and the GitHub
    // release can never drift apart.
    let release = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    println!("cargo:rustc-env=CYB_VERSION=v{release}  {hash}{mark} {stamp}");
    // Phone chrome: one row. Keep the full x.y.z so local 0.15.8 / 0.15.9
    // read apart; drop the git hash (the neuron is the id) and the date.
    let time = stamp.split_whitespace().last().unwrap_or(stamp.as_str());
    println!("cargo:rustc-env=CYB_VERSION_SHORT=v{release}{mark} {time}");
    // Re-stamp whenever HEAD moves.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
}
