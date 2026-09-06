//! Stamp the build so the chrome can say which cyb is running.
//!
//! `CYB_VERSION` = short git hash (+ `*` when the tree is dirty) and the
//! build minute. The one question it answers is the one that kept coming
//! up: "is the window I am looking at the build I just made?"

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
    println!("cargo:rustc-env=CYB_VERSION={hash}{mark} {stamp}");
    // Re-stamp whenever HEAD moves.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
}
